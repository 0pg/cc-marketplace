---
name: compile
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement spec",
  "create source files", or uses "/compile". Default: incremental (changed only), use --all for full processing.
  Performs TDD workflow (RED→GREEN→REFACTOR) to ensure compiled code passes tests.

  <example>
  <context>
  사용자가 feature 브랜치에서 src/auth/CLAUDE.md를 수정한 후 코드 생성을 요청합니다.
  프로젝트에는 총 5개의 CLAUDE.md가 있습니다.
  </context>
  <user_request>/compile</user_request>
  <assistant_response>
  브랜치 base commit 기준 변경 감지 중...

  === 변경 감지 결과 ===
  기준: abc1234 (main 브랜치 분기점)
  - 변경됨: 1개
  - 변경 없음: 4개 (건너뜀)

  코드 생성을 시작합니다...

  [1/1] src/auth/CLAUDE.md (modified)
  ✓ CLAUDE.md 파싱 완료 - 함수 2개, 타입 2개
  ✓ 테스트 생성
  ✓ 구현 생성
  ✓ 테스트 실행: 5 passed

  === 생성 완료 ===
  처리: 1개 | 건너뜀: 4개 | 테스트: 5 passed
  </assistant_response>
  </example>

  <example>
  <context>
  사용자가 프로젝트의 모든 CLAUDE.md를 처리하려고 합니다.
  </context>
  <user_request>/compile --all</user_request>
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

## 목적

CLAUDE.md 파일을 기반으로 소스 코드를 생성합니다.
CLAUDE.md가 명세(specification)가 되고, 소스 코드가 산출물이 됩니다.

**기본 동작은 incremental** - 변경된 CLAUDE.md만 처리하여 시간을 절약합니다.

## 사용법

```bash
# 기본 사용 (변경분만 처리 - incremental)
/compile

# 전체 CLAUDE.md 처리
/compile --all

# 특정 경로만 처리
/compile --path src/auth

# 특정 commit 기준으로 변경 감지
/compile --base abc1234

# 기존 파일 덮어쓰기
/compile --conflict overwrite
```

## 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--all` | `false` | 전체 CLAUDE.md 처리 (변경 감지 무시) |
| `--path` | `.` | 처리 대상 경로 |
| `--base` | `auto` | 비교 기준 (`--all` 시 무시) |
| `--include-untracked` | `true` | untracked 포함 (`--all` 시 무시) |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 (`skip` \| `overwrite`) |

## 워크플로우

```
/compile
    │
    ├─ --all 플래그? ─ Yes ─→ 모든 CLAUDE.md 검색
    │                           │
    └─ No ─→ Skill("diff-analyze")
               │
               ├─ 변경 없음 → 조기 종료
               └─ 변경 있음 → 변경된 파일만
                               │
    ←───────────────────────────┘
    │
    ▼
병렬 처리 (run_in_background=True)
    │
    ▼
결과 수집 및 보고
```

### 1. 대상 파일 결정

#### --all 모드 (전체 처리)

```bash
# 지정 경로 하위의 모든 CLAUDE.md 찾기
find {path} -name "CLAUDE.md" -type f | sort
```

#### 기본 모드 (incremental)

```python
# diff-analyze Skill 호출
Skill("claude-md-plugin:diff-analyze",
      path=path,
      base=base,
      include_untracked=include_untracked)

# 결과 읽기
diff_result = read_json(".claude/diff-analyze-result.json")

# 변경 없으면 조기 종료
if len(diff_result["changed_files"]) == 0:
    print("변경된 CLAUDE.md가 없습니다.")
    print(f"전체 CLAUDE.md: {diff_result['total_claude_md_count']}개")
    print(f"기준: {diff_result['base_ref'][:8]} ({diff_result['base_description']})")
    print("\n💡 Tip: 모든 CLAUDE.md를 처리하려면 /compile --all을 사용하세요.")
    return  # 조기 종료

# 변경 내역 보고
print(f"""
=== 변경 감지 결과 ===
기준: {diff_result['base_ref'][:8]} ({diff_result['base_description']})
- 변경됨: {len(diff_result['changed_files'])}개
- 변경 없음: {diff_result['unchanged_count']}개 (건너뜀)
""")
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
# 결과 디렉토리 준비
mkdir -p .claude/compile-results

# 모든 compiler Task를 병렬로 실행
tasks = []
for file_info in target_files:
    claude_md_path = file_info["path"] if isinstance(file_info, dict) else file_info
    status = file_info.get("status", "all") if isinstance(file_info, dict) else "all"
    target_dir = dirname(claude_md_path)
    detected_language = detect_language(target_dir)
    output_name = target_dir.replace("/", "-").replace(".", "root")

    print(f"  • {claude_md_path} ({status}) - 시작")

    # compiler Agent 병렬 실행 (run_in_background=True)
    task = Task(
        prompt=f"""
        CLAUDE.md 경로: {claude_md_path}
        대상 디렉토리: {target_dir}
        감지된 언어: {detected_language}
        충돌 처리: {conflict_mode}
        결과 파일: .claude/compile-results/{output_name}.json
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

# --all 모드
if all_mode:
    print(f"""
=== 생성 완료 ===
총 CLAUDE.md: {len(target_files)}개
생성된 파일: {total_files}개
건너뛴 파일: {total_skipped}개
테스트: {total_tests_passed} passed, {total_tests_failed} failed
""")
# incremental 모드
else:
    print(f"""
=== 생성 완료 ===
처리: {len(target_files)}개 | 건너뜀: {unchanged_count}개 | 테스트: {total_tests_passed} passed
생성된 파일: {total_files}개
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
[GREEN] 구현 생성 + 테스트 통과 (최대 5회 재시도)
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

### Incremental 모드 (기본)

```
브랜치 base commit 기준 변경 감지 중...

=== 변경 감지 결과 ===
기준: abc1234 (main 브랜치 분기점)
- 변경됨: 2개
- 변경 없음: 5개 (건너뜀)

  • src/auth/CLAUDE.md (modified)
  • src/new/CLAUDE.md (added)

코드 생성을 시작합니다...

병렬로 2개 처리 중...
  • src/auth/CLAUDE.md (modified) - 시작
  • src/new/CLAUDE.md (added) - 시작

결과 수집 중...
✓ src/auth/CLAUDE.md - 5 tests passed
✓ src/new/CLAUDE.md - 3 tests passed

=== 생성 완료 ===
처리: 2개 | 건너뜀: 5개 | 테스트: 8 passed
생성된 파일: 6개
```

### --all 모드

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

상세 결과: .claude/compile-results/
```

### 변경 없는 경우 (incremental 모드)

```
브랜치 base commit 기준 변경 감지 중...

변경된 CLAUDE.md가 없습니다.
전체 CLAUDE.md: 7개
기준: abc1234 (main 브랜치 분기점)

💡 Tip: 모든 CLAUDE.md를 처리하려면 /compile --all을 사용하세요.
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md 파일을 찾을 수 없습니다" 메시지 출력 |
| 파싱 오류 | 해당 파일 건너뛰고 계속 진행, 오류 로그 |
| 언어 감지 실패 | 사용자에게 언어 선택 질문 |
| 테스트 실패 | 경고 표시, 수동 수정 필요 안내 |
| 파일 쓰기 실패 | 에러 로그, 해당 파일 건너뛰기 |
| Git 저장소 아님 (incremental) | "Git 저장소에서만 incremental 모드를 사용할 수 있습니다. --all 옵션을 사용하세요." |
| base ref 없음 | "지정된 기준을 찾을 수 없습니다: {base}" 오류 |

## 출력 파일

```
.claude/
├── diff-analyze-result.json    # diff 분석 결과 (incremental 모드)
└── compile-results/
    ├── src-auth.json           # compiler Agent 결과
    ├── src-utils.json          # compiler Agent 결과
    └── summary.json            # 전체 요약
```
