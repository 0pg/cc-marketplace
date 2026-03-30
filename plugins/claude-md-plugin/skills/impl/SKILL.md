---
name: impl
version: 1.2.0
aliases: [define, requirements]
description: |
  This skill should be used when the user asks to "define requirements", "write spec",
  "create CLAUDE.md from requirements", "define behavior before coding", or uses "/impl".
  Analyzes natural language requirements and generates CLAUDE.md without implementing code.
  Follows ATDD principle: specification first, then code generation via /compile.
  Trigger keywords: 요구사항 정의, 스펙 작성, 명세 먼저
user_invocable: true
allowed-tools: [Read, Glob, Write, Task, AskUserQuestion, Bash, Skill]
---

# /impl

요구사항(자연어 또는 User Story)을 분석하여 **CLAUDE.md + DEVELOPERS.md**를 생성/업데이트.
**코드 구현 없이** 요구사항 정의만 수행하여 "명세 먼저" 원칙을 따름.

## Triggers

- `/impl`
- `요구사항 정의`
- `스펙 작성`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `requirement` | 예 | - | 요구사항 텍스트 |
| `--path` | 아니오 | `.` | 대상 경로 |
| `--auto` | 아니오 | false | impl→compile→validate 자율 루프 실행 |
| `--auto-max-iter` | 아니오 | `3` | 최대 compile-validate 사이클 수 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 기존 CLAUDE.md 인덱스 생성

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index.json"
```

### 2. 프로젝트 컨벤션 읽기

project root CLAUDE.md의 `## Conventions` 섹션이 있으면 읽기.

### 3. Decompose 세션 파일 생성

`${TMP_DIR}decompose-session.md`:

```markdown
# Decompose Session
type: decompose | project_root: {project_root}

## User Requirement
{사용자 요구사항 텍스트}

## Existing Modules Index
{scan-claude-md 결과: path, purpose 쌍}

## Project Conventions
{project root Conventions 또는 "None"}
```

### 4. Decompose agent 디스패치

```
Task(decompose):
  세션 파일: ${TMP_DIR}decompose-session.md
  결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

### 5. Decompose 결과 읽기

decompose result block에서 `result_file` 경로를 추출하여 Read.

결과 JSON에서 `scope`, `modules[]`, `unassigned[]`, `ambiguous[]` 파악.

`unassigned`가 있으면 사용자에게 안내:
```
⚠ 다음 요구사항이 어느 모듈에도 배치되지 않았습니다:
  - {unassigned 항목들}
impl 완료 후 직접 추가하거나 다시 /impl을 실행하세요.
```

### 6. scope 분기

#### scope = single

단일 impl 세션 파일 생성 후 impl agent 1개 디스패치:

`${TMP_DIR}impl-session.md`:

```markdown
# Impl Session
type: impl | project_root: {project_root}

## User Requirement
{사용자 요구사항 텍스트 전체}

## Existing Modules Index
{scan-claude-md 결과: path, purpose 쌍}

## Project Conventions
{project root Conventions 또는 "None"}
```

```
Task(impl):
  세션 파일: ${TMP_DIR}impl-session.md
  프로젝트 루트: {project_root}

  세션 파일을 읽고 CLAUDE.md + DEVELOPERS.md를 생성해주세요.
```

#### scope = multi

**6a. 사용자 승인**

AskUserQuestion으로 분해 계획을 제시하고 승인 요청:

```
분해 계획:
  • {path} ({action}) — {purpose_hint}
  • {path} ({action}) — {purpose_hint}
  ...

{ambiguous가 있으면}
⚠ 모호한 판단:
  - {ambiguous 항목들}

이 계획으로 진행할까요? (수정이 필요하면 알려주세요)
```

수정 요청 시, 요청 유형에 따라 처리 방식이 다름 (최대 1회 루프):

| 수정 유형 | 처리 방식 |
|----------|----------|
| path 변경, purpose_hint 수정, 모듈 추가/삭제 | SKILL이 직접 `decompose-result.json` 편집 |
| 요구사항 재분배, 모듈 병합/분리 | `decompose-session.md`에 `## User Modification` 섹션 추가 후 Task(decompose) 재호출 |

재호출 시 세션 파일에 추가할 섹션:
```markdown
## User Modification
{사용자의 수정 요청 내용}
```
decompose agent는 이 섹션을 읽어 이전 분해 결과를 수정하는 방향으로 재실행한다.

취소 시: 종료.

**6b. root-first 정렬**

`modules[]`를 `depth` ASC 정렬. 같은 depth는 `depends_on`이 없는 것 우선.

**6c+6d. depth 루프 — 세션 파일 생성과 디스패치를 depth별로 실행**

각 depth를 순서대로 처리한다. **세션 파일 생성은 해당 depth 직전에** 수행하여
이전 depth의 impl 결과(CLAUDE.md)가 인덱스에 반영되도록 한다.

```
for depth in sorted_depths:  # 0, 1, 2, ...

  1. 현재 depth 모듈들의 세션 파일 생성:
     scan-claude-md를 재실행하여 최신 인덱스 획득
     (이전 depth impl이 생성한 CLAUDE.md가 포함됨)

     각 모듈에 대해 ${TMP_DIR}impl-session-{dir-safe}.md 생성:

     ---
     # Impl Session
     type: impl | project_root: {project_root} | target_path: {module.path} | action: {module.action} | parallel: true

     ## User Requirement
     {module.requirement_refs}

     ## Purpose Hint
     {module.purpose_hint}

     ## Source Concept
     {module.source_concept}

     ## Existing Modules Index
     {최신 scan-claude-md 결과}

     ## Project Conventions
     {project root Conventions 또는 "None"}
     ---

  2. 현재 depth Task(impl) 병렬 디스패치 (최대 3개):

     Task(impl) — ${TMP_DIR}impl-session-{dir-safe-A}.md
     Task(impl) — ${TMP_DIR}impl-session-{dir-safe-B}.md  (있으면)
     Task(impl) — ${TMP_DIR}impl-session-{dir-safe-C}.md  (있으면)

     각 Task 지시:
       세션 파일: ${TMP_DIR}impl-session-{dir-safe}.md
       프로젝트 루트: {project_root}
       세션 파일을 읽고 CLAUDE.md + DEVELOPERS.md를 생성해주세요.

  3. 현재 depth 완료 대기 → 다음 depth로
```

> **왜 depth별로 나누는가:** depth=1 모듈(자식)의 impl agent는 depth=0 모듈(부모)의 CLAUDE.md를
> Phase 1.5(Dependency Exploration)에서 Read해야 한다. 부모 impl 완료 전에 세션 파일을 생성하면
> 인덱스가 stale하여 부모 컨텍스트를 누락한다.

### 7. 변경사항 표시

```bash
git diff --stat
```

### 8. 결과

```
---impl-result---
scope: single | multi
modules:
  - {path}: {status} ({action})
unassigned_count: N
---end-impl-result---
```

## DO / DON'T

**DO:**
- 항상 decompose를 먼저 호출하여 scope 판단 위임
- scope=single이어도 decompose를 생략하지 않음
- multi의 경우 사용자 승인 후 병렬 impl 디스패치
- unassigned 요구사항은 사용자에게 안내

**DON'T:**
- decompose 없이 직접 Task(impl) 디스패치
- impl agent에 분해 판단 위임
- 사용자 승인 없이 multi 모드 자동 실행

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| 요구사항 인자 없음 | AskUserQuestion으로 요구사항 수집 |
| decompose agent 실패 | 에러 보고 후 종료 |
| impl agent 실패 (단일 모듈) | 경고, 나머지 모듈 계속 |
| 사용자 승인 취소 | status: cancelled_by_user 반환 |

---

## Auto Mode (--auto)

`--auto` 플래그가 있으면 impl 완료 후 자동으로 compile → validate → impl update 루프를 실행한다.
**Phase 0 이후 AskUserQuestion 사용 금지.**

원본 요구사항 텍스트를 변수에 보존하여 impl update 시 재사용한다.

### Phase 0: 초기 impl (일반 워크플로우와 동일)

- 위 Workflow Step 0-8 전체 실행
- single 모드: AskUserQuestion 허용 (brainstorming + 명확화)
- multi 모드: 사용자 승인 1회 (분해 계획)
- CLAUDE.md + DEVELOPERS.md 생성 완료 → Auto Loop 진입

### Auto Loop

`auto_iter = 0`, `prev_total_violations = -1`

#### Auto Phase 1: Compile

```
Skill("claude-md-plugin:compile", args: "--all --conflict overwrite")
```

compile-result에서 `status` 확인:
- `failed` → 경고 후 Auto Loop 종료 (코드가 없으면 validate 불가)
- `success | partial` → Auto Phase 2로

#### Auto Phase 2: Validate

```
Skill("claude-md-plugin:validate", args: "--report-only")
```

validate-result 파싱:

```
total_violations = schema_errors + convention_issues + boundary_issues + semantic_drift
```

- `total_violations == 0` → **성공 종료**
- `total_violations == prev_total_violations` → **진전 없음 종료** (무한루프 방지)
- `auto_iter >= auto_max_iter` → **max_iter 종료**
- 그 외 → 위반 상세 추출 → Auto Phase 3

**위반 상세 추출:**

```
Glob("${TMP_DIR}validate-session-*.md")
```

각 세션 파일의 `## Deterministic Results` 섹션 Read → 모듈 경로와 위반 내용 수집.
위반이 발견된 모듈만 대상으로 선별.

`prev_total_violations = total_violations`

#### Auto Phase 3: Impl Update

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-auto-{iter}.json"
```

위반이 발견된 각 모듈에 대해 세션 파일 생성:
`${TMP_DIR}impl-session-auto-{iter}-{dir-safe}.md`

```markdown
# Impl Session (Auto Mode)
type: impl | project_root: {project_root} | target_path: {path} | action: update | parallel: true

## User Requirement
{원본 요구사항 텍스트 (Phase 0에서 보존)}

## Auto-Fix Context
auto_iteration: {n}
validate_violations:
  schema_errors: {n}
  convention_issues: {n}
  boundary_issues: {n}
  semantic_drift: {n}

이 모듈에서 검증 위반이 발견되었습니다.
기존 CLAUDE.md와 DEVELOPERS.md를 읽고, compile 후 validate가 통과할 수 있도록
Requirements와 Constraints를 구체화·보완하세요.
CLAUDE.md는 SSOT이므로 요구사항을 더 명확하게 기술하는 방향으로 개선합니다.

## Violations Detail
{validate-session 파일에서 추출한 이 모듈의 위반 상세}

## Existing Modules Index
{최신 scan-claude-md 결과}

## Project Conventions
{project root Conventions 또는 "None"}
```

Task(impl) 병렬 디스패치 (최대 3개). **AskUserQuestion 금지.**

`auto_iter++` → Auto Phase 1로 루프

### Auto Phase 4: 종료 보고

**성공 종료 (`total_violations == 0`):**

```
✓ Auto mode 완료 ({auto_iter} iteration(s))
  impl: CLAUDE.md + DEVELOPERS.md 생성
  compile: 코드 생성 완료
  validate: 모든 검증 통과
```

**실패 종료 (max_iter 도달 | 진전 없음 | compile 실패):**

```
⚠ Auto mode 종료 (이유: {사유})
  반복 횟수: {auto_iter}/{auto_max_iter}
  남은 이슈: schema_errors={n}, convention={n}, boundary={n}, semantic_drift={n}
  /validate 또는 /impl을 수동으로 실행하여 해소하세요.
```

### Auto Mode 오류 처리

| 상황 | 대응 |
|------|------|
| compile failed | 루프 종료, 오류 보고 |
| validate 세션 파일 없음 | total_violations 집계만으로 처리 |
| impl update 모두 실패 | 경고, 루프 계속 (다음 compile 시도) |
| 진전 없음 (동일 violations) | 루프 조기 종료 |
| max_iter 초과 | 루프 종료, 남은 이슈 보고 |
