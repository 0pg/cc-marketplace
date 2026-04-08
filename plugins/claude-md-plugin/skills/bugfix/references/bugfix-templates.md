# Bugfix Templates

## Bugfix Session File Format

```markdown
# Bugfix Session
type: bugfix | path: {path}

## Bug Description
expected: {E — what user expects}
actual: {A — current behavior}
                               ← structured format for agent clarity; spec intent is "user description — expected vs actual"

## Error Message
{stack trace or error output, if provided — "none" if absent}

## Target File
{specific file path, if --file was provided — "none" if absent}

## Layer 1: Requirements (CLAUDE.md)
path: {selected CLAUDE.md path}

Purpose: {parsed purpose}
Requirements:
- REQ-N: {text}
Domain Context: {parsed domain context}

## Layer 2: Constraints (DEVELOPERS.md)
path: {DEVELOPERS.md path, "none" if absent}
agent_observations: {DEVELOPERS.md path}#Agent Observations

Constraints:
- CONST-N: {text}
Technical Context: {parsed technical context}

## Layer 3: Source Files
language: {detected language}
files:
- {file path}: {content or "listing only" if content omitted}

## Node History
has_history: {true|false}
source_changed: {true|false}
commits_included: {N} | total_found: {M}
{for each commit in node-history JSON:}
### {short_hash} — {subject}
timestamp: {timestamp} | breaking: {true|false}
{for each file_diff:}
**{file_type} — {section}**: {changes summary}
{end for}
{end for}
source_changed_files: {list or "none"}

## Conventions
{hierarchy-resolved Conventions from project root — Module Boundaries and Project Structure sections}
```

## Escalation Format

When judgment is ambiguous, the SKILL presents this format via AskUserQuestion:

```
판단이 필요합니다.

## 현재 상황
- 사용자 기대 (E): "{expected}"
- 현재 동작 (A): "{actual}"
- CLAUDE.md REQ-N: "{spec text}"
  (또는: "이 동작에 대한 Requirement 없음")

## 판단 근거가 모호한 이유
"{구체적 이유}"

## 선택지
A) 스펙과 코드 모두 E에 맞게 수정한다
   → 실행 순서: CLAUDE.md REQ-N 먼저 수정 → spec commit → /dev로 코드 재생성
   (Fix-Highest-Layer-First: 코드는 SSOT 수정 이후 derived됨)
B) 스펙을 수정한다 (E를 요구사항으로 추가/변경)
   → CLAUDE.md에 신규 Requirement 추가 → spec commit → /dev 재생성
C) 현재 동작(A)이 올바름 (버그 아님)
   → 버그 리포트 종료

어떻게 처리할까요?
```

## Result Block Format

Returned by bugfixer agent to the SKILL:

```
---bugfix-result---
status: fixed | escalated | not_a_bug | failed
root_cause_layer: 1 | 2 | 3 | multi | unknown
judgment: unambiguous | ambiguous
fix_type: spec_update | constraints_update | code_fix | none
fix_description: {what was fixed or what the issue is}
test_result: passed | skipped | failed   ← (Layer 3 only; skipped for L1/L2)
                               ← optional fields below: agent passes these to SKILL for user interaction
[escalation:                   ← populated when judgment==ambiguous
  expected: {E}
  actual: {A}
  spec: {S text or "none"}
  reason: {why ambiguous}
  choices: [A, B, C]]
[proposed_change: {text}]      ← populated when L1/L2 fix is proposed (agent's fix suggestion)
---end-bugfix-result---
```

## diff-node-history Field Mapping

| CLI field | Meaning in Judgment Algorithm |
|-----------|-------------------------------|
| `has_history=true` + commits have section changes | spec 변경됨 (recent history exists) |
| `source_changed=true` + no section changes in commits | 소스 변경, spec 변경 없음 → 코드 이탈 |
| `has_history=false` | git 미사용 또는 해당 노드 변경 이력 없음 → git 증거 불충분 |
| `source_changed=false` + `has_history=false` | 최근 변경 없음 → git 증거 불충분 |
