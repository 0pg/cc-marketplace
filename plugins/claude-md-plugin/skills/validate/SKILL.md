---
name: validate
version: 4.0.0
aliases: [check, verify, lint, fix-drift, handle-violation, resolve]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "lint documentation",
  "resolve drift", "fix documentation mismatch", "sync docs with code", or uses "/validate".
  Runs deterministic CLI validation (schema, convention structure, boundary, DEVELOPERS.md existence)
  and semantic validator agent for comprehensive drift detection, with interactive auto-fix.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, 문서 린트, drift 해소, 위반 해소
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

# /validate

CLAUDE.md와 실제 코드 간의 일치 여부를 검증하고, 발견된 이슈를 대화형으로 해소합니다.

## Triggers

- `/validate`
- `CLAUDE.md 검증`
- `drift 검사`
- `drift 해소`
- `위반 해소`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 |
| `--strict` | 아니오 | false | DEVELOPERS.md 부재를 error로 취급, DEVELOPERS.md 내용 drift 검증 활성화 |
| `--report-only` | 아니오 | false | 검증만 수행, auto-fix 스킵 (/compile --validate 내부용) |

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

수집된 파일이 없으면: "대상 경로에 CLAUDE.md가 없습니다." → 종료.

### 2. Deterministic 검증 (CLI only)

#### 2a. 스키마 검증 + auto-fix

각 CLAUDE.md에 대해 스키마 검증:

```bash
$CLI_PATH validate-schema --file "$claude_md" --output "${TMP_DIR}schema-${dir_safe}.json"
```

실패 시 auto-fix 시도:
```bash
$CLI_PATH fix-schema --file "$claude_md"
# 재검증
$CLI_PATH validate-schema --file "$claude_md" --output "${TMP_DIR}schema-${dir_safe}.json"
```

auto-fix 후에도 실패하면 해당 모듈을 스키마 오류로 보고하고 Phase 3 대상에서 제외.

#### 2b. Convention 구조 검증

스키마 통과한 모듈에 대해 Convention 섹션 구조를 검증:

```bash
$CLI_PATH validate-convention --project-root {project_root} --output "${TMP_DIR}convention-result.json"
```

결과에서 `MISSING_CONVENTION`, `MISSING_SUBSECTION` 이슈를 수집합니다.
Convention 구조 이슈는 보고만 하며 Phase 3 진행을 차단하지 않습니다.

#### 2c. Boundary 검증

스키마 통과한 각 디렉토리에 대해 트리 구조 의존성을 검증:

```bash
$CLI_PATH resolve-boundary --path {dir} --claude-md {dir}/CLAUDE.md --output "${TMP_DIR}boundary-${dir_safe}.json"
```

결과의 `violations` 배열에서 `PARENT_REFERENCE`, `SIBLING_REFERENCE` 이슈를 수집합니다.

#### 2d. DEVELOPERS.md 존재 확인 (INV-3)

각 CLAUDE.md 디렉토리에 DEVELOPERS.md가 존재하는지 확인:
- 부재 시: `--strict`면 ERROR, 아니면 WARNING
- 결과를 Phase 2 deterministic results에 통합

#### Phase 2 결과 저장

모든 Deterministic 검증 결과를 저장:
```bash
# ${TMP_DIR}deterministic-results.json에 schema/convention/boundary/developers-md 결과 통합
```

**Gate 규칙**: Schema 실패 모듈만 Phase 3 스킵. Convention/Boundary/DEVELOPERS.md 결과는 보고만.

### 3. Semantic 검증 (validator agent)

**스킵 조건**: `--report-only`이면 Phase 3 전체 스킵.

스키마 통과한 CLAUDE.md 디렉토리를 배치로 나누어 `Task(validator)` 실행.

**배치 규칙**: 최대 3개 디렉토리를 병렬 처리.

각 배치:
```
Task(validator): "검증 대상: {directory}\nstrict: {true|false}"
```

validator agent가 검증하는 카테고리:
1. **Requirements Drift** — 코드가 명시된 요구사항을 위반/미적용
2. **Convention CODE_VIOLATION** — **architectural 규칙만** (의존성 방향, 패턴 준수 등).
   syntactic 규칙(네이밍, 포맷)은 린터 영역이므로 검증하지 않음.
3. **DEVELOPERS.md Content Drift** — `--strict`에서만 실행.
   Constraints/Technical Context와 코드 불일치 검증.

결과를 `${TMP_DIR}validate-progress.jsonl`에 누적:
```bash
echo '{"directory":"{dir}","issues":{n},"status":"{status}"}' >> "${TMP_DIR}validate-progress.jsonl"
```

### 4. Auto-fix (Interactive)

**스킵 조건**: `--report-only`이면 스킵 → Phase 5(보고서)로 직행.

Phase 2+3에서 탐지된 이슈를 그룹화하여 사용자에게 제시합니다.

#### 4a. Deterministic fix (자동)

| 이슈 | 처리 |
|------|------|
| Schema FAIL (fix-schema 성공) | Phase 2a에서 이미 자동 수정 |
| DEVELOPERS.md MISSING | 목록만 보고, 사용자에게 일괄 /decompile 제안 (Phase 4c) |

#### 4b. Semantic fix (대화형)

탐지된 semantic drift 이슈를 모듈별로 그룹화하여 AskUserQuestion:

| Drift 유형 | 해소 옵션 |
|------------|----------|
| Requirements VIOLATED | Fix Code (`/compile --conflict overwrite`), Fix Doc (CLAUDE.md 수정), Skip |
| Requirements STALE | Remove (제약 삭제), Keep (유지) |
| Convention CODE_VIOLATION | Fix Code (코드 수정), Skip |
| DEVELOPERS.md CONSTRAINTS_STALE | Fix Doc, Skip |

Fix Code 선택 시:
  Skill("claude-md-plugin:compile", args: "--path {module_path} --conflict overwrite")

Fix Doc 선택 시:
  AskUserQuestion으로 구체적 변경 확인 → Edit으로 CLAUDE.md/DEVELOPERS.md 수정

#### 4c. DEVELOPERS.md 일괄 생성

MISSING 목록이 있으면:
  AskUserQuestion: "DEVELOPERS.md가 없는 {n}개 모듈에 대해 생성하시겠습니까?"
  옵션: [생성 (/decompile), 건너뛰기]

#### 4d. 재검증 (선택)

fix가 1건 이상 실행된 경우:
  AskUserQuestion: "재검증을 실행하시겠습니까?"
  → 실행 시 Phase 2부터 재실행 (--report-only 모드)

### 5. 통합 보고서 생성

Phase 2 (Deterministic) 결과와 Phase 3 (Semantic) 결과, Phase 4 (Auto-fix) 결과를 병합하여 단일 보고서를 생성합니다:

```markdown
# Validation Report

## 요약

| 지표 | 값 |
|------|-----|
| 검증 대상 | {total}개 모듈 |
| 스키마 통과 | {schema_pass}/{total} |
| Drift 없음 | {clean}/{schema_pass} |
| 총 이슈 | {total_issues}개 |
| Auto-fix | {fixed}/{total_issues}개 해소 |

## 스키마 오류

| 모듈 | 오류 |
|------|------|
| {path} | {error_message} |

## Drift 이슈

### {module_path}

| 카테고리 | 유형 | 설명 | 신뢰도 | 해소 |
|----------|------|------|--------|------|
| Requirements | VIOLATED | {description} | MEDIUM | {Fix Code / Fix Doc / Skip / -} |
| Requirements | STALE | {description} | LOW | {Remove / Keep / -} |
| Convention | MISSING_CONVENTION | project_root에 Conventions 없음 | HIGH | - |
| Convention | MISSING_SUBSECTION | 필수 서브섹션 없음 | HIGH | - |
| Convention | CODE_VIOLATION | {description} | MEDIUM | {Fix Code / Skip / -} |
| DEVELOPERS.md | MISSING | DEVELOPERS.md 부재 | HIGH | {Generate / Skip / -} |
| DEVELOPERS.md | CONSTRAINTS_STALE | {description} | MEDIUM | {Fix Doc / Skip / -} |
| Boundary | PARENT_REFERENCE | 부모 참조 위반 | HIGH | - |
| Boundary | SIBLING_REFERENCE | 형제 참조 위반 | HIGH | - |
```

보고서를 `${TMP_DIR}validate-report.md`에 저장하고 사용자에게 출력합니다.

결과 블록:
```
---validate-result---
status: clean | issues_found | issues_resolved
modules: {total}
schema_pass: {n}/{total}
drift_free: {n}/{schema_pass}
total_issues: {n}
auto_fixed: {n}
detail_file: ${TMP_DIR}validate-report.md
---end-validate-result---
```

## DO / DON'T

**DO:**
- 스키마 검증 → auto-fix → drift 검증 순서
- validator agent 병렬 실행 (최대 3개)
- 결과를 파일로 저장
- Phase 4에서 사용자 승인 후 fix 실행

**DON'T:**
- 사용자 승인 없이 CLAUDE.md나 소스코드 수정
- 스키마 실패한 모듈에 drift 검증 수행
- 사용자에게 각 validator의 진행 상황 중계

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| validator agent 실패 | 해당 모듈 스킵, 경고 |
| auto-fix 실패 | 스키마 오류로 보고 |

## Examples

<example>
<user_request>/validate src</user_request>
<assistant_response>
CLAUDE.md 4개 수집 완료.

스키마 검증: 4/4 통과
DEVELOPERS.md 존재: 3/4 (src/legacy MISSING — WARNING)
Drift 검증 진행 중... (배치 1/2)

5개 이슈 발견:

[1/5] src/auth: Requirements VIOLATED — "토큰 만료 최대 7일" vs 코드 14일
  해소 방법: [Fix Code / Fix Doc / Skip]
→ Fix Code

/compile 실행 중... 완료.

[2/5] src/utils: Requirements STALE — "Redis 캐시 TTL" 관련 코드 없음
  해소 방법: [Remove / Keep]
→ Remove

CLAUDE.md에서 해당 Requirement 삭제 완료.

[3/5] src/legacy: Convention CODE_VIOLATION — 의존성 방향 규칙 위반
  해소 방법: [Fix Code / Skip]
→ Skip

[4/5] src/legacy: Boundary PARENT_REFERENCE — ../utils 참조 발견
  (deterministic 이슈 — 수동 해소 필요)

[5/5] DEVELOPERS.md가 없는 1개 모듈(src/legacy)에 대해 생성하시겠습니까?
  [생성 / 건너뛰기]
→ 건너뛰기

재검증을 실행하시겠습니까? [실행 / 건너뛰기]
→ 건너뛰기

Validation Report
=================

| 지표 | 값 |
|------|-----|
| 검증 대상 | 4개 모듈 |
| 스키마 통과 | 4/4 |
| Drift 없음 | 1/4 |
| 총 이슈 | 5개 |
| Auto-fix | 2/5개 해소 |
</assistant_response>
</example>
