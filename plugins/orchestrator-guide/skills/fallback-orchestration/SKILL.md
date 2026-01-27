---
name: fallback-orchestration
description: |
  Fallback 워크플로우. spec.md + task.md 기반 에이전트 위임 워크플로우.
  다른 워크플로우가 없을 때 기본으로 사용됩니다.
---

# Fallback Workflow

> spec.md + task.md 기반 에이전트 위임 워크플로우

이 문서는 `fallback` 워크플로우가 선택된 경우의 실행 단계를 정의합니다.

---

## Step 0: Spec 정의 (plan mode)

spec/spec.md가 없거나 불명확한 경우:

1. **EnterPlanMode 호출**하여 plan mode 진입
2. 코드베이스 탐색 (Explore agent 활용)
3. AskUserQuestion으로 요구사항 명확화
4. spec.md 작성 (plan 파일 형식)
5. **ExitPlanMode로 사용자 승인** 요청
6. spec/spec.md로 저장

**장점:**
- plan mode의 탐색 단계에서 코드베이스 파악
- 설계 단계에서 요구사항 구체화
- 승인 워크플로우로 사용자 확인

**상세 가이드**: `spec-format.md` 참조

## Step 0.5: Task 정의 (plan mode)

task.md가 없는 경우:

1. **EnterPlanMode 호출**하여 plan mode 진입
2. spec.md 기반 구현 계획 수립
3. TASK-VERIFY 1:1 매핑으로 task.md 작성
4. **ExitPlanMode로 사용자 승인** 요청
5. spec/task.md로 저장

**필수 준수:**
- 모든 TASK에는 대응하는 VERIFY 존재 (1:1)
- VERIFY는 동작 기반으로 작성

**상세 가이드**: `task-format.md` 참조

## Step 1: 구현 시작 (모드 선택)

task.md 기반 구현 시작 시 모드를 선택:

| 상황 | 권장 모드 |
|------|----------|
| task가 명확하고 단순 | 일반 모드 |
| 구현 접근법이 불확실 | plan mode |
| 여러 파일/모듈 수정 | plan mode |
| 단일 파일 수정 | 일반 모드 |

**plan mode로 구현 시:**
- plan mode 시작 시 task.md 내용을 plan 파일에 포함
- TASK-XXX를 구현 대상으로 참조
- plan 완료 후 task.md 상태 업데이트 ([~] → [x])

**일반 모드로 구현 시:**
- task.md를 직접 참조하여 구현
- 에이전트 위임 시 5요소 프로토콜 사용

## Step 2: 컨텍스트 파악
```
1. 대상 모듈 확인
2. spec/task.md 읽기
3. notes.md 존재 시 읽기
4. 불명확하면 AskUserQuestion으로 질문
```

## Step 3: 작업 분해 및 크기 평가
```
task.md를 참조하여:
- 작업을 세부 항목으로 분해
- 각 항목의 크기 평가 (분할 필요 여부)
- 적절한 역할 할당
- 의존성 명시 (병렬 가능 여부 판단)
```

**의존성 표기법:**
| 표기 | 의미 |
|------|------|
| `[depends: X]` | X 완료 후 시작 (순차) |
| `[depends: X, Y]` | X와 Y 모두 완료 후 시작 |
| `[parallel: X]` | X와 병렬 실행 가능 |
| `[blocks: X]` | 이 작업이 X를 블로킹 |

## Step 4: 에이전트 위임
```
Task 도구 사용:
- subagent_type: 역할에 맞는 에이전트 선택
- model: 작업 복잡도에 따른 모델 선택
- run_in_background: 병렬 실행 시 true
- prompt: 5요소 위임 프로토콜 사용
```

## Step 5: 검증 계획 수립
```
검증 시작 전 Verification Plan 명시:
- target_modules: 검증 대상 모듈 목록
- 나머지 세부사항은 모델이 상황에 맞게 판단
```

## Step 6: 결과 수집 및 검증
```
1. ARB(Agent Result Block) 수집
2. Verification Plan에 따라 검증 체인 실행
3. 검증 결과에 따라 후속 작업 결정
```
