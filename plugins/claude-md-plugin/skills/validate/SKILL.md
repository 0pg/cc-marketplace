---
name: validate
version: 1.2.0
aliases: [check, verify, lint]
trigger:
  - /validate
  - 문서 검증
  - check documentation
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify spec matches implementation", or uses "/validate". Runs drift-validator, export-validator,
  and convention-review in parallel.
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill]
---

# /validate

<example>
<context>
사용자가 전체 프로젝트의 CLAUDE.md를 검증하려고 합니다.
</context>
<user>/validate</user>
<assistant_response>
CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 개선 권장: 1개
- 개선 필요: 1개
</assistant_response>
</example>

<example>
<context>
사용자가 특정 디렉토리의 CLAUDE.md만 검증하려고 합니다.
</context>
<user>/validate src/</user>
<assistant_response>
CLAUDE.md 검증 보고서
=====================

상세 결과
---------
src/auth (양호)
  Drift: 0개 이슈
  Export 커버리지: 100% (19/19 예측 성공)
</assistant_response>
</example>

CLAUDE.md 문서의 품질과 코드 일치 여부를 검증합니다.

## Triggers

- `/validate`
- `CLAUDE.md 검증`
- `문서 검증`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 검증 대상 경로 (디렉토리 또는 파일) |

## Workflow

### 0. Completeness 검증 (audit)

CLI로 CLAUDE.md 완성도를 먼저 검사합니다:

```bash
./plugins/claude-md-plugin/core/target/release/claude-md-core audit \
  --root {path} \
  --only-issues \
  --output .claude/tmp/{session-id}-audit-result.json
```

**결과 상태:**
| 상태 | 의미 |
|------|------|
| `missing` | CON-1 충족하지만 CLAUDE.md 없음 → **생성 필요** |
| `unexpected` | CON-1 미충족인데 CLAUDE.md 있음 → 삭제 검토 |

### 1. 대상 수집

Glob으로 대상 경로의 모든 CLAUDE.md 수집:

```
Glob("**/CLAUDE.md", path={path})
```

### 1.5. code-convention.md 확인

프로젝트 루트에서 `code-convention.md`를 확인합니다:

```
convention_exists = Read({project_root}/code-convention.md)가 성공하는지 여부
```

### 2. 병렬 검증 실행

각 CLAUDE.md에 대해 validator들을 **단일 메시지에서 병렬로 Task 호출**:

```
# 반드시 단일 메시지에서 모든 Task를 호출하여 병렬 실행
For each claude_md_file:
  directory = dirname(claude_md_file)

  Task(drift-validator, prompt="검증 대상: {directory}")
  Task(export-validator, prompt="검증 대상: {directory}")
```

**code-convention.md가 존재하면** convention-review도 병렬로 실행:

```
if convention_exists:
  For each claude_md_file:
    directory = dirname(claude_md_file)
    Task(code-reviewer, prompt="
      검증 대상: {directory}
      code-convention.md 경로: {project_root}/code-convention.md
      결과는 .claude/tmp/{session-id}-convention-review-{target}.json 형태로 저장하고 경로만 반환
    ")
```

**code-convention.md가 없으면** convention-review를 건너뛰고 보고서에 안내 메시지를 포함합니다.

**중요**: 성능 최적화를 위해 모든 Task를 하나의 응답에서 호출해야 합니다.

### 3. 결과 수집

각 validator는 구조화된 블록으로 결과를 반환합니다:

```
---drift-validator-result---
status: success | failed
result_file: .claude/tmp/{session-id}-drift-{dir-safe-name}.md
directory: {directory}
issues_count: {N}
---end-drift-validator-result---
```

```
---export-validator-result---
status: success | failed
result_file: .claude/tmp/{session-id}-export-{dir-safe-name}.md
directory: {directory}
export_coverage: {0-100}
---end-export-validator-result---
```

```
---code-reviewer-result---
status: passed | fixed | warning
result_file: .claude/tmp/{session-id}-convention-review-{dir-safe-name}.json
directory: {directory}
convention_score: {0-100}
violations_count: {N}
auto_fixed_count: {N}
---end-code-reviewer-result---
```

### 4. 통합 보고서 생성

결과 파일들을 Read하여 다음 형식으로 통합 보고서 생성:

```markdown
# CLAUDE.md 검증 보고서

## Completeness (CLAUDE.md 완성도)

| 상태 | 개수 |
|------|------|
| Complete | 5 |
| Missing | 2 |
| Unexpected | 1 |

**Missing (생성 필요):**
- src/utils (소스 3개)
- src/api (소스 5개)

**Unexpected (삭제 검토):**
- docs/examples (소스 0개, 하위 1개)

## 요약

| 디렉토리 | Drift 이슈 | Export 커버리지 점수 | Convention | 상태 |
|----------|-----------|------------|------------|------|
| src/auth | 0 | 100% | 95% | 양호 |
| src/utils | 2 | 85% | 88% | 개선 필요 |

> Convention 열은 code-convention.md가 존재할 때만 표시됩니다.
> code-convention.md가 없으면 `/project-setup`을 실행하여 생성할 수 있습니다.

## 상세 결과

### src/auth
#### Drift 검증
(drift-validator 결과 파일 내용)

#### Export 커버리지 검증
(export-validator 결과 파일 내용)

#### Convention 검증
(code-reviewer 결과 파일 내용)

### src/utils
...
```

### 5. 임시 파일 정리

`.claude/tmp/{session-id}-{prefix}-{target}` 형태의 임시 파일들은 세션 종료 시 자동으로 정리됩니다.

## 성공 기준

| 상태 | 조건 |
|------|------|
| **양호** | Drift 이슈 0개 AND Export 커버리지 점수 100% AND Convention 90% 이상 |
| **개선 권장** | Drift 1-2개 OR Export 커버리지 점수 90-99% OR Convention 80-89% |
| **개선 필요** | Drift 3개 이상 OR Export 커버리지 점수 90% 미만 OR Convention 80% 미만 |

| code-convention.md | 판정 로직 |
|--------------------|----------|
| 있음 | Drift + Export + Convention 종합 판정 |
| 없음 | Drift + Export만으로 판정, Convention 열 생략 |

## 출력 예시

```
/validate src/

CLAUDE.md 검증 보고서
=====================

Completeness (CLAUDE.md 완성도)
-------------------------------
Complete: 3개 | Missing: 1개 | Unexpected: 0개

⚠ Missing (생성 필요):
  - src/api (소스 5개)

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 개선 권장: 1개
- 개선 필요: 1개

상세 결과
---------

src/auth (양호)
  Drift: 0개 이슈
  Export 커버리지: 100% (19/19 예측 성공)
  Convention: 95% (위반 1건 - auto-fixed)

src/utils (개선 권장)
  Drift: 2개 이슈
    - STALE: formatDate export가 코드에 없음
    - MISSING: parseNumber export가 문서에 없음
  Export 커버리지: 95% (17/18 예측 성공)
  Convention: 88% (위반 3건)
    - helper.ts:15 - "user_id" → "userId" (camelCase)
    - helper.ts:23 - inconsistent import order
    - types.ts:8 - double quotes → single quotes

src/legacy (개선 필요)
  Drift: 5개 이슈
    - UNCOVERED: 3개 파일이 Structure에 없음
    - MISMATCH: 2개 시그니처 불일치
  Export 커버리지: 78% (14/18 예측 성공)
  Convention: 72% (위반 8건)
```

## 관련 컴포넌트

- `core/target/release/claude-md-core audit`: CLAUDE.md 완성도 검증 (CLI)
- `agents/drift-validator.md`: 코드-문서 일치 검증
- `agents/export-validator.md`: Export 커버리지 검증
- `agents/code-reviewer.md`: 코드 품질 + 컨벤션 검증 (code-convention.md 기반)
