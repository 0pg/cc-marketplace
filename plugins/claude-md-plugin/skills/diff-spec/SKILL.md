---
name: diff-spec
version: 2.0.0
aliases: [spec-diff, contract-diff]
description: |
  This skill should be used when the user asks to "compare CLAUDE.md versions",
  "show spec changes", "diff specification", "what changed in the contract",
  or uses "/diff-spec".
  Shows semantic diff between two versions of a CLAUDE.md, comparing Purpose, Requirements, Domain Context, and Conventions.
  Trigger keywords: 스펙 변경, 문서 비교, 문서 diff, 명세 변경
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write]
---

# /diff-spec

CLAUDE.md의 버전 간 시맨틱 diff를 표시합니다.

단순 텍스트 diff가 아닌, 섹션별로 구조화된 변경 분석을 제공합니다.

## Triggers

- `/diff-spec`
- `스펙 변경`
- `문서 비교`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 예 | - | CLAUDE.md가 있는 디렉토리 경로 |
| `--ref` | 아니오 | `HEAD` | 비교 대상 git ref (commit hash, branch, tag) |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 두 버전 로드

```bash
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
- 텍스트 변경 여부: CHANGED / UNCHANGED

#### Requirements 비교 (항목 단위)

각 requirement를 매칭하여 변경 분류:

| 변경 유형 | 조건 |
|-----------|------|
| **ADDED** | 현재에만 존재 |
| **REMOVED** | 이전에만 존재 |
| **MODIFIED** | 양쪽에 유사 항목 존재, 내용 다름 |
| **UNCHANGED** | 양쪽 동일 |

#### Domain Context 비교

항목별 변경 분류 (ADDED / REMOVED / MODIFIED / UNCHANGED)

#### Conventions 비교 (서브섹션 단위)

각 서브섹션별 변경 여부:
- Project Structure, Module Boundaries, Naming Conventions
- Language & Runtime, Coding Rules, Naming Rules

### 3. 시맨틱 Diff 보고서 생성

```markdown
# 시맨틱 Diff: {path}

**비교:** {ref} → 현재 (working copy)

## 요약

| 섹션 | 추가 | 제거 | 변경 | 상태 |
|------|------|------|------|------|
| Purpose | - | - | 1 | CHANGED |
| Requirements | 2 | 1 | 1 | BREAKING |
| Domain Context | 1 | 0 | 0 | MODIFIED |
| Conventions | 0 | 0 | 1 | MODIFIED |

## Purpose 변경

- 이전: "JWT 기반 인증 모듈"
+ 현재: "JWT 및 OAuth2 기반 인증 모듈"

## Requirements 변경

### ADDED
- `+ 동시 세션 최대 5개`
- `+ OAuth2 PKCE 필수`

### REMOVED
- `- 레거시 MD5 해시 지원` [BREAKING]

### MODIFIED
- `토큰 만료 최대 7일` → `토큰 만료 최대 14일`

### UNCHANGED
- `UTF-8 인코딩만 허용`

## Domain Context 변경

### ADDED
- `+ OAuth2 IdP: Google, GitHub 지원`

## Conventions 변경

### MODIFIED
- `Coding Rules`: 린트 규칙 추가
```

### 4. 결과 출력

보고서를 사용자에게 직접 출력합니다.
파일 사본은 `${TMP_DIR}diff-spec-{dir-safe-name}.md`에 저장합니다.

## DO / DON'T

**DO:**
- parse-claude-md CLI로 두 버전을 구조적으로 파싱
- Requirements를 항목 단위로 ADDED/REMOVED/MODIFIED 비교
- BREAKING 변경(Requirements 제거/수정)을 명확히 표시
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

## Examples

<example>
<user_request>/diff-spec src/auth</user_request>
<assistant_response>
시맨틱 Diff: src/auth
=======================
비교: HEAD → 현재

요약
----
| 섹션 | 추가 | 제거 | 변경 | 상태 |
|------|------|------|------|------|
| Requirements | 1 | 0 | 1 | MODIFIED |

Requirements 변경
-----------
+ 동시 세션 최대 5개
~ 토큰 만료: 7일 → 14일

다음 단계:
  /impact src/auth — 영향받는 모듈 분석
</assistant_response>
</example>

<example>
<user_request>/diff-spec src/utils --ref v2.0.0</user_request>
<assistant_response>
시맨틱 Diff: src/utils
========================
비교: v2.0.0 → 현재

요약
----
| 섹션 | 추가 | 제거 | 변경 | 상태 |
|------|------|------|------|------|
| Domain Context | 1 | 0 | 0 | MODIFIED |

Domain Context 변경
-----------
+ Redis 6.0 캐시 레이어 도입

변경 없음 — 하위 호환.
</assistant_response>
</example>
