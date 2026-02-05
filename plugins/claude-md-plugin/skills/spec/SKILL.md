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
│                    작성 + 자동 리뷰         │
└─────────────────────────────────────────────┘

        │
        ▼
┌─────────────────────────────────────────────┐
│ spec-agent AGENT                            │
│                                             │
│ Phase 1. 요구사항 분석                      │
│ Phase 2. 모호한 부분 AskUserQuestion        │
│ Phase 2.7. Task 정의 (상태 파일 저장)       │
│                                             │
│ Phase 2.5. 아키텍처 설계 분석               │
│   ├── Skill("tree-parse") → 프로젝트 구조   │
│   ├── Skill("dependency-graph") → 의존성    │
│   ├── 관련 모듈 CLAUDE.md Exports 파악      │
│   ├── 모듈 배치 결정 (신규 vs 확장)         │
│   ├── 인터페이스 설계 가이드라인            │
│   └── 경계 명확성 검증 (Exports 참조)       │
│                                             │
│ Phase 3. 대상 위치 결정 (Phase 2.5 결과)    │
│ Phase 4. 기존 CLAUDE.md 존재시 병합         │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │      ITERATION CYCLE (최대 3회)         │ │
│ │                                         │ │
│ │ Phase 5. CLAUDE.md 생성                 │ │
│ │ Phase 5.5. IMPLEMENTS.md Planning       │ │
│ │     │                                   │ │
│ │     ▼                                   │ │
│ │ Phase 5.7. Task(spec-reviewer) 자동리뷰 │ │
│ │     │                                   │ │
│ │     ▼                                   │ │
│ │ Phase 5.8. 판정                         │ │
│ │     ├── approve → Phase 6               │ │
│ │     └── feedback → Phase 5 (재생성)     │ │
│ └─────────────────────────────────────────┘ │
│                                             │
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
3. **Task 정의** - 요구사항을 구체적 Task로 분해 (상태 파일에 저장)
4. **아키텍처 설계 분석**
   - tree-parse, dependency-graph로 기존 코드베이스 분석
   - 모듈 배치 결정 (신규 생성 vs 기존 확장)
   - 인터페이스 설계 가이드라인 생성
   - 경계 명확성 검증 (Exports 참조)
5. 대상 경로 결정 (아키텍처 분석 결과 활용)
6. 기존 CLAUDE.md 존재시 smart merge
7. **리뷰-피드백 사이클 (최대 3회)**
   - 템플릿 기반 CLAUDE.md 생성
   - IMPLEMENTS.md Planning Section 생성
   - **spec-reviewer Agent로 자동 리뷰**
   - approve → 다음 단계 / feedback → 피드백 반영 후 재생성
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

리뷰 결과:
  - 반복 횟수: {review_iterations}회
  - 최종 점수: {final_review_score}
  - 상태: {review_status} (approve|warning)

검증 결과: 스키마 검증 통과

다음 단계:
  - /compile로 코드 구현 가능 (IMPLEMENTS.md Implementation Section도 업데이트됨)
  - /validate로 문서-코드 일치 검증 가능
```

### 리뷰-피드백 사이클

spec-reviewer Agent가 생성된 문서를 자동으로 검증합니다.

**검증 항목:**

| Check ID | 설명 | 필수 |
|----------|------|------|
| REQ-COVERAGE | 모든 요구사항이 문서에 반영 | Yes |
| TASK-COMPLETION | 모든 Task가 문서에 매핑 | Yes |
| SCHEMA-VALID | 스키마 준수 | Yes |
| EXPORT-MATCH | 요구사항 함수/타입이 Exports에 존재 | No |
| BEHAVIOR-MATCH | 요구사항 시나리오가 Behavior에 존재 | No |

**Approve 기준:**

| 조건 | 임계값 |
|------|--------|
| 총점 | >= 80 |
| REQ-COVERAGE | 100% |
| SCHEMA-VALID | passed |
| TASK-COMPLETION | >= 80% |

**반복 종료 조건:**
- approve 판정
- 최대 반복 횟수(3회) 도달
- 개선 진전 없음 (이전 점수 대비 5점 미만 상승)

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
