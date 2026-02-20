<!--
  debugger-templates.md
  Consolidated reference for the debugger agent.
  Contains: Input type classification, root cause type definitions,
  fix strategy templates, stack trace patterns, CLI usage guide,
  CLI output JSON structures, result template format, decision tree.

  Loaded at runtime by the debugger agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/bugfix/references/debugger-templates.md"
-->

# Debugger Templates & Reference

## Input Type Classification

### Type A: Technical Error
에러 클래스명이나 스택 트레이스가 포함된 입력.

**판별 패턴:**
- JS/TS: `TypeError`, `ReferenceError`, `SyntaxError`, `RangeError`
- Python: `ValueError`, `KeyError`, `AttributeError`, `TypeError`
- Go: `panic:`, `fatal error:`
- Rust: `thread .+ panicked`, `error[E`
- Java/Kotlin: `Exception`, `Error` (suffix)
- 스택 트레이스 라인 존재 (`at `, `File "`, `goroutine`)

**초기 대응:** 에러 위치에서 직접 L3 탐색 시작.

### Type B: Test Failure
테스트 이름이나 테스트 파일이 포함된 입력.

**판별 패턴:**
- `--test` 인자 존재
- 파일명 패턴: `*.test.*`, `*_test.*`, `test_*`, `*spec*`
- 테스트 러너 출력 키워드: `FAIL`, `FAILED`, `failing`, `AssertionError`

**초기 대응:** 테스트 실행 → 에러 캡처 → L3 탐색.

### Type C: Functional Description
위 둘 다 아닌 입력. CS 인입, 기능 버그 리포트, 비기술적 설명.

**판별:** 에러 클래스 없음, 스택 트레이스 없음, 테스트 파일명 없음.

**초기 대응:** `scan-claude-md`로 모듈 탐색 → 관련 테스트 찾기 → 실행.

## Root Cause Type Definitions

### L1: CLAUDE.md (Spec) Issues

| 타입 | 설명 | 증거 패턴 |
|------|------|----------|
| **SPEC_BEHAVIOR_GAP** | Behavior에 이 에러 시나리오가 없음 | `spec_json.behaviors`에 매칭 시나리오 없음 |
| **SPEC_EXPORT_MISMATCH** | Exports 시그니처와 코드 불일치 | 시그니처 정규화 후 비교 실패 |
| **SPEC_CONTRACT_GAP** | Contract에 이 에러 조건이 없음 | `spec_json.contracts.throws`에 에러 타입 없음 |
| **SPEC_STALE** | CLAUDE.md가 코드보다 오래됨 | `git log` 타임스탬프 비교 |

### L2: IMPLEMENTS.md (Plan) Issues

| 타입 | 설명 | 증거 패턴 |
|------|------|----------|
| **PLAN_ERROR_HANDLING_GAP** | Error Handling에 에러 타입 미포함 | Error Handling 테이블에 에러 없음 |
| **PLAN_ALGORITHM_FLAW** | Algorithm 섹션 설계 자체가 잘못됨 | 알고리즘과 에러 로직의 인과관계 |
| **PLAN_STATE_GAP** | State Management에 상태 전이 미기술 | 상태 관련 에러 + State 섹션 누락 |
| **PLAN_CONSTANT_MISMATCH** | Key Constants와 코드 상수 불일치 | 코드 상수값 vs IMPLEMENTS.md 명세값 차이 |

### L3: Source Code Issues (진단용 — 수정은 L1/L2에서)

L3 finding은 코드의 증상이며, 근본 원인은 항상 L1/L2에 있다.
소스코드는 "바이너리"이므로 직접 패치하지 않고, L1/L2를 수정한 뒤 `/compile`로 재생성한다.

| 타입 | 설명 | 증거 패턴 | L1/L2 수정 방향 |
|------|------|----------|----------------|
| **CODE_SPEC_DIVERGENCE** | 코드가 스펙/플랜을 따르지 않음 | 스펙 behavior 존재 + 코드 동작 불일치 | 스펙이 맞으면 `/compile`로 재생성 |
| **CODE_LOGIC_ERROR** | 코드 자체의 로직 버그 | 스펙/플랜 모두 올바르나 코드 잘못됨 | IMPLEMENTS.md Algorithm 보강 후 `/compile` |
| **CODE_GUARD_MISSING** | guard clause/입력 검증 누락 | Contract precondition 존재 + 코드에 guard 없음 | Contract 명확화 후 `/compile` |
| **CODE_IMPLEMENTATION_BUG** | 코드가 IMPLEMENTS.md 플랜을 따르지 않음 | 플랜 기술 존재 + 코드가 플랜 불이행 | 플랜이 맞으면 `/compile`로 재생성 |

## Fix Strategy Templates

### L1 Fix (CLAUDE.md)
```markdown
## L1 Fix: {root_cause_type}

**대상 파일:** {claude_md_path}
**대상 섹션:** {Exports | Behavior | Contract}

**현재:**
{current_content}

**수정안:**
{proposed_content}

**근거:** {why_this_fix}
**후속 조치:** `/compile --path {dir} --conflict overwrite` 권장
```

### L2 Fix (IMPLEMENTS.md)
```markdown
## L2 Fix: {root_cause_type}

**대상 파일:** {implements_md_path}
**대상 섹션:** {Error Handling | Algorithm | State Management | Key Constants}

**현재:**
{current_content}

**수정안:**
{proposed_content}

**근거:** {why_this_fix}
**후속 조치:** `/compile --path {dir} --conflict overwrite` 권장
```

### L3 Fix (소스코드 직접 수정 안 함 — L1/L2 수정 후 `/compile`)

L3 finding은 코드 증상. 수정은 항상 L1/L2 문서에서 수행하고 `/compile`로 소스코드를 재생성한다.

```markdown
## L3 Finding → L1/L2 Fix: {root_cause_type}

**증상 위치:** {source_file}:{line_number} ({function_name})
**코드 증상:** {current_code_behavior}

**근본 원인:** {L1 또는 L2 어느 문서의 어느 섹션이 부족/불일치}

**수정 대상:** {claude_md_path 또는 implements_md_path}
**수정 섹션:** {Exports | Behavior | Contract | Algorithm | Error Handling}

**수정안:**
{proposed_doc_content}

**후속 조치:** `/compile --path {dir} --conflict overwrite`
```

## Stack Trace Patterns

### Error Type + Message Extraction

| 언어 | 패턴 |
|------|------|
| JS/TS | `^(\w+Error): (.+)$` |
| Python | traceback 마지막 `^\w+Error: (.+)$` |
| Go | `panic: (.+)` or `fatal error: (.+)` |
| Rust | `thread .+ panicked at '(.+)'` |
| Java/Kotlin | `^([\w.]+Exception): (.+)$` |

### Error Location Extraction (file:line:function)

| 언어 | 패턴 | 추출 |
|------|------|------|
| JS/TS | `at (\w+[\.\w]*) \((.+):(\d+):(\d+)\)` | function, file, line |
| Python | `File "(.+)", line (\d+), in (\w+)` | file, line, function |
| Go | `\t(.+\.go):(\d+)` (함수명은 이전 줄) | file, line |
| Rust | `at (.+\.rs):(\d+):(\d+)` | file, line |
| Java/Kotlin | `at .+\((.+\.(?:java\|kt)):(\d+)\)` | file, line |

### Frame Filter (제외)
- `node_modules/`, `site-packages/`, `vendor/`, `.cargo/registry/`, `<anonymous>`

### Test Expected vs Actual

| 프레임워크 | Expected 패턴 | Actual 패턴 |
|-----------|--------------|------------|
| Jest/Vitest | `Expected: (.+)` | `Received: (.+)` |
| pytest | `assert .+ == (.+)` | `assert (.+) ==` |
| Go | `expected (.+)` | `got (.+)` |
| Rust | `left: (.+)` | `right: (.+)` |

## CLI Usage Guide

### scan-claude-md (Type C: 모듈 탐색)
```bash
$CLI_PATH scan-claude-md --root {project_root} --output ${TMP_DIR}debug-scan-index.json
```
결과: `[{dir, purpose, export_names}]` — 각 모듈의 목적과 export 이름.
기능 설명의 키워드와 의미적 매칭하여 관련 모듈 특정.

### parse-claude-md (L1: 스펙 교차 검증)
```bash
$CLI_PATH parse-claude-md --file {claude_md_path}
```
결과: 구조화된 JSON (`spec_json`) — exports, behaviors, contracts, domain_context.
L1 검증에서 에러와 관련된 스펙 요소 교차 검증에 사용.

### analyze-code (L3: 코드 export 추출)
```bash
$CLI_PATH analyze-code --path {directory}
```
결과: 코드의 실제 exports, dependencies, behaviors를 JSON으로 추출.
L1 교차 검증 시 코드 측 데이터로 사용.

### validate-schema (사전 환경 검증)
```bash
$CLI_PATH validate-schema --file {claude_md_path}
```
결과: `{valid: bool, errors: [], warnings: []}`.
`valid: false`이면 CLAUDE.md 스키마 자체 오류 → `/validate` 먼저 실행 안내.

### diff-compile-targets (사전 환경 검증)
```bash
$CLI_PATH diff-compile-targets --root {project_root}
```
결과: 변경 감지된 CLAUDE.md 목록.
대상 모듈이 `targets`에 포함되면 미컴파일 변경 → `/compile` 먼저 실행 안내.

### resolve-boundary (바운더리 위반 확인)
```bash
$CLI_PATH resolve-boundary --path {directory} --claude-md {claude_md_path}
```
결과: `{violations: [...]}`.
INV-1 위반이 버그 원인일 수 있음.

## CLI Output JSON Structures

### parse-claude-md 출력

```json
{
  "name": "auth",
  "purpose": "User authentication module",
  "exports": {
    "functions": [{"name": "validateToken", "signature": "validateToken(token: string): Promise<Claims>", "is_async": true}],
    "types": [{"name": "Claims", "definition": "Claims { userId: string, role: Role }", "kind": "interface"}],
    "classes": [{"name": "TokenManager", "constructor_signature": "TokenManager(secret: string)"}],
    "enums": [],
    "variables": []
  },
  "dependencies": {
    "external": ["jsonwebtoken@9.0.0"],
    "internal": ["./types"]
  },
  "behaviors": [
    {"input": "valid JWT token", "output": "Claims object", "category": "success"},
    {"input": "expired token", "output": "TokenExpiredError", "category": "error"}
  ],
  "contracts": [{"function_name": "validateToken", "preconditions": [], "postconditions": [], "throws": [], "invariants": []}],
  "structure": {"subdirs": [], "files": []}
}
```

### analyze-code 출력

```json
{
  "path": "src/auth",
  "language": "typescript",
  "exports": [{"name": "validateToken", "kind": "function", "signature": "..."}],
  "dependencies": {"internal": [...], "external": [...]},
  "behaviors": [...]
}
```

### resolve-boundary 출력

```json
{
  "path": "src/auth",
  "direct_files": [{"name": "index.ts", "type": "typescript"}],
  "subdirs": [{"name": "jwt", "has_claude_md": true}],
  "violations": [{"violation_type": "Parent", "reference": "../utils", "line_number": 15}]
}
```

## Result Template

```markdown
# Debug 결과: {directory}

## Bug Report

- **에러 타입:** {error_type}
- **에러 메시지:** {error_message}
- **에러 위치:** {file}:{line} ({function})
- **입력 타입:** Type A (기술적 에러) | Type B (테스트 실패) | Type C (기능 설명)

## 사전 환경 검증

- **스키마 검증:** PASS | FAIL ({errors})
- **미컴파일 변경:** NONE | DETECTED ({reason})
- **바운더리 위반:** NONE | DETECTED ({violations})

## 3-Layer 분석

### L3: Source Code

- **에러 위치 코드:** {code_snippet}
- **관련 심볼:** {symbols}
- **코드 분석 결과:** {analyze_code_output}

### L1: CLAUDE.md (Spec)

- **Exports 비교:** MATCH | MISMATCH | NOT_FOUND
- **Behavior 커버리지:** COVERED | GAP | PARTIAL
- **Contract 검증:** VALID | VIOLATION | GAP

### L2: IMPLEMENTS.md (Plan)

- **Error Handling:** COVERED | GAP
- **Algorithm:** CORRECT | FLAW | DIVERGENCE
- **State Management:** N/A | COVERED | GAP
- **Key Constants:** N/A | MATCH | MISMATCH

## Root Cause

- **계층:** L1 | L2 | L3 | MULTI
- **타입:** {root_cause_type}
- **요약:** {one_line_summary}

## Fix 제안 (CLAUDE.md / IMPLEMENTS.md)

{fix_details_per_layer}

## 후속 조치

- **`/compile --path {dir} --conflict overwrite`** 로 소스코드 재생성
- **테스트 재실행:** PASSED | FAILED | SKIPPED
```

## Decision Tree (Root Cause Classification)

```
Bug Report (에러 메시지 / 테스트 실패 / 잘못된 동작)
    |
    +-- CLAUDE.md 존재?
    |   NO --> /decompile 먼저 실행하여 CLAUDE.md 생성 제안
    |   YES |
    |       +-- Behavior가 이 시나리오를 커버하는가?
    |       |   NO --> L1: SPEC_BEHAVIOR_GAP
    |       |   YES |
    |       +-- Exports 시그니처가 코드와 일치하는가?
    |       |   NO --> L1: SPEC_EXPORT_MISMATCH
    |       |   YES |
    |       +-- IMPLEMENTS.md 존재?
    |       |   NO --> L1 분석 (L2 스킵, IMPLEMENTS.md 생성 권장)
    |       |   YES |
    |       +-- Error Handling이 이 에러를 다루는가?
    |       |   NO --> L2: PLAN_ERROR_HANDLING_GAP
    |       |   YES |
    |       +-- Algorithm이 이 로직을 기술하는가?
    |       |   NO --> L2: PLAN_ALGORITHM_FLAW
    |       |   YES |
    |       +-- 코드가 IMPLEMENTS.md대로 구현되어 있는가?
    |           NO --> IMPLEMENTS.md 맞으면 `/compile`로 재생성
    |           YES --> IMPLEMENTS.md Algorithm 보강 후 `/compile`
    |
    +-- 모든 경우: Fix는 CLAUDE.md/IMPLEMENTS.md → `/compile`로 재생성
```
