---
name: context-update
description: |
  코드 변경 사항을 감지하고 해당 CLAUDE.md를 업데이트합니다.
  변경된 파일과 관련된 컨텍스트가 여전히 유효한지 검증하고 필요시 수정합니다.
trigger:
  - /context-update
  - 컨텍스트 동기화
  - CLAUDE.md 업데이트
  - 컨텍스트 갱신
tools:
  - Read
  - Glob
  - Grep
  - Task
  - Write
  - Bash
  - AskUserQuestion
---

# Context Update Skill

## 목적

코드 변경 후 CLAUDE.md가 코드와 일치하도록 업데이트합니다.

## 워크플로우

### 1. 변경 감지

```bash
# Git diff로 변경된 파일 확인
git diff --name-only HEAD~1
git diff --name-only --staged
git status --porcelain
```

변경 유형 분류:
- 새 파일 추가
- 기존 파일 수정
- 파일 삭제
- 파일 이동/이름 변경

### 2. 영향 분석

변경된 파일과 관련된 CLAUDE.md 식별:

```
변경된 파일: src/auth/token.rs

영향받는 CLAUDE.md:
  - src/auth/CLAUDE.md (직접 영향)
  - src/api/CLAUDE.md (token 참조 시)
```

### 3. Diff 생성

각 CLAUDE.md에 대해 필요한 변경사항 분석:

```markdown
=== src/auth/CLAUDE.md 업데이트 필요 ===

[Magic Numbers & Constants]
+ TOKEN_EXPIRY = 3600 (신규 추가)
  - 의미: 토큰 만료 시간 (초)
  - 유래: ???

[Design Decisions]
~ JWT 라이브러리 변경: jsonwebtoken → jose
  - 이유: ???
```

### 4. 사용자 확인

변경 전 사용자에게 확인:

```
다음 변경사항을 적용하시겠습니까?

src/auth/CLAUDE.md:
  - TOKEN_EXPIRY 상수 추가 (의미 확인 필요)
  - JWT 라이브러리 변경 이유 확인 필요

[질문] TOKEN_EXPIRY = 3600의 의미는?
[질문] jsonwebtoken에서 jose로 변경한 이유는?
```

### 5. 업데이트 적용

사용자 확인 후 CLAUDE.md 업데이트:

```python
Task(
    subagent_type="context-generator",
    prompt="대상: src/auth (token.rs 변경됨)\n기존 CLAUDE.md: [내용]\n변경 사항: [diff]"
)
```

## 변경 유형별 처리

| 변경 유형 | 처리 방식 |
|----------|----------|
| 상수 추가 | 의미/유래 질문 후 추가 |
| 상수 변경 | 변경 이유 질문 후 업데이트 |
| 상수 삭제 | CLAUDE.md에서 제거 |
| 로직 변경 | 관련 컨텍스트 재검토 |
| 파일 삭제 | 관련 항목 정리 |

## 자동 알림 연동

`hooks/hooks.json`의 PostToolUse 훅이 코드 변경을 감지하면 업데이트 필요 여부를 알립니다.
사용자가 `/context-update`로 명시적 업데이트를 트리거합니다.
