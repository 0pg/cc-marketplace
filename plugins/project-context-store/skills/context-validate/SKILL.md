---
name: context-validate
description: |
  CLAUDE.md의 품질을 종합 검증합니다.
  Drift 검증(코드-문서 일치)과 재현성 검증(도메인 이해도 시뮬레이션)을
  병렬로 수행하여 통합 보고서를 생성합니다.
trigger:
  - /context-validate
  - 컨텍스트 검증
  - CLAUDE.md 검증
  - 문서 검증
  - 재현성 검증
tools:
  - Read
  - Glob
  - Grep
  - Task
---

# Context Validate Skill

## 목적

CLAUDE.md의 품질을 종합 검증합니다.

1. **Drift 검증**: 코드와 문서가 일치하는가?
2. **재현성 검증**: 문서로 도메인을 파악하여 코드를 작성할 수 있는가?

## 핵심 개념: 확률적 재현가능성

검증 질문: "이 CLAUDE.md로 도메인을 파악하여 여러 번 시도 시 동일한 코드를 작성할 수 있는가?"

## 검증 방식 비교

| 구분 | Drift 검증 | 재현성 검증 |
|------|-----------|------------|
| Agent | drift-validator | reproducibility-validator |
| 목적 | 코드-문서 **불일치** 탐지 | **도메인 이해도** 검증 |
| 방법 | 문서 내용과 코드 직접 비교 | AI가 문서만 보고 도메인 이해 후 코드 구조 예측 |
| 질문 | "문서와 코드가 일치하는가?" | "문서로 도메인을 파악하여 코드를 작성할 수 있는가?" |
| 탐지 대상 | 오래된 값, 삭제된 항목, 새 항목 | 도메인 컨텍스트 누락, 비즈니스 규칙 설명 부족 |

## 워크플로우

### 1. CLAUDE.md 수집

```
1. Glob으로 대상 경로의 모든 CLAUDE.md 파일 탐지
2. 각 파일의 위치와 담당 디렉토리 매핑
```

### 2. 병렬 검증 실행

**모든 CLAUDE.md에 대해 모든 Task를 한번에 병렬 실행**:

```python
# 모든 Task를 단일 메시지에서 호출하여 전체 병렬 실행
tasks = []
for claude_md in target_claude_mds:
    tasks.append(Task(
        subagent_type="project-context-store:drift-validator",
        prompt=f"검증 대상: {claude_md}"
    ))
    tasks.append(Task(
        subagent_type="project-context-store:reproducibility-validator",
        prompt=f"검증 대상: {claude_md}"
    ))
# 모든 Task 동시 호출 (단일 메시지)
```

**중요**: 모든 Task 호출은 반드시 **단일 메시지**에서 수행하여 전체 병렬 실행

### 3. 결과 취합

두 에이전트의 결과를 수집하여 통합:

```
1. Drift 검증 결과 수집 (STALE, MISMATCH, MISSING, ORPHAN)
2. 재현성 검증 결과 수집 (도메인 이해도, 누락 항목)
3. 심각도별 분류 및 정렬
4. 권장 조치 통합
```

### 4. 통합 보고서 생성

```markdown
=== Context Validation Report ===

## 요약
| 항목 | 결과 |
|------|------|
| 검증된 CLAUDE.md | 5개 |
| Drift 문제 | 3개 |
| 도메인 이해도 문제 | 2개 |
| 전체 상태 | 개선 필요 |

---

## Part 1: Drift 검증 (코드-문서 일치)

drift-validator 에이전트의 결과

### src/auth/CLAUDE.md

#### [HIGH] MISMATCH
| 항목 | 문서 값 | 코드 값 | 위치 |
|------|---------|---------|------|
| TOKEN_EXPIRY | 3600 | 7200 | token.rs:15 |

#### [MEDIUM] MISSING
| 항목 | 코드 값 | 위치 |
|------|---------|------|
| MAX_REFRESH_COUNT | 5 | token.rs:18 |

---

## Part 2: 재현성 검증 (도메인 이해도)

reproducibility-validator 에이전트의 결과

### src/auth/CLAUDE.md

도메인 이해도: 85%

#### 예측 실패 항목

| 예측 실패 항목 | 실제 코드 | 원인 |
|---------------|----------|------|
| REFRESH_INTERVAL | 300 | 도메인 상수 설명 없음 |
| cleanup_expired() | session.rs:45 | 비즈니스 규칙 설명 없음 |

---

## 권장 조치

### 즉시 조치 필요 (High)
1. TOKEN_EXPIRY 값 업데이트 (3600 → 7200)

### 권장 (Medium)
2. MAX_REFRESH_COUNT 문서화 추가
3. REFRESH_INTERVAL의 도메인 의미 추가
4. cleanup_expired() 비즈니스 규칙 설명 추가

### 실행 명령
/context-update src/auth
```

## 성공 기준

| 상태 | 조건 |
|------|------|
| 양호 | Drift 문제 0개 AND 도메인 이해도 90% 이상 |
| 개선 권장 | Drift 문제 1-2개 OR 도메인 이해도 70-89% |
| 개선 필요 | Drift 문제 3개 이상 OR 도메인 이해도 70% 미만 |

## 사용 예시

### 전체 프로젝트 검증
```
/context-validate
```

### 특정 경로 검증
```
/context-validate src/auth
```

## Drift 유형 참조

| 유형 | 설명 | 심각도 |
|------|------|--------|
| STALE | 문서화된 항목이 코드에 없음 | High |
| MISMATCH | 값이 다름 | High |
| MISSING | 코드에 있지만 문서화되지 않음 | Medium |
| ORPHAN | 참조 파일 없음 | Low |
