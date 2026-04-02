---
name: impl-step
description: |
  Inter-session spec workflow step executor.
  Reads .claude/workflows/{dir-safe}/state.json and executes the next pending step.
  Use when a spec workflow was interrupted, or to run pipeline steps in separate sessions/CI jobs.
  Trigger keywords: spec 재개, 워크플로우 재개, resume spec
argument-hint: "--target <path>"
allowed-tools: [Bash, Read, Write, Task, Skill]
---

# /spec-step

중단된 spec 워크플로우를 이어서 실행합니다.

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `--target` | 예 | - | 재개할 모듈의 target_path (예: `src/auth`) |

## Workflow

### 1. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 2. state.json 읽기

```bash
DIR_SAFE=$(echo "{--target 인자값}" | tr '/' '-')
STATE_FILE=".claude/workflows/$DIR_SAFE/state.json"
```

state.json을 Read하여 다음 필드 추출:
- `status`, `round`, `plan_file`, `last_reviewer_result`
- `target_path`, `action`, `project_root`, `user_requirement`

state.json이 없으면:
```
⚠ .claude/workflows/{dir-safe}/state.json 없음.
  /spec을 먼저 실행하거나 --target 경로를 확인하세요.
```
종료.

### 3. status 분기

#### status = awaiting-review

Reviewer 세션 파일 생성 `${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md`:
```
# Spec Reviewer Session
type: spec-reviewer | round: {round}
plan_file: {plan_file}
dir_safe: {dir-safe}
```

Task(impl-reviewer) 디스패치:
```
세션 파일: ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

result block에서 verdict 추출 → artifact promote + state 갱신:
```bash
cp "${TMP_DIR}spec-reviewer-result-{dir-safe}-v{round}.md" \
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

완료 후 출력:
```
[spec-step] review round {round}: {verdict}
다음 실행: /spec-step --target {target_path}
```

#### status = awaiting-revise

round >= 5 확인:
- 해당 시: state.json `status` = `max-safety-exceeded` 갱신 후 종료
  ```
  ⚠ max_safety(5) 도달. /spec-step --target {target_path} 로 execute 단계를 실행하세요.
  ```

그 외:

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-step.json"
```

Revise 세션 파일 생성 `${TMP_DIR}spec-plan-session-{dir-safe}.md`:
```
# Spec Plan Session
type: spec-plan | mode: revise | round: {round+1} | project_root: {project_root}
target_path: {target_path}
action: {action}

## User Requirement
{user_requirement}

## Reviewer Feedback File
feedback_file: {last_reviewer_result}

## Existing Plan File
existing_plan_file: {plan_file}

## Existing Modules Index
{scan-claude-md 결과}

## Project Conventions
{project root Conventions 또는 "None"}
```

Task(impl, mode=revise) 디스패치:
```
세션 파일: ${TMP_DIR}spec-plan-session-{dir-safe}.md
프로젝트 루트: {project_root}
세션 파일을 읽고 mode=revise로 실행계획을 개선해주세요.
```

완료 후 promote + state 갱신:
```bash
cp "${TMP_DIR}spec-plan-{dir-safe}.md" ".claude/workflows/{dir-safe}/spec-plan.md"
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

완료 후 출력:
```
[spec-step] revise round {round+1} 완료
다음 실행: /spec-step --target {target_path}
```

#### status = approved 또는 max-safety-exceeded

max-safety-exceeded 시 추가 출력:
```
⚠ Socratic loop가 max_safety 초과로 종료되었습니다. 생성된 문서를 반드시 검토하세요.
```

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-step.json"
```

Execute 세션 파일 생성 `${TMP_DIR}spec-execute-session-{dir-safe}.md`:
```
# Spec Execute Session
type: spec-execute | mode: execute | project_root: {project_root}
target_path: {target_path}
action: {action}

## Approved Plan File
plan_file: {plan_file}

## User Requirement
{user_requirement}

## Existing Modules Index
{scan-claude-md 결과}

## Project Conventions
{project root Conventions 또는 "None"}
```

Task(impl, mode=execute) 디스패치:
```
세션 파일: ${TMP_DIR}spec-execute-session-{dir-safe}.md
프로젝트 루트: {project_root}
세션 파일을 읽고 mode=execute로 CLAUDE.md + DEVELOPERS.md를 생성해주세요.
```

완료 후 state 갱신 + auto-commit:
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

git add "{target_path}/CLAUDE.md" "{target_path}/DEVELOPERS.md"
git commit -m "feat({target_path}): {action} CLAUDE.md + DEVELOPERS.md

요구사항: {user_requirement 최초 150자}
workflow: .claude/workflows/{dir-safe}/state.json"
```

완료 후 출력:
```
[spec-step] execute 완료 → status: executed
다음 실행: /dev --path {target_path}
```

#### status = executed

```
ℹ CLAUDE.md + DEVELOPERS.md 생성 완료.
  다음 단계: /dev --path {target_path}
```

#### status = compiled | done

```
ℹ 워크플로우가 이미 완료되었습니다 (status: {status}).
```

### 4. 결과

```
---spec-step-result---
target_path: {target_path}
status_before: {이전 status}
status_after: {이후 status}
round: {round}
---end-spec-step-result---
```

## DO / DON'T

**DO:**
- state.json에서만 상태 읽기
- 각 step 완료 후 즉시 state.json 갱신
- TMP artifacts를 .claude/workflows/로 promote
- auto-commit에 CLAUDE.md + DEVELOPERS.md만 포함

**DON'T:**
- TMP_DIR에서 이전 세션 artifacts 직접 참조 (state.json의 plan_file, last_reviewer_result 사용)
- state.json 없이 step 실행
- AskUserQuestion 사용
- scope=multi 워크플로우에서 사용

## 오류 처리

| 상황 | 대응 |
|------|------|
| state.json 없음 | 안내 메시지 출력 후 종료 |
| Task 실패 | 오류 보고 후 state.json 갱신 없이 종료 (재실행 가능) |
| git commit 실패 (staged 파일 없음) | 경고 출력 후 계속 (CLAUDE.md 미생성 시) |
| multi-scope workflow | state.json은 scope=single에서만 생성됩니다. multi-scope (/spec이 multi로 분류한 경우) 재개는 미지원. |
