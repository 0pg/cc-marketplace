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
        결과는 .claude/tmp/{session-id}에 저장하고 경로만 반환해주세요.
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
