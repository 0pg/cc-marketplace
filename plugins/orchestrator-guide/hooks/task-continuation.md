---
name: task-continuation
event: SessionStart  # IMPLEMENTED via session-start.sh enhancement
status: implemented
description: |
  세션 시작 시 미완료 작업을 감지하고 재개를 제안합니다.
  oh-my-opencode의 Todo Continuation Enforcement 패턴 적용.
  SessionIdle은 미지원이므로 SessionStart hook에 통합 구현.
config:
  idle_timeout_seconds: N/A  # SessionStart는 즉시 실행
  grace_period_ms: N/A
---

> **IMPLEMENTATION STATUS**: SessionStart hook (`session-start.sh`)에 통합 구현됨.
> SessionIdle 대신 세션 시작 시점에 이전 세션의 미완료 작업을 감지합니다.
>
> **Detection Priority**:
> 1. `[~]` 진행중 작업 (task.md)
> 2. `active_tasks` (orchestrator-state.json)
> 3. 미완료 ARB (status != success/completed)
> 4. `[ ]` 대기 작업 (10개 미만일 때만)

# Task Continuation Hook

> 미완료 작업을 감지하고 자동 재개를 제안하는 훅

## 목적

- 작업 조기 종료 방지
- 미완료 작업 자동 감지
- 세션 연속성 보장
- 사용자 의도 확인

---

## 트리거 조건

```yaml
trigger:
  event: SessionIdle
  timeout: 5초

conditions:
  - TodoWrite에 in_progress 상태 항목 존재
  - 또는 orchestrator-state.json에 active_tasks 존재
  - 또는 task.md에 [~] 상태 항목 존재
```

---

## 감지 로직

### 1. TodoWrite 확인

```yaml
check:
  - in_progress 상태 항목 확인
  - pending 상태 항목 확인

if_found:
  - 미완료 작업으로 판정
  - 재개 제안
```

### 2. orchestrator-state.json 확인

```yaml
check:
  - active_tasks 배열 확인
  - pending_handoffs 배열 확인

if_found:
  - 미완료 작업으로 판정
  - 재개 제안
```

### 3. task.md 확인

```yaml
check:
  - [~] 상태 항목 확인 (진행 중)
  - depends가 만족된 [ ] 항목 확인

if_found:
  - 다음 작업 제안
```

---

## 재개 제안 메시지

### 기본 형식

```markdown
⏰ 미완료 작업 감지

**작업**: {task_name}
**상태**: {status}
**진행률**: {progress or "알 수 없음"}

계속 진행하시겠습니까?
- [Y] 이어서 진행
- [N] 작업 종료
- [S] 다른 작업으로 전환

5초 후 자동으로 [Y] 선택됩니다...
```

### 상세 형식 (여러 작업)

```markdown
⏰ 미완료 작업 목록

| # | 작업 | 상태 | 우선순위 |
|---|------|------|----------|
| 1 | {task_1} | in_progress | high |
| 2 | {task_2} | pending | medium |
| 3 | {task_3} | pending | low |

어떤 작업을 진행하시겠습니까?
- [1-3] 해당 작업 진행
- [A] 순서대로 모두 진행
- [N] 작업 종료
```

---

## 자동 재개 로직

### 카운트다운

```yaml
countdown:
  duration: 5초
  default_action: 재개 (Y)

cancel_conditions:
  - 사용자 입력 감지
  - 명시적 거부 (N)
```

### Grace Period

```yaml
grace_period:
  duration: 500ms
  purpose: |
    빠른 연속 상호작용 시 불필요한 재개 방지
    사용자가 의도적으로 멈춘 경우와 구분
```

---

## 재개 액션

### 단일 작업 재개

```yaml
action: |
  1. task.md에서 작업 상세 확인
  2. 이전 ARB 확인 (있으면)
  3. 적절한 에이전트에 위임
  4. TodoWrite 상태 업데이트
```

### 체인 재개

```yaml
action: |
  1. pending_handoffs 확인
  2. 다음 에이전트 결정
  3. 이전 ARB 컨텍스트로 위임
  4. 체인 계속 진행
```

---

## 사용자 선택 처리

### [Y] 이어서 진행

```yaml
action: |
  - 마지막 상태에서 재개
  - 에이전트에 컨텍스트 전달
  - TodoWrite 업데이트
```

### [N] 작업 종료

```yaml
action: |
  - 현재 상태 저장
  - orchestrator-state.json 업데이트
  - "작업이 일시 중지되었습니다" 메시지
  - 다음 세션에서 재개 가능
```

### [S] 다른 작업으로 전환

```yaml
action: |
  - 현재 작업 pending으로 변경
  - AskUserQuestion으로 다음 작업 선택
  - 선택된 작업으로 전환
```

---

## Background Task 인식

### 백그라운드 작업 감지

```yaml
check:
  - run_in_background로 실행된 Task 확인
  - 아직 완료되지 않은 백그라운드 작업

if_running:
  - 재개 제안 억제
  - "백그라운드 작업 진행 중" 표시
```

### 백그라운드 완료 시

```yaml
on_complete:
  - 결과 알림
  - 다음 단계 제안
  - 필요 시 핸드오프 실행
```

---

## 상태 업데이트

### TodoWrite 동기화

```yaml
sync:
  - task.md [~] → in_progress
  - task.md [x] → completed
  - task.md [ ] → pending
```

### orchestrator-state.json 업데이트

```yaml
update:
  - active_tasks 갱신
  - pending_handoffs 갱신
  - 타임스탬프 갱신
```

---

## 예외 처리

### 충돌 상태

```yaml
condition: |
  TodoWrite와 task.md 상태 불일치

action: |
  "상태 불일치가 감지되었습니다."
  "task.md 기준으로 동기화하시겠습니까? [Y/n]"
```

### 오래된 작업

```yaml
condition: |
  마지막 업데이트가 24시간 이상 전

action: |
  "오래된 작업이 감지되었습니다."
  "새로 시작하시겠습니까? [Y/n]"
```

---

## 사용 예시

### 정상 재개

```
[5초 유휴 감지]

⏰ 미완료 작업 감지

**작업**: TASK-005: User API 구현
**상태**: in_progress
**진행률**: clippy 통과, 테스트 대기

계속 진행하시겠습니까?
- [Y] 이어서 진행
- [N] 작업 종료

5초 후 자동으로 [Y] 선택됩니다... 4... 3... 2... 1...

[자동 재개]
→ 테스트 실행 계속...
```

### 백그라운드 작업 중

```
[5초 유휴 감지]

🔄 백그라운드 작업 진행 중

- backend-impl: 실행 중 (2분 경과)
- frontend-impl: 실행 중 (2분 경과)

완료 시 알림을 받으시겠습니까? [Y/n]
```

---

## Claude Code Hook Event Limitations

### Supported Events (as of 2025)

Claude Code supports the following hook events:

| Event | Description |
|-------|-------------|
| `SessionStart` | Session begins |
| `SessionEnd` | Session ends |
| `UserPromptSubmit` | User submits prompt |
| `PreToolUse` | Before tool execution |
| `PostToolUse` | After tool execution |
| `PostToolUseFailure` | Tool execution failed |
| `Notification` | Notifications sent |
| `Stop` | Main agent finished |
| `SubagentStart` | Subagent starts |
| `SubagentStop` | Subagent finished |
| `PreCompact` | Before context compaction |
| `PermissionRequest` | Permission dialog shown |

### NOT Supported

- `SessionIdle` - **This event does not exist**
- Any time-based/polling hooks
- Inactivity detection hooks

---

## Alternative Approaches

Since `SessionIdle` is not supported, consider these alternatives:

### Option 1: Stop Hook + State Check

Use the `Stop` hook to check for incomplete tasks when Claude finishes responding.

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/plugins/orchestrator-guide/hooks/check-incomplete-tasks.sh"
          }
        ]
      }
    ]
  }
}
```

**Pros**: Fires after each response, can remind about pending tasks
**Cons**: Only triggers after Claude stops, not during idle time

### Option 2: SessionStart Hook Enhancement

Enhance the existing `SessionStart` hook to check for incomplete tasks from previous sessions.

```bash
# In session-start.sh, add:
if [ -f "$CLAUDE_PROJECT_DIR/spec/task.md" ]; then
  # Check for [~] (in-progress) tasks
  if grep -q "^\[~\]" "$CLAUDE_PROJECT_DIR/spec/task.md"; then
    echo "WARNING: Incomplete tasks detected from previous session"
  fi
fi
```

**Pros**: Works within existing infrastructure
**Cons**: Only at session start, not during session

### Option 3: Notification Hook

The `Notification` hook fires when "Claude is waiting for your input" (after 60 seconds idle).

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/plugins/orchestrator-guide/hooks/on-notification.sh"
          }
        ]
      }
    ]
  }
}
```

**Pros**: Closest to "idle" detection (fires after 60s of input idle)
**Cons**: Fixed 60-second timeout, cannot customize; fires for other notifications too

### Option 4: Manual Workflow (Recommended)

Implement task continuation as a manual check pattern in orchestrator workflow:

1. **At session start**: SessionStart hook reports incomplete tasks
2. **Before delegating**: Orchestrator checks task.md for [~] items
3. **After completion**: Stop hook reminds about remaining tasks
4. **User prompt**: Add `/continue` or `/resume` skill command

This is the most reliable approach within Claude Code's current limitations.

---

## Recommended Implementation

Given the limitations, implement a hybrid approach:

### 1. Enhance SessionStart (already registered)

Add incomplete task detection to existing `session-start.sh`.

### 2. Add Stop Hook (new registration)

Register Stop hook to remind about pending tasks after each response.

### 3. Create `/continue` Skill

Create a skill that can be manually invoked to check and resume tasks.

### Registration Status

| Hook | Event | Status |
|------|-------|--------|
| session-start | SessionStart | Registered |
| task-continuation | SessionIdle | **Not Supported** |
| stop-check | Stop | Recommended |
| notification-check | Notification | Optional |
