---
name: incremental-generate
aliases: [igen]
description: |
  변경된 CLAUDE.md만 선택적으로 코드 생성합니다.
  Git diff 기반으로 변경을 감지하여 처리 시간을 최적화합니다.

  <example>
  <context>
  사용자가 feature 브랜치에서 src/auth/CLAUDE.md와 src/api/CLAUDE.md를 수정한 후 코드 생성을 요청합니다.
  프로젝트에는 총 7개의 CLAUDE.md가 있습니다.
  </context>
  <user_request>/incremental-generate</user_request>
  <assistant_response>
  브랜치 base commit 기준 변경 감지 중...

  === 변경 감지 결과 ===
  기준: abc1234 (main 브랜치 분기점)
  - 변경됨: 2개
  - 변경 없음: 5개 (건너뜀)

  코드 생성을 시작합니다...

  병렬로 2개 처리 중...
    • src/auth/CLAUDE.md (modified) - 시작
    • src/api/CLAUDE.md (modified) - 시작

  결과 수집 중...
  ✓ src/auth/CLAUDE.md - 5 tests passed
  ✓ src/api/CLAUDE.md - 3 tests passed

  === 생성 완료 ===
  처리: 2개 | 건너뜀: 5개 | 테스트: 8 passed
  </assistant_response>
  </example>
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# Incremental Generate Skill

## 목적

변경된 CLAUDE.md만 선택적으로 코드 생성합니다.
Git diff 기반으로 변경을 감지하여 전체 regeneration 대신 incremental 처리로 시간을 절약합니다.

## 사용법

```bash
# 기본 사용 - 브랜치 생성 이후 변경된 CLAUDE.md만 처리
/incremental-generate
/igen  # 단축 명령

# 특정 경로만
/incremental-generate --path src/auth

# 특정 commit 기준
/incremental-generate --base abc1234

# main 브랜치 기준
/incremental-generate --base main

# 새로 추가된 untracked CLAUDE.md 제외
/incremental-generate --include-untracked false

# 기존 파일 덮어쓰기
/incremental-generate --conflict overwrite
```

## 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `--path` | `.` | 처리 대상 경로 |
| `--base` | `auto` | 비교 기준 (`auto`=merge-base, 또는 특정 commit/branch) |
| `--include-untracked` | `true` | 새로 추가된 CLAUDE.md 포함 여부 |
| `--conflict` | `skip` | 기존 파일과 충돌 시 처리 방식 (`skip` \| `overwrite`) |

## 워크플로우

### 1. Diff 분석

```python
# diff-analyze Skill 호출
Skill("claude-md-plugin:diff-analyze",
      path=path,
      base=base,
      include_untracked=include_untracked)

# 결과 읽기
diff_result = read_json(".claude/diff-analyze-result.json")
```

### 2. 변경 없으면 조기 종료

```python
if len(diff_result["changed_files"]) == 0:
    print("변경된 CLAUDE.md가 없습니다.")
    print(f"전체 CLAUDE.md: {diff_result['total_claude_md_count']}개")
    print(f"기준: {diff_result['base_ref'][:8]} ({diff_result['base_description']})")
    return  # 조기 종료
```

### 3. 변경 내역 보고

```python
changed_count = len(diff_result["changed_files"])
unchanged_count = diff_result["unchanged_count"]

print(f"""
=== 변경 감지 결과 ===
기준: {diff_result['base_ref'][:8]} ({diff_result['base_description']})
- 변경됨: {changed_count}개
- 변경 없음: {unchanged_count}개 (건너뜀)
""")

# 변경된 파일 목록 표시
for file in diff_result["changed_files"]:
    print(f"  • {file['path']} ({file['status']})")
```

### 4. 언어 자동 감지

```python
def detect_language(directory):
    # 1. 기존 파일 확장자 기반
    extensions = get_file_extensions(directory)
    language = infer_language_from_extensions(extensions)
    if language:
        return language

    # 2. 부모 디렉토리 참조
    parent_lang = detect_from_parent(directory)
    if parent_lang:
        return parent_lang

    # 3. 사용자 질문
    return ask_user_for_language()
```

### 5. generator Agent 호출 (병렬 처리)

```python
# 결과 디렉토리 준비
mkdir -p .claude/generate-results

print("\n코드 생성을 시작합니다...\n")
print(f"병렬로 {changed_count}개 처리 중...")

# 모든 generator Task를 병렬로 실행
tasks = []
for file_info in diff_result["changed_files"]:
    claude_md_path = file_info["path"]
    status = file_info["status"]
    target_dir = dirname(claude_md_path)
    detected_language = detect_language(target_dir)
    output_name = target_dir.replace("/", "-").replace(".", "root")

    print(f"  • {claude_md_path} ({status}) - 시작")

    # generator Agent 병렬 실행 (run_in_background=True)
    task = Task(
        prompt=f"""
        CLAUDE.md 경로: {claude_md_path}
        대상 디렉토리: {target_dir}
        감지된 언어: {detected_language}
        충돌 처리: {conflict_mode}
        결과 파일: .claude/generate-results/{output_name}.json
        """,
        subagent_type="generator",
        run_in_background=True
    )
    tasks.append({
        "task": task,
        "path": claude_md_path,
        "output_name": output_name
    })

# 모든 Task 완료 대기 및 결과 수집
print("\n결과 수집 중...")
results = []
for task_info in tasks:
    wait_for_task(task_info["task"])
    result = read_json(f".claude/generate-results/{task_info['output_name']}.json")
    results.append(result)

    # 개별 결과 출력
    if result["success"]:
        print(f"✓ {task_info['path']} - {result['tests']['passed']} tests passed")
    else:
        print(f"✗ {task_info['path']} - 실패: {result['error']}")
```

### 6. 결과 수집 및 보고

```python
total_processed = len(results)
total_success = sum(1 for r in results if r["success"])
total_failed = total_processed - total_success
total_tests_passed = sum(r["tests"]["passed"] for r in results)
total_tests_failed = sum(r["tests"]["failed"] for r in results)
total_files_generated = sum(len(r["generated_files"]) for r in results)

print(f"""
=== 생성 완료 ===
처리: {total_processed}개 | 건너뜀: {unchanged_count}개 | 테스트: {total_tests_passed} passed
생성된 파일: {total_files_generated}개
""")

if total_failed > 0:
    print(f"⚠ 실패: {total_failed}개 - 수동 확인 필요")
```

## /generate와의 차이점

| 항목 | /generate | /incremental-generate |
|------|-----------|----------------------|
| 처리 대상 | 모든 CLAUDE.md | 변경된 CLAUDE.md만 |
| 변경 감지 | 없음 | Git diff 기반 |
| 사용 케이스 | 전체 regeneration | 브랜치 작업 중 incremental 생성 |
| 성능 | 전체 처리 | 변경분만 처리 (빠름) |

## 내부 TDD 워크플로우

generator Agent의 TDD 워크플로우는 `/generate`와 동일합니다:

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

## 오류 처리

| 상황 | 대응 |
|------|------|
| Git 저장소 아님 | "Git 저장소에서만 사용할 수 있습니다" 메시지 출력 |
| 변경 없음 | "변경된 CLAUDE.md가 없습니다" 메시지, 조기 종료 |
| base ref 없음 | "지정된 기준을 찾을 수 없습니다: {base}" 오류 |
| generator 실패 | 해당 파일 실패 기록, 다음 파일 계속 진행 |
| 테스트 실패 | 경고 표시, 수동 수정 필요 안내 |

## 출력 예시

### 변경된 파일이 있는 경우

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

### 변경된 파일이 없는 경우

```
브랜치 base commit 기준 변경 감지 중...

변경된 CLAUDE.md가 없습니다.
전체 CLAUDE.md: 7개
기준: abc1234 (main 브랜치 분기점)

💡 Tip: 모든 CLAUDE.md를 처리하려면 /generate를 사용하세요.
```

## 출력 파일

diff-analyze 결과와 generator 결과가 저장됩니다:

```
.claude/
├── diff-analyze-result.json    # diff 분석 결과
└── generate-results/
    ├── src-auth.json           # generator Agent 결과
    ├── src-new.json            # generator Agent 결과
    └── summary.json            # 전체 요약 (optional)
```
