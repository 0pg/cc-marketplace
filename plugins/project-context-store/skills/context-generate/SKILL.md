---
name: context-generate
description: |
  context-generator 에이전트를 시작하여 프로젝트에 CLAUDE.md를 생성합니다.
  Skill은 시작점에서 단일 Task만 시작하고, 재귀 탐색은 Agent가 담당합니다.
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

## 참고: Agent 동작

다음은 context-generator Agent가 담당합니다 (Skill이 하는 일 아님):
- 재귀적 하위 디렉토리 탐색 및 Task 생성
- 코드 분석 및 컨텍스트 추출
- 사용자 질문 (불명확한 부분)
- 기존 CLAUDE.md 처리 (병합/덮어쓰기)

상세 내용은 `agents/context-generator.md` 참조.
