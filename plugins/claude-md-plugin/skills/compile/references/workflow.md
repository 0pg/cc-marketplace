# Compile Workflow Details

## 증분 Compile 워크플로우 (--incremental)

### 전체 데이터 흐름

```
┌──────────────────────────────────────────────────────────────────┐
│ Step 1: 변경 분석                                                 │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [1] git-status-analyzer                                         │
│      │                                                           │
│      └─→ uncommitted_dirs: ["src/auth", "src/new-feature"]       │
│                │                                                 │
│                ▼                                                 │
│  [2] commit-comparator (← uncommitted_dirs 입력)                 │
│      │                                                           │
│      ├─→ outdated_dirs: ["src/utils"]                            │
│      │   (스펙 커밋 > 소스 커밋)                                   │
│      │                                                           │
│      └─→ no_source_dirs: ["src/new-module"]                      │
│          (CLAUDE.md만 있고 소스 없음)                              │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 2: 대상 결정                                                 │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  compile_targets = A ∪ B ∪ C                                     │
│                                                                  │
│  A: uncommitted_dirs    ["src/auth", "src/new-feature"]          │
│  B: outdated_dirs       ["src/utils"]                            │
│  C: no_source_dirs      ["src/new-module"]                       │
│                                                                  │
│  → compile_targets = ["src/auth", "src/new-feature",             │
│                       "src/utils", "src/new-module"]             │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 3: Compile 실행                                             │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  for each target in compile_targets:                             │
│      if target in no_source_dirs:                                │
│          # 신규 모듈 - IMPLEMENTS.md 자동 생성                     │
│          create_default_implements_md(target)                    │
│          Task(compiler, mode="create")                           │
│      else:                                                       │
│          # 기존 모듈 - 업데이트                                    │
│          Task(compiler, mode="update")                           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 4: 사후 분석                                                 │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  for each compiled_module:                                       │
│      if module not in no_source_dirs:                            │
│          # 기존 모듈만 diff 분석 (신규는 before 없음)              │
│          Skill("interface-diff")                                  │
│              Before = CLAUDE.md Exports (스펙)                    │
│              After  = Source exports (구현 결과)                   │
│              → changes, breaking_change                          │
│                                                                  │
│  if any(breaking_change):                                        │
│      Skill("dependency-tracker")                                  │
│          → 영향받는 모듈 분석                                      │
│          → 재컴파일 권장                                          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### no_source 케이스 상세

신규 모듈(CLAUDE.md만 있고 소스 없음)은 다음과 같이 처리됩니다:

```python
def handle_no_source_module(module_path):
    # 1. IMPLEMENTS.md 자동 생성 (없는 경우)
    implements_path = f"{module_path}/IMPLEMENTS.md"
    if not exists(implements_path):
        create_default_implements_md(implements_path)

    # 2. compiler Agent 호출 (신규 생성 모드)
    Task(
        prompt=f"""
        신규 모듈 생성 (no_source)
        CLAUDE.md: {module_path}/CLAUDE.md
        IMPLEMENTS.md: {implements_path}
        모드: create (기존 소스 없음)
        """,
        subagent_type="compiler"
    )

    # 3. interface-diff 건너뜀
    # 이유: before 상태가 없음 (신규 모듈이므로 이전 구현 없음)

    # 4. 결과 보고
    # "신규 생성됨" 으로 분류
```

---

## 언어 자동 감지

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

## IMPLEMENTS.md 존재 확인 및 자동 생성

```python
for claude_md_path in target_files:
    target_dir = dirname(claude_md_path)
    implements_md_path = f"{target_dir}/IMPLEMENTS.md"

    # IMPLEMENTS.md 없으면 자동 생성
    if not file_exists(implements_md_path):
        print(f"  ⚠ {implements_md_path} 없음 - 자동 생성")
        # 기본 Planning Section으로 IMPLEMENTS.md 생성
        create_default_implements_md(implements_md_path)
```

## compiler Agent 병렬 호출

```python
# 모든 compiler Task를 병렬로 실행
tasks = []
for claude_md_path in target_files:
    target_dir = dirname(claude_md_path)
    implements_md_path = f"{target_dir}/IMPLEMENTS.md"
    detected_language = detect_language(target_dir)
    output_name = target_dir.replace("/", "-").replace(".", "root")

    print(f"  • {claude_md_path} - 시작")

    # compiler Agent 병렬 실행 (run_in_background=True)
    task = Task(
        prompt=f"""
        CLAUDE.md 경로: {claude_md_path}
        IMPLEMENTS.md 경로: {implements_md_path}
        대상 디렉토리: {target_dir}
        감지된 언어: {detected_language}
        충돌 처리: {conflict_mode}
        결과는 .claude/tmp/{session-id}-compile-{target}.json 형태로 저장하고 경로만 반환해주세요.
        """,
        subagent_type="compiler",
        run_in_background=True
    )
    tasks.append(task)
```

## 결과 수집 및 보고

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

## 파일 충돌 처리 로직

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

## 내부 TDD 워크플로우

사용자에게 노출되지 않는 내부 프로세스:

```
CLAUDE.md + IMPLEMENTS.md 파싱
     │
     ▼
[RED] behaviors → 테스트 코드 생성 (실패 확인)
     │
     ▼
[GREEN] 구현 생성 + 테스트 통과 (최대 3회 재시도)
     │   └─ IMPLEMENTS.md Planning Section 참조
     ▼
[REFACTOR] 프로젝트 컨벤션 적용 + 회귀 테스트
     │
     ▼
파일 충돌 처리
     │
     ▼
IMPLEMENTS.md Implementation Section 업데이트
     │   - Algorithm, Key Constants, Error Handling
     │   - State Management, Implementation Guide
     ▼
결과 반환
```

---

## Post-Compile 검증 + Self-Healing

### 전체 흐름

```
결과 수집 완료
     │
     ▼
┌─────────────────────────────────────────────────┐
│ Post-Compile 검증 + Self-Healing                │
│                                                 │
│ retry_count = 0                                 │
│ WHILE retry_count < 3:                          │
│   1. 검증 실행 (drift/export-validator 병렬)    │
│   2. 이슈 분류: compile_related vs unrelated    │
│   3. if 모두 "양호": break                      │
│   4. compile_related → 자동 healing → 재컴파일 │
│   5. unrelated만 → AskUserQuestion → 선택 적용 │
│   6. retry_count++                              │
└─────────────────────────────────────────────────┘
     │
     ▼
최종 검증 결과 보고
```

### 이슈 분류 기준

compile 결과의 `generated_files`, `modified_symbols`를 validator 이슈와 비교하여 분류합니다.

| Validator 이슈 | 변경된 파일/export에 해당? | 처리 |
|---------------|-------------------------|------|
| MISSING export | O (이번에 생성된 함수) | 자동 healing |
| MISMATCH signature | O (이번에 수정된 함수) | 자동 healing |
| UNCOVERED file | O (이번에 생성된 파일) | 자동 healing |
| MISSING export | X (기존 함수) | AskUserQuestion |
| MISMATCH signature | X (기존 함수) | AskUserQuestion |
| UNCOVERED file | X (기존 파일) | AskUserQuestion |

### 자동 Healing 흐름 (compile_related_issues)

```
1. 이슈 분석
   "이번 compile에서 생성한 validateToken 함수의 시그니처가 CLAUDE.md와 다름"

2. CLAUDE.md 맥락 추가:
   - Domain Context에 변경 이유 추가
   - Exports 시그니처 업데이트

3. 재컴파일 (compiler Agent 재호출)

4. 재검증
```

### 수동 Healing 흐름 (unrelated_issues)

```
1. AskUserQuestion:
   "기존 코드에서 발견된 이슈입니다:
    - src/legacy/helper.ts: parseDate export가 CLAUDE.md에 없음

    어떻게 처리할까요?"

   옵션:
   - CLAUDE.md 수정 (코드에 맞춤) - Recommended
   - 코드 수정 (CLAUDE.md에 맞춤)
   - 무시하고 진행

2. 선택에 따라 처리:

   [CLAUDE.md 수정]
   → CLAUDE.md의 Structure/Exports 섹션 업데이트
   → 기존 코드에 맞게 문서를 수정

   [코드 수정]
   → Skill("compile") 재실행
   → CLAUDE.md 스펙에 맞게 코드를 재생성
   → 기존 코드가 스펙과 다르면 덮어쓰기

   [무시하고 진행]
   → WARNING 상태로 종료
   → 이슈는 남아있지만 compile 완료 처리
```

### 이슈 분류 로직 (CLASSIFY_ISSUES)

```
INPUT: validator_issues, compile_result
OUTPUT: compile_related[], unrelated[]

generated_files = compile_result.generated_files
modified_symbols = compile_result에서 생성된 심볼들 추출

FOR EACH issue IN validator_issues:
    IF issue.file IN generated_files OR issue.symbol IN modified_symbols:
        compile_related에 추가
    ELSE:
        unrelated에 추가

RETURN compile_related, unrelated
```

### 통합 재시도 흐름

```
retry_count = 0
WHILE retry_count < 3:
    validation_results = VALIDATE(compiled_nodes)

    # 상태 판정: "양호"가 아니면 모두 healing 대상
    IF ALL nodes are "양호":
        RETURN SUCCESS

    # 이슈 추출 (개선 권장 + 개선 필요 모두 대상)
    issues = EXTRACT_ISSUES(validation_results)

    # 이슈 분류
    compile_related, unrelated = CLASSIFY_ISSUES(issues, compile_result)

    # 1. compile_related 이슈: 자동 healing
    IF compile_related 있음:
        CLAUDE.md 맥락 추가
        재컴파일 실행
        retry_count++
        CONTINUE  # 재검증으로 돌아감

    # 2. unrelated 이슈만 있음: 사용자 확인
    IF unrelated 있음:
        choice = AskUserQuestion(
            "CLAUDE.md 수정 (코드에 맞춤)",      # → CLAUDE.md 편집
            "코드 수정 (CLAUDE.md에 맞춤)",      # → compile skill 재실행
            "무시하고 진행"
        )

        IF choice == "무시":
            RETURN WARNING

        IF choice == "CLAUDE.md 수정":
            CLAUDE.md 편집 (Structure/Exports 섹션 업데이트)

        IF choice == "코드 수정":
            Skill("compile") 재실행  # CLAUDE.md 스펙에 맞게 코드 재생성

        retry_count++

RETURN WARNING  # 3회 후에도 실패
```
