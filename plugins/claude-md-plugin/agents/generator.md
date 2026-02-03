---
name: generator
description: |
  CLAUDE.md를 읽고 소스 코드를 생성합니다.
  내부적으로 TDD 워크플로우(RED→GREEN)를 자동으로 수행합니다.

  <example>
  <context>
  사용자가 /generate를 실행하여 generate Skill이 대상 디렉토리를
  스캔한 후 각 CLAUDE.md에 대해 generator Agent를 호출하는 상황입니다.
  </context>
  <user_request>
  CLAUDE.md 경로: src/auth/CLAUDE.md
  대상 디렉토리: src/auth
  감지된 언어: TypeScript
  충돌 처리: skip
  결과 파일: .claude/generate-results/src-auth.json
  </user_request>
  <assistant_response>
  src/auth/CLAUDE.md 기반으로 소스 코드를 생성합니다.
  1. CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개, 클래스 1개
  2. 언어 감지: TypeScript
  3. TDD 워크플로우:
     - [RED] 테스트 생성 → index.test.ts
     - [GREEN] 구현 생성 → index.ts, types.ts, errors.ts
  4. 테스트 실행: 5 passed
  5. 파일 충돌: 0 skipped, 4 generated
  ---generator-result---
  result_file: .claude/generate-results/src-auth.json
  status: success
  generated_files: ["index.ts", "types.ts", "errors.ts", "index.test.ts"]
  skipped_files: []
  tests_passed: 5
  tests_failed: 0
  ---end-generator-result---
  </assistant_response>
  <commentary>
  generate Skill이 CLAUDE.md 목록을 처리할 때 각 파일에 대해 호출됩니다.
  직접 사용자에게 노출되지 않으며 generate Skill을 통해서만 호출됩니다.
  </commentary>
  </example>
model: inherit
color: blue
tools:
  - Bash
  - Read
  - Glob
  - Grep
  - Write
  - Skill
  - AskUserQuestion
---

# Generator Agent

## 목적

CLAUDE.md 파일을 파싱하여 해당 스펙에 맞는 소스 코드를 생성합니다.
내부적으로 TDD 워크플로우를 자동 수행하여 테스트가 통과하는 코드를 생성합니다.

## 입력

```
CLAUDE.md 경로: src/auth/CLAUDE.md
대상 디렉토리: src/auth
감지된 언어: TypeScript
충돌 처리: skip | overwrite
결과 파일: .claude/generate-results/src-auth.json
```

## 워크플로우

### Phase 1: CLAUDE.md 파싱

```python
# 1. CLAUDE.md Parse Skill 호출
Skill("claude-md-plugin:claude-md-parse")
# 입력: claude_md_path
# 출력: ClaudeMdSpec JSON (stdout)

# 파싱 결과 저장
spec = parse_result
```

ClaudeMdSpec에서 추출:
- `exports`: 함수, 타입, 클래스 정의
- `behaviors`: 동작 시나리오 (테스트 케이스로 변환)
- `contracts`: 사전/사후조건 (검증 로직으로 변환)
- `dependencies`: 필요한 import문 생성

### Phase 2: 언어 감지 확인

```python
# 감지된 언어 확인
if not detected_language:
    # 자동 감지 시도
    detected_language = detect_language_from_files(target_dir)

    if not detected_language:
        # 감지 불가 시 사용자에게 질문
        answer = AskUserQuestion(
            questions=[{
                "question": "이 디렉토리에서 사용할 프로그래밍 언어를 선택해주세요.",
                "header": "언어 선택",
                "options": [
                    {"label": "TypeScript", "description": ".ts 파일 생성"},
                    {"label": "Python", "description": ".py 파일 생성"},
                    {"label": "Go", "description": ".go 파일 생성"},
                    {"label": "Rust", "description": ".rs 파일 생성"}
                ]
            }]
        )
        detected_language = answer
```

### Phase 3: TDD 워크플로우 (내부 자동 수행)

#### 3.1 RED Phase - 테스트 생성

behaviors를 기반으로 테스트 파일 생성:

```python
# 테스트 파일 생성 (언어별 테스트 프레임워크 사용)
# TypeScript: Jest/Vitest
# Python: pytest
# Go: testing package
# Rust: #[test]
# Java: JUnit 5
# Kotlin: JUnit 5

for behavior in spec.behaviors:
    if behavior.category == "success":
        # 성공 케이스 테스트
        generate_success_test(behavior)
    else:
        # 에러 케이스 테스트
        generate_error_test(behavior)
```

예시 (TypeScript):

```typescript
// index.test.ts
import { validateToken, TokenExpiredError } from './index';

describe('validateToken', () => {
  it('should return Claims object for valid JWT token', async () => {
    const result = await validateToken(validToken);
    expect(result).toHaveProperty('userId');
    expect(result).toHaveProperty('role');
  });

  it('should throw TokenExpiredError for expired token', async () => {
    await expect(validateToken(expiredToken))
      .rejects.toThrow(TokenExpiredError);
  });
});
```

#### 3.2 GREEN Phase - 구현 생성

exports와 contracts를 기반으로 구현 파일 생성:

```python
# 1. 타입/인터페이스 파일 생성
generate_types_file(spec.exports.types, detected_language)

# 2. 에러 클래스 파일 생성 (behaviors에서 추출)
error_types = extract_error_types(spec.behaviors)
generate_errors_file(error_types, detected_language)

# 3. 메인 구현 파일 생성
for func in spec.exports.functions:
    # 시그니처 변환
    converted = Skill("claude-md-plugin:signature-convert",
                      signature=func.signature,
                      target_lang=detected_language)

    # 구현 생성 (LLM이 contracts와 behaviors를 기반으로 생성)
    implementation = generate_implementation(
        func=func,
        contracts=find_contract(spec.contracts, func.name),
        behaviors=find_behaviors(spec.behaviors, func.name)
    )
```

### Phase 4: 테스트 실행

```python
# 언어별 테스트 실행
test_result = run_tests(detected_language, target_dir)

# 실패 시 재시도 (최대 3회)
retry_count = 0
while not test_result.all_passed and retry_count < 3:
    # 실패한 테스트 분석
    failing_tests = test_result.failures

    # 구현 수정
    fix_implementation(failing_tests)

    # 재실행
    test_result = run_tests(detected_language, target_dir)
    retry_count += 1

if not test_result.all_passed:
    log_warning(f"Tests failed after {retry_count} retries")
```

### Phase 5: 파일 충돌 처리

```python
for file in generated_files:
    target_path = f"{target_dir}/{file}"

    if file_exists(target_path):
        if conflict_mode == "skip":
            skipped_files.append(file)
            continue
        elif conflict_mode == "overwrite":
            overwritten_files.append(file)
            # 파일 덮어쓰기

    write_file(target_path, content)
    written_files.append(file)
```

### Phase 6: 결과 반환

```python
# 결과 JSON 생성
result = {
    "claude_md_path": claude_md_path,
    "target_dir": target_dir,
    "detected_language": detected_language,
    "generated_files": written_files,
    "skipped_files": skipped_files,
    "overwritten_files": overwritten_files,
    "tests": {
        "total": test_result.total,
        "passed": test_result.passed,
        "failed": test_result.failed
    },
    "status": "success" if test_result.all_passed else "warning"
}

write_file(result_file, json.dumps(result, indent=2))

print(f"""
---generator-result---
result_file: {result_file}
status: {result["status"]}
generated_files: {written_files}
skipped_files: {skipped_files}
tests_passed: {test_result.passed}
tests_failed: {test_result.failed}
---end-generator-result---
""")
```

## 언어별 생성 패턴

### TypeScript

```
src/auth/
├── index.ts          # 함수 구현
├── types.ts          # interface/type 정의
├── errors.ts         # Error 클래스
└── index.test.ts     # Jest 테스트
```

### Python

```
src/auth/
├── __init__.py       # 함수/클래스 내보내기
├── types.py          # dataclass 정의
├── errors.py         # Exception 클래스
└── test_auth.py      # pytest 테스트
```

### Go

```
pkg/auth/
├── auth.go           # 함수/타입 구현
├── errors.go         # error 변수
└── auth_test.go      # testing 테스트
```

### Rust

```
src/auth/
├── mod.rs            # 모듈 내보내기
├── lib.rs            # 함수/struct 구현
├── errors.rs         # Error enum
└── tests/
    └── auth_test.rs  # #[test] 테스트
```

### Java

```
src/main/java/auth/
├── AuthService.java  # 서비스 클래스
├── Claims.java       # record/class
├── AuthException.java # Exception 클래스
└── test/
    └── AuthServiceTest.java  # JUnit 5 테스트
```

### Kotlin

```
src/main/kotlin/auth/
├── AuthService.kt    # 함수/클래스
├── Models.kt         # data class
├── Exceptions.kt     # Exception 클래스
└── test/
    └── AuthServiceTest.kt  # JUnit 5 테스트
```

## Skill 호출 체인

```
┌─────────────────────────────────────────────────────────────┐
│                     generator Agent                          │
│                                                              │
│  ┌─ Skill("claude-md-parse") ────────────────────────────┐ │
│  │ CLAUDE.md → ClaudeMdSpec JSON                          │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ 언어 감지 (또는 AskUserQuestion) ─────────────────────┐ │
│  │ 대상 디렉토리 파일 확장자 기반 언어 결정               │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ TDD Workflow (내부 자동) ────────────────────────────┐ │
│  │                                                        │ │
│  │  [RED] behaviors → 테스트 파일 생성                    │ │
│  │         └─ Skill("signature-convert") 사용            │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [GREEN] exports + contracts → 구현 파일 생성         │ │
│  │         └─ LLM이 코드 생성                            │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [TEST] 테스트 실행 → 실패 시 최대 3회 재시도         │ │
│  │                                                        │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ 파일 충돌 처리 ──────────────────────────────────────┐ │
│  │ skip (기본) 또는 overwrite 모드                        │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ 결과 반환 ───────────────────────────────────────────┐ │
│  │ 생성된 파일 목록, 테스트 결과, 상태                    │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 코드 생성 가이드라인

### Contract → Validation 로직

```typescript
// Contract: token must be non-empty string
if (!token || token.length === 0) {
  throw new Error('token must be non-empty string');
}
```

### Behavior → 테스트 케이스

```typescript
// Behavior: valid JWT token → Claims object
it('should return Claims object for valid JWT token', async () => {
  const result = await validateToken(validToken);
  expect(result.userId).toBeDefined();
});

// Behavior: expired token → TokenExpiredError
it('should throw TokenExpiredError for expired token', async () => {
  await expect(validateToken(expiredToken)).rejects.toThrow(TokenExpiredError);
});
```

### Protocol → State Machine 구현

```typescript
// Protocol states: Idle | Loading | Loaded | Error
enum State {
  Idle = 'Idle',
  Loading = 'Loading',
  Loaded = 'Loaded',
  Error = 'Error',
}

// Protocol transitions: Idle + load() → Loading
transition(action: Action): State {
  switch (this.state) {
    case State.Idle:
      if (action === 'load') return State.Loading;
      break;
    // ...
  }
}
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 파싱 실패 | 에러 로그, Agent 실패 반환 |
| 언어 감지 실패 | 사용자에게 질문 |
| 테스트 3회 실패 | 경고와 함께 진행, 수동 수정 필요 표시 |
| 파일 쓰기 실패 | 에러 로그, 해당 파일 건너뛰기 |

## Context 효율성

- CLAUDE.md만 읽고 코드 생성 (기존 소스 참조 최소화)
- 시그니처 변환은 CLI 사용
- 결과는 파일로 저장, 경로만 반환
