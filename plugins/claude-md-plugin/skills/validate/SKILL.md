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
2. ERROR 이슈에 대해 AskUserQuestion으로 수정 방향 확인
3. 승인된 수정 적용 (Edit)
4. 수정 후 재검증

### 5. 통합 보고서

```
---validate-result---
status: clean | issues_found | fixed
total_modules: {n}
schema_errors: {n}
convention_issues: {n}
boundary_issues: {n}
semantic_drift: {n}
auto_fixed: {n}
---end-validate-result---
```

## DO / DON'T

**DO:**
- Phase 2 CLI 검증을 반드시 먼저 수행
- 스키마 실패 모듈은 semantic 검증에서 제외
- validator agent에게 세션 파일로 위임

**DON'T:**
- 스키마 실패 모듈에 semantic 검증 수행
- 사용자 승인 없이 문서 수정
- CLAUDE.md 직접 수정 (auto-fix는 fix-schema CLI만)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| validator agent 실패 (단일 모듈) | 경고, 나머지 계속 |
