---
name: spec
version: 1.0.0
aliases: [define, requirements, spec-out]
trigger:
  - /spec
  - 요구사항 정의
  - write specification
description: |
  This skill should be used when the user asks to "define requirements", "write spec",
  "create CLAUDE.md from requirements", "define behavior before coding", or uses "/spec".
  Analyzes natural language requirements and generates CLAUDE.md without implementing code.
  Follows ATDD principle: specification first, then code generation via /compile.
allowed-tools: [Read, Glob, Write, Task, Skill, AskUserQuestion]
---

# Spec Skill

## 목적

요구사항(자연어 또는 User Story)을 분석하여 **CLAUDE.md + IMPLEMENTS.md** 쌍을 생성/업데이트합니다.
**코드 구현 없이** behavior 정의만 수행하여 ATDD/TDD의 "명세 먼저" 원칙을 따릅니다.

## 듀얼 문서 시스템

```
/spec "요구사항"
    │
    ├─→ CLAUDE.md 생성/업데이트 (WHAT)
    │   - Purpose, Domain Context, Exports, Behavior, Contract, Protocol
    │
    └─→ IMPLEMENTS.md [Planning Section] 업데이트 (HOW 계획)
        - Architecture Decisions (모듈 배치, 인터페이스 설계, 의존성 방향)
        - Dependencies Direction
        - Implementation Approach
        - Technology Choices
```

## 아키텍처

```
User: /spec "요구사항"
        │
        ▼
┌─────────────────────────────────────────────┐
│ spec SKILL (Entry Point)                    │
│                                             │
│ Task(spec-agent) → 요구사항 분석 및         │
│                    CLAUDE.md + IMPLEMENTS.md│
│                    작성                     │
└─────────────────────────────────────────────┘

        │
        ▼
┌─────────────────────────────────────────────┐
│ spec-agent AGENT                            │
│                                             │
│ Phase 1. 요구사항 분석                      │
│ Phase 2. 모호한 부분 AskUserQuestion        │
│                                             │
│ [NEW] Phase 2.5. 아키텍처 설계 분석         │
│   ├── Skill("tree-parse") → 프로젝트 구조   │
│   ├── Skill("dependency-graph") → 의존성    │
│   ├── 관련 모듈 CLAUDE.md Exports 파악      │
│   ├── 모듈 배치 결정 (신규 vs 확장)         │
│   ├── 인터페이스 설계 가이드라인            │
│   └── 경계 명확성 검증 (Exports 참조)       │
│                                             │
│ Phase 3. 대상 위치 결정 (Phase 2.5 결과)    │
│ Phase 4. 기존 CLAUDE.md 존재시 병합         │
│ Phase 5. CLAUDE.md 생성                     │
│ Phase 5.5. IMPLEMENTS.md Planning Section   │
│   └── Architecture Decisions 섹션 포함      │
│ Phase 6. Skill("schema-validate") → 검증    │
│ Phase 7. 최종 저장                          │
└─────────────────────────────────────────────┘
```

## 워크플로우

### 1. 요구사항 수신

사용자로부터 요구사항을 수신합니다:
- 자연어 설명
- User Story (As a..., I want..., So that...)
- Feature 목록
- 기능 요청

### 2. CLAUDE.md + IMPLEMENTS.md 생성 (spec-agent)

```python
# spec-agent Agent 호출
Task(
    subagent_type="claude-md-plugin:spec-agent",
    prompt=f"""
사용자 요구사항:
{user_requirement}

프로젝트 루트: {project_root}

요구사항을 분석하고 CLAUDE.md와 IMPLEMENTS.md를 생성해주세요.
""",
    description="Generate CLAUDE.md + IMPLEMENTS.md from requirements"
)
```

**spec-agent 워크플로우:**
1. 요구사항에서 Purpose, Exports, Behaviors, Contracts 추출
2. 모호한 부분은 AskUserQuestion으로 명확화
3. **[NEW] 아키텍처 설계 분석**
   - tree-parse, dependency-graph로 기존 코드베이스 분석
   - 모듈 배치 결정 (신규 생성 vs 기존 확장)
   - 인터페이스 설계 가이드라인 생성
   - 경계 명확성 검증 (Exports 참조)
4. 대상 경로 결정 (아키텍처 분석 결과 활용)
5. 기존 CLAUDE.md 존재시 smart merge
6. 템플릿 기반 CLAUDE.md 생성
7. **IMPLEMENTS.md Planning Section 생성**
   - Architecture Decisions: 모듈 배치, 인터페이스, 의존성 방향
   - Dependencies Direction: 필요한 의존성과 위치
   - Implementation Approach: 구현 전략과 대안
   - Technology Choices: 기술 선택 근거
8. 스키마 검증 (1회)
9. 최종 저장

### 3. 최종 결과 보고

```
=== /spec 완료 ===

생성/업데이트된 파일:
  ✓ {target_path}/CLAUDE.md (WHAT - 스펙)
  ✓ {target_path}/IMPLEMENTS.md (HOW - Planning Section)

아키텍처 결정:
  - Module Placement: {module_placement} ({create|extend})
  - 경계 명확성 준수: ✓
  - 의존성 그래프: .claude/dependency-graph.json

스펙 요약:
  - Purpose: {purpose}
  - Exports: {export_count}개
  - Behaviors: {behavior_count}개
  - Contracts: {contract_count}개

구현 계획 요약:
  - Architecture Decisions: 모듈 배치, 인터페이스 설계, 의존성 방향
  - Dependencies: {dependency_count}개
  - Implementation Approach: {approach_summary}
  - Technology Choices: {choice_count}개

검증 결과: 스키마 검증 통과

다음 단계:
  - /compile로 코드 구현 가능 (IMPLEMENTS.md Implementation Section도 업데이트됨)
  - /validate로 문서-코드 일치 검증 가능
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 요구사항 불명확 | spec-agent가 AskUserQuestion으로 명확화 |
| 대상 경로 모호 | 후보 목록 제시 후 선택 요청 |
| 기존 CLAUDE.md와 충돌 | 병합 전략 제안 |
| 기존 IMPLEMENTS.md와 충돌 | Planning Section만 업데이트 (Implementation Section 유지) |
| 스키마 검증 실패 | 경고와 함께 이슈 보고 |

## /decompile과의 차이점

| 측면 | /decompile | /spec |
|------|------------|-------|
| 입력 | 기존 소스 코드 | 사용자 요구사항 |
| 방향 | Code → CLAUDE.md | Requirements → CLAUDE.md |
| 목적 | 기존 코드 문서화 | 새 기능 명세 정의 |
| 사용 시점 | 레거시 코드 정리 | 신규 개발 시작 전 |

## 패러다임

```
전통적 개발:        요구사항 → 코드 → (문서)
ATDD with /spec:    요구사항 → CLAUDE.md → /compile → 코드
                              ↑
                          Source of Truth
```

`/spec`은 ATDD의 "Acceptance Criteria 먼저" 원칙을 CLAUDE.md 기반으로 구현합니다.
