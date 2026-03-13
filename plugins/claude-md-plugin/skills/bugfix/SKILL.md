---
name: bugfix
version: 1.0.0
aliases: [diagnose, troubleshoot, fix-bug]
description: |
  This skill should be used when the user asks to "bugfix code", "fix a bug", "diagnose an error",
  "trace a test failure", "find root cause", or uses "/bugfix".
  Traces root cause through CLAUDE.md (spec), DEVELOPERS.md (context, optional), and Source Code layers.
  Trigger keywords: 버그 진단, 버그 수정, 에러 추적, 테스트 실패, 런타임 에러
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# /bugfix

compile 결과물(소스코드)의 런타임 버그/에러를 진단.
근본 원인을 CLAUDE.md(스펙), DEVELOPERS.md(맥락, optional), Source Code 3계층으로 추적하여 적절한 레벨에서 수정.

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

**3.2. 의미적 매칭 & 신뢰도 분류:**

기능 설명의 키워드와 각 모듈의 `purpose` + `export_names` 매칭 후 결과 분류:

```
매칭 결과 분류:
- 확실 (1개, purpose 직접 일치) → 바로 3.3으로 진행
- 후보 다수 (2-3개) → Exports 요약과 함께 AskUserQuestion으로 선택
- 매칭 실패 (0개) → Grep fallback:
    Grep: pattern="{keyword}" glob="**/*.{ts,py,go,rs}" head_limit=30
  → 여전히 없으면 AskUserQuestion으로 경로 직접 입력 요청
```

**3.3. 매칭된 모듈(최대 3개)에서 관련 테스트 탐색:**
```
Glob: {matched_dir}/**/*test*  또는  {matched_dir}/**/*spec*
```

**3.4. 관련 테스트 실행 → 실패 시 Type A/B 흐름 합류.**

**3.5. 전체 통과 → Behavior 교차 분석 (CLAUDE.md Behavior vs 실제 동작 비교).**

### 4. 대상 식별

`path`에서 CLAUDE.md + DEVELOPERS.md 존재 확인:

| 상태 | 진단 범위 |
|------|----------|
| 둘 다 있음 | 3-layer 진단 (L1+L2+L3) |
| CLAUDE.md만 | L1+L3 진단 (L2 스킵 — 2계층 fallback) |
| 없음 | `/decompile` 먼저 실행하여 CLAUDE.md 생성 제안 |

### 5. 사전 검증 (CLI) — 리스크 레벨 분류

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

**5.2. 미컴파일 변경 확인:**
```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

**5.3. 리스크 레벨 분류:**

| 검증 결과 | 조건 | 리스크 | 대응 |
|-----------|------|--------|------|
| 스키마 FAIL (필수 섹션 누락) | Exports/Behavior 오류 | **HIGH** | 차단 + AskUserQuestion 오버라이드 확인 |
| 스키마 FAIL (선택 섹션만) | 경고만 | **LOW** | 경고 후 계속 |
| 미컴파일: `untracked`/`no-source-code` | 소스코드 없음 | **HIGH** | 차단 + `/compile` 안내 |
| 미컴파일: `staged`/`modified`/`spec-newer` | 코드-스펙 불일치 | **MEDIUM** | 경고 + AskUserQuestion |
| 스키마 FAIL + 미컴파일 | 복합 | **HIGH** (에스컬레이션) | 차단 + 단계별 해결 안내 |
| 둘 다 PASS | 정상 | **NONE** | 그대로 진행 |

**HIGH 리스크 차단 시:**
```
AskUserQuestion: "사전 검증에서 HIGH 리스크가 발견되었습니다. {상세 설명}. 그래도 진행할까요?"
옵션: [오버라이드하고 진행, 먼저 해결 후 재시도]
```

**오버라이드 시 후속 처리:**
- debugger Task 호출 시 `risk_override: true` 플래그 전달
- debugger가 영향받는 계층 findings에 `confidence: LOW` 강제

### 6. 진단 (debugger agent)

```
Task(debugger):
  대상 디렉토리: {path}
  CLAUDE.md: {claude_md_path}
  DEVELOPERS.md: {developers_md_path}
  에러 정보: {error_message}
  테스트: {test_name_or_none}
  스키마 검증: PASS | FAIL ({errors})
  미컴파일 변경: NONE | DETECTED ({reason})
  리스크 레벨: NONE | LOW | MEDIUM | HIGH
  리스크 오버라이드: false | true
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
fix_targets: [CLAUDE.md]
compile_path: {dir}
compile_required: true | false
test_command: {command} | N/A
---end-debugger-result---
```

### 6.5. 수정사항 Diff 표시

debugger가 CLAUDE.md를 수정한 후, compile 전에 변경사항을 표시합니다:

```
Bash: git diff HEAD -- {path}/CLAUDE.md
```

**변경 없음:** 스킵.

표시 후 안내:
> "문서 수정사항을 확인하세요. 이어서 /compile을 자동 실행합니다."

### 7. Compile (소스코드 재생성)

debugger result의 `compile_required` 필드에 따라 `/compile` 자동 실행:

| 조건 | 동작 |
|------|------|
| `compile_required: true` | `Skill("claude-md-plugin:compile", args: "--path {compile_path} --conflict overwrite")` 실행 |
| `compile_required: false` | 스킵 (진단 실패 또는 사용자 dry-run) |
| compile 실패 | 실패 보고 + 수동 점검 안내 |

### 8. 검증 (원본 테스트 재실행)

compile 성공 후, 원본 테스트를 재실행하여 fix를 검증:

| 조건 | 동작 |
|------|------|
| `test_command` 있음 | 해당 테스트 명령어 실행 |
| `test_command: N/A` + Type B (테스트 실패) | Step 1에서 수집한 테스트로 실행 |
| 그 외 | 스킵 (compile 자체 테스트로 갈음) |

### 9. 결과 보고

**성공:**
```
/bugfix 결과
=========

Root Cause: {root_cause_layer} - {root_cause_type}
요약: {summary}

수정된 문서: {fix_targets}
재현: {reproduction}
Compile: PASS
검증: PASS ({test_command})

상세 결과: {result_file}
```

**부분 성공 (검증 실패):**
```
/bugfix 결과
=========

Root Cause: {root_cause_layer} - {root_cause_type}
요약: {summary}

수정된 문서: {fix_targets}
재현: {reproduction}
Compile: PASS
검증: FAIL ({test_command})

⚠ 추가 `/bugfix`로 남은 문제를 진단하세요.

상세 결과: {result_file}
```

**실패 (compile 실패):**
```
/bugfix 결과
=========

Root Cause: {root_cause_layer} - {root_cause_type}
요약: {summary}

수정된 문서: {fix_targets}
재현: {reproduction}
Compile: FAIL

⚠ 수동 점검이 필요합니다. 상세 결과를 확인하세요.

상세 결과: {result_file}
```

`reproduction` 필드:
- `REPRODUCED`: 에러가 정상적으로 재현됨
- `STATIC_ANALYSIS_ONLY`: 재현 실패 → 코드 분석만으로 진행
- `N/A`: 재현 시도 없음 (에러 메시지만 제공)

## DO / DON'T

**DO:**
- 근본 원인을 가능한 높은 계층(L1 > L2 > L3)으로 추적
- Fix는 항상 CLAUDE.md에서 수행 (소스코드는 "바이너리")
- Fix 전 사용자 승인 (AskUserQuestion)
- Fix 적용 후 `/compile` 자동 실행 (`compile_required: true`인 경우)
- Compile 후 원본 테스트 재실행 검증

**DON'T:**
- 소스코드 직접 수정 (항상 문서 수정 → `/compile`로 재생성)
- compile 없이 bugfix 완료 보고 금지 (`compile_required: true`인 경우)
- 사용자 승인 없이 CLAUDE.md 수정
- 전체 소스 디렉토리 읽기 (에러 위치 중심 타깃 분석)
- 상세 진단을 context에 반환 (`${TMP_DIR}` 파일 사용)

## 참조 자료

- `references/debugger-templates.md`: Root cause types, fix strategies, stack trace patterns, CLI guide, result template (debugger agent가 런타임에 `cat`으로 로드)

## 관련 컴포넌트

- `agents/debugger.md`: 3-layer 진단 및 수정 agent

## Examples

<example>
<context>
사용자가 에러 메시지와 함께 버그 수정을 요청합니다.
</context>
<user_request>/bugfix --error "TypeError: validateToken is not a function" --path src/auth</user_request>
<assistant_response>
src/auth에서 에러를 진단합니다...

사전 검증:
  스키마: PASS
  미컴파일 변경: NONE

3-layer 진단을 실행합니다...

/compile 자동 실행 중...

/bugfix 결과
=========

Root Cause: L1 - SPEC_EXPORT_MISMATCH
요약: CLAUDE.md exports validateToken as standalone but code defines it as class method

수정된 문서: [CLAUDE.md]
Compile: PASS
검증: PASS (npx jest src/auth --no-coverage)

상세 결과: .claude/tmp/debug-src-auth.md
</assistant_response>
</example>

<example>
<context>
사용자가 실패하는 테스트로 버그 수정을 요청합니다.
</context>
<user_request>/bugfix --test "should return empty array for no results"</user_request>
<assistant_response>
테스트를 실행하고 에러를 캡처합니다...

사전 검증:
  스키마: PASS
  미컴파일 변경: NONE

3-layer 진단을 실행합니다...

/compile 자동 실행 중...

/bugfix 결과
=========

Root Cause: L3 - CODE_SPEC_DIVERGENCE
요약: Code returns null instead of empty array as specified in CLAUDE.md Behavior

수정된 문서: (스펙/플랜 정확 — 재컴파일로 해결)
Compile: PASS
검증: PASS (npx jest --testNamePattern "should return empty array for no results")

상세 결과: .claude/tmp/debug-src-utils.md
</assistant_response>
</example>

<example>
<context>
사용자가 기능 설명으로 버그 수정을 요청합니다.
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

/compile 자동 실행 중...

/bugfix 결과
=========

Root Cause: L1 - SPEC_BEHAVIOR_GAP
요약: Token refresh on expiry not specified in CLAUDE.md Behavior

수정된 문서: [CLAUDE.md]
Compile: PASS
검증: PASS (npx jest --testNamePattern "should refresh token on expiry")

상세 결과: .claude/tmp/debug-src-auth.md
</assistant_response>
</example>
