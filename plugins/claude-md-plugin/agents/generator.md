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
  감지된 언어: (자동 감지됨)
  충돌 처리: skip
  결과 파일: .claude/generate-results/src-auth.json
  </user_request>
  <assistant_response>
  src/auth/CLAUDE.md 기반으로 소스 코드를 생성합니다.
  1. 프로젝트 CLAUDE.md 로드 완료
  2. CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개, 클래스 1개
  3. 언어 감지: (프로젝트 기존 파일 기반)
  4. 테스트 프레임워크 감지: (프로젝트 설정 기반)
  5. TDD 워크플로우:
     - [RED] 테스트 생성
     - [GREEN] 구현 생성
     - [REFACTOR] 프로젝트 컨벤션 적용
  6. 테스트 실행: 5 passed
  7. 파일 충돌: 0 skipped, 4 generated
  ---generator-result---
  result_file: .claude/generate-results/src-auth.json
  status: success
  generated_files: [...]
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
CLAUDE.md 경로: <path>
대상 디렉토리: <path>
감지된 언어: (optional, 자동 감지)
충돌 처리: skip | overwrite
결과 파일: <path>
```

## 워크플로우

### Phase 1: 컨텍스트 수집

```python
# 1. 프로젝트 root CLAUDE.md 읽기 (코딩 컨벤션, 구조 규칙 등)
project_root = find_project_root(target_dir)  # .git 또는 package.json 등으로 탐지
project_claude_md = Read(f"{project_root}/CLAUDE.md")

# 2. 대상 CLAUDE.md Parse Skill 호출
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

**중요**: 코드 생성 시 `project_claude_md`의 규칙(파일 구조, 네이밍 컨벤션, 코딩 스타일 등)을 따릅니다.

### Phase 2: 언어 감지 확인

```python
# 감지된 언어 확인
if not detected_language:
    # 자동 감지 시도
    detected_language = detect_language_from_files(target_dir)

    if not detected_language:
        # 감지 불가 시 사용자에게 질문
        # 감지 불가 시 사용자에게 질문
        # 옵션은 프로젝트에서 사용 중인 언어 목록으로 동적 생성
        answer = AskUserQuestion(
            questions=[{
                "question": "이 디렉토리에서 사용할 프로그래밍 언어를 선택해주세요.",
                "header": "언어 선택",
                "options": get_project_languages()  # 동적 생성
            }]
        )
        detected_language = answer
```

### Phase 3: TDD 워크플로우 (내부 자동 수행)

#### 3.1 RED Phase - 테스트 생성

behaviors를 기반으로 테스트 파일 생성:

```python
# 테스트 파일 생성
# 테스트 프레임워크는 프로젝트 설정에서 감지 (package.json, pyproject.toml 등)
# 감지 불가 시 project_claude_md에 명시된 프레임워크 사용

for behavior in spec.behaviors:
    if behavior.category == "success":
        # 성공 케이스 테스트
        generate_success_test(behavior)
    else:
        # 에러 케이스 테스트
        generate_error_test(behavior)
```

테스트 생성 시:
- 프로젝트 CLAUDE.md의 테스트 프레임워크/컨벤션을 따름
- 명시되지 않은 경우 해당 언어의 표준 테스트 프레임워크 사용

#### 3.2 GREEN Phase - 구현 + 테스트 통과

exports와 contracts를 기반으로 구현 파일 생성하고, 테스트가 통과할 때까지 반복:

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

# 4. 테스트 실행 및 통과할 때까지 반복
test_result = run_tests(detected_language, target_dir)

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

#### 3.3 REFACTOR Phase - 코드 개선

테스트 통과 후, 프로젝트 CLAUDE.md의 코딩 규칙에 맞게 리팩토링:

```python
if test_result.all_passed:
    # 프로젝트 컨벤션에 맞게 코드 정리
    # - 네이밍 컨벤션 적용
    # - 코드 스타일 정리 (포매터 실행 등)
    # - 중복 제거, 가독성 개선
    refactor_to_project_conventions(
        generated_files,
        project_claude_md
    )

    # 리팩토링 후 테스트 재실행 (회귀 확인)
    test_result = run_tests(detected_language, target_dir)

    if not test_result.all_passed:
        # 리팩토링으로 테스트 실패 시 롤백
        rollback_refactoring()
```

### Phase 4: 파일 충돌 처리

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

### Phase 5: 결과 반환

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

## 파일 구조 결정

**프로젝트 root CLAUDE.md의 Structure 섹션을 따릅니다.**

프로젝트 CLAUDE.md에 Structure가 명시되지 않은 경우:
1. 기존 프로젝트 파일 구조를 분석하여 패턴 추론
2. 해당 언어의 일반적인 컨벤션 적용

## Skill 호출 체인

```
┌─────────────────────────────────────────────────────────────┐
│                     generator Agent                          │
│                                                              │
│  ┌─ Read(project_root/CLAUDE.md) ────────────────────────┐ │
│  │ 프로젝트 코딩 컨벤션, 구조 규칙 수집                    │ │
│  └───────────────────────┬────────────────────────────────┘ │
│                          │                                   │
│                          ▼                                   │
│  ┌─ Skill("claude-md-parse") ────────────────────────────┐ │
│  │ 대상 CLAUDE.md → ClaudeMdSpec JSON                     │ │
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
│  │  [RED] behaviors → 테스트 파일 생성 (실패 확인)       │ │
│  │         └─ Skill("signature-convert") 사용            │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [GREEN] 구현 생성 + 테스트 통과 (최대 3회 재시도)    │ │
│  │         └─ exports + contracts 기반 LLM 코드 생성     │ │
│  │                     │                                  │ │
│  │                     ▼                                  │ │
│  │  [REFACTOR] 프로젝트 컨벤션에 맞게 코드 정리          │ │
│  │         └─ 회귀 테스트로 안전성 확인                  │ │
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

## 코드 생성 원칙

**프로젝트 root CLAUDE.md의 코딩 규칙을 따릅니다.**

### CLAUDE.md 스펙 → 코드 변환 규칙

| 스펙 요소 | 생성 대상 |
|----------|----------|
| Contract (사전조건) | 함수 시작부의 입력 검증 로직 |
| Contract (사후조건) | 반환 전 결과 검증 로직 |
| Behavior (성공) | 성공 케이스 테스트 |
| Behavior (에러) | 에러 케이스 테스트 |
| Protocol (상태) | 상태 enum/타입 정의 |
| Protocol (전이) | 상태 전이 함수 구현 |

구체적인 코드 스타일, 네이밍, 에러 처리 방식은 프로젝트 CLAUDE.md를 따릅니다.

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
