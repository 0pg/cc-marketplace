# Verification Protocol

> 검증 단계 인터페이스 정의

## Verification Stage 스키마

```yaml
stage:
  id: string                    # 고유 식별자
  name: string                  # 표시 이름
  description: string

  # 실행 조건
  condition:
    requires: [stage_id, ...]   # 선행 단계 (의존성)
    when: string                # 조건식 (예: "spec_exists AND task_exists")

  # 실행 방식
  execution:
    type: command | agent | custom
    command: string             # type: command
    agent: string               # type: agent ("plugin:agent" 형식)

  # 결과 처리
  result:
    pass_criteria: string
    on_fail:
      action: block | retry | delegate_fix | log_and_proceed
      target_agent: string
      max_retries: number
    severity: critical | high | medium | low
```

## Verification Chain 스키마

```yaml
verification_chain:
  id: string
  name: string

  # 기본 체인 상속 (선택)
  extends: chain_id

  # 단계 목록
  stages:
    - stage_ref: string         # 기존 단계 참조
    - id: string                # 또는 인라인 정의
      name: string
      ...

  # 확장 방식 (extends 사용 시)
  modifications:
    insert_before:
      target: stage_id
      stages: [...]
    insert_after:
      target: stage_id
      stages: [...]
    replace:
      target: stage_id
      with: stage_definition
    remove: [stage_id, ...]
```

## Workflow Adapter에서 Verification 제공

Orchestration Skill의 references/ 디렉토리에 verification-chain.md 포함:

```
skills/{workflow}-orchestration/
├── SKILL.md
└── references/
    └── verification-chain.md   # 검증 체인 정의
```

SKILL.md의 Verification 섹션에서 참조:

```markdown
## Verification

검증 체인: `references/verification-chain.md` 참조
```

## Stage 실행 규약

### Execution Types

| type | 실행 방식 |
|------|----------|
| `command` | Bash 도구로 명령어 실행 |
| `agent` | Task 도구로 에이전트 위임 |
| `custom` | 모델이 protocol 해석 |

### Result Actions

| action | 동작 |
|--------|------|
| `block` | 체인 중단, 보고 |
| `retry` | max_retries까지 재시도 |
| `delegate_fix` | target_agent에 수정 요청 후 재시도 |
| `log_and_proceed` | 이슈 기록, 다음 단계 진행 |

## 프로젝트별 Verification 커스터마이징

project-config에서:

```yaml
verification_chain:
  extends: default
  modifications:
    insert_after:
      target: tests
      stages:
        - id: security-scan
          execution:
            type: command
            command: "cargo audit"
```
