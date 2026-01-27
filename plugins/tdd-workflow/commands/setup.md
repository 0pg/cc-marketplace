---
description: TDD Workflow를 현재 프로젝트에 등록합니다.
---

# TDD Workflow Setup

## 수행 작업

1. `.claude/project-config.md` 파일 확인 (없으면 생성)
2. `workflows:` 섹션에 `tdd-workflow` 추가 (중복 방지)
3. 등록 완료 확인 메시지 출력

## 구현

1. project-config.md 읽기 (없으면 기본 템플릿 사용)
2. workflows 섹션 파싱
3. tdd-workflow가 없으면 추가
4. 파일 저장
5. 결과 출력

## project-config 수정 예시

Before:
```yaml
workflows:
  - default
```

After:
```yaml
workflows:
  - default
  - tdd-workflow
```

## 완료 메시지

```
tdd-workflow가 프로젝트에 등록되었습니다.

사용 방법:
- /orchestrator 실행 시 워크플로우 선택 가능
- 또는 "TDD로 구현해줘" 요청 시 자동 선택
- 스킬 직접 호출: Skill("tdd-dev:test-design")
```

## 주의사항

- project-config.md가 없으면 기본 템플릿으로 생성
- workflows 섹션이 없으면 추가
- tdd-workflow가 이미 있으면 중복 추가하지 않음
