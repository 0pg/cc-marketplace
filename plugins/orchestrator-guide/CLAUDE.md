# orchestrator-guide Plugin Development Guide

## CRITICAL: Protocol vs Implementation Separation

**이 플러그인의 최우선 설계 원칙입니다. 모든 수정/추가 작업에서 반드시 준수하세요.**

```
플러그인 = WHAT (프로토콜)    ←→    모델 = HOW (구현)
```

### 원칙

- **플러그인은 "무엇을 해야 하는가"만 정의한다**
- **플러그인은 "어떻게 해야 하는가"를 정의하지 않는다**
- **구체적 판단과 실행은 모델에 위임한다**

### 올바른 예시 vs 잘못된 예시

| 영역 | 플러그인 (What) | 모델 (How) |
|------|----------------|-----------|
| 작업 크기 | "위임 전 크기를 평가해야 한다" | 파일 수, 복잡도 등 구체적 기준 판단 |
| 분할 | "크면 분할할 수 있다" | 수직/수평/체크포인트 중 선택 |
| 에이전트 | "역할 기반으로 선택한다" | 어떤 에이전트를 쓸지 결정 |
| 검증 | "검증해야 한다" | 어떤 명령어로 검증할지 결정 |
| 모델 선택 | "적절한 모델을 사용한다" | haiku/sonnet/opus 중 선택 |

**잘못된 작성:**
```markdown
# BAD - 구체적 기준을 플러그인이 정의
작업이 5개 파일 이상이면 분할하세요.
haiku는 간단한 작업에, opus는 복잡한 작업에 사용하세요.
```

**올바른 작성:**
```markdown
# GOOD - 프로토콜만 정의, 판단은 모델에 위임
위임 전 작업 크기를 평가하세요.
크다고 판단되면 분할할 수 있습니다.
작업 복잡도에 따라 적절한 모델을 선택하세요.
```

### 이 원칙이 중요한 이유

1. **프로젝트 독립성**: 구체적 기준은 프로젝트마다 다름
2. **모델 역량 활용**: 모델이 컨텍스트를 보고 최적 판단
3. **유지보수성**: 프로토콜만 바꾸면 모든 프로젝트에 적용
4. **유연성**: 새로운 상황에도 모델이 적응 가능

---

## Project Structure

```
orchestrator-guide/
├── .claude-plugin/
│   └── plugin.json           # 플러그인 매니페스트
├── config/
│   └── project-config.template.md  # 사용자용 설정 템플릿
├── skills/                   # 스킬 정의 (Claude Code 자동 감지)
│   ├── orchestrator/SKILL.md # 메인 오케스트레이션
│   ├── planner/SKILL.md      # 계획 및 스펙 생성
│   ├── delegator/SKILL.md    # 위임 프롬프트 생성
│   └── rust-code-convention/SKILL.md
├── hooks/                    # 훅 정의
│   ├── hooks.json            # 훅 설정
│   ├── session-start.sh      # SessionStart 스크립트
│   └── *.md                  # 훅 문서
├── agents/                   # 에이전트 패턴 문서
│   ├── worker.md             # Worker Agent (Phase 단위 위임)
│   ├── verification-chain.md
│   └── test-quality-reviewer.md
└── templates/                # 위임 템플릿
    ├── delegation-prompt.md
    ├── arb-template.md
    ├── owi-template.md       # Orchestrator-Worker Instruction
    ├── wrb-template.md       # Worker Result Block
    ├── spec-format.md
    └── task-format.md
```

## Core Interfaces

수정 시 반드시 유지해야 하는 인터페이스:

### 1. 5요소 위임 프로토콜
- GOAL, CONTEXT, CONSTRAINTS, SUCCESS, HANDOFF

### 2. ARB (Agent Result Block)
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

### 3. Verification Plan
```yaml
---verification-plan---
target_modules: [{module_1}, {module_2}]
---end-verification-plan---
```

### 4. OWI (Orchestrator-Worker Instruction)
Orchestrator가 Worker에게 Phase를 위임할 때 사용:
```yaml
---orchestrator-instruction---
phase: {phase_name}
phase_ref: {phase_id}
context_scope: |
  # 맥락 범위
objective: |
  # Phase 목표
constraints: |
  # 제약 사항
expected_agents: |
  # 예상 에이전트 역할
---end-orchestrator-instruction---
```

### 5. WRB (Worker Result Block)
Worker가 Phase 완료 후 Orchestrator에 보고:
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
  # 다음 Phase 전달 맥락
issues: []
recommendations: |
  # Orchestrator 권장사항
---end-worker-result---
```

## Adding New Skills

1. `skills/{skill-name}/SKILL.md` 생성
2. YAML frontmatter 필수:
```yaml
---
name: skill-name
description: |
  스킬 설명. 트리거 키워드 포함.
allowed-tools: [Tool1, Tool2]
---
```
3. 본문에 프로토콜 작성 (구체적 구현 판단은 모델에 위임)

## Adding New Hooks

1. `hooks/` 에 스크립트 추가 (`.sh`)
2. `hooks/hooks.json` 에 등록:
```json
{
  "hooks": {
    "HookType": [{
      "hooks": [{
        "type": "command",
        "command": "path/to/script.sh"
      }]
    }]
  }
}
```
3. 문서화용 `.md` 파일 추가

## Modifying Templates

`templates/` 디렉토리의 마크다운 파일 수정.
- `delegation-prompt.md`: 역할 기반 위임 템플릿
- `arb-template.md`: 결과 보고 형식

## Testing

플러그인 설치 후 테스트:
1. `/orchestrator` - 오케스트레이션 워크플로우
2. `/planner` - 계획 생성
3. `/delegator` - 위임 프롬프트 생성

## File Conventions

- 스킬 파일: `SKILL.md` (대문자)
- 훅 스크립트: `kebab-case.sh`
- 문서: `kebab-case.md`
- 설정: `kebab-case.json`