---
name: refactor
version: 2.0.0
aliases: [split, merge, restructure]
description: |
  This skill should be used when the user asks to "refactor module", "split module",
  "merge modules", "restructure", or uses "/refactor".
  Performs document-level refactoring: split one CLAUDE.md into multiple, or merge multiple into one.
  Uses Requirements grouping for split decisions and /impact for affected module analysis.
  Trigger keywords: 리팩토링, 모듈 분할, 모듈 병합, 구조 변경
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Skill, AskUserQuestion]
---

# /refactor

CLAUDE.md 수준의 모듈 분할/병합 리팩토링을 수행합니다.

코드가 아닌 **문서를 먼저 리팩토링**하고, `/compile`로 코드를 재생성합니다.

## Triggers

- `/refactor`
- `모듈 분할`, `모듈 병합`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 예 | - | 리팩토링 대상 모듈 경로 |
| `--mode` | 아니오 | (사용자 선택) | `split` \| `merge` |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 대상 분석

대상 CLAUDE.md를 Read하고 현재 구조를 파악합니다:

```bash
$CLI_PATH parse-claude-md --file {path}/CLAUDE.md > "${TMP_DIR}refactor-current.json"
```

### 2. 리팩토링 모드 결정

`--mode`가 지정되지 않으면 AskUserQuestion으로 질문:

```
AskUserQuestion: "어떤 리팩토링을 수행하시겠습니까?"
옵션:
A) split — 하나의 모듈을 여러 모듈로 분할
B) merge — 여러 모듈을 하나로 병합
```

### 3. 리팩토링 계획 수립

#### Split 모드

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

#### Merge 모드

1. 병합 대상 모듈을 AskUserQuestion으로 지정:
   ```
   AskUserQuestion: "병합할 모듈들의 경로를 입력하세요 (쉼표 구분)"
   ```
2. 각 모듈의 CLAUDE.md를 Read
3. Requirements 중복 확인
4. AskUserQuestion으로 병합 계획 확인

### 4. 영향 분석

리팩토링 전 영향 분석을 실행합니다:

```
Skill("claude-md-plugin:impact", args: "{path}")
```

HIGH 영향이 있으면 경고 후 계속 여부 확인.

### 5. 리팩토링 실행

#### Split

1. 새 디렉토리 생성
2. 원본 CLAUDE.md에서 해당 Requirements/Domain Context를 추출하여 새 CLAUDE.md 생성
3. 원본 CLAUDE.md에서 이동된 항목 제거, Purpose 업데이트
4. DEVELOPERS.md도 분할

#### Merge

1. 병합 대상 CLAUDE.md들을 Read
2. Purpose 통합, Requirements 중복 제거, Domain Context 병합하여 새 CLAUDE.md 생성
3. DEVELOPERS.md 통합
4. 병합된 원본 파일 삭제 여부 AskUserQuestion으로 확인

### 6. 스키마 검증

리팩토링된 CLAUDE.md를 검증합니다:

```bash
$CLI_PATH validate-schema --file {new_path}/CLAUDE.md --strict
```

검증 실패 시 `fix-schema` 자동 실행.

### 7. 의존 모듈 업데이트 안내

영향받는 모듈의 참조를 업데이트하도록 안내합니다:

```
리팩토링 완료. 다음 모듈의 참조를 업데이트하세요:
- src/api/CLAUDE.md: src/auth → src/auth/token
- src/middleware/CLAUDE.md: src/auth → src/auth/token

이후:
  /compile --all --conflict overwrite  — 전체 재컴파일
  /validate  — 검증
```

### 8. 코드 재생성 (선택)

```
AskUserQuestion: "리팩토링된 문서로 코드를 재생성하시겠습니까?"
옵션: [예 (/compile --all --conflict overwrite), 아니오 (수동 처리)]
```

## DO / DON'T

**DO:**
- 문서(CLAUDE.md)를 먼저 리팩토링, 코드는 /compile로 재생성
- /impact으로 영향 분석 후 진행
- Requirements 그루핑으로 분할 기준 결정
- 사용자 승인 후 실행
- 스키마 검증 후 완료

**DON'T:**
- 소스코드 직접 리팩토링 (문서 → 코드 순서)
- 사용자 승인 없이 파일 삭제/이동
- 영향 분석 없이 진행

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| parse-claude-md 실패 | CLI 오류 표시 |
| 스키마 검증 실패 | fix-schema 시도, 실패 시 경고 |
| /impact 실패 | 경고 후 계속 여부 확인 |

## Examples

<example>
<user_request>/refactor src/auth --mode split</user_request>
<assistant_response>
src/auth/CLAUDE.md를 분석합니다...

Purpose: JWT 기반 인증 및 세션 관리
Requirements: 4개

분할 제안:
  src/auth/token/CLAUDE.md:
    Purpose: JWT 토큰 인증
    Requirements: [토큰 만료 최대 7일, UTF-8 인코딩 필수]

  src/auth/session/CLAUDE.md:
    Purpose: 세션 관리
    Requirements: [동시 세션 최대 5개, 비활성 세션 30분 만료]

영향 분석:
  src/api — src/auth 참조 → 경로 변경 필요

계속 진행하시겠습니까? [예/아니오]
→ 예

리팩토링 완료:
  Created: src/auth/token/CLAUDE.md + DEVELOPERS.md
  Created: src/auth/session/CLAUDE.md + DEVELOPERS.md
  Updated: src/auth/CLAUDE.md

다음 단계:
  1. src/api/CLAUDE.md 참조 경로 업데이트
  2. /compile --all --conflict overwrite
  3. /validate
</assistant_response>
</example>
