---
name: spec-step
description: |
  Inter-session spec workflow step executor.
  Reads .claude/workflows/{dir-safe}/state.json and executes the next pending step.
  Use when a spec workflow was interrupted, or to run pipeline steps in separate sessions/CI jobs.
  Trigger keywords: resume spec, resume workflow, resume spec
argument-hint: "--target <path>"
allowed-tools: [Bash, Read, Write, Task]
---

# /spec-step

Resumes and continues an interrupted spec workflow.

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `--target` | Yes | - | target_path of the module to resume (e.g., `src/auth`) |

## Workflow

### 1. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR="/tmp/claude-md/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 2. Read state.json

```bash
DIR_SAFE=$(echo "{--target argument value}" | tr '/' '-')
STATE_FILE=".claude/workflows/$DIR_SAFE/state.json"
```

Read state.json and extract the following fields:
- `status`, `round`, `plan_file`, `last_reviewer_result`
- `target_path`, `action`, `project_root`, `user_requirement`
- `explore_status`

If state.json does not exist:
```
⚠ .claude/workflows/{dir-safe}/state.json not found.
  Run /spec first or verify the --target path.
```
Exit.

**Self Socratic Loop guard:**

If `explore_status` is `"pending"` or `"in-progress"` or is missing from state.json:
```
⚠ Self Socratic Loop was not completed for this workflow.
  Run /spec --path {target_path} again to restart requirement concretization.
```
Exit without proceeding.

If `explore_status` is `"approved"`, `"user-resolved"`, `"short-circuited"`, or `"best-effort"`:
proceed to Status Branching below.

### 3. Status Branching

#### status = awaiting-review

Create reviewer session file `${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md`:
```
# Spec Reviewer Session
type: spec-reviewer | round: {round}
plan_file: {plan_file}
dir_safe: {dir-safe}
```

Dispatch Task(impl-reviewer):
```
Session file: ${TMP_DIR}spec-reviewer-session-{dir-safe}-v{round}.md
Save results to ${TMP_DIR} and return only the path
```

Extract verdict from result block → artifact promote + state update:
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

Output after completion:
```
[spec-step] review round {round}: {verdict}
Next run: /spec-step --target {target_path}
```

#### status = awaiting-revise

Convergence signal comes from the reviewer (`verdict: approved` or
`progress: no`). The round counter here is a **safety net**, not a convergence
criterion — if it fires, the reviewer loop failed to terminate through its own
signals and that is a bug, not a normal termination.

```
safety_net_rounds = 5   # runaway bug-guard; NOT a convergence criterion
```

If `round >= safety_net_rounds`: update state.json `status` =
`max-safety-exceeded` and exit. This surfaces a bug to the caller; it is not
the expected way the loop ends.
  ```
  ⚠ safety net (round={round}) tripped — reviewer loop did not converge via
    its own signals. Run /spec-step --target {target_path} to execute the
    execute step manually.
  ```

Otherwise:

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-step.json"
```

Create revise session file `${TMP_DIR}spec-plan-session-{dir-safe}.md`:
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
{scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

Dispatch Task(impl, mode=revise):
```
Session file: ${TMP_DIR}spec-plan-session-{dir-safe}.md
Project root: {project_root}
Read the session file and improve the execution plan with mode=revise.
```

After completion, promote + state update:
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

Output after completion:
```
[spec-step] revise round {round+1} complete
Next run: /spec-step --target {target_path}
```

#### status = approved or max-safety-exceeded

Additional output for max-safety-exceeded:
```
⚠ Socratic loop terminated due to exceeding max_safety. Please review the generated documents.
```

```bash
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}claude-md-index-step.json"
```

Create execute session file `${TMP_DIR}spec-execute-session-{dir-safe}.md`:
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
{scan-claude-md result}

## Project Conventions
{project root Conventions or "None"}
```

Dispatch Task(impl, mode=execute):
```
Session file: ${TMP_DIR}spec-execute-session-{dir-safe}.md
Project root: {project_root}
Read the session file and generate CLAUDE.md + DEVELOPERS.md with mode=execute.
```

After completion, state update + auto-commit:
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
git commit -m "spec({target_path}): {action} CLAUDE.md + DEVELOPERS.md

requirement: {user_requirement first 150 chars}
workflow: .claude/workflows/{dir-safe}/state.json"
```

Output after completion:
```
[spec-step] execute complete → status: executed
Next run: /dev --path {target_path}
```

#### status = executed

```
ℹ CLAUDE.md + DEVELOPERS.md generation complete.
  Next step: /dev --path {target_path}
```

#### status = compiled | done

```
ℹ Workflow already completed (status: {status}).
```

### 4. Result

```
---spec-step-result---
target_path: {target_path}
status_before: {previous status}
status_after: {new status}
round: {round}
---end-spec-step-result---
```

## DO / DON'T

**DO:**
- Read state only from state.json
- Update state.json immediately after each step completes
- Promote TMP artifacts to .claude/workflows/
- Include only CLAUDE.md + DEVELOPERS.md in auto-commit

**DON'T:**
- Directly reference previous session artifacts from TMP_DIR (use plan_file, last_reviewer_result from state.json)
- Execute a step without state.json
- Use AskUserQuestion
- Use with scope=multi workflows

## Error Handling

| Situation | Response |
|-----------|----------|
| state.json not found | Print guidance message and exit |
| Task failed | Report error and exit without updating state.json (can be re-run) |
| git commit failed (no staged files) | Print warning and continue (when CLAUDE.md was not generated) |
| multi-scope workflow | state.json is only created for scope=single. Resuming multi-scope workflows (cases where /spec classified as multi) is not supported. |
