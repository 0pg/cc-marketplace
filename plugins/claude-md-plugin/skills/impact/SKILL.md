---
name: impact
version: 2.0.0
aliases: [impact-analysis, affected]
description: |
  This skill should be used when the user asks to "analyze impact of CLAUDE.md changes",
  "find affected modules", "check breaking changes", "what depends on this module",
  or uses "/impact".
  Analyzes CLAUDE.md changes (Requirements, Purpose) and reports which dependent modules are affected.
  Trigger keywords: 영향 분석, 의존 모듈, 변경 영향, breaking change
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write]
---

# /impact

CLAUDE.md 변경이 다른 모듈에 미치는 영향을 분석합니다.

## Triggers

- `/impact`
- `영향 분석`
- `변경 영향`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 변경된 CLAUDE.md가 있는 디렉토리 경로 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 변경 감지

```bash
git diff HEAD -- {path}/CLAUDE.md
```

git diff가 없으면:
```bash
git diff --cached -- {path}/CLAUDE.md
```

변경이 없으면: "변경된 CLAUDE.md가 없습니다." → 종료.

### 2. 섹션별 변경 분석

현재/이전 버전을 파싱하여 비교합니다:

```bash
# 현재 버전 파싱
$CLI_PATH parse-claude-md --file {path}/CLAUDE.md > "${TMP_DIR}impact-current.json"

# 이전 버전 파싱
git show HEAD:{path}/CLAUDE.md > "${TMP_DIR}impact-prev-claude.md" 2>/dev/null
if [ $? -eq 0 ]; then
  $CLI_PATH parse-claude-md --file "${TMP_DIR}impact-prev-claude.md" > "${TMP_DIR}impact-prev.json"
fi
```

두 버전의 JSON을 Read하여 섹션별 diff:

| 섹션 | 비교 단위 | 영향 수준 |
|------|----------|----------|
| **Purpose** | 텍스트 변경 | HIGH (모듈 역할 변경) |
| **Requirements** 추가 | 항목 단위 | HIGH (새 제약 → 의존 모듈 코드 영향 가능) |
| **Requirements** 제거 | 항목 단위 | MEDIUM (제약 완화) |
| **Requirements** 수정 | 항목 단위 | HIGH (제약 변경 → 의존 모듈 동작 변경 가능) |
| **Domain Context** | 항목 변경 | LOW (맥락 정보) |

이전 버전이 없으면 (새 모듈) 모든 항목을 ADDED로 분류합니다.

### 3. 의존 모듈 검색

변경된 모듈을 참조하는 다른 모듈을 탐색합니다:

```bash
# 전체 CLAUDE.md 인덱스 생성
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}impact-index.json"
```

인덱스를 Read하여:
1. 각 CLAUDE.md에서 변경 대상 경로를 Grep으로 검색
2. 해당 모듈의 CLAUDE.md를 Read하여 의존 관계 확인

### 4. 영향 보고서 생성

```markdown
# 변경 영향 분석: {path}

## 변경 요약

| 섹션 | 변경 유형 | 영향 수준 |
|------|----------|----------|
| Purpose | 변경 | HIGH |
| Requirements | 추가 (2), 수정 (1) | HIGH |
| Domain Context | 수정 (1) | LOW |

## Requirements 변경 상세

### 추가
- `+ 동시 접속 최대 100명`
- `+ UTF-8 인코딩 필수`

### 수정
- `토큰 만료 최대 7일` → `토큰 만료 최대 14일`

## 영향받는 모듈

### HIGH (코드 수정 필요)

#### src/api
- **참조 방식**: CLAUDE.md에서 {path} 참조
- **영향**: Requirements 변경으로 인한 동작 변경 가능
- **추천**: `/validate src/api` → `/compile --path src/api --conflict overwrite`

### LOW (확인만 필요)

(없음)

## 추천 액션

1. `/validate` — 영향받는 모듈 검증
2. `/compile --path src/api --conflict overwrite` — 재컴파일
```

보고서를 `${TMP_DIR}impact-{dir-safe-name}.md`에 저장합니다.

### 5. 결과 출력

보고서 내용을 사용자에게 직접 출력합니다.

## DO / DON'T

**DO:**
- git diff로 실제 변경된 섹션만 분석
- Requirements 변경을 항목 단위로 추적
- 영향 수준별 분류 (HIGH/MEDIUM/LOW)
- 구체적인 추천 액션 제시

**DON'T:**
- CLAUDE.md나 소스코드 수정 (분석/보고만)
- 전체 프로젝트 CLAUDE.md 읽기 (인덱스 기반 필터링 후 필요한 것만)
- 변경 없는 섹션 분석

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md를 찾을 수 없음" 메시지 |
| git diff 없음 | "변경사항 없음" 안내 |
| 이전 버전 없음 | 모든 항목을 ADDED로 분류 |
| scan-claude-md 실패 | CLI 오류 표시, 수동 분석 안내 |

## Examples

<example>
<user_request>/impact src/auth</user_request>
<assistant_response>
변경 영향 분석: src/auth
==============================

변경 요약
---------
| 섹션 | 변경 유형 | 영향 수준 |
|------|----------|----------|
| Requirements | 수정 (1) | HIGH |

Requirements 변경 상세
-------------------
~ `토큰 만료 최대 7일` → `토큰 만료 최대 14일`

영향받는 모듈
-----------
HIGH (1개 모듈):
  src/api
    - Requirements 변경으로 인한 동작 변경 가능
    - 추천: /validate src/api

추천 액션:
  /validate src/api
  /compile --path src/api --conflict overwrite
</assistant_response>
</example>
