# Refactor Templates

## Split 모드 알고리즘

1. CLAUDE.md의 Purpose를 분석하여 다중 책임 여부 확인
2. **Requirements 그루핑**: 관련 있는 Requirements를 그룹화
   - 같은 도메인 개념을 참조하는 Requirements를 그룹으로 묶음
   - 독립적인 Requirements 그룹 = 분할 후보
3. `analyze-code` CLI로 파일 그루핑:
   ```bash
   $CLI_PATH analyze-code --path {path}
   ```
   - 각 Requirements 그룹에 해당하는 소스 파일을 매핑
4. AskUserQuestion으로 분할 계획 확인:
   ```
   분할 제안:
   {path}/ → {path}/token/ + {path}/session/

   {path}/token/CLAUDE.md:
     Purpose: 토큰 관련 인증
     Requirements: [토큰 만료 최대 7일, ...]

   {path}/session/CLAUDE.md:
     Purpose: 세션 관리
     Requirements: [동시 세션 최대 5개, ...]

   계속 진행하시겠습니까?
   ```

## Merge 모드 알고리즘

1. 병합 대상 모듈을 AskUserQuestion으로 지정:
   ```
   AskUserQuestion: "병합할 모듈들의 경로를 입력하세요 (쉼표 구분)"
   ```
2. 각 모듈의 CLAUDE.md를 Read
3. Requirements 중복 확인
4. AskUserQuestion으로 병합 계획 확인

## 의존 모듈 업데이트 안내 템플릿

```
리팩토링 완료. 다음 모듈의 참조를 업데이트하세요:
- src/api/CLAUDE.md: src/auth → src/auth/token
- src/middleware/CLAUDE.md: src/auth → src/auth/token

이후:
  /compile --all --conflict overwrite  — 전체 재컴파일
  /validate  — 검증
```

## 코드 재생성 플로우

```
AskUserQuestion: "리팩토링된 문서로 코드를 재생성하시겠습니까?"
옵션: [예 (/compile --all --conflict overwrite), 아니오 (수동 처리)]
```
