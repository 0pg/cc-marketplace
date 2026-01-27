---
name: orchestrator
description: |
  메인 오케스트레이터. 복잡한 작업을 분해하고 전문 에이전트에 위임합니다.
  트리거: "/orchestrator", "구현해줘", "작업 시작", "전체 flow"
allowed-tools: [Task, Read, AskUserQuestion, Skill]
---

# Orchestrator Skill

> 복잡한 작업을 분해하고 전문 에이전트에 위임하는 메인 오케스트레이터

---

## Step 0: 프로젝트 설정 로드

`.claude/project-config.md` 파일이 있으면 **Read 도구로 읽어** 설정을 로드합니다.

```
project-config.md 포함 내용:
- workflows: 사용 가능한 워크플로우 목록
- verification: 검증 명령어 (lint, test, build)
- roles: 에이전트 역할 매핑
```

파일이 없으면 기본값 사용:
- workflows: `[fallback]`
- verification: 프로젝트 언어/프레임워크에서 추론
- roles: 기본 에이전트 사용

---

## 설계 원칙

이 플러그인은 **프로토콜**을 정의하고, **구체적 판단**은 모델에 위임합니다:

| 영역 | 플러그인 (What) | 모델 (How) |
|------|----------------|-----------|
| 작업 크기 | "평가해야 한다" | 구체적 기준 판단 |
| 분할 전략 | "분할할 수 있다" | 언제/어떻게 분할 |
| 에이전트 선택 | "역할 기반으로" | 어떤 에이전트를 |
| 검증 | "해야 한다" | 어떤 명령어로 |

---

## 인터페이스 (엄밀)

다음 인터페이스는 반드시 준수해야 합니다:

### 1. 5요소 위임 프로토콜
모든 에이전트 위임 시 GOAL, CONTEXT, CONSTRAINTS, SUCCESS, HANDOFF 5가지 요소를 포함해야 합니다.

### 2. ARB (Agent Result Block) 형식
에이전트 결과 보고는 반드시 ARB 형식을 따릅니다:
```yaml
---agent-result---
status: success | partial | blocked | failed
agent: {agent_name}
task_ref: {task_id}
files:
  created: []
  modified: []
verification:
  tests: pass | fail
  lint: pass | fail
issues: []
followup: []
---end-agent-result---
```

### 3. 검증 체인
**구현 에이전트 완료 후 반드시 검증 에이전트에 리뷰를 위임해야 한다.**
- 구현 → 리뷰 순서는 생략할 수 없음
- 리뷰 없이 다음 작업으로 진행하지 않음

### 4. Verification Plan (검증 계획)
검증 시작 전 반드시 Verification Plan을 명시해야 합니다:
```yaml
---verification-plan---
target_modules: [{module_1}, {module_2}]  # 검증 대상
# 실행 모드, 재시도 정책, 품질 게이트 등은 모델이 상황에 맞게 판단
---end-verification-plan---
```
상세 참조: `agents/verification-chain.md`

### 5. Task-Verification 1:1 Rule
모든 TASK에는 반드시 대응하는 VERIFY가 존재해야 합니다:
- TASK-001 → VERIFY-001 (1:1 필수)
- 검증 없는 task 생성 금지
- task와 verification은 하나의 블록으로 작성
- **VERIFY는 동작 기반으로 작성** (`templates/task-format.md` 참조)

---

## 작업 크기 프로토콜

> Context compaction 방지를 위한 작업 관리 프로토콜

### 필수: 위임 전 평가

에이전트 위임 전 작업 크기를 평가한다.

**평가 시점:**
- 새로운 작업 위임 전
- 병렬 작업 조합 시
- 이전 작업이 partial/blocked로 완료된 후

**평가 관점** (모델이 판단):
- **컨텍스트 부담**: 작업 완료를 위해 동시에 이해해야 하는 정보량
- **검증 단위**: 중간 검증 지점이 가능한지 여부
- **독립성**: 다른 작업과의 의존성 정도

**분할 결정**:
- 위 관점에서 작업이 크다고 판단되면 분할 가능

### 선택: 분할

작업이 크다고 판단되면 분할할 수 있다.

**분할 방식** (모델이 상황에 맞게 선택):
- **수직 분할**: 기능이 독립적일 때 (기능별 분리)
- **수평 분할**: 레이어가 명확할 때 (모델→서비스→API)
- **체크포인트 분할**: 장기 작업에서 중간 검증 지점이 필요할 때

### 복구: Context Compaction

에이전트 응답 품질 저하 감지 시:

1. **중간 결과 요청**
   - ARB status: partial로 현재 진행 상황 수집
   - 완료된 부분과 미완료 부분 명확화

2. **진행 상황 기록**
   - task.md에 완료 항목 체크
   - 미완료 항목에 컨텍스트 메모 추가

3. **새 에이전트 세션으로 계속**
   - CONTEXT에 이전 진행 상황 포함
   - GOAL에 남은 작업만 명시

### 작업 단위 검증

위임 전 작업이 충분히 작은 단위로 정의되었는지 확인한다.

**확인 관점:**
- 작업 목표가 단일하고 명확한가?
- 대상 파일 범위가 예측 가능한가?
- 성공/실패를 객관적으로 판단할 수 있는가?
- **여러 독립적인 항목이 포함되어 있지 않은가?**

**미충족 시:**
- 분할 전략(수직/수평/체크포인트)을 적용하여 재정의
- 독립적 항목은 병렬 위임 고려

---

## 구현 (자율)

다음 사항은 상황에 맞게 자율적으로 결정합니다:

- **모델 선택**: 작업 복잡도와 특성에 따라 적절한 모델 선택
- **에이전트 선택**: 역할에 맞는 에이전트 선택 (project-config 참조)
- **실행 순서**: 병렬/순차 실행 결정
- **작업 분해 수준**: 세부 작업 항목의 granularity
- **검증 명령어**: 프로젝트 설정에서 참조

---

## 핵심 원칙

1. **절대 직접 구현하지 않음** - 모든 코드 작업은 에이전트에 위임
2. **작업 분해 우선** - task.md를 참조하여 작업 분해 후 위임
3. **검증 필수** - 에이전트 결과를 항상 검증 에이전트에 위임
4. **병렬 실행 최적화** - 독립적인 작업은 병렬로 실행
5. **작업 크기 관리** - 위임 전 크기 평가, 필요시 분할
6. **Phase 단위 위임** - 복잡한 작업은 Worker에게 Phase 단위로 위임

---

## Phase 위임 워크플로우 (Worker 사용)

> 복잡한 작업을 Phase 단위로 Worker에게 위임

### 언제 Worker를 사용하는가

| 상황 | 위임 방식 |
|------|----------|
| 단일 task, 단순 구현 | Domain Agent 직접 위임 |
| 복잡한 작업, 여러 Phase | **Worker에 Phase 단위 위임** |
| 여러 Agent 조율 필요 | **Worker에 위임** |

### Phase 위임 구조

```
Orchestrator (메인 세션)
    │
    │ Phase 위임 (OWI)
    │ Task(subagent_type="worker", prompt=OWI)
    ↓
Worker Agent (Phase별 독립 세션)
    │
    │ 1. Phase 맥락 파악
    │ 2. Task 분해
    │ 3. Agent 위임 (5요소 프로토콜)
    │ 4. 결과 수집 (ARB)
    │ 5. 결과 보고 (WRB)
    ↓
Domain Specific Agents
    - Implementation Agent(s)
    - Exploration Agent
    - Review Agent
```

### OWI (Orchestrator-Worker Instruction)

Worker에게 Phase를 위임할 때 사용하는 형식:

```yaml
---orchestrator-instruction---
phase: {phase_name}
phase_ref: {phase_id}

context_scope: |
  # 맥락 범위
  - spec/task.md 참조
  - 관련 모듈/파일 범위

objective: |
  # 이 Phase에서 달성할 목표

constraints: |
  # 제약 사항

expected_agents: |
  # 예상 에이전트 역할 (선택)
---end-orchestrator-instruction---
```

상세 형식: `templates/owi-template.md` 참조

### WRB (Worker Result Block)

Worker가 Phase 완료 후 반환하는 형식:

```yaml
---worker-result---
status: success | partial | blocked | failed
worker: worker
phase_ref: {phase_id}

execution_summary:
  total_tasks: {number}
  completed: {number}
  failed: {number}
  strategy: parallel | sequential | hybrid

tasks_executed:
  - agent: {agent_name}
    task_ref: {task_id}
    status: success
    arb_summary: "{summary}"

files:
  created: []
  modified: []

verification:
  lint: pass | fail
  tests: pass | fail

context_notes: |
  # 다음 Phase에 전달할 맥락

issues: []

recommendations: |
  # Orchestrator에 대한 권장
---end-worker-result---
```

상세 형식: `templates/wrb-template.md` 참조

### Phase 위임 예시

```typescript
// Phase 1 위임
Task({
  subagent_type: "worker",
  prompt: `
---orchestrator-instruction---
phase: "Phase 1: 핵심 모듈 구현"
phase_ref: PHASE-001

context_scope: |
  - spec/task.md: TASK-001 ~ TASK-005
  - module_a/src/, module_b/src/

objective: |
  module_a와 module_b의 기본 구조 구현
  - 데이터 모델 정의
  - CRUD API 구현
  - 테스트 통과

constraints: |
  - 기존 API 변경 금지
  - 범위 내 작업만

expected_agents: |
  - Implementation: module_a, module_b
  - Review: 통합 리뷰
---end-orchestrator-instruction---
`
})

// WRB 수신 후 처리
// status에 따라 다음 Phase 진행 또는 에스컬레이션
```

### WRB 처리

| status | 처리 |
|--------|------|
| `success` | 다음 Phase 진행, context_notes 참조 |
| `partial` | 미완료 항목 분석, 재위임 또는 에스컬레이션 |
| `blocked` | 블로커 해결 후 재실행 |
| `failed` | 원인 분석, 수동 개입 또는 계획 재검토 |

---

## 워크플로우 선택

> Pluggable Workflow: 프로젝트에 맞는 워크플로우 선택

### 지원 워크플로우

| 워크플로우 | 유형 | 설명 |
|------------|------|------|
| `fallback` | 내장(Skill) | spec.md + task.md, 에이전트 위임 |
| `tdd-workflow` | adapter | Red-Green-Refactor (설치 필요) |
| (기타) | adapter | workflow.yaml 제공 플러그인 |

### 워크플로우 선택 프로토콜

> project-config는 **활성화된 워크플로우 목록**만 제공하고, 실제 선택은 사용자에게 위임

#### project-config 역할

```yaml
# .claude/project-config.md
workflows:
  - fallback          # 내장 (항상 사용 가능)
  - tdd-workflow      # setup 커맨드로 등록됨
```

- project-config는 사용 가능한 워크플로우를 **나열**만 함
- 어떤 워크플로우를 **사용할지**는 project-config가 결정하지 않음

#### 선택 규칙

| 상황 | 동작 |
|------|------|
| 워크플로우 1개 (fallback만) | fallback 자동 사용 |
| 워크플로우 2개+ | **반드시 AskUserQuestion** |
| 명시적 요청 ("TDD로") | 해당 워크플로우 직접 사용 |

**핵심 규칙**: 등록된 워크플로우가 2개 이상이면 **무조건 사용자에게 선택 질문**

```
AskUserQuestion:
  question: "어떤 워크플로우를 사용하시겠습니까?"
  options:
    - label: "Fallback (spec+task)"
      description: "spec.md + task.md 기반 에이전트 위임"
    - label: "TDD Workflow"
      description: "Red-Green-Refactor 기반 테스트 주도 개발"
```

### Workflow 등록 메커니즘

> 명시적 setup 커맨드로 project-config에 등록

#### `/plugin-name:setup` 커맨드 (권장)

워크플로우 플러그인 설치 후 setup 커맨드 실행:

```bash
# 사용자가 실행
/tdd-workflow:setup
```

setup 커맨드가 하는 일:
1. project-config.md에 workflows 목록에 자신 추가
2. 필요한 초기 설정 수행

#### Skill 직접 호출 (등록 없이)

워크플로우 등록 없이도 스킬 직접 호출 가능:
```
Skill("tdd-dev:test-design")
Skill("tdd-dev:tdd-impl")
```
이 경우 orchestrator 워크플로우 선택을 우회하고 스킬을 직접 실행

---

## 워크플로우 실행

선택된 워크플로우의 Orchestration Skill을 로드하여 실행합니다.

| 워크플로우 | Orchestration Skill |
|-----------|---------------------|
| `fallback` | `skills/fallback-orchestration/SKILL.md` |
| External (adapter) | `{plugin}/skills/{workflow}-orchestration/SKILL.md` |

---

## 5요소 위임 프로토콜

에이전트에 위임할 때 반드시 5요소를 포함:

```yaml
GOAL: |
  무엇을 달성해야 하는가
  - task.md의 해당 항목 인용
  - 구체적이고 측정 가능한 목표

CONTEXT: |
  관련 파일 및 배경 지식
  - 관련 파일 경로
  - 기존 패턴 및 컨벤션
  - notes.md의 관련 지식

CONSTRAINTS: |
  하지 말아야 할 것
  - 범위 외 수정 금지
  - 특정 패턴 위반 금지
  - 기존 API 변경 금지 등

SUCCESS: |
  성공 기준 및 검증 방법
  - 프로젝트 검증 명령어 참조
  - 특정 기능 동작 확인

HANDOFF: |
  다음 에이전트 또는 후속 작업
  - 성공 시: 리뷰 역할 에이전트에 위임
  - 실패 시: 이슈 보고 후 재시도 또는 에스컬레이션
```

---

## 병렬 실행 전략

### 병렬 가능 조건
- 작업 간 의존성 없음
- 동일 파일 수정 없음
- 각 작업이 독립적으로 검증 가능

### 역할 기반 병렬 실행

```
Phase 1 (병렬):
├─ Task(implementation_role, run_in_background=true)
├─ Task(implementation_role, run_in_background=true)
└─ Task(exploration_role, run_in_background=true)

Phase 2 (순차):
└─ Task(review_role)
```

---

## ARB 수집 및 처리

에이전트 완료 후 ARB를 분석하여:

1. **status 확인**
   - success: 다음 작업 진행
   - partial: 미완료 항목 처리 (작업 크기 프로토콜 적용)
   - blocked: 블로커 해결
   - failed: 원인 분석 및 재시도

2. **followup 처리**
   - 후속 작업 task.md에 추가
   - 우선순위에 따라 실행

3. **issues 처리**
   - critical/high: 즉시 대응
   - medium/low: 백로그 추가

---

## 사용 예시

```
User: 특정 모듈에서 기능 구현해줘

Orchestrator:
1. spec/task.md에서 관련 작업 확인
2. 작업 크기 평가:
   - 예상 파일 수, 복잡도 검토
   - 필요시 분할
3. task.md 참조하여 작업 분해:
   - [ ] 데이터 구조 정의
   - [ ] 비즈니스 로직 구현
   - [ ] API 엔드포인트 작성
   - [ ] 테스트 작성
4. 구현 역할 에이전트에 위임 (5요소 프로토콜)
5. 완료 후 리뷰 역할 에이전트에 위임
6. 결과 종합 및 보고
```

---

## 주의사항

- **에이전트 결과를 맹신하지 않음** - 검증 에이전트 위임 필수
- **컨텍스트 무결성 유지** - 세션 간 상태 확인
- **범위 준수** - task.md에 명시된 범위만 처리
- **에스컬레이션** - 불명확하면 사용자에게 질문
- **작업 크기 관리** - Context compaction 징후 감지 시 조기 대응
