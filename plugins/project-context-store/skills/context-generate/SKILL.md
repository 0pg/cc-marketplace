---
name: context-generate
description: |
  프로젝트의 소스 코드 디렉토리를 분석하여 각 디렉토리에 CLAUDE.md 컨텍스트 파일을 생성합니다.
  코드 재현에 필요한 모든 컨텍스트(magic numbers, 도메인 규칙, 설계 결정 등)를 문서화합니다.
trigger:
  - /context-generate
  - 컨텍스트 생성해줘
  - CLAUDE.md 생성
  - 프로젝트 문서화
tools:
  - Read
  - Glob
  - Task
  - AskUserQuestion
---

# Context Generate Skill

## 목적

프로젝트의 모든 소스 코드 디렉토리에 CLAUDE.md를 생성하여 코드 재현성을 보장합니다.

## 워크플로우

### 1. 시작점 결정

```
1. 사용자가 특정 경로를 지정했다면 해당 경로 사용
2. 지정하지 않았다면 프로젝트 루트에서 시작
3. 시작점에서 context-generator 에이전트 단일 Task 생성
```

### 2. 에이전트 시작

루트 디렉토리에서 단일 Task를 시작합니다. 에이전트가 재귀적으로 하위 디렉토리를 탐색합니다.

**최종 결과**: 소스 코드가 있는 **모든** 하위 디렉터리에 각각 CLAUDE.md가 생성됩니다.
예시:
- src/CLAUDE.md
- src/auth/CLAUDE.md
- src/auth/jwt/CLAUDE.md
- src/api/CLAUDE.md
- lib/CLAUDE.md

각 CLAUDE.md는 해당 디렉터리의 직접 파일만 담당합니다.

```python
Task(
    subagent_type="project-context-store:context-generator",
    prompt=f"대상: {start_directory}"
)
```

**재귀 패턴의 장점:**
- Skill이 전체 디렉토리 구조를 미리 파악할 필요 없음
- Agent가 트리 구조를 자연스럽게 탐색
- 각 Agent가 자신의 컨텍스트에만 집중
- 실패 격리 (한 브랜치 실패해도 다른 브랜치 계속)
- Agent가 하위 디렉토리 Task를 병렬로 생성 (Skill은 단일 Task만 시작)

### 3. 결과 보고

에이전트가 완료되면 결과를 요약합니다:

```
=== Context Generation Report ===

생성된 CLAUDE.md 파일들:
  - src/CLAUDE.md
  - src/auth/CLAUDE.md
  - src/api/CLAUDE.md
  - lib/CLAUDE.md

사용자 질문: N개 응답됨
```

## 사용자 상호작용

context-generator 에이전트가 불명확한 부분을 발견하면 AskUserQuestion으로 질문합니다.

**질문 예시:**
- Magic number의 의미/유래
- 특이한 조건분기의 비즈니스 이유
- 도메인 규칙의 근거 (법규, 정책, 요구사항 등)
- 설계 결정의 배경과 대안 검토 여부
- 외부 시스템 연동의 구체적 스펙

**원칙: 추측하지 않고 질문합니다.**

## 기존 CLAUDE.md 처리

이미 CLAUDE.md가 존재하는 경우:

1. 기존 내용 읽기
2. 새로운 분석과 비교
3. 사용자에게 병합 방식 질문:
   - 덮어쓰기 (기존 내용 삭제)
   - 병합 (새 내용 추가)
   - 건너뛰기 (기존 유지)
