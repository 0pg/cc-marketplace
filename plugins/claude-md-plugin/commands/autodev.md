---
name: autodev
description: |
  Use when the user wants to autonomously develop a feature end-to-end without manual steps.
  Runs requirements → CLAUDE.md → code → validation loop until complete.
  단계별 명령 없이 요구사항만 주면 처음부터 끝까지 자율 실행.
  Trigger keywords: 자동 개발, 처음부터 끝까지, 자율 구현
argument-hint: '"requirement" [--path path] [--max-iter N]'
allowed-tools: [Read, Glob, Write, Task, AskUserQuestion, Bash, Skill]
---

# /autodev

요구사항을 처음부터 끝까지 자율 실행합니다.
스펙 정의(impl) → 코드 생성(compile) → 검증(validate) 루프를 모두 자율적으로 완료합니다.

**최초 요구사항 확인 1회를 제외하고 사람의 개입 없이 완료.**

## Triggers

- `/autodev`
- `자동 개발`
- `처음부터 끝까지 구현해줘`
- `자율 구현`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `requirement` | 예* | - | 구현할 요구사항 텍스트 |
| `--path` | 아니오 | `.` | 대상 경로 |
| `--max-iter` | 아니오 | `5` | compile-validate 사이클 최대 횟수 |

\* 요구사항 없으면 AskUserQuestion으로 1회 수집.

## /impl --auto 와의 차이

| 항목 | /impl --auto | /autodev |
|------|-------------|----------|
| 요구사항 확인 | brainstorming + 최대 2회 질문 | 최대 1회 질문 |
| 모드 | single=brainstorming, multi=승인 | 항상 자율(parallel) |
| max_iter 기본값 | 3 | 5 |
| 사용법 | `/impl --auto "..."` | `/autodev "..."` |

## Workflow

### Step 1: 요구사항 확인 (최대 1회)

요구사항 텍스트가 있으면 즉시 Step 2로 진행.

없거나 너무 모호하면 AskUserQuestion 1회:
- "어떤 기능을 구현할까요? 핵심 동작과 대상 경로를 간략히 알려주세요."

이후 **모든 단계는 자율 실행 — AskUserQuestion 금지.**

### Step 2: 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

다음 값 보존:
- `{original_requirement}`: 요구사항 텍스트
- `{impl_path}`: `--path` 인자 (기본값 `.`)
- `{max_iter}`: `--max-iter` 인자 (기본값 `5`)

### Step 3: CLAUDE.md 인덱스 생성

```bash
$CLI_PATH scan-claude-md --root {impl_path} --output "${TMP_DIR}claude-md-index.json"
```

### Step 4: Decompose (범위 자동 판단)

`${TMP_DIR}decompose-session.md` 생성:

```markdown
# Decompose Session
type: decompose | project_root: {impl_path}

## User Requirement
{original_requirement}

## Existing Modules Index
{scan-claude-md 결과: path, purpose 쌍}

## Project Conventions
{project root Conventions 또는 "None"}
```

```
Task(decompose):
  세션 파일: ${TMP_DIR}decompose-session.md
  결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

decompose result에서 `scope` 및 `modules[]` 확인.

### Step 5: Impl (스펙 정의) — AskUserQuestion 금지

모든 impl agent를 **parallel 모드**로 실행 (single/multi 무관).

#### scope = single

`${TMP_DIR}impl-session.md` 생성:

```markdown
# Impl Session
type: impl | project_root: {impl_path} | parallel: true

## User Requirement
{original_requirement}

## Existing Modules Index
{scan-claude-md 결과}

## Project Conventions
{project root Conventions 또는 "None"}
```

```
Task(impl):
  세션 파일: ${TMP_DIR}impl-session.md
  프로젝트 루트: {impl_path}
  세션 파일을 읽고 CLAUDE.md + DEVELOPERS.md를 생성해주세요.
```

#### scope = multi

`modules[]`를 depth ASC 정렬.

depth 루프 (0, 1, 2, ... 순서):
1. scan-claude-md 재실행 → 최신 인덱스
2. 현재 depth 각 모듈의 세션 파일 생성 (`${TMP_DIR}impl-session-{dir-safe}.md`, `parallel: true`):

   ```markdown
   # Impl Session
   type: impl | project_root: {impl_path} | target_path: {module.path} | action: {module.action} | parallel: true

   ## User Requirement
   {module.requirement_refs}

   ## Purpose Hint
   {module.purpose_hint}

   ## Existing Modules Index
   {최신 scan-claude-md 결과}

   ## Project Conventions
   {project root Conventions 또는 "None"}
   ```

3. Task(impl) 병렬 디스패치 (최대 3개)
4. 완료 대기 → 다음 depth

### Step 6: Auto Loop

`auto_iter = 0`

#### Auto Phase 1: Compile

```
Skill("claude-md-plugin:compile", args: "--conflict overwrite --path {impl_path}")
```

`failed` → 루프 종료, 오류 보고.
`success | partial` → Auto Phase 2로.

#### Auto Phase 2: Validate

```
Skill("claude-md-plugin:validate", args: "{impl_path} --report-only")
```

```
total_violations = schema_errors + convention_issues + boundary_issues + semantic_drift
```

- `total_violations == 0` → **성공 종료 → Step 7**
- `auto_iter >= max_iter` → **max_iter 종료 → Step 7**
- 그 외 → **위반 상세 추출 → Auto Phase 3**

**위반 상세 추출:**

validate-result의 `result_files` 목록 → 각 파일 Read:
- `## Summary: Total issues: N > 0`인 파일 → 해당 모듈 impl update 대상
- `## Issues` 섹션 → 모듈별 위반 상세 수집

`result_files` 없거나 모든 파일이 issues=0이면: Phase 3 생략, `auto_iter++` → Auto Phase 1로.

#### Auto Phase 3: Impl Update

```bash
$CLI_PATH scan-claude-md --root {impl_path} --output "${TMP_DIR}claude-md-index-auto-{auto_iter}.json"
```

위반 모듈별 세션 파일 생성:
`${TMP_DIR}impl-session-auto-{auto_iter}-{dir-safe}.md`

```markdown
# Impl Session (Auto Mode)
type: impl | project_root: {impl_path} | target_path: {path} | action: update | parallel: true

## User Requirement
{original_requirement}

## Auto-Fix Context
auto_iteration: {auto_iter}
validate_violations:
  schema_errors: {n}
  convention_issues: {n}
  boundary_issues: {n}
  semantic_drift: {n}

이 모듈에서 검증 위반이 발견되었습니다.
기존 CLAUDE.md와 DEVELOPERS.md를 읽고, compile 후 validate가 통과하도록
Requirements와 Constraints를 구체화·보완하세요.
CLAUDE.md는 SSOT이므로 요구사항을 더 명확하게 기술하는 방향으로 개선합니다.

## Violations Detail
{result_file의 ## Issues 섹션 내용}

## Existing Modules Index
{최신 scan-claude-md 결과}

## Project Conventions
{project root Conventions 또는 "None"}
```

Task(impl) 병렬 디스패치 (최대 3개). AskUserQuestion 금지.

`auto_iter++` → Auto Phase 1로 루프.

### Step 7: 결과 보고

**성공 종료 (`total_violations == 0`):**

```
✓ autodev 완료 ({auto_iter} iteration(s))
  impl:     CLAUDE.md + DEVELOPERS.md 생성
  compile:  코드 생성 완료
  validate: 모든 검증 통과
```

**실패 종료 (max_iter 도달 | compile 실패):**

```
⚠ autodev 종료 (이유: {사유})
  반복 횟수: {auto_iter}/{max_iter}
  남은 이슈: schema_errors={n}, convention={n}, boundary={n}, semantic_drift={n}
  /validate 또는 /impl로 수동 해소하세요.
```

```bash
git diff --stat
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 요구사항 없음 | Step 1에서 AskUserQuestion 1회 |
| decompose 실패 | 에러 보고 후 종료 |
| impl agent 실패 (단일 모듈) | 경고, 나머지 계속 |
| compile failed | 루프 종료, 오류 보고 |
| result_files 없거나 모두 issues=0 | Phase 3 생략, compile 재시도 |
| max_iter 초과 | 루프 종료, 남은 이슈 보고 |
