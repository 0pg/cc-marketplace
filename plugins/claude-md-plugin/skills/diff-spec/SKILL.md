---
name: diff-spec
version: 1.0.0
aliases: [spec-diff, contract-diff]
description: |
  This skill should be used when the user asks to "compare CLAUDE.md versions",
  "show spec changes", "diff specification", "what changed in the contract",
  or uses "/diff-spec".
  Shows semantic diff between two versions of a CLAUDE.md contract.
  Trigger keywords: 스펙 변경, 계약 비교, 문서 diff, 명세 변경
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write]
---

> **DEPRECATED (v6.0.0)**: This skill depends on CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in a future version.

# /diff-spec

CLAUDE.md 계약의 버전 간 시맨틱 diff를 표시합니다.

단순 텍스트 diff가 아닌, 계약 섹션별로 구조화된 변경 분석을 제공합니다.

## Triggers

- `/diff-spec`
- `스펙 변경`
- `계약 비교`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 예 | - | CLAUDE.md가 있는 디렉토리 경로 |
| `--ref` | 아니오 | `HEAD` | 비교 대상 git ref (commit hash, branch, tag) |

## Workflow

### 1. 두 버전 로드

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")

TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

# 현재 버전 파싱
$CLI_PATH parse-claude-md --file {path}/CLAUDE.md > "${TMP_DIR}diff-spec-current.json"

# 이전 버전 (git ref) 파싱
git show {ref}:{path}/CLAUDE.md > "${TMP_DIR}diff-spec-prev.md" 2>/dev/null
$CLI_PATH parse-claude-md --file "${TMP_DIR}diff-spec-prev.md" > "${TMP_DIR}diff-spec-prev.json"
```

이전 버전이 없으면 (새 파일) 모든 항목을 ADDED로 분류합니다.

### 2. 섹션별 시맨틱 비교

두 파싱 결과 JSON을 Read하여 섹션별로 비교합니다:

#### Purpose 비교
- 텍스트 변경 여부

#### Exports 비교
각 export를 이름으로 매칭하여 변경 분류:

| 변경 유형 | 조건 |
|-----------|------|
| **ADDED** | 현재에만 존재 |
| **REMOVED** | 이전에만 존재 |
| **SIGNATURE_CHANGED** | 양쪽에 존재, 시그니처 다름 |
| **UNCHANGED** | 양쪽 동일 |

#### Behavior 비교
각 behavior를 input/output 패턴으로 매칭:

| 변경 유형 | 조건 |
|-----------|------|
| **ADDED** | 현재에만 존재 |
| **REMOVED** | 이전에만 존재 |
| **MODIFIED** | 유사한 input, 다른 output |

#### Contract 비교
각 function의 contract를 비교:
- preconditions 추가/제거
- postconditions 추가/제거
- throws 추가/제거

#### Dependencies 비교
- 추가된 의존성
- 제거된 의존성
- 버전 변경

#### Structure 비교
- 추가된 파일/디렉토리
- 제거된 파일/디렉토리

### 3. 시맨틱 Diff 보고서 생성

```markdown
# 계약 시맨틱 Diff: {path}

**비교:** {ref} → 현재 (working copy)

## 요약

| 섹션 | 추가 | 제거 | 변경 | 상태 |
|------|------|------|------|------|
| Exports | 1 | 1 | 1 | BREAKING |
| Behavior | 2 | 0 | 1 | MODIFIED |
| Contract | 1 | 0 | 0 | ADDED |
| Dependencies | 0 | 1 | 0 | MODIFIED |
| Structure | 1 | 0 | 0 | MODIFIED |

## Exports 변경

### ADDED
- `+ revokeToken(tokenId: string): Promise<void>`

### REMOVED
- `- formatDate(date: Date): string` [BREAKING]

### SIGNATURE_CHANGED
- `validateToken(token: string): boolean`
  → `validateToken(token: string, options?: ValidateOptions): Promise<boolean>` [BREAKING]

### UNCHANGED
- `Claims { userId: string, exp: number }`

## Behavior 변경

### ADDED
- `+ 토큰 폐기 시 → 즉시 무효화`
- `+ 만료된 토큰 갱신 시 → RefreshTokenError`

### MODIFIED
- `유효한 토큰 → Claims 반환` → `유효한 토큰 → Claims 반환 (옵션에 따라 캐시 적용)`

## Contract 변경

### validateToken
- `+ precondition: options.timeout must be positive if provided`

## Dependencies 변경

### REMOVED
- `- lodash@4.17.21`

## Structure 변경

### ADDED
- `+ cache.ts: 토큰 캐시 관리`
```

### 4. 결과 출력

보고서를 사용자에게 직접 출력합니다.
파일 사본은 `${TMP_DIR}diff-spec-{dir-safe-name}.md`에 저장합니다.

## 출력 예시

```
/diff-spec src/auth

계약 시맨틱 Diff: src/auth
===========================
비교: HEAD → 현재

요약
----
| 섹션       | 추가 | 제거 | 변경 | 상태     |
|------------|------|------|------|----------|
| Exports    | 1    | 1    | 1    | BREAKING |
| Behavior   | 2    | 0    | 1    | MODIFIED |
| Contract   | 1    | 0    | 0    | ADDED    |

Exports 변경
-----------
+ revokeToken(tokenId: string): Promise<void>
- formatDate(date: Date): string  [BREAKING]
~ validateToken: 시그니처 변경  [BREAKING]
  (token: string): boolean
  → (token: string, options?: ValidateOptions): Promise<boolean>

다음 단계:
  /impact src/auth — 영향받는 모듈 분석
```

## DO / DON'T

**DO:**
- parse-claude-md CLI로 두 버전을 구조적으로 파싱
- 이름 기반 매칭으로 추가/제거/변경 분류
- BREAKING 변경(시그니처 변경, 삭제)을 명확히 표시
- `/impact` 연계 안내

**DON'T:**
- CLAUDE.md나 소스코드 수정 (분석만)
- 단순 텍스트 diff 출력 (구조적 시맨틱 diff 제공)
- 전체 프로젝트 분석 (대상 경로만)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md를 찾을 수 없음" 메시지 |
| 이전 버전 없음 (새 파일) | 모든 항목을 ADDED로 분류 |
| git ref 없음 | "ref를 찾을 수 없음" 메시지 |
| parse-claude-md 실패 | Raw text diff fallback |

## 관련 컴포넌트

- CLI: `parse-claude-md` (CLAUDE.md 구조적 파싱)
- `/impact`: diff-spec 결과 후 영향 분석 연계

## Examples

<example>
<user_request>/diff-spec src/auth</user_request>
<assistant_response>
계약 시맨틱 Diff: src/auth
===========================
비교: HEAD → 현재

요약
----
| 섹션    | 추가 | 제거 | 변경 | 상태     |
|---------|------|------|------|----------|
| Exports | 1    | 0    | 1    | BREAKING |

Exports 변경
-----------
+ revokeToken(tokenId: string): Promise<void>
~ validateToken: 시그니처 변경 [BREAKING]

다음 단계:
  /impact src/auth — 영향받는 모듈 분석
</assistant_response>
</example>

<example>
<user_request>/diff-spec src/utils --ref v2.0.0</user_request>
<assistant_response>
계약 시맨틱 Diff: src/utils
============================
비교: v2.0.0 → 현재

요약
----
| 섹션    | 추가 | 제거 | 변경 | 상태      |
|---------|------|------|------|-----------|
| Exports | 2    | 0    | 0    | COMPATIBLE|

Exports 변경
-----------
+ parseNumber(input: string): number
+ formatCurrency(amount: number, locale: string): string

변경 없음 — 하위 호환.
</assistant_response>
</example>
