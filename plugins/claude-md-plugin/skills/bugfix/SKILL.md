---
name: bugfix
version: 1.0.0
aliases: [fix, debug]
description: |
  This skill should be used when the user reports a bug, unexpected behavior, or asks to
  "fix this bug", "debug this", "something is broken", "not working as expected", or uses "/bugfix".
  Traces root cause through 3 layers: CLAUDE.md (requirements) → DEVELOPERS.md (constraints)
  → source code. Fixes at the highest affected layer following Fix-Highest-Layer-First principle.
  Never patches code while leaving CLAUDE.md inconsistent.
  Trigger keywords: fix bug, debug, unexpected behavior, broken, not working
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, AskUserQuestion, Skill]
---

# /bugfix

Traces the root cause of a reported bug through 3 layers and fixes at the highest affected layer.

## Triggers

- `/bugfix`
- `fix bug`
- `something is broken`

## Arguments

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `description` | Yes | — | 버그 설명 (expected vs actual behavior) |
| `--path` | No | `.` | 대상 경로 |
| `--error` | No | — | 에러 메시지 또는 스택 트레이스 |
| `--file` | No | — | 버그가 있는 특정 파일 경로 |

## Invariants

```
INV-bugfix-1: Conflict Resolution
  Code always defers to CLAUDE.md.
  - CLAUDE.md correct → fix code (Layer 3)
  - CLAUDE.md incorrect → fix CLAUDE.md first (Layer 1), then regenerate code
  Never patch code while leaving CLAUDE.md inconsistent.

INV-bugfix-2: Ambiguity Escalation
  Layer 3 (code): autonomous fix when judgment is unambiguous.
  Layer 1/2 (SSOT documents): always require user approval before modification,
    even when judgment is unambiguous. Modifying the SSOT has broader impact than a code fix.
  Any degree of ambiguity (any layer) → escalate to user with structured context.
```

## Workflow

### 0. Initialization

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Bug Context 수집

`description` 인수에서 E (expected), A (actual) 파싱.

E가 불명확하면:
```
AskUserQuestion:
  "버그를 좀 더 구체적으로 설명해주세요.
  - 기대하는 동작 (expected): ?
  - 실제 동작 (actual): ?"
```

### 2. CLAUDE.md 및 소스 파일 선정

#### 2a. CLAUDE.md 선정

```
if --file 제공:
  --file 경로에서 상위로 올라가며 첫 번째 CLAUDE.md 탐색
  예) src/auth/login.ts → src/auth/CLAUDE.md → src/CLAUDE.md → CLAUDE.md

else (--path 사용):
  $CLI_PATH scan-claude-md --root {path} --output "${TMP_DIR}claude-md-index.json"
  {path} 내 최상위 CLAUDE.md 선택 (index 내 depth가 가장 낮은 항목)
```

Conventions: 프로젝트 루트 CLAUDE.md에서 `## Conventions` 섹션 추출 (계층 상속).

CLAUDE.md를 찾지 못하면: `"CLAUDE.md를 찾을 수 없습니다. --path 또는 --file을 확인하세요."` → exit.

#### 2b. DEVELOPERS.md 선정

선정된 CLAUDE.md와 같은 디렉토리의 `DEVELOPERS.md`. 없으면 세션 파일에 "none" 기록.

#### 2c. 소스 파일 선정

```
if --file 제공:
  해당 파일 + 같은 디렉토리의 소스 파일 목록 (Glob)

else:
  선정된 CLAUDE.md 디렉토리 내 소스 파일 목록 (Glob)
  파일 수 ≤ 10: 내용 포함
  파일 수 > 10: 목록만 포함 (agent가 필요 시 직접 Read)
```

언어 감지: 소스 파일 확장자 기반 (`.ts/.tsx` → typescript, `.rs` → rust, `.py` → python, `.go` → go)

#### 2d. diff-spec-range 실행

```bash
$CLI_PATH diff-spec-range \
  --file {selected_CLAUDE.md_path} \
  --root {project_root} \
  --output "${TMP_DIR}spec-diff-{dir-safe}.json"
```

> `dir-safe`: path의 슬래시를 하이픈으로 치환 (e.g., `src/auth` → `src-auth`)

### 3. Session File 생성

`${TMP_DIR}bugfix-session-{dir-safe}.md`

Format: `skills/bugfix/references/bugfix-templates.md` "Bugfix Session File Format" 참고.

Include all of:
- Bug Description (E, A from Step 1)
- Error Message (`--error` value or "none")
- Target File (`--file` value or "none")
- Layer 1: selected CLAUDE.md content (Purpose + Requirements + Domain Context)
- Layer 2: DEVELOPERS.md content (Constraints + Technical Context) or "none"
- Layer 3: source file list (content if ≤ 10 files, listing only if > 10)
- Recent Spec Changes: parsed diff-spec-range JSON output
- Conventions: hierarchy-resolved from project root

### 4. Task(bugfixer) dispatch

```
Task(bugfixer):
  Session file: ${TMP_DIR}bugfix-session-{dir-safe}.md
  Target path: {path}
```

Extract from result block: `status`, `root_cause_layer`, `judgment`, `fix_type`, `fix_description`, `test_result`, `escalation` (if present), `proposed_change` (if present).

### 5. Result 처리

#### status == not_a_bug

```
현재 동작이 기대 동작과 일치합니다. 버그가 없거나 이미 수정되었습니다.
Expected: {E}
Actual: {A}
```
Exit.

#### judgment == ambiguous

AskUserQuestion with escalation format from `skills/bugfix/references/bugfix-templates.md`:

```
판단이 필요합니다.

## 현재 상황
- 사용자 기대 (E): "{escalation.expected}"
- 현재 동작 (A): "{escalation.actual}"
- CLAUDE.md: "{escalation.spec}"

## 판단 근거가 모호한 이유
"{escalation.reason}"

## 선택지
A) 스펙과 코드 모두 E에 맞게 수정한다
   → 실행 순서: CLAUDE.md 먼저 수정 → spec commit → /dev로 코드 재생성
B) 스펙을 수정한다 (E를 요구사항으로 추가/변경)
   → CLAUDE.md에 신규 Requirement 추가 → spec commit → /dev 재생성
C) 현재 동작(A)이 올바름 (버그 아님)
   → 버그 리포트 종료

어떻게 처리할까요?
```

Handle choice:
- A or B → treat as `root_cause_layer: 1` → Step 5a
- C → exit

#### root_cause_layer == 1 (unambiguous 또는 사용자가 A/B 선택)

Step 5a:
```
AskUserQuestion:
  "CLAUDE.md를 다음과 같이 수정하겠습니다.
  파일: {CLAUDE.md path}
  수정 내용: {proposed_change or fix_description}
  
  진행할까요?"
```

승인 시:
1. Edit CLAUDE.md (apply proposed change)
2. Commit:
   ```bash
   git add {CLAUDE.md path}
   git commit -m "spec({path}): fix requirement — {fix_description one-liner}"
   ```
3. /dev 재생성:
   ```
   Skill("claude-md-plugin:dev", args: "--path {path} --conflict overwrite")
   ```

Exit (no bugfix commit for L1 fix).

#### root_cause_layer == 2 (unambiguous)

Step 5b:
```
AskUserQuestion:
  "DEVELOPERS.md를 다음과 같이 수정하겠습니다.
  파일: {DEVELOPERS.md path}
  수정 내용: {proposed_change or fix_description}
  
  진행할까요?"
```

승인 시:
1. Edit DEVELOPERS.md (apply proposed change)
2. /dev 재생성:
   ```
   Skill("claude-md-plugin:dev", args: "--path {path} --conflict overwrite")
   ```

Exit (no bugfix commit for L2 fix; /dev creates dev commit internally).

#### root_cause_layer == 3 (unambiguous)

Read `test_result` from result block:
- `passed` → proceed to Step 6 (bugfix commit)
- `failed` → `"테스트 실패. fix_description: {fix_description}"` → exit status=failed
- `skipped` (fix_description mentions "/dev rerun"):
  ```
  Skill("claude-md-plugin:dev", args: "--path {path} --conflict overwrite")
  ```
  Exit (no bugfix commit; /dev creates dev commit).

#### root_cause_layer == multi

Process in order:
1. If L1 present → Step 5a (L1 fix with approval + spec commit + /dev)
2. If L2 present → Step 5b (L2 fix with approval + /dev)
3. Check if L3 issue remains after /dev rerun. If yes, and test_result == passed → Step 6.
4. If test_result == failed → exit status=failed.

#### status == failed

```
"버그 수정에 실패했습니다.
원인: {fix_description}
권고: DEVELOPERS.md Constraints를 검토하고 /dev를 재실행하거나, 직접 코드를 수정하세요."
```

### 6. bugfix commit (L3 fix 포함 시에만)

```bash
git add {modified source files} {added test files}
git commit -m "bugfix({path}): {fix_description one-liner}

Root cause: Layer 3 — {fix_description}

Changes:
- {list of modified/added files}"
```

### 7. Result

```
---bugfix-complete---
status: fixed | escalated | not_a_bug | failed
root_cause_layer: {N}
fix_type: {type}
summary: {one sentence}
---end-bugfix-complete---
```

## Error Handling

| Situation | Response |
|-----------|----------|
| E가 description에서 불명확 | AskUserQuestion: expected 동작 구체화 |
| CLAUDE.md 없음 | "CLAUDE.md를 찾을 수 없습니다." → exit |
| bugfixer agent 실패 | warn + escalate to user with raw error |
| /dev 재생성 실패 | report + exit status=failed |
| Layer 3: 테스트 3회 실패 | systematic-debugging 아키텍처 이슈 의심, 사용자 상담 요청 |

## DO / DON'T

**DO:**
- Always select CLAUDE.md before creating session file
- Include diff-spec-range output in session file
- Process L1 → L2 → L3 in that order for multi-layer
- Always ask user approval before modifying CLAUDE.md or DEVELOPERS.md

**DON'T:**
- Patch code without checking CLAUDE.md first (INV-bugfix-1)
- Skip user approval for L1/L2 modifications (INV-bugfix-2)
- Create bugfix commit for L1/L2 fixes (spec + dev commits are sufficient)
- Dispatch bugfixer agent without a complete session file
