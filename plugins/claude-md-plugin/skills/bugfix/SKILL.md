---
name: bugfix
version: 1.0.0
aliases: [diagnose, troubleshoot, fix-bug]
description: |
  This skill should be used when the user asks to "bugfix code", "fix a bug", "diagnose an error",
  "trace a test failure", "find root cause", or uses "/bugfix".
  Traces root cause through CLAUDE.md (spec), IMPLEMENTS.md (plan), and Source Code layers.
  Trigger keywords: 디버그, 버그 수정, 에러 추적, 테스트 실패, 런타임 에러
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, AskUserQuestion]
---

# /bugfix

compile 결과물(소스코드)의 런타임 버그/에러를 진단.
근본 원인을 CLAUDE.md(스펙), IMPLEMENTS.md(플랜), Source Code 3계층으로 추적하여 적절한 레벨에서 수정.

## Triggers

- `/bugfix`
- `디버그`, `버그 수정`
- `에러 추적`, `테스트 실패`
- `런타임 에러`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 버그 수정 대상 경로 |
| `--error` | 아니오 | (없음) | 에러 메시지, 스택 트레이스, 또는 기능 설명 |
| `--test` | 아니오 | (없음) | 실패하는 테스트 이름/파일 |

## Workflow

### 1. Bug Report 수집

`--error`/`--test` 인자가 없으면 AskUserQuestion으로 에러 정보 수집:

```
AskUserQuestion: "버그 수정할 에러 정보를 알려주세요."
옵션: [에러 메시지/스택 트레이스 붙여넣기, 실패하는 테스트 이름, 잘못 동작하는 기능 설명]
```

### 2. 입력 타입 분류

에러 정보를 3가지 타입으로 분류:

| 타입 | 판별 기준 | 초기 대응 |
|------|----------|----------|
| **Type A (기술적 에러)** | 에러 클래스명, 스택 트레이스 포함 | 에러 위치에서 직접 L3 탐색 시작 |
| **Type B (테스트 실패)** | `--test` 인자 또는 테스트 파일명 포함 | 테스트 실행 → 에러 캡처 → L3 탐색 |
| **Type C (기능 설명)** | 에러 클래스 없음, 기술 용어 부족 | 모듈 탐색 먼저 → 관련 테스트 찾기 |

### 3. (Type C 전용) 모듈 탐색

기능 설명이 입력된 경우, 기술적 진단 대상을 먼저 특정:

**3.1. 전체 CLAUDE.md 인덱스 생성:**
```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    (cd "$CORE_DIR" && cargo build --release)
fi

TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}debug-scan-index.json"
```

**3.2. 의미적 매칭:** 기능 설명의 키워드와 각 모듈의 `purpose` + `export_names` 매칭.

**3.3. 매칭된 모듈(최대 3개)에서 관련 테스트 탐색:**
```
Glob: {matched_dir}/**/*test*  또는  {matched_dir}/**/*spec*
```

**3.4. 관련 테스트 실행 → 실패 시 Type A/B 흐름 합류.**

**3.5. 전체 통과 → Behavior 교차 분석 (CLAUDE.md Behavior vs 실제 동작 비교).**

### 4. 대상 식별

`path`에서 CLAUDE.md + IMPLEMENTS.md 존재 확인:

| 상태 | 진단 범위 |
|------|----------|
| 둘 다 있음 | 3-layer 진단 (L1+L2+L3) |
| CLAUDE.md만 | L1+L3 진단 (L2 스킵, IMPLEMENTS.md 생성 권장) |
| 없음 | `/decompile` 먼저 실행하여 CLAUDE.md 생성 제안 |

### 5. 사전 검증 (CLI)

CLAUDE.md가 존재할 때만 실행:

```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    (cd "$CORE_DIR" && cargo build --release)
fi
```

**5.1. 스키마 검증:**
```bash
$CLI_PATH validate-schema --file {claude_md_path}
```
- `valid: false` → "CLAUDE.md 스키마 오류 발견. `/validate` 먼저 실행하거나 스키마를 수정하세요" 안내

**5.2. 미컴파일 변경 확인:**
```bash
$CLI_PATH diff-compile-targets --root {project_root}
```
- 대상 모듈이 `targets`에 포함 → "CLAUDE.md가 소스 코드보다 최신입니다. `/compile --path {path}`로 먼저 재컴파일하세요" 안내
- 사용자가 그래도 진행 원하면 계속

### 6. 진단 (debugger agent)

```
Task(debugger):
  대상 디렉토리: {path}
  CLAUDE.md: {claude_md_path}
  IMPLEMENTS.md: {implements_md_path}
  에러 정보: {error_message}
  테스트: {test_name_or_none}
  스키마 검증: PASS | FAIL ({errors})
  미컴파일 변경: NONE | DETECTED ({reason})
  결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

debugger agent가 구조화된 블록으로 결과 반환:

```
---debugger-result---
result_file: ${TMP_DIR}debug-{dir-safe-name}.md
status: success | failed
root_cause_layer: L1 | L2 | L3 | MULTI
root_cause_type: SPEC_BEHAVIOR_GAP | PLAN_ERROR_HANDLING_GAP | CODE_LOGIC_ERROR | ...
summary: <한 줄 근본 원인 설명>
fix_targets: [CLAUDE.md, IMPLEMENTS.md]
compile_path: {dir}
---end-debugger-result---
```

### 7. 결과 보고

debugger agent 결과를 사용자에게 보고:

**보고 형식:**
```
/bugfix 결과
=========

Root Cause: {root_cause_layer} - {root_cause_type}
요약: {summary}

수정된 문서: {fix_targets}

⚠ `/compile --path {compile_path} --conflict overwrite`로 소스 코드를 재생성하세요.

상세 결과: {result_file}
```

## DO / DON'T

**DO:**
- 근본 원인을 가능한 높은 계층(L1 > L2 > L3)으로 추적
- Fix는 항상 CLAUDE.md / IMPLEMENTS.md에서 수행 (소스코드는 "바이너리")
- Fix 전 사용자 승인 (AskUserQuestion)
- 수정 후 `/compile --path <dir> --conflict overwrite` 권장

**DON'T:**
- 소스코드 직접 수정 (항상 문서 수정 → `/compile`로 재생성)
- `/compile` 자동 실행 (사용자 판단)
- 사용자 승인 없이 CLAUDE.md/IMPLEMENTS.md 수정
- 전체 소스 디렉토리 읽기 (에러 위치 중심 타깃 분석)
- 상세 진단을 context에 반환 (`${TMP_DIR}` 파일 사용)

## 참조 자료

- `references/debugger-templates.md`: Root cause types, fix strategies, stack trace patterns, CLI guide, result template (debugger agent가 런타임에 `cat`으로 로드)

## 관련 컴포넌트

- `agents/debugger.md`: 3-layer 진단 및 수정 agent

## Examples

<example>
<context>
사용자가 에러 메시지와 함께 디버그를 요청합니다.
</context>
<user_request>/bugfix --error "TypeError: validateToken is not a function" --path src/auth</user_request>
<assistant_response>
src/auth에서 에러를 진단합니다...

사전 검증:
  스키마: PASS
  미컴파일 변경: NONE

3-layer 진단을 실행합니다...

/bugfix 결과
=========

Root Cause: L1 - SPEC_EXPORT_MISMATCH
요약: CLAUDE.md exports validateToken as standalone but code defines it as class method

수정된 문서: [CLAUDE.md]

⚠ `/compile --path src/auth --conflict overwrite`로 소스 코드를 재생성하세요.

상세 결과: .claude/tmp/debug-src-auth.md
</assistant_response>
</example>

<example>
<context>
사용자가 실패하는 테스트로 디버그를 요청합니다.
</context>
<user_request>/bugfix --test "should return empty array for no results"</user_request>
<assistant_response>
테스트를 실행하고 에러를 캡처합니다...

사전 검증:
  스키마: PASS
  미컴파일 변경: NONE

3-layer 진단을 실행합니다...

/bugfix 결과
=========

Root Cause: L3 - CODE_SPEC_DIVERGENCE
요약: Code returns null instead of empty array as specified in CLAUDE.md Behavior

수정된 문서: [CLAUDE.md, IMPLEMENTS.md] (스펙/플랜 정확 — 재컴파일로 해결)

⚠ `/compile --path src/utils --conflict overwrite`로 소스 코드를 재생성하세요.

상세 결과: .claude/tmp/debug-src-utils.md
</assistant_response>
</example>

<example>
<context>
사용자가 기능 설명으로 디버그를 요청합니다.
</context>
<user_request>/bugfix --error "로그인 시 토큰 만료되면 자동 갱신이 안 됩니다"</user_request>
<assistant_response>
기능 설명에서 관련 모듈을 탐색합니다...

관련 모듈 발견:
  1. src/auth (purpose: JWT 토큰 인증)
  2. src/session (purpose: 세션 관리)

관련 테스트 실행 중...
  src/auth: 12 passed, 1 failed
  → 실패 테스트 발견: "should refresh token on expiry"

3-layer 진단을 실행합니다...

/bugfix 결과
=========

Root Cause: MULTI - PLAN_ERROR_HANDLING_GAP + SPEC_BEHAVIOR_GAP
요약: Token refresh on expiry not specified in CLAUDE.md Behavior, not handled in IMPLEMENTS.md

수정된 문서: [CLAUDE.md, IMPLEMENTS.md]

⚠ `/compile --path src/auth --conflict overwrite`로 소스 코드를 재생성하세요.

상세 결과: .claude/tmp/debug-src-auth.md
</assistant_response>
</example>
