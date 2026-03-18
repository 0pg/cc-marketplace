---
name: impact
version: 1.0.0
aliases: [impact-analysis, affected]
description: |
  This skill should be used when the user asks to "analyze impact of CLAUDE.md changes",
  "find affected modules", "check breaking changes", "what depends on this module",
  or uses "/impact".
  Analyzes contract (CLAUDE.md) changes and reports which dependent modules are affected.
  Trigger keywords: 영향 분석, 의존 모듈, 변경 영향, breaking change, 계약 변경
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task]
---

# /impact

계약(CLAUDE.md) 변경이 다른 모듈에 미치는 영향을 분석합니다.

**Code-First + Spec-as-Contract 모델**: 계약 변경은 breaking change일 수 있습니다.
Exports 시그니처 변경/삭제는 해당 export를 참조하는 모든 모듈에 영향을 줍니다.

## Triggers

- `/impact`
- `영향 분석`
- `변경 영향`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 변경된 계약이 있는 디렉토리 경로 |

## Workflow

### 1. 변경 대상 식별

대상 경로의 CLAUDE.md 변경 사항을 감지합니다:

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")

TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

**변경 감지 방법:**

```bash
# git diff로 Exports 변경 감지
git diff HEAD -- {path}/CLAUDE.md
```

git diff가 없으면 (새 파일이거나 unstaged):
```bash
git diff --cached -- {path}/CLAUDE.md
git status -- {path}/CLAUDE.md
```

변경이 없으면 사용자에게 알리고 종료:
> "변경된 CLAUDE.md가 없습니다. 특정 경로를 지정하세요."

### 2. 변경된 Exports 분석

CLAUDE.md의 Exports 섹션 변경을 구조적으로 분석합니다:

```bash
# 현재 버전 파싱
$CLI_PATH parse-claude-md --file {path}/CLAUDE.md > "${TMP_DIR}impact-current.json"

# 이전 버전 파싱 (git show)
git show HEAD:{path}/CLAUDE.md > "${TMP_DIR}impact-prev-claude.md" 2>/dev/null
if [ $? -eq 0 ]; then
  $CLI_PATH parse-claude-md --file "${TMP_DIR}impact-prev-claude.md" > "${TMP_DIR}impact-prev.json"
fi
```

두 버전의 Exports를 비교하여 변경 분류:

| 변경 유형 | 영향 수준 | 설명 |
|-----------|----------|------|
| **REMOVED** | BREAKING | export 삭제 — 의존 모듈 컴파일 실패 |
| **SIGNATURE_CHANGED** | BREAKING | 시그니처 변경 — 의존 모듈 호출 실패 |
| **BEHAVIOR_CHANGED** | VERIFY | 동작 변경 — 의존 모듈 재검증 필요 |
| **ADDED** | COMPATIBLE | 새 export 추가 — 기존 의존 모듈 영향 없음 |
| **UNCHANGED** | NONE | 변경 없음 |

이전 버전이 없으면 (새 모듈) 모든 export를 ADDED로 분류합니다.

### 3. 의존 모듈 검색

변경된 Exports를 Dependencies로 참조하는 모듈을 검색합니다:

```bash
# 전체 CLAUDE.md 인덱스 생성
$CLI_PATH scan-claude-md --root {project_root} --output "${TMP_DIR}impact-index.json"
```

인덱스를 Read하여 각 모듈의 Dependencies 섹션에서 변경 대상 모듈을 참조하는 모듈을 찾습니다:

**검색 방법:**
1. 인덱스의 각 모듈에 대해 CLAUDE.md를 Read
2. Dependencies 섹션에서 변경 대상 경로를 참조하는지 확인
3. 참조하는 모듈의 Dependencies에서 사용하는 symbol 목록 추출
4. 변경된 export 중 해당 symbol이 포함되는지 교차 확인

### 4. 영향 보고서 생성

```markdown
# 계약 변경 영향 분석: {path}

## 변경 요약

| Export | 변경 유형 | 영향 수준 |
|--------|----------|----------|
| `validateToken` | SIGNATURE_CHANGED | BREAKING |
| `Claims` | UNCHANGED | NONE |
| `revokeToken` | ADDED | COMPATIBLE |
| `formatDate` | REMOVED | BREAKING |

## 영향받는 모듈

### BREAKING (코드 수정 필요)

#### src/api
- **참조하는 export:** `validateToken`, `formatDate`
- **영향:**
  - `validateToken` 시그니처 변경 → 호출 코드 수정 필요
  - `formatDate` 삭제 → 대체 구현 필요
- **추천:** `/compile --path src/api --conflict overwrite`

#### src/middleware
- **참조하는 export:** `validateToken`
- **영향:** `validateToken` 시그니처 변경 → 호출 코드 수정 필요
- **추천:** `/compile --path src/middleware --conflict overwrite`

### VERIFY (재검증 권장)

(없음)

### COMPATIBLE (영향 없음)

- `revokeToken` 추가 — 기존 모듈에 영향 없음

## 추천 액션

1. **BREAKING 모듈 재컴파일:**
   ```
   /compile --path src/api --conflict overwrite
   /compile --path src/middleware --conflict overwrite
   ```

2. **전체 검증:**
   ```
   /validate
   ```
```

보고서를 `${TMP_DIR}impact-{dir-safe-name}.md`에 저장합니다.

### 5. 결과 출력

보고서 내용을 사용자에게 직접 출력합니다.

## 출력 예시

```
/impact src/auth

계약 변경 영향 분석: src/auth
==============================

변경 요약
---------
| Export          | 변경 유형          | 영향 수준   |
|-----------------|-------------------|------------|
| validateToken   | SIGNATURE_CHANGED | BREAKING   |
| revokeToken     | ADDED             | COMPATIBLE |

영향받는 모듈
-----------

BREAKING (1개 모듈):
  src/api
    - validateToken 시그니처 변경 → 호출 코드 수정 필요
    - 추천: /compile --path src/api --conflict overwrite

COMPATIBLE:
  revokeToken 추가 — 기존 모듈에 영향 없음

추천 액션:
  /compile --path src/api --conflict overwrite
  /validate
```

## DO / DON'T

**DO:**
- git diff로 실제 변경된 Exports만 분석
- Dependencies 참조를 정확히 추적하여 영향 모듈 식별
- BREAKING/VERIFY/COMPATIBLE 수준별로 영향 분류
- 구체적인 추천 액션 제시 (/compile 경로 포함)

**DON'T:**
- CLAUDE.md나 소스코드 수정 (분석/보고만)
- 전체 프로젝트 CLAUDE.md 읽기 (인덱스 기반 필터링 후 필요한 것만)
- 변경 없는 export 분석 (변경분만 추적)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | "CLAUDE.md를 찾을 수 없음" 메시지 출력 |
| git diff 없음 (변경 없음) | "변경사항 없음" 안내 |
| 이전 버전 없음 (새 모듈) | 모든 export를 ADDED로 분류 |
| scan-claude-md 실패 | CLI 오류 표시, 수동 분석 안내 |
| 의존 모듈 CLAUDE.md 읽기 실패 | 해당 모듈 스킵, 경고 표시 |

## 관련 컴포넌트

- CLI: `scan-claude-md`, `parse-claude-md` (인덱스 및 파싱)
- `/validate`: 영향 분석 후 전체 검증에 사용
- `/compile`: 영향받는 모듈 재컴파일에 사용

## Examples

<example>
<user_request>/impact src/auth</user_request>
<assistant_response>
계약 변경 영향 분석: src/auth
==============================

변경 요약
---------
| Export          | 변경 유형          | 영향 수준   |
|-----------------|-------------------|------------|
| validateToken   | SIGNATURE_CHANGED | BREAKING   |
| Claims          | UNCHANGED         | NONE       |

영향받는 모듈
-----------
BREAKING (1개 모듈):
  src/api
    - validateToken 시그니처 변경
    - 추천: /compile --path src/api --conflict overwrite

추천 액션:
  /compile --path src/api --conflict overwrite
  /validate
</assistant_response>
</example>
</content>
</invoke>