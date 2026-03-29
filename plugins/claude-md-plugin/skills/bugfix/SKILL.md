---
name: bugfix
version: 3.0.0
aliases: [diagnose, troubleshoot, fix-bug]
description: |
  This skill should be used when the user asks to "bugfix code", "fix a bug", "diagnose an error",
  "trace a test failure", "find root cause", or uses "/bugfix".
  Traces root cause through 3 layers: CLAUDE.md (requirements), DEVELOPERS.md (context), Source Code.
  Document-First: identifies which SSOT document gap caused the bug → fixes document → regenerates code.
  Trigger keywords: 버그 진단, 버그 수정, 에러 추적, 테스트 실패, 런타임 에러
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# /bugfix

소스코드의 런타임 버그/에러를 진단.
근본 원인을 CLAUDE.md(요구사항), DEVELOPERS.md(맥락, optional), Source Code 3계층으로 추적.
**Document-First**: SSOT 문서의 어떤 갭이 이 버그를 초래했는지 파악 → 문서 보강 → 코드 재생성.

## Triggers

- `/bugfix`
- `버그 진단`, `버그 수정`
- `에러 추적`, `테스트 실패`
- `런타임 에러`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 버그 수정 대상 경로 |
| `--error` | 아니오 | (없음) | 에러 메시지, 스택 트레이스, 또는 기능 설명 |
| `--test` | 아니오 | (없음) | 실패하는 테스트 이름/파일 |

## Workflow — 문서 → 스펙(세션 파일) → 코드

### 1. Context Intake

`--error`/`--test` 인자 확인. 없으면 AskUserQuestion으로 에러 정보 수집.

### 2. 입력 타입 분류

| 타입 | 판별 기준 |
|------|----------|
| **Type A (기술적 에러)** | 에러 클래스명, 스택 트레이스 포함 |
| **Type B (테스트 실패)** | `--test` 인자 또는 테스트 파일명 포함 |
| **Type C (기능 설명)** | 에러 클래스 없음, 기술 용어 부족 |

Type C: `references/bugfix-workflow.md` Step 3 참조 (CLAUDE.md 인덱스 → 모듈 매칭 → 테스트 탐색).

### 3. 대상 식별

`path`에서 CLAUDE.md + DEVELOPERS.md 존재 확인:

| 상태 | 진단 범위 |
|------|----------|
| 둘 다 있음 | 3-layer 진단 (L1+L2+L3) |
| CLAUDE.md만 | L1+L3 진단 (L2 스킵) |
| 없음 | `/decompile` 먼저 실행 제안 |

### 4. 사전 검증 (CLI) — 리스크 레벨 분류

`references/bugfix-workflow.md` Step 5 참조.
스키마 검증 + 미컴파일 변경 확인 → NONE/LOW/MEDIUM/HIGH 리스크 분류.

### 5. 분석 세션 파일 생성

CLAUDE.md + DEVELOPERS.md에서 에러 관련 Requirements/Constraints만 추출하여 분석 세션 파일 생성:

```markdown
# Bugfix Analysis: {path}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Error
{error_message}
test: {test_name_or_none}

## Requirements (from CLAUDE.md)
{에러 관련 Requirements만 추출}

## Constraints (from DEVELOPERS.md)
{에러 관련 Constraints만 추출}

## Pre-validation
schema: PASS | FAIL
uncompiled_changes: NONE | DETECTED
risk_level: NONE | LOW | MEDIUM | HIGH
```

→ `${TMP_DIR}bugfix-analysis-{dir}.md`

### 6. 진단 — Task(debugger)

분석 세션 파일을 debugger agent에 전달:

```
Task(debugger):
  세션 파일: ${TMP_DIR}bugfix-analysis-{dir}.md
  대상 디렉토리: {path}
  결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

debugger가 진단:
- SSOT 문서의 **어떤 갭**이 이 버그를 초래했는지 파악
- 결과: root_cause + document_gap

```
---debugger-result---
result_file: ${TMP_DIR}debug-{dir-safe-name}.md
status: success | failed
root_cause_layer: L1 | L2 | L3 | MULTI
root_cause_type: SPEC_REQUIREMENTS_GAP | SPEC_REQUIREMENTS_MISMATCH | CONTEXT_CONSTRAINT_GAP | CODE_LOGIC_ERROR | ...
document_gap: {어떤 문서의 어떤 섹션이 누락/부정확}
summary: <한 줄>
compile_path: {dir}
compile_required: true | false
test_command: {command} | N/A
---end-debugger-result---
```

### 7. 문서 보강

debugger 결과에 따라 분기:

| Root Cause | 처리 |
|-----------|------|
| **L1**: Requirements 갭 | CLAUDE.md Requirements 보강 (사용자 승인 필수) |
| **L2**: Constraints 갭 | DEVELOPERS.md Constraints 보강 |
| **L3 + 문서 갭**: 미명시 제약 위반 | DEVELOPERS.md Constraints에 위반 제약 추가 |
| **L3 + 문서 정확**: 코드 생성 오류 | 문서 보강 없이 Step 8로 (compile 힌트만 추가) |

문서 수정 후 Diff 표시:
```bash
git diff HEAD -- {path}/CLAUDE.md {path}/DEVELOPERS.md
```

### 8. compile 세션 파일 생성 → /compile

보강된 문서에서 compile 세션 파일 생성 → /compile 실행:

```
Skill("claude-md-plugin:compile", args: "--path {compile_path} --conflict overwrite")
```

`compile_required: false`이면 스킵.

### 9. 검증

compile 성공 후 원본 테스트 재실행:

| 조건 | 동작 |
|------|------|
| `test_command` 있음 | 해당 테스트 실행 |
| `test_command: N/A` + Type B | Step 1에서 수집한 테스트로 실행 |
| 그 외 | compile 자체 테스트로 갈음 |

### 10. 결과 반환

---bugfix-result---
status: success | partial | failed
root_cause: {layer} - {type}
document_gap: {어떤 문서의 어떤 섹션이 누락/부정확했는지}
summary: {one-line}
documents_fixed: [{files}]
compile: PASS | FAIL | SKIPPED
verification: PASS | FAIL | SKIPPED
test_command: {command} | N/A
reproduction: REPRODUCED | STATIC_ANALYSIS_ONLY | N/A
detail_file: {result_file}
---end-bugfix-result---

## DO / DON'T

**DO:**
- 버그를 **문서 갭**으로 귀결: "SSOT에 어떤 스펙이 빠져서 이 버그가 발생했는가?"
- L1/L2 root cause → 문서 보강 → /compile 재생성
- L3 root cause + 문서 정확 → /compile 재실행 (문서 보강 없이)
- L1 root cause (Requirements 변경) → 사용자 승인 필수
- 진단 결과를 ${TMP_DIR} 파일로 저장

**DON'T:**
- 소스코드 직접 수정 (코드는 /compile로만 재생성)
- 사용자 승인 없이 CLAUDE.md Requirements 수정
- 전체 소스 디렉토리 읽기 (에러 위치 중심 타깃 분석)

## 참조 자료

- `references/debugger-templates.md`: Root cause types, fix strategies, stack trace patterns
- `references/bugfix-workflow.md`: Type C 모듈 탐색 (Step 3) + 사전 검증 리스크 분류 (Step 5)

## Examples

<example>
<context>
에러 메시지로 버그 수정 요청. L1 root cause — Requirements에 standalone function 미명시.
</context>
<user_request>/bugfix --error "TypeError: validateToken is not a function" --path src/auth</user_request>
<assistant_response>
사전 검증: 스키마 PASS, 미컴파일 변경 NONE

분석 세션 파일 생성... 3-layer 진단 실행...

Document Gap: CLAUDE.md Requirements에 "validateToken은 standalone function"이 미명시

CLAUDE.md 보강: Requirements에 "validateToken은 standalone function으로 export" 추가
DEVELOPERS.md 보강: Constraints에 "validateToken must be exported as named function" 추가

/compile 실행 중...

---bugfix-result---
status: success
root_cause: L1 - SPEC_REQUIREMENTS_MISMATCH
document_gap: CLAUDE.md Requirements — validateToken의 export 형태 미명시
summary: validateToken을 standalone function으로 명시하여 해소
documents_fixed: [CLAUDE.md, DEVELOPERS.md]
compile: PASS
verification: PASS
test_command: npx jest src/auth --no-coverage
detail_file: .claude/tmp/debug-src-auth.md
---end-bugfix-result---
</assistant_response>
</example>

<example>
<context>
테스트 실패로 요청. L3 root cause — 문서는 정확, 코드 생성 오류.
</context>
<user_request>/bugfix --test "should return empty array for no results"</user_request>
<assistant_response>
테스트 실행 → 에러 캡처

사전 검증: 스키마 PASS, 미컴파일 변경 NONE

분석 세션 파일 생성... 3-layer 진단 실행...

Document Gap: 없음 (문서 정확, 코드가 Requirements를 미준수)

/compile 재실행 중... (문서 보강 없이 코드 재생성)

---bugfix-result---
status: success
root_cause: L3 - CODE_SPEC_DIVERGENCE
document_gap: none (문서 정확, 코드 생성 오류)
summary: Code returns null instead of empty array — /compile 재생성으로 해소
documents_fixed: []
compile: PASS
verification: PASS
test_command: npx jest --testNamePattern "should return empty array"
detail_file: .claude/tmp/debug-src-utils.md
---end-bugfix-result---
</assistant_response>
</example>

<example>
<context>
기능 설명으로 요청. L1 root cause — Requirements에 기능 누락.
</context>
<user_request>/bugfix --error "로그인 시 토큰 만료되면 자동 갱신이 안 됩니다"</user_request>
<assistant_response>
모듈 탐색: src/auth (JWT 토큰 인증), src/session (세션 관리)
관련 테스트 실행: src/auth 12 passed, 1 failed

분석 세션 파일 생성... 3-layer 진단 실행...

Document Gap: CLAUDE.md Requirements에 "토큰 만료 시 자동 갱신" 기능 미정의

CLAUDE.md 보강: Requirements에 "만료된 토큰은 자동으로 갱신한다" 추가 (사용자 승인)
DEVELOPERS.md 보강: Constraints에 "refreshToken when token.expiresAt <= now" 추가

/compile 실행 중...

---bugfix-result---
status: success
root_cause: L1 - SPEC_REQUIREMENTS_GAP
document_gap: CLAUDE.md Requirements — token refresh on expiry 미정의
summary: Token refresh 요구사항 추가 후 코드 재생성
documents_fixed: [CLAUDE.md, DEVELOPERS.md]
compile: PASS
verification: PASS
test_command: npx jest --testNamePattern "should refresh token on expiry"
detail_file: .claude/tmp/debug-src-auth.md
---end-bugfix-result---
</assistant_response>
</example>
