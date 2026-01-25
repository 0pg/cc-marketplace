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
  - Grep
  - Task
  - Write
  - AskUserQuestion
---

# Context Generate Skill

## 목적

프로젝트의 모든 소스 코드 디렉토리에 CLAUDE.md를 생성하여 코드 재현성을 보장합니다.

## 워크플로우

### 1. 소스 코드 디렉토리 탐지

```
1. Glob으로 소스 파일 탐지
   - 확장자: .rs, .py, .ts, .js, .tsx, .jsx, .go, .java, .cpp, .c, .h

2. 제외 디렉토리:
   - node_modules, target, dist, build, vendor, .git
   - __pycache__, .venv, venv, .tox
   - coverage, .next, .nuxt

3. CLAUDE.md 생성 대상 결정:
   - 소스 코드가 있는 모든 디렉토리에 각각 CLAUDE.md 생성
   - 각 CLAUDE.md는 해당 디렉토리의 파일만 담당
```

### 2. 디렉토리별 Task 생성

각 대상 디렉토리에 대해 독립된 Task를 생성합니다.

```python
# 병렬 처리를 위해 여러 Task 동시 생성
for directory in target_directories:
    Task(
        subagent_type="context-generator",
        prompt=f"대상: {directory} ({file_list})"
    )
```

**병렬 처리의 장점:**
- 여러 디렉토리 동시 작업
- 컨텍스트 격리 (디렉토리 간 간섭 방지)
- 개별 실패 격리 (하나 실패해도 다른 작업 계속)

### 3. 결과 수집 및 보고

```
=== Context Generation Report ===

성공: 5개 CLAUDE.md 생성
  - src/auth/CLAUDE.md
  - src/api/CLAUDE.md
  - src/utils/CLAUDE.md
  - src/models/CLAUDE.md
  - src/services/CLAUDE.md

실패: 1개
  - src/legacy: [사유]

사용자 질문: 12개 응답됨
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
