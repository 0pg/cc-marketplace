---
description: Workflow adapter 플러그인 생성 도우미. 새 워크플로우를 scaffolding합니다.
---

# Workflow Adapter Helper

> 새 workflow adapter 플러그인을 간편하게 생성

## 워크플로우 (3단계)

### Phase 1: Discovery

사용자에게 워크플로우 정보 수집:

1. **기본 정보**
   - 워크플로우 이름 (kebab-case)
   - 설명 (1-2문장)
   - 트리거 키워드들

2. **Phase 정의** (1개 이상)
   각 phase에 대해:
   - Phase 이름
   - 호출할 skill (plugin:skill 형식)
   - 예상 산출물
   - 완료 조건

3. **옵션**
   - 의존 플러그인 (code-convention 등)
   - 검증 체인 커스터마이징 여부

### Phase 2: Scaffolding

templates/ 기반 파일 생성:

1. 디렉토리 구조 생성
   ```
   {workflow-name}/
   ├── .claude-plugin/plugin.json
   ├── commands/setup.md
   ├── skills/{workflow}-orchestration/
   │   ├── SKILL.md
   │   └── references/verification-chain.md
   └── README.md
   ```

2. 템플릿 변수 치환하여 파일 작성

### Phase 3: Verification & Next Steps

생성 결과 확인 및 안내:
- 생성된 파일 목록
- 테스트 방법
- setup 커맨드 실행 안내

## AskUserQuestion 흐름

### Step 1: 기본 정보
```
question: "워크플로우 기본 정보를 입력해주세요"
fields:
  - 워크플로우 이름 (kebab-case)
  - 설명 (1-2문장)
  - 트리거 키워드 (쉼표 구분)
```

### Step 2: Phase 정의 (반복)
```
question: "Phase를 정의해주세요"
fields:
  - Phase 이름
  - 호출할 Skill (plugin:skill)
  - 예상 산출물
  - 완료 조건
options:
  - "Phase 추가"
  - "완료"
```

### Step 3: 옵션 (선택)
```
question: "추가 옵션을 선택해주세요"
options:
  - 커스텀 검증 체인 정의
  - 의존 플러그인 추가
  - 기본값으로 진행
```

## 생성 위치

기본: `plugins/{workflow-name}/`
사용자가 다른 경로 지정 가능

## 템플릿

각 템플릿 파일은 `templates/` 디렉토리에 위치.
변수: `{{workflow_name}}`, `{{description}}`, `{{triggers}}`, `{{phases}}` 등
