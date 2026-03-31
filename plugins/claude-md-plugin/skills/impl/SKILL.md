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
| `--auto-max-iter` | 아니오 | `3` | impl update 최대 시도 횟수. N회 시도 후 최종 compile+validate 포함하여 총 N+1회 검증 |

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

**6a. Plan 세션 파일 생성**

`${TMP_DIR}impl-plan-session-{dir-safe}.md`:

```markdown
# Impl Plan Session
type: impl-plan | mode: plan | round: 1 | project_root: {project_root}
target_path: TBD
action: TBD

## User Requirement
{사용자 요구사항 텍스트 전체}

## Existing Modules Index
{scan-claude-md 결과: path, purpose 쌍}

## Project Conventions
{project root Conventions 또는 "None"}
```

**6b. Task(impl, mode=plan) 디스패치**

```
Task(impl):
  세션 파일: ${TMP_DIR}impl-plan-session-{dir-safe}.md
  프로젝트 루트: {project_root}

  세션 파일을 읽고 mode=plan으로 실행계획(plan.md)을 생성해주세요.
```

결과 block에서 `plan_file`, `target_path`, `action`, `dir-safe` 추출.

> `dir_safe`: target_path의 슬래시를 하이픈으로 치환 (예: `src/auth` → `src-auth`)

**6b-1. Workflow state 초기화**

```bash
WORKFLOW_DIR=".claude/workflows/{dir-safe}"
mkdir -p "$WORKFLOW_DIR"
cp "{plan_file}" "$WORKFLOW_DIR/impl-plan.md"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
cat > "$WORKFLOW_DIR/state.json" << 'STATEOF'
{
  "workflow_id": "{dir-safe}-TIMESTAMP_PLACEHOLDER",
  "target_path": "{target_path}",
  "dir_safe": "{dir-safe}",
  "action": "{action}",
  "status": "awaiting-review",
  "round": 1,
  "plan_file": ".claude/workflows/{dir-safe}/impl-plan.md",
  "last_reviewer_result": "",
  "project_root": "{project_root}",
  "user_requirement": "{사용자 요구사항 텍스트 최초 500자 — JSON 특수문자(\" \\ 개행) escape 필수}",
  "created_at": "TIMESTAMP_PLACEHOLDER",
  "updated_at": "TIMESTAMP_PLACEHOLDER"
}
STATEOF
# Replace TIMESTAMP_PLACEHOLDER with actual timestamp
sed -i '' "s/TIMESTAMP_PLACEHOLDER/$TIMESTAMP/g" "$WORKFLOW_DIR/state.json"
```

**6c. Socratic Loop**

`round = 1`, `max_safety = 5`

```
loop:
  1. Reviewer 세션 파일 생성:
     ${TMP_DIR}impl-reviewer-session-{dir-safe}-v{round}.md:
       # Impl Reviewer Session
       type: impl-reviewer | round: {round}
       plan_file: {plan_file}
       dir_safe: {dir-safe}

  2. Task(impl-reviewer) 디스패치:
       세션 파일: ${TMP_DIR}impl-reviewer-session-{dir-safe}-v{round}.md
       결과는 ${TMP_DIR}에 저장하고 경로만 반환

     result block에서 verdict 추출.

     2-1. Artifact promote + state.json 갱신 (verdict 반영):
     ```bash
     cp "${TMP_DIR}impl-reviewer-result-{dir-safe}-v{round}.md" \
        ".claude/workflows/{dir-safe}/reviewer-v{round}.md"
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'approved' if '{verdict}' == 'approved' else 'awaiting-revise'
     s['last_reviewer_result'] = '.claude/workflows/{dir-safe}/reviewer-v{round}.md'
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```

  3. if verdict == "approved":
       break

  4. if round >= max_safety:
       ⚠ Socratic loop가 {max_safety}회 반복 후 종료됩니다.
         최선의 계획으로 진행합니다.
     ```bash
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'max-safety-exceeded'
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```
       break

  5. Revise 세션 파일 생성:
     ${TMP_DIR}impl-plan-session-{dir-safe}.md (덮어쓰기):
       # Impl Plan Session
       type: impl-plan | mode: revise | round: {round+1} | project_root: {project_root}
       target_path: {target_path}
       action: {action}

       ## User Requirement
       {사용자 요구사항 텍스트 전체}

       ## Reviewer Feedback File
       feedback_file: ${TMP_DIR}impl-reviewer-result-{dir-safe}-v{round}.md

       ## Existing Plan File
       existing_plan_file: {plan_file}

       ## Existing Modules Index
       {scan-claude-md 결과}

       ## Project Conventions
       {project root Conventions 또는 "None"}

  6. Task(impl, mode=revise) 디스패치:
       세션 파일: ${TMP_DIR}impl-plan-session-{dir-safe}.md
       프로젝트 루트: {project_root}

       세션 파일을 읽고 mode=revise로 실행계획을 개선해주세요.

     결과 block에서 plan_file 업데이트 확인.

     6-1. Revise artifact promote + state.json 갱신:
     ```bash
     cp "${TMP_DIR}impl-plan-{dir-safe}.md" ".claude/workflows/{dir-safe}/impl-plan.md"
     python3 -c "
     import json
     from datetime import datetime, timezone
     with open('.claude/workflows/{dir-safe}/state.json') as f:
         s = json.load(f)
     s['status'] = 'awaiting-review'
     s['round'] = {round} + 1
     s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
     with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
         json.dump(s, f, indent=2, ensure_ascii=False)
     "
     ```

  7. round++
  → 1로 돌아감
```

**6d. Execute 세션 파일 생성**

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-exec.json"
```

`${TMP_DIR}impl-execute-session-{dir-safe}.md`:

```markdown
# Impl Execute Session
type: impl-execute | mode: execute | project_root: {project_root}
target_path: {target_path}
action: {action}

## Approved Plan File
plan_file: {plan_file}

## User Requirement
{사용자 요구사항 텍스트 전체}

## Existing Modules Index
{최신 scan-claude-md 결과}

## Project Conventions
{project root Conventions 또는 "None"}
```

**6e. Task(impl, mode=execute) 디스패치**

```
Task(impl):
  세션 파일: ${TMP_DIR}impl-execute-session-{dir-safe}.md
  프로젝트 루트: {project_root}

  세션 파일을 읽고 mode=execute로 CLAUDE.md + DEVELOPERS.md를 생성해주세요.
```

**6e-1. Execute 완료 후 state 갱신 + auto-commit**

**커밋 메시지 구성:**

impl agent는 Execute 완료 후 커밋 메시지를 다음 규칙으로 생성합니다:

1. **summary**: 이번 변경의 핵심을 한 줄로 (예: "OAuth2 인증 추가", "수수료 정책 변경")
2. **[BREAKING]** (선택): Requirements 삭제 또는 대규모 방향 전환이 있을 때만 포함
3. **전환 맥락**: 1-2문장. 문서는 "현재 상태"를 기술하지만, 커밋 메시지는 "어디서 어디로 전환하는가"를 기술
   - 좋은 예: "session 기반 인증에 OAuth2를 추가 경로로 도입. 레거시 클라이언트 지원을 위해 session 유지."
   - 나쁜 예: "인증 시스템 업데이트" (방향성 없음)
4. **Changes**: before/after 비교하여 added/modified/removed로 분류
   - 해당 없는 항목은 생략 (예: removed 없으면 removed 줄 생략)

```bash
python3 -c "
import json
from datetime import datetime, timezone
with open('.claude/workflows/{dir-safe}/state.json') as f:
    s = json.load(f)
s['status'] = 'executed'
s['updated_at'] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')
with open('.claude/workflows/{dir-safe}/state.json', 'w') as f:
    json.dump(s, f, indent=2, ensure_ascii=False)
"

# CLAUDE.md + DEVELOPERS.md만 커밋 (TMP 파일 및 workflow state 제외)
git add "{target_path}/CLAUDE.md" "{target_path}/DEVELOPERS.md"
git commit -m "impl({target_path}): [BREAKING] {summary}

{전환 맥락 — 어디서 어디로, 왜 이 변경을 하는가 1-2문장}

Changes:
- added: {추가된 Requirements/Constraints 목록}
- modified: {변경된 Requirements/Constraints 목록}
- removed: {삭제된 Requirements/Constraints 목록}"
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

  2. 현재 depth 모듈들: Plan 세션 파일 생성 + Task(impl, mode=plan) 병렬 디스패치 (최대 3개)

     각 모듈에 대해 `${TMP_DIR}impl-plan-session-{dir-safe}.md` 생성:
     ```
     # Impl Plan Session
     type: impl-plan | mode: plan | round: 1 | project_root: {project_root} | parallel: true
     target_path: {module.path}
     action: {module.action}

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
     ```

     Task(impl, mode=plan) 병렬 디스패치:
     ```
     Task(impl) — ${TMP_DIR}impl-plan-session-{dir-safe-A}.md
     Task(impl) — ${TMP_DIR}impl-plan-session-{dir-safe-B}.md  (있으면)
     Task(impl) — ${TMP_DIR}impl-plan-session-{dir-safe-C}.md  (있으면)
     ```

     각 Task 지시:
       세션 파일: ${TMP_DIR}impl-plan-session-{dir-safe}.md
       프로젝트 루트: {project_root}
       세션 파일을 읽고 mode=plan으로 실행계획(plan.md)을 생성해주세요.
       (parallel 모드 — AskUserQuestion 금지)

  3. 각 모듈 Socratic Loop (모듈별 순차 실행, round=1, max_safety=5):

     각 모듈에 대해 아래를 순서대로 실행:

     ```
     loop:
       a. Reviewer 세션 파일 생성:
          ${TMP_DIR}impl-reviewer-session-{dir-safe}-v{round}.md:
            # Impl Reviewer Session
            type: impl-reviewer | round: {round}
            plan_file: ${TMP_DIR}impl-plan-{dir-safe}.md
            dir_safe: {dir-safe}

       b. Task(impl-reviewer) 디스패치:
            세션 파일: ${TMP_DIR}impl-reviewer-session-{dir-safe}-v{round}.md
            결과는 ${TMP_DIR}에 저장하고 경로만 반환

          result block에서 verdict 추출.

       c. if verdict == "approved" → break

       d. if round >= max_safety:
            ⚠ 모듈 {module.path}: Socratic loop {max_safety}회 반복 후 종료.
            break

       e. Revise 세션 파일 생성:
          ${TMP_DIR}impl-plan-session-{dir-safe}.md (덮어쓰기):
            # Impl Plan Session
            type: impl-plan | mode: revise | round: {round+1} | project_root: {project_root} | parallel: true
            target_path: {module.path}
            action: {module.action}

            ## User Requirement
            {module.requirement_refs}

            ## Reviewer Feedback File
            feedback_file: ${TMP_DIR}impl-reviewer-result-{dir-safe}-v{round}.md

            ## Existing Plan File
            existing_plan_file: ${TMP_DIR}impl-plan-{dir-safe}.md

            ## Existing Modules Index
            {scan-claude-md 결과}

            ## Project Conventions
            {project root Conventions 또는 "None"}

       f. Task(impl, mode=revise) 디스패치:
            세션 파일: ${TMP_DIR}impl-plan-session-{dir-safe}.md
            세션 파일을 읽고 mode=revise로 실행계획을 개선해주세요.
            (parallel 모드 — AskUserQuestion 금지)

       g. round++
     ```

     > **왜 모듈별 순차인가:** 각 모듈의 reviewer loop iteration이 이전 결과에 의존하므로
     > loop 내부는 순차 불가피. 단, 모듈간 loop는 독립이므로 병렬 실행 가능하나
     > SKILL context 보호를 위해 순차 처리.

  4. Execute 세션 파일 생성 + Task(impl, mode=execute) 병렬 디스패치 (최대 3개):

     각 모듈에 대해 `${TMP_DIR}impl-execute-session-{dir-safe}.md` 생성:
     ```
     # Impl Execute Session
     type: impl-execute | mode: execute | project_root: {project_root} | parallel: true
     target_path: {module.path}
     action: {module.action}

     ## Approved Plan File
     plan_file: ${TMP_DIR}impl-plan-{dir-safe}.md

     ## User Requirement
     {module.requirement_refs}

     ## Existing Modules Index
     {최신 scan-claude-md 결과}

     ## Project Conventions
     {project root Conventions 또는 "None"}
     ```

     Task(impl, mode=execute) 병렬 디스패치:
     ```
     Task(impl) — ${TMP_DIR}impl-execute-session-{dir-safe-A}.md
     Task(impl) — ${TMP_DIR}impl-execute-session-{dir-safe-B}.md  (있으면)
     Task(impl) — ${TMP_DIR}impl-execute-session-{dir-safe-C}.md  (있으면)
     ```

     각 Task 지시:
       세션 파일: ${TMP_DIR}impl-execute-session-{dir-safe}.md
       프로젝트 루트: {project_root}
       세션 파일을 읽고 mode=execute로 CLAUDE.md + DEVELOPERS.md를 생성해주세요.
       (parallel 모드 — AskUserQuestion 금지)

  5. 현재 depth 완료 대기 → 다음 depth로
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

> **주의:** compile은 소스 파일 확장자로 언어를 자동 감지합니다.
> 소스 파일이 없는 신규 프로젝트에서 첫 compile 시 언어를 묻는 질문이 발생할 수 있습니다.
> 이 경우 자율 실행이 중단됩니다. 빈 프로젝트에서는 `--auto` 실행 전에
> 언어를 나타내는 파일(package.json, go.mod, Cargo.toml 등)을 추가하거나
> `/compile`을 먼저 한 번 실행하세요.

다음 값을 Auto Mode 진입 시 보존:
- `{original_requirement}`: 사용자 요구사항 텍스트 (Phase 0에서 추출)
- `{impl_path}`: `--path` 인자값 (기본값 `.`)

### Phase 0: 초기 impl (일반 워크플로우와 동일)

- 위 Workflow Step 0-8 전체 실행
- single 모드: AskUserQuestion 허용 (brainstorming + 명확화)
- multi 모드: 사용자 승인 1회 (분해 계획)
- CLAUDE.md + DEVELOPERS.md 생성 완료 → Auto Loop 진입

### Auto Loop

`auto_iter = 0`

#### Auto Phase 1: Compile

```
Skill("claude-md-plugin:compile", args: "--conflict overwrite --path {impl_path}")
```

compile-result에서 `status` 확인:
- `failed` → 경고 후 Auto Loop 종료 (코드가 없으면 validate 불가)
- `success | partial` → Auto Phase 2로

#### Auto Phase 2: Validate

```
Skill("claude-md-plugin:validate", args: "{impl_path} --report-only")
```

validate-result 파싱:

```
total_violations = schema_errors + convention_issues + boundary_issues + semantic_drift
```

- `total_violations == 0` → **성공 종료**
- `auto_iter >= auto_max_iter` → **max_iter 종료**
- 그 외 → 위반 상세 추출 → Auto Phase 3

**위반 상세 추출:**

validate-result의 `result_files` 목록에서 각 파일 Read:
- `## Summary`의 `Total issues: N` > 0인 파일 → 해당 모듈이 impl update 대상
- `## Issues` 섹션 → 모듈별 위반 상세 수집 (REQUIREMENTS_NOT_IMPLEMENTED 등)
- result_files가 없으면 (semantic 검증 대상 없음): total_violations > 0이어도 Phase 3 생략

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
{original_requirement}

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
{result_file의 ## Issues 섹션에서 추출한 이 모듈의 위반 상세}

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

**실패 종료 (max_iter 도달 | compile 실패):**

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
| result_files 없음 (schema/convention만) | Phase 3 생략, compile 재시도 |
| impl update 모두 실패 | 경고, 루프 계속 (다음 compile 시도) |
| max_iter 초과 | 루프 종료, 남은 이슈 보고 |
