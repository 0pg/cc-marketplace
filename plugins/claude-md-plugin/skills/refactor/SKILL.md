---
name: refactor
version: 1.0.0
aliases: [split, merge, restructure]
description: |
  This skill should be used when the user asks to "refactor module", "split module",
  "merge modules", "restructure", or uses "/refactor".
  Performs contract-level refactoring: split one CLAUDE.md into multiple, or merge multiple into one.
  Uses /impact to analyze affected modules before applying changes.
  Trigger keywords: 리팩토링, 모듈 분할, 모듈 병합, 구조 변경
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task, Skill, AskUserQuestion]
---

> **DEPRECATED (v6.0.0)**: This skill depends on CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in a future version.

# /refactor

계약(CLAUDE.md) 수준의 모듈 분할/병합 리팩토링을 수행합니다.

코드가 아닌 **계약을 먼저 리팩토링**하고, `/compile`로 코드를 재생성합니다.

## Triggers

- `/refactor`
- `모듈 분할`, `모듈 병합`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 예 | - | 리팩토링 대상 모듈 경로 |
| `--mode` | 아니오 | (사용자 선택) | `split` \| `merge` |

## Workflow

### 1. 대상 분석

대상 CLAUDE.md를 Read하고 현재 구조를 파악합니다:

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

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

1. 대상 CLAUDE.md의 Exports를 분석하여 분할 후보를 제안합니다:
   - 응집도 기준: 관련 있는 Exports를 그룹화
   - Dependencies 기준: 의존 관계가 적은 그룹으로 분리
2. AskUserQuestion으로 분할 계획 확인:
   ```
   분할 계획:
   src/auth/ → src/auth/token/ + src/auth/session/

   src/auth/token/CLAUDE.md:
     - validateToken, revokeToken, Claims

   src/auth/session/CLAUDE.md:
     - createSession, destroySession, SessionConfig

   계속 진행하시겠습니까?
   ```

#### Merge 모드

1. 병합 대상 모듈을 지정하도록 요청합니다.
2. 각 모듈의 CLAUDE.md를 Read하여 Exports 충돌을 확인합니다.
3. AskUserQuestion으로 병합 계획 확인.

### 4. 영향 분석 (자동)

리팩토링 전 영향 분석을 자동 실행합니다:

```
Skill("claude-md-plugin:impact", args: "{path}")
```

BREAKING 영향이 있으면 경고 후 계속 여부 확인.

### 5. 계약 리팩토링 실행

#### Split

1. 새 디렉토리 생성
2. 원본 CLAUDE.md에서 해당 Exports/Behaviors/Contracts를 추출하여 새 CLAUDE.md 생성
3. 원본 CLAUDE.md에서 이동된 항목 제거
4. Dependencies 업데이트 (새 모듈 간 참조 추가)
5. DEVELOPERS.md도 분할 (File Map 재구성)

#### Merge

1. 병합 대상 CLAUDE.md들을 Read
2. Exports/Behaviors/Contracts를 통합하여 새 CLAUDE.md 생성
3. 병합된 원본 파일 삭제
4. Dependencies 업데이트 (참조 경로 변경)
5. DEVELOPERS.md 통합

### 6. 스키마 검증

리팩토링된 CLAUDE.md를 검증합니다:

```bash
$CLI_PATH validate-schema --file {new_path}/CLAUDE.md --strict
```

### 7. 의존 모듈 업데이트 안내

영향받는 모듈의 Dependencies를 업데이트하도록 안내합니다:

```
리팩토링 완료. 다음 모듈의 Dependencies 경로를 업데이트하세요:
- src/api/CLAUDE.md: src/auth → src/auth/token
- src/middleware/CLAUDE.md: src/auth → src/auth/token

이후:
  /compile --all --conflict overwrite  — 전체 재컴파일
  /validate  — 검증
```

### 8. 코드 재생성 (선택)

AskUserQuestion으로 자동 재컴파일 여부 확인:

```
AskUserQuestion: "리팩토링된 계약으로 코드를 재생성하시겠습니까?"
옵션: [예 (/compile --all --conflict overwrite), 아니오 (수동 처리)]
```

## DO / DON'T

**DO:**
- 계약(CLAUDE.md)을 먼저 리팩토링, 코드는 /compile로 재생성
- /impact으로 영향 분석 후 진행
- 사용자 승인 후 실행
- 스키마 검증 후 완료

**DON'T:**
- 소스코드 직접 리팩토링 (계약 → 코드 순서)
- 사용자 승인 없이 파일 삭제/이동
- 영향 분석 없이 진행

## Examples

<example>
<user_request>/refactor src/auth --mode split</user_request>
<assistant_response>
src/auth/CLAUDE.md를 분석합니다...

Exports: 6개 (validateToken, revokeToken, Claims, createSession, destroySession, SessionConfig)

분할 제안:
  src/auth/token/CLAUDE.md: validateToken, revokeToken, Claims
  src/auth/session/CLAUDE.md: createSession, destroySession, SessionConfig

영향 분석:
  src/api — validateToken 참조 → src/auth/token으로 경로 변경 필요

계속 진행하시겠습니까? [예/아니오]

→ 예

리팩토링 완료:
  Created: src/auth/token/CLAUDE.md
  Created: src/auth/session/CLAUDE.md
  Updated: src/auth/CLAUDE.md (Structure 업데이트)

다음 단계:
  1. src/api/CLAUDE.md Dependencies 경로 업데이트
  2. /compile --all --conflict overwrite
  3. /validate
</assistant_response>
</example>
