---
name: session-start
event: SessionStart
description: |
  세션 시작 시 오케스트레이터 컨텍스트를 주입합니다.
  작업 대상 crate, task.md, notes.md를 읽어 컨텍스트를 구성합니다.
---

# Session Start Hook

> 세션 시작 시 자동으로 오케스트레이터 컨텍스트를 주입하는 훅

## 실행 시점

- 새 세션 시작 시
- 세션 재개 시
- 컨텍스트 새로고침 요청 시

---

## 실행 프로세스

### Step 1: 대상 Crate 파악

```yaml
methods:
  - 사용자 요청에서 crate 명시 확인
  - 현재 작업 디렉터리 기반 추론
  - 이전 세션의 컨텍스트 확인
  - 불명확 시 사용자에게 질문

priority:
  1. 명시적 지정
  2. 이전 세션 컨텍스트
  3. 사용자 질문
```

### Step 2: spec/task.md 읽기

```yaml
path: crates/<crate>/spec/task.md

extract:
  - 현재 진행 중 작업 [~]
  - 다음 작업 [ ]
  - 완료된 작업 [x]
  - 의존성 관계
```

### Step 3: spec/notes.md 읽기 (존재 시)

```yaml
path: crates/<crate>/spec/notes.md

extract:
  - 학습된 패턴
  - 이전 결정 사항
  - 주의사항
  - 블로커 및 해결책
```

### Step 4: orchestrator-state.json 확인

```yaml
path: .claude/orchestrator-state.json

extract:
  - 마지막 세션 ID
  - 활성 작업
  - 완료된 ARB
  - 대기 중인 핸드오프
```

---

## 주입 메시지 형식

```markdown
## 오케스트레이터 세션 컨텍스트

### 대상 Crate
- **crate**: {crate_name}
- **경로**: crates/{crate_name}

### 현재 작업
- **진행 중**: {current_task or "없음"}
- **다음 작업**: {next_task or "없음"}
- **블로커**: {blocker or "없음"}

### 이전 세션
- **마지막 ARB**: {last_arb_status}
- **대기 핸드오프**: {pending_handoff or "없음"}

### notes.md 요약
{notes_summary or "notes.md 없음"}

---

**안내**: 불명확한 부분이 있으면 질문하세요.
위임 시 5요소 프로토콜(GOAL/CONTEXT/CONSTRAINTS/SUCCESS/HANDOFF)을 사용하세요.
```

---

## 컨텍스트 무결성 검사

### 검사 항목

```yaml
session_integrity:
  - 에이전트 불신 원칙 이해 여부
  - 위임 4요소(→5요소) 인식 여부
  - 병렬 실행 전략 인식 여부

state_integrity:
  - orchestrator-state.json 유효성
  - task.md와 state 일관성
  - 미완료 ARB 존재 여부
```

### 무결성 경고

```markdown
⚠️ 컨텍스트 무결성 경고

다음 항목에서 불일치가 감지되었습니다:
- {inconsistency_description}

권장 조치:
- {recommended_action}

새 세션 시작을 권장합니까? [Y/n]
```

---

## 상태 복구

### orchestrator-state.json 복구

```yaml
recovery_actions:
  - active_tasks가 있으면 상태 확인
  - pending_handoffs가 있으면 다음 단계 안내
  - completed_arbs에서 마지막 결과 요약
```

### task.md 동기화

```yaml
sync_actions:
  - state의 active_tasks와 task.md의 [~] 비교
  - 불일치 시 task.md 기준으로 복구
  - 복구 내역 사용자에게 보고
```

---

## 에러 처리

### task.md 없음

```yaml
action: |
  "spec/task.md가 없습니다."
  "/planner 스킬을 사용하여 계획을 먼저 수립하세요."
```

### crate 불명확

```yaml
action: |
  AskUserQuestion:
    "어떤 crate에서 작업하시겠습니까?"
    options: [backend, frontend, backtest, Other]
```

### state 손상

```yaml
action: |
  "orchestrator-state.json이 손상되었습니다."
  "task.md 기준으로 상태를 재구성합니다."
  # state 초기화 및 task.md 기반 재구성
```

---

## 사용 예시

### 정상 시작

```
[SessionStart Hook 실행]

## 오케스트레이터 세션 컨텍스트

### 대상 Crate
- **crate**: backend
- **경로**: crates/backend

### 현재 작업
- **진행 중**: TASK-005: User API 구현
- **다음 작업**: TASK-006: 인증 미들웨어
- **블로커**: 없음

### 이전 세션
- **마지막 ARB**: success (backend-impl)
- **대기 핸드오프**: rust-code-reviewer

### notes.md 요약
- SQLite created_at은 TEXT(ISO 8601)로 저장
- argon2 사용하여 비밀번호 해시

---
안내: 불명확한 부분이 있으면 질문하세요.
```

### 복구 필요 시

```
[SessionStart Hook 실행]

⚠️ 이전 세션 미완료 작업 감지

- **작업**: TASK-005: User API 구현
- **상태**: in_progress
- **마지막 ARB**: partial

다음 중 선택하세요:
1. 이어서 작업 계속
2. 처음부터 다시 시작
3. 다른 작업으로 전환
```
