# Workflow Protocol

> 워크플로우 adapter 플러그인이 구현해야 하는 인터페이스

## 개요

Orchestrator는 pluggable workflow를 지원합니다:
- **Default workflow**: spec.md + task.md 기반 (orchestrator 내장)
- **External workflow**: Skill 기반 adapter 플러그인

## Workflow Adapter 요구사항

### 필수 구조

> **검증 결과**: Claude Code는 workflow.yaml을 자동 인식하지 않음.
> 워크플로우 adapter 플러그인은 **Skill 기반** 구조 사용.

워크플로우 adapter 플러그인 구조:

```
{workflow-name}/
├── .claude-plugin/
│   └── plugin.json
├── commands/
│   └── setup.md              # 워크플로우 등록 커맨드
└── skills/
    └── {workflow}-orchestration/
        ├── SKILL.md          # 워크플로우 정의 (phases + verification)
        └── references/       # 참조 문서
```

### Orchestration Skill 형식

```markdown
---
name: {workflow}-orchestration
description: |
  워크플로우 설명.
  "트리거 키워드1", "트리거 키워드2" 요청 시 활성화.
---

# {Workflow Name} Orchestration

## Phases

### Phase 1: {phase_name}
- 호출: `Skill("plugin:skill")`
- 산출물: {expected_output}
- 완료 조건: {success_criteria}

### Phase N: ...

## Verification

검증 체인: `references/verification-chain.md` 참조
```

### 예시: TDD Orchestration Skill

```markdown
---
name: tdd-orchestration
description: |
  TDD/ATDD 기반 개발 워크플로우.
  "TDD로", "테스트 주도", "Red-Green-Refactor" 요청 시 활성화.
---

# TDD Orchestration

## Phases

### Phase 1: 테스트 설계
- 호출: `Skill("tdd-dev:test-design")`
- 산출물: `.claude/tdd-spec.md`
- 완료 조건: tdd-spec.md 생성

### Phase 2: TDD 구현
- 호출: `Skill("tdd-dev:tdd-impl")`
- 산출물: 테스트 코드 + 구현 코드
- 완료 조건: 모든 테스트 통과

## Verification

검증 체인: `references/verification-chain.md` 참조
```

## Workflow 등록 메커니즘

> 명시적 setup 커맨드로 project-config에 등록

### 등록 흐름

```
1. 사용자가 workflow 플러그인 설치
2. 사용자가 /plugin-name:setup 실행
3. setup 커맨드가 project-config에 워크플로우 등록
4. Orchestrator가 project-config의 workflows 목록 참조
```

### project-config 형식

```yaml
# .claude/project-config.md
workflows:
  - default           # 항상 존재 (내장)
  - tdd-workflow      # setup으로 등록됨
  - custom-workflow   # setup으로 등록됨
```

### Orchestrator의 워크플로우 선택

| 상황 | 동작 |
|------|------|
| workflows 1개 | 자동 선택 |
| workflows 2개+ | **AskUserQuestion으로 선택 요청** |
| 명시적 요청 ("TDD로") | 해당 워크플로우 직접 사용 |

## Phase 실행 규약

- Orchestrator가 Orchestration Skill의 Phases 섹션을 해석
- 각 Phase의 `Skill()` 호출 실행
- Phase 간 데이터 전달: `expected_output` 파일을 통해
- 실패 처리: Orchestrator가 ARB 기반으로 판단
