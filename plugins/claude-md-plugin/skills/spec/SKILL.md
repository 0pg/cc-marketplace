---
name: spec
description: |
  This skill should be used when the user wants to define requirements and
  generate/update CLAUDE.md files. It analyzes requirements and creates
  CLAUDE.md documentation without implementing any code.

  Trigger keywords:
  - "/spec"
  - "요구사항 정의"
  - "스펙 작성"
  - "behavior 정의"
  - "Define specification"
  - "Create CLAUDE.md from requirements"
allowed-tools: [Read, Glob, Task, Skill, AskUserQuestion]
---

# Spec Skill

## 목적

요구사항(자연어 또는 User Story)을 분석하여 CLAUDE.md를 생성/업데이트합니다.
**코드 구현 없이** behavior 정의만 수행하여 ATDD/TDD의 "명세 먼저" 원칙을 따릅니다.

## 아키텍처

```
User: /spec "요구사항"
        │
        ▼
┌─────────────────────────────────────────────┐
│ spec SKILL (Entry Point)                    │
│                                             │
│ 1. Task(spec-clarifier) → 스펙 명확화       │
│ 2. Task(spec-writer) → CLAUDE.md 작성       │
└─────────────────────────────────────────────┘

        │
        ▼
┌─────────────────────────────────────────────┐
│ spec-clarifier AGENT                        │
│                                             │
│ - 요구사항 분석                             │
│ - 모호한 부분 AskUserQuestion              │
│ - 구조화된 스펙 도출                        │
│ - 대상 CLAUDE.md 위치 결정                  │
│ → .claude/spec-results/clarified.json       │
└─────────────────────────────────────────────┘

        │
        ▼
┌─────────────────────────────────────────────┐
│ spec-writer AGENT                           │
│                                             │
│ - 기존 CLAUDE.md 존재 여부 확인             │
│ - 존재: Skill("claude-md-parse") + 병합     │
│ - 신규: 템플릿 기반 생성                    │
│ - Skill("schema-validate") → 검증           │
│ → 최종 CLAUDE.md 저장                       │
└─────────────────────────────────────────────┘
```

## 워크플로우

### 1. 결과 디렉토리 준비

```bash
mkdir -p .claude/spec-results
```

### 2. 요구사항 수신

사용자로부터 요구사항을 수신합니다:
- 자연어 설명
- User Story (As a..., I want..., So that...)
- Feature 목록
- 기능 요청

### 3. 스펙 명확화 (spec-clarifier Agent)

```python
# spec-clarifier Agent 호출
Task(
    subagent_type="claude-md-plugin:spec-clarifier",
    prompt=f"""
사용자 요구사항:
{user_requirement}

프로젝트 루트: {project_root}

요구사항을 분석하고 CLAUDE.md 스펙을 명확화해주세요.
결과 파일: .claude/spec-results/clarified.json
""",
    description="Clarify specification"
)
```

**spec-clarifier 출력** (`.claude/spec-results/clarified.json`):
```json
{
  "clarified_spec": {
    "purpose": "인증 토큰 검증 및 발급",
    "exports": [
      { "name": "validateToken", "signature": "validateToken(token: string): Promise<Claims>" },
      { "name": "Claims", "kind": "type", "definition": "{ userId: string, role: Role }" }
    ],
    "behaviors": [
      { "input": "valid JWT token", "output": "Claims object" },
      { "input": "expired token", "output": "TokenExpiredError" }
    ],
    "contracts": [
      { "function": "validateToken", "preconditions": ["token is non-empty"], "postconditions": ["returns valid Claims"] }
    ],
    "protocol": null
  },
  "target_path": "src/auth",
  "action": "create"
}
```

### 4. 스펙 명확화 결과 확인

```python
# clarified.json 읽기
clarified = read_json(".claude/spec-results/clarified.json")

# 사용자에게 결과 표시
print(f"""
=== 스펙 명확화 완료 ===

대상 위치: {clarified["target_path"]}
액션: {clarified["action"]}

Purpose: {clarified["clarified_spec"]["purpose"]}

Exports:
{format_exports(clarified["clarified_spec"]["exports"])}

Behaviors:
{format_behaviors(clarified["clarified_spec"]["behaviors"])}

계속하시겠습니까?
""")
```

### 5. CLAUDE.md 작성 (spec-writer Agent)

```python
# spec-writer Agent 호출
Task(
    subagent_type="claude-md-plugin:spec-writer",
    prompt=f"""
명확화된 스펙: .claude/spec-results/clarified.json
대상 경로: {clarified["target_path"]}
액션: {clarified["action"]}

CLAUDE.md를 생성/업데이트해주세요.
""",
    description="Write CLAUDE.md"
)
```

### 6. 최종 결과 보고

```
=== /spec 완료 ===

생성/업데이트된 파일:
  ✓ {target_path}/CLAUDE.md

스펙 요약:
  - Purpose: {purpose}
  - Exports: {export_count}개
  - Behaviors: {behavior_count}개
  - Contracts: {contract_count}개

검증 결과: 스키마 검증 통과

다음 단계:
  - /generate로 코드 구현 가능
  - /validate로 문서-코드 일치 검증 가능
```

## 결과 파일

| 파일 | 생성자 | 설명 |
|------|--------|------|
| `.claude/spec-results/clarified.json` | spec-clarifier | 명확화된 스펙 |
| `.claude/spec-results/validation.json` | spec-writer | 스키마 검증 결과 |
| `{target_path}/CLAUDE.md` | spec-writer | 최종 CLAUDE.md |

## 오류 처리

| 상황 | 대응 |
|------|------|
| 요구사항 불명확 | spec-clarifier가 AskUserQuestion으로 명확화 |
| 대상 경로 모호 | spec-clarifier가 후보 목록 제시 후 선택 요청 |
| 기존 CLAUDE.md와 충돌 | spec-writer가 병합 전략 제안 |
| 스키마 검증 실패 | spec-writer가 최대 5회 재시도 |

## /init과의 차이점

| 측면 | /init | /spec |
|------|-------|-------|
| 입력 | 기존 소스 코드 | 사용자 요구사항 |
| 방향 | Code → CLAUDE.md | Requirements → CLAUDE.md |
| 목적 | 기존 코드 문서화 | 새 기능 명세 정의 |
| 사용 시점 | 레거시 코드 정리 | 신규 개발 시작 전 |

## 패러다임

```
전통적 개발:        요구사항 → 코드 → (문서)
ATDD with /spec:    요구사항 → CLAUDE.md → /generate → 코드
                              ↑
                          Source of Truth
```

`/spec`은 ATDD의 "Acceptance Criteria 먼저" 원칙을 CLAUDE.md 기반으로 구현합니다.
