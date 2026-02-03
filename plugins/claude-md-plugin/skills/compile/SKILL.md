---
name: compile
version: 1.0.0
aliases: [gen, generate, build]
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement spec",
  "create source files", or uses "/compile". Processes all CLAUDE.md files in the target path.
  Performs TDD workflow (RED→GREEN→REFACTOR) to ensure compiled code passes tests.

  <example>
  <context>
  사용자가 프로젝트의 CLAUDE.md를 처리하려고 합니다.
  </context>
  <user_request>/compile</user_request>
  <assistant_response>
  프로젝트에서 CLAUDE.md 파일을 검색합니다...

  발견된 CLAUDE.md 파일:
  1. src/auth/CLAUDE.md
  2. src/utils/CLAUDE.md

  코드 생성을 시작합니다...

  [1/2] src/auth/CLAUDE.md
  ✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개
  ✓ 테스트 생성
  ✓ 구현 생성
  ✓ 테스트 실행: 5 passed

  [2/2] src/utils/CLAUDE.md
  ✓ CLAUDE.md 파싱 완료 - 함수 3개
  ✓ 테스트 생성
  ✓ 구현 생성
  ✓ 테스트 실행: 3 passed

  === 생성 완료 ===
  총 CLAUDE.md: 2개
  생성된 파일: 7개
  테스트: 8 passed, 0 failed
  </assistant_response>
  </example>
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# Compile Skill

## Core Philosophy

**CLAUDE.md(소스) → Source Code(바이너리) = Compile**

```
CLAUDE.md (스펙)  ─── /compile ──→  Source Code (구현)
```

전통적 컴파일러가 소스코드를 바이너리로 변환하듯,
`/compile`은 CLAUDE.md 명세를 실행 가능한 소스코드로 변환합니다.

## 목적

CLAUDE.md 파일을 기반으로 소스 코드를 생성합니다.
CLAUDE.md가 명세(specification)가 되고, 소스 코드가 산출물이 됩니다.

## 사용법

```bash
# 기본 사용 (전체 CLAUDE.md 처리)
/compile

# 특정 경로만 처리
/compile --path src/auth

# 기존 파일 덮어쓰기
/compile --conflict overwrite
```

## 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리 대상 경로 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 (`skip` \| `overwrite`) |

## 워크플로우

```
/compile
    │
    ▼
모든 CLAUDE.md 검색
    │
    ▼
병렬 처리 (run_in_background=True)
    │
    ▼
결과 수집 및 보고
```

### 1. 대상 파일 결정

```bash
# 지정 경로 하위의 모든 CLAUDE.md 찾기 (root CLAUDE.md 제외)
Glob("**/CLAUDE.md") | filter(exclude_root)
```

### 2. 언어 자동 감지

각 CLAUDE.md가 있는 디렉토리의 언어를 감지합니다.

**감지 순서:**
1. 대상 디렉토리의 기존 소스 파일 확장자
2. 부모/형제 CLAUDE.md의 언어 정보
3. 감지 불가 시 사용자에게 질문

```python
def detect_language(directory):
    # 1. 기존 파일 확장자 기반 (동적 감지)
    extensions = get_file_extensions(directory)
    language = infer_language_from_extensions(extensions)
    if language:
        return language

    # 2. 부모 디렉토리 참조
    parent_lang = detect_from_parent(directory)
    if parent_lang:
        return parent_lang

    # 3. 사용자 질문 (프로젝트에서 사용 중인 언어 목록으로 옵션 생성)
    return ask_user_for_language()
```

### 3. compiler Agent 호출 (병렬 처리)

```python
# 모든 compiler Task를 병렬로 실행
tasks = []
for claude_md_path in target_files:
    target_dir = dirname(claude_md_path)
    detected_language = detect_language(target_dir)
    output_name = target_dir.replace("/", "-").replace(".", "root")

    print(f"  • {claude_md_path} - 시작")

    # compiler Agent 병렬 실행 (run_in_background=True)
    task = Task(
        prompt=f"""
        CLAUDE.md 경로: {claude_md_path}
        대상 디렉토리: {target_dir}
        감지된 언어: {detected_language}
        충돌 처리: {conflict_mode}
        결과는 scratchpad에 저장하고 경로만 반환해주세요.
        """,
        subagent_type="compiler",
        run_in_background=True
    )
    tasks.append(task)
```

### 4. 결과 수집 및 보고

```python
total_files = 0
total_skipped = 0
total_tests_passed = 0
total_tests_failed = 0

for result_file in result_files:
    result = read_json(result_file)
    total_files += len(result["generated_files"])
    total_skipped += len(result["skipped_files"])
    total_tests_passed += result["tests"]["passed"]
    total_tests_failed += result["tests"]["failed"]

print(f"""
=== 생성 완료 ===
총 CLAUDE.md: {len(target_files)}개
생성된 파일: {total_files}개
건너뛴 파일: {total_skipped}개
테스트: {total_tests_passed} passed, {total_tests_failed} failed
""")
```

## 언어 및 테스트 프레임워크

**프로젝트에서 사용 중인 언어와 테스트 프레임워크를 자동 감지합니다.**

감지 방법:
- 언어: 파일 확장자 기반
- 테스트 프레임워크: 프로젝트 설정 파일 분석 (package.json, pyproject.toml, Cargo.toml 등)

## 내부 TDD 워크플로우

사용자에게 노출되지 않는 내부 프로세스:

```
CLAUDE.md 파싱
     │
     ▼
[RED] behaviors → 테스트 코드 생성 (실패 확인)
     │
     ▼
[GREEN] 구현 생성 + 테스트 통과 (최대 3회 재시도)
     │
     ▼
[REFACTOR] 프로젝트 컨벤션 적용 + 회귀 테스트
     │
     ▼
파일 충돌 처리
     │
     ▼
결과 반환
```

## 파일 충돌 처리

| 모드 | 동작 |
|------|------|
| `skip` (기본) | 기존 파일 유지, 새 파일만 생성 |
| `overwrite` | 기존 파일 덮어쓰기 |

```python
# --conflict skip (기본)
if file_exists(target_path):
    print(f"⏭ Skipped: {target_path}")
    skipped_files.append(target_path)
    continue

# --conflict overwrite
if file_exists(target_path):
    print(f"↻ Overwriting: {target_path}")
```

## 출력 예시

```
프로젝트에서 CLAUDE.md 파일을 검색합니다...

발견된 CLAUDE.md 파일:
1. src/auth/CLAUDE.md
2. src/utils/CLAUDE.md

코드 생성을 시작합니다...

[1/2] src/auth/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개, 클래스 1개
✓ 테스트 생성 (5 test cases)
✓ 구현 생성
✓ 테스트 실행: 5 passed

[2/2] src/utils/CLAUDE.md
✓ CLAUDE.md 파싱 완료 - 함수 3개
✓ 테스트 생성 (3 test cases)
✓ 구현 생성
✓ 테스트 실행: 3 passed

=== 생성 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 7개
건너뛴 파일: 0개
테스트: 8 passed, 0 failed
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md 파일을 찾을 수 없습니다" 메시지 출력 |
| 파싱 오류 | 해당 파일 건너뛰고 계속 진행, 오류 로그 |
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| 테스트 실패 | 경고 표시, 수동 수정 필요 안내 |
| 파일 쓰기 실패 | 에러 로그, 해당 파일 건너뛰기 |

## 출력 파일

모든 임시 결과 파일은 scratchpad에 저장됩니다.
