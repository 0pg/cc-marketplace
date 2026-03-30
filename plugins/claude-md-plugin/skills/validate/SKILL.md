---
name: validate
version: 1.0.0
aliases: [check, verify, lint, fix-drift, resolve]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "lint documentation",
  "resolve drift", "fix documentation mismatch", "sync docs with code", or uses "/validate".
  Runs deterministic CLI validation (schema, convention structure, boundary, DEVELOPERS.md existence)
  and semantic validator agent for comprehensive drift detection, with interactive auto-fix.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, drift 해소
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

# /validate

CLAUDE.md와 실제 코드 간의 일치 여부를 검증하고, 발견된 이슈를 대화형으로 해소합니다.

## Triggers

- `/validate`
- `CLAUDE.md 검증`
- `drift 검사`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 |
| `--strict` | 아니오 | false | DEVELOPERS.md 부재를 error로, 내용 drift 검증 활성화 |
| `--report-only` | 아니오 | false | 검증만 수행, auto-fix 스킵 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. CLAUDE.md 수집

```
Glob("{path}/**/CLAUDE.md")
```

수집 파일 없으면: "대상 경로에 CLAUDE.md가 없습니다." → 종료.

### 2. Deterministic 검증 (CLI only, no agent)

#### 2a. 스키마 검증 + auto-fix

각 CLAUDE.md에 대해:

```bash
$CLI_PATH validate-schema --file "$claude_md" --dir "$dir" --output "${TMP_DIR}schema-${dir_safe}.json"
```

실패 시 auto-fix:
```bash
$CLI_PATH fix-schema --file "$claude_md"
$CLI_PATH validate-schema --file "$claude_md" --dir "$dir" --output "${TMP_DIR}schema-${dir_safe}.json"
```

auto-fix 후에도 실패 → 스키마 오류 보고, Phase 3 대상 제외.

#### 2b. Convention 구조 검증

```bash
$CLI_PATH validate-convention --project-root {project_root} --output "${TMP_DIR}convention-result.json"
```

`MISSING_CONVENTION`, `MISSING_SUBSECTION` 이슈 수집. Phase 3 차단하지 않음.

#### 2c. Boundary 검증

스키마 통과한 각 디렉토리:
```bash
$CLI_PATH resolve-boundary --path {dir} --claude-md {dir}/CLAUDE.md --output "${TMP_DIR}boundary-${dir_safe}.json"
```

`PARENT_REFERENCE`, `SIBLING_REFERENCE` 이슈 수집.

#### 2d. DEVELOPERS.md 존재 확인 (INV-3)

각 CLAUDE.md 디렉토리에 DEVELOPERS.md 존재 확인:
- 부재 시: `--strict`면 ERROR, 아니면 WARNING

### 2.5 변경 스펙 + 테스트 커버리지 맵 구성

스키마 통과한 각 대상에 대해:

#### 2.5a. 변경 스펙 탐지

```bash
$CLI_PATH diff-spec-range --file {dir}/CLAUDE.md --root {project_root} \
  --output "${TMP_DIR}spec-diff-${dir_safe}.json"
```

결과 필드: `changed_requirements[]`, `source_changed_files[]`, `source_changed`, `all_requirements`

- `all_requirements=true`: git 저장소 아님 또는 첫 커밋 → 전체 Requirements 검증 대상
- `source_changed=false` AND `changed_requirements` 비어 있음 → 변경 없음, semantic 검증 스킵 가능

#### 2.5b. 테스트 커버리지 맵 구성

**Step 1: target_dir 범위 필터링 + source_changed 재판정**

`spec-diff-${dir_safe}.json`의 `source_changed_files`에서 `{target_dir}` 하위 파일만 필터:
```
target_source_files = source_changed_files.filter(f => f.startsWith({target_dir}))
```
- `all_requirements=true`이면: `target_source_files` = Glob(`{target_dir}/**/*.{rs,ts,js,py}`) 전체
- `target_source_files` 비어있고 `changed_requirements` 비어있음 → **semantic 검증 스킵** (모듈 내 실제 변경 없음)

**Step 2: 소스 파일별 공개 함수 추출**

각 `target_source_file`에 대해 언어별 패턴으로 공개 함수 목록 추출:

| 언어 | Grep 패턴 | 추출 대상 |
|------|----------|---------|
| Rust | `^pub fn \|^    pub fn ` in {source_file} | fn 이름 |
| TypeScript/JS | `^export (function\|const\|async function) ` in {source_file} | 심볼 이름 |
| Python | `^def [^_]\|^    def [^_]` in {source_file} | fn 이름 (private _ 제외) |

결과: `public_fns = [fn_name, ...]` per source_file

**Step 3: 테스트 파일 탐색**

언어별 탐색 경로 (target_dir 내부 + project_root/tests/ 통합 테스트 포함):

| 언어 | 탐색 경로 | 테스트 식별 패턴 |
|------|----------|--------------|
| Rust | `{target_dir}/**/*.rs` + `{project_root}/tests/**/*.rs` | `#[test]` (-A 1 → fn 이름) |
| TypeScript/JS | `{target_dir}/**/*.{test,spec}.{ts,js}` + `{project_root}/**/__tests__/**` | `it\(\|test\(\|describe\(` (-A 1 → 이름) |
| Python | `{target_dir}/**/{test_*.py,*_test.py}` + `{project_root}/tests/**/*.py` | `^def test_` (-A 1 → fn 이름) |

`test_files_found` = 탐색된 테스트 파일 수

**Step 4: 함수 참조 확인 (calls[] 채우기)**

각 테스트 케이스 fn 내에서 Step 2의 `public_fns` 항목을 Grep:
```
for each test_fn in test_cases:
  calls = [fn for fn in public_fns if Grep(fn, in test_fn body)]
```

결과 구조 (모듈별 JSON):
```json
[
  {
    "source_file": "src/agent/mod.rs",
    "public_fns": ["spawn_agent", "AgentResult"],
    "test_files_found": 1,
    "test_cases": [
      { "name": "test_spawn_agent_success", "calls": ["spawn_agent"], "line": "tests/agent_test.rs:15" }
    ]
  },
  {
    "source_file": "src/tracker/linear.rs",
    "public_fns": ["update_issue"],
    "test_files_found": 0,
    "test_cases": []
  }
]
```

테스트 파일 없으면: `test_files_found: 0` 기록 (TEST_MISSING 신호).

### 3. 세션 파일 생성 + Semantic 검증 (validator agent)

스키마 통과한 각 대상에 대해:

1. CLAUDE.md 파싱:
```bash
$CLI_PATH parse-claude-md --file {dir}/CLAUDE.md --output "${TMP_DIR}parsed-${dir_safe}.json"
```

2. Convention 계층 해소 (project > module)

3. DEVELOPERS.md 읽기 (strict 시)

4. 세션 파일 Write → `${TMP_DIR}validate-session-{dir-safe}.md`:

```markdown
# Validate Task: {path}
type: validate | target: {path} | strict: {true|false}

## CLAUDE.md Content
Purpose: {parsed purpose}
Requirements:
{parsed requirements list}
Domain Context: {parsed domain context}

## Conventions (resolved)
{계층 해소된 Conventions}

## DEVELOPERS.md Content (strict only)
Constraints:
{parsed constraints}
Technical Context:
{parsed technical context}

## Deterministic Results
{Phase 2에서 발견된 CLI 이슈 요약}

## Changed Requirements (diff-spec-range 결과)
all_requirements: {true|false}
source_changed: {true|false}  ← target_dir 범위로 필터링된 값
추가/변경: {changed_requirements list — action + text}
변경된 소스 파일 (target_dir 내): {target_source_files list}

## Test Coverage Map
{2.5b에서 Grep으로 구성한 JSON 배열}
모듈 범위 한정: {target directory}만 포함
```

5. `Task(validator)` 디스패치 (병렬 배치, 최대 3개):
```
세션 파일: ${TMP_DIR}validate-session-{dir-safe}.md
검증 대상: {directory}
strict: {true|false}
```

### 4. Auto-fix (Interactive)

`--report-only`가 아니면:

1. 전체 이슈 목록 출력 (deterministic + semantic)
2. ERROR/WARNING 이슈에 대해 drift 유형별 해소 방향 제시 + AskUserQuestion:

   **해소 방향 규칙 (CLAUDE.md = SSOT 원칙):**

   | Drift Type | 기본 해소 | 사용자 선택지 |
   |---|---|---|
   | REQUIREMENTS_NOT_IMPLEMENTED | /compile 권장 | (a) /compile 재생성 (b) CLAUDE.md에서 요구사항 제거 |
   | REQUIREMENTS_PARTIALLY_IMPLEMENTED | /compile 권장 | (a) /compile 재생성 (b) 현재 상태에 맞게 CLAUDE.md 조정 |
   | REQUIREMENTS_VIOLATED | 사용자 판단 필수 | (a) 코드가 틀림 → /compile (b) 요구사항이 바뀜 → CLAUDE.md 업데이트 |
   | CONVENTION_*_VIOLATION | 코드 수정 | (a) 코드 수정 (b) Convention 규칙 완화 |
   | CONSTRAINT_NOT_ENFORCED | 코드 수정 | (a) 코드 수정 (b) DEVELOPERS.md Constraint 업데이트 |
   | TECH_CONTEXT_STALE | 문서 수정 | DEVELOPERS.md 자동 업데이트 |

   **수정 범위 제한:**
   - validate의 직접 Edit: 기존 코드의 수정 (기존 함수/구조체 내 변경)
   - 신규 코드 생성 (새 파일, 새 함수, 새 모듈): "/compile {path}로 재생성하세요" 안내만 제공, validate 내에서 /compile 자동 호출하지 않음
   - CLAUDE.md/DEVELOPERS.md 수정: 사용자 명시적 승인 후에만

3. 승인된 수정 적용
4. 수정 후 재검증

### 5. 통합 보고서

Task(validator) 결과 block에서 `result_file` 경로를 수집하여 목록으로 포함:

```
---validate-result---
status: clean | issues_found | fixed
total_modules: {n}
schema_errors: {n}
convention_issues: {n}
boundary_issues: {n}
semantic_drift: {n}
auto_fixed: {n}
result_files:
  - {TMP_DIR}validate-{dir-safe}.md
  - {TMP_DIR}validate-{dir-safe-2}.md
  (스키마 통과하여 semantic 검증이 실행된 모듈만. 없으면 생략)
---end-validate-result---
```

## DO / DON'T

**DO:**
- Phase 2 CLI 검증을 반드시 먼저 수행
- 스키마 실패 모듈은 semantic 검증에서 제외
- validator agent에게 세션 파일로 위임

**DON'T:**
- 스키마 실패 모듈에 semantic 검증 수행
- 사용자 승인 없이 CLAUDE.md/DEVELOPERS.md 수정 (auto-fix는 fix-schema CLI 또는 사용자 명시적 승인 시에만)
- 신규 코드 생성 (새 파일, 새 함수, 새 모듈 — /compile의 역할)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| validator agent 실패 (단일 모듈) | 경고, 나머지 계속 |
