---
name: validate
version: 1.2.0
aliases: [check, verify, lint]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "check export coverage", "lint documentation", or uses "/validate". Runs validator agent for comprehensive validation, then verifies and fixes confirmed issues via multi-agent pipeline.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, 문서 린트, export 커버리지
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task]
---

# /validate

CLAUDE.md 문서의 품질과 코드 일치 여부를 검증.

## Triggers

- `/validate`
- `CLAUDE.md 검증`
- `문서 검증`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 검증 대상 경로 (디렉토리 또는 파일) |

## Workflow

### 1. 대상 수집

Glob으로 대상 경로의 모든 CLAUDE.md 수집:

```
Glob("**/CLAUDE.md", path={path})
```

### 1.5. 스키마 검증 (CLI)

validate SKILL이 직접 Bash로 CLI를 실행하여 각 CLAUDE.md의 스키마를 검증합니다.

**임시 디렉토리 초기화:**
```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

**각 CLAUDE.md에 대해 CLI 실행:**
```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    (cd "$CORE_DIR" && cargo build --release)
fi

for claude_md in ${targets}; do
  dir_safe=$(echo "$claude_md" | sed 's/\//-/g' | sed 's/\.//g')
  $CLI_PATH validate-schema \
    --file "$claude_md" --strict \
    --output "${TMP_DIR}schema-${dir_safe}.json"
done
```

**결과 JSON (~500bytes/file):**
```json
{"file": "src/auth/CLAUDE.md", "valid": false, "errors": [{"error_type": "MissingSection", "message": "Missing required section: Behavior", "section": "Behavior"}], "warnings": []}
```

> **참고:** `validate-schema` CLI는 `.claude/extract-results/`를 사용하지만,
> validate는 세션 임시 결과이므로 `${TMP_DIR}`에 저장합니다.
> `CLAUDE_SESSION_ID`가 설정되면 `.claude/tmp/{sessionId}/`로 세션별 격리되고,
> 미설정 시 `.claude/tmp/`에 fallback합니다.

validate SKILL이 각 JSON을 Read하여 스키마 이슈를 수집합니다.
- `valid: true` → 스키마 통과, drift 검증 진행
- `valid: false` → **auto-fix 시도**: `claude-md-core fix-schema`를 실행하여 누락된 allow-none 섹션을 자동 추가한 후, `validate-schema`를 재실행합니다. 재검증 후에도 실패하면 스키마 이슈를 기록합니다. drift 검증은 스키마 결과와 무관하게 진행합니다.

```bash
# Auto-fix (검증 실패 시만)
$CLI_PATH fix-schema --file "$claude_md"
# 재검증
$CLI_PATH validate-schema --file "$claude_md" --strict --output "${TMP_DIR}schema-${dir_safe}.json"
```

### 2. 배치 Drift 검증

validator agent를 **최대 3개씩 배치 처리**하여 context 폭발을 방지합니다.

**배치 처리 규칙:**
- 대상 CLAUDE.md 목록을 최대 3개씩 나누어 배치 생성
- 각 배치 내의 validator agent Task를 **단일 메시지에서 병렬로 호출**
- 배치 완료 후 다음 배치 진행

**진행 파일 초기화:**
```bash
: > "${TMP_DIR}validate-progress.jsonl"
```

**각 배치 완료 후, 결과를 `${TMP_DIR}validate-progress.jsonl` 파일에 append:**

validator agent의 결과 블록을 파싱하여 아래 형식으로 append합니다:
```bash
printf '{"directory":"src/auth","status":"success","issues_count":0,"export_coverage":95,"result_file":"%svalidate-src-auth.md"}\n' "$TMP_DIR" >> "${TMP_DIR}validate-progress.jsonl"
printf '{"directory":"src/utils","status":"success","issues_count":2,"export_coverage":72,"result_file":"%svalidate-src-utils.md"}\n' "$TMP_DIR" >> "${TMP_DIR}validate-progress.jsonl"
```

**compact 대비:**
- compact이 발생해도 `${TMP_DIR}validate-progress.jsonl`에 이전 배치 결과가 보존됨
- 최종 보고서 생성 시 context가 아닌 이 파일을 읽어서 생성
- validator agent의 상세 결과도 개별 `${TMP_DIR}validate-*.md` 파일에 저장되어 있음
- **compact 후 재개:** `${TMP_DIR}validate-progress.jsonl`을 Read하여 이미 완료된 directory 목록을 확인하고, 나머지 대상만 다음 배치로 처리. 중복 실행 방지를 위해 JSONL의 `directory` 필드와 대상 목록을 대조.

### 3. 결과 수집

validator agent는 구조화된 블록으로 결과를 반환:

```
---validate-result---
status: success | failed
result_file: ${TMP_DIR}validate-{dir-safe-name}.md
directory: {directory}
issues_count: {N}
export_coverage: {0-100}
---end-validate-result---
```

### 4. 중간 결과 확인

`${TMP_DIR}validate-progress.jsonl`을 Read하여 이슈가 있는 디렉토리를 파악합니다. 이슈가 있는 디렉토리는 Step 5 재검증 대상이 됩니다.

**이슈 없는 경우:** 모든 디렉토리가 이슈 0개이고 스키마도 모두 PASS이면, Step 5-6을 스킵하고 Step 7로 직행합니다.

### 5. 이슈 재검증 (issue-verifier)

검증 보고서에서 이슈가 있는 디렉토리만 대상으로, issue-verifier agent를 통해 각 이슈가 진짜 문제인지 재검증합니다.

**재검증 대상 선별:**
- `${TMP_DIR}validate-progress.jsonl`에서 `issues_count > 0`인 디렉토리만 추출
- 스키마 검증에서도 `valid: false`인 디렉토리 포함

**배치 처리 규칙:**
- validator agent와 동일하게 **최대 3개씩 배치 처리**
- 각 배치 내의 issue-verifier agent Task를 **단일 메시지에서 병렬로 호출**

**각 배치에서 issue-verifier agent 호출:**
```
Task(issue-verifier):
  검증 대상: {directory}
  검증 결과 파일: ${TMP_DIR}validate-{dir-safe-name}.md
  CLAUDE.md: {directory}/CLAUDE.md
  스키마 결과: ${TMP_DIR}schema-{dir-safe-name}.json
```

**결과 수집:**

issue-verifier agent는 구조화된 블록으로 결과를 반환:
```
---issue-verifier-result---
status: success | failed
result_file: ${TMP_DIR}verified-{dir-safe-name}.md
directory: {directory}
total_issues: {N}
confirmed_issues: {N}
false_positives: {N}
---end-issue-verifier-result---
```

**진행 파일 업데이트:**
```bash
printf '{"directory":"%s","phase":"verify","confirmed_issues":%d,"false_positives":%d,"result_file":"%s"}\n' \
  "$directory" "$confirmed" "$false_positives" "${TMP_DIR}verified-${dir_safe}.md" \
  >> "${TMP_DIR}validate-progress.jsonl"
```

**스킵 조건:** 이슈가 0개인 디렉토리는 재검증을 스킵합니다.

### 6. 이슈 수정 (issue-fixer)

재검증에서 CONFIRMED된 이슈가 있는 디렉토리를 대상으로, issue-fixer agent를 통해 CLAUDE.md를 수정합니다.

**수정 대상 선별:**
- Step 5에서 `confirmed_issues > 0`인 디렉토리만 추출

**배치 처리 규칙:**
- **최대 3개씩 배치 처리**
- 각 배치 내의 issue-fixer agent Task를 **단일 메시지에서 병렬로 호출**

**각 배치에서 issue-fixer agent 호출:**
```
Task(issue-fixer):
  수정 대상: {directory}
  재검증 결과 파일: ${TMP_DIR}verified-{dir-safe-name}.md
  CLAUDE.md: {directory}/CLAUDE.md
```

**결과 수집:**

issue-fixer agent는 구조화된 블록으로 결과를 반환:
```
---issue-fixer-result---
status: success | failed
result_file: ${TMP_DIR}fixed-{dir-safe-name}.md
directory: {directory}
fixed_count: {N}
skipped_count: {N}
schema_revalidation: PASS | FAIL
---end-issue-fixer-result---
```

**진행 파일 업데이트:**
```bash
printf '{"directory":"%s","phase":"fix","fixed_count":%d,"skipped_count":%d,"schema_revalidation":"%s","result_file":"%s"}\n' \
  "$directory" "$fixed" "$skipped" "$schema_result" "${TMP_DIR}fixed-${dir_safe}.md" \
  >> "${TMP_DIR}validate-progress.jsonl"
```

**스킵 조건:** CONFIRMED 이슈가 0개인 디렉토리는 수정을 스킵합니다.

### 7. 통합 보고서 생성

`${TMP_DIR}validate-progress.jsonl`을 Read하여 스키마 검증, Drift 검증, 재검증, 수정 결과를 병합한 통합 보고서를 생성합니다.

**JSONL 파싱 방법:** 같은 파일에 phase별 라인이 혼재합니다. `directory` 필드 기준으로 그룹화:
- `phase` 필드 없음 → validate 결과 (`issues_count`, `export_coverage`)
- `"phase":"verify"` → verifier 결과 (`confirmed_issues`, `false_positives`)
- `"phase":"fix"` → fixer 결과 (`fixed_count`, `skipped_count`, `schema_revalidation`)

이슈가 없어 verify/fix 단계를 스킵한 디렉토리는 phase 라인이 없으므로 `-` 로 표시.

**보고서 형식:**
```markdown
# CLAUDE.md 검증 보고서

## 요약

| 디렉토리 | 스키마 | Drift 이슈 | 확인됨 | 오탐 | 수정됨 | Export 커버리지 | 상태 |
|----------|--------|-----------|--------|------|--------|---------------|------|
| src/auth | PASS | 0 | - | - | - | 95% | 양호 |
| src/utils | PASS | 3 | 2 | 1 | 2 | 78%→85% | 수정 완료 |
| src/legacy | FAIL (1) | 5 | 4 | 1 | 3 | 45%→60% | 개선 필요 |

## 상세 결과

### src/auth

#### 스키마 검증
- PASS

#### Drift 검증
- 이슈 없음

### src/utils

#### 스키마 검증
- PASS

#### Drift 검증 (원본)
- STALE: formatDate export가 코드에 없음
- MISSING: parseNumber export가 문서에 없음
- MISSING: _internalHelper export가 문서에 없음

#### 재검증 결과
- STALE formatDate → **CONFIRMED** (코드에서 삭제 확인)
- MISSING parseNumber → **CONFIRMED** (public API)
- MISSING _internalHelper → **FALSE_POSITIVE** (private 헬퍼)

#### 수정 결과
- formatDate: Exports 섹션에서 제거 ✓
- parseNumber: Exports 섹션에 추가 ✓
```

**중요:** context에 남아있는 결과가 아닌, 파일에 누적된 결과를 사용합니다.
- `${TMP_DIR}validate-progress.jsonl`: 요약 정보 (모든 phase)
- `${TMP_DIR}schema-*.json`: 스키마 검증 결과
- `${TMP_DIR}validate-*.md`: Drift 검증 상세 결과
- `${TMP_DIR}verified-*.md`: 재검증 상세 결과
- `${TMP_DIR}fixed-*.md`: 수정 상세 결과

### 8. 임시 파일 정리

`${TMP_DIR}` 내 임시 파일은 세션별로 격리되어 다른 세션과 충돌하지 않음. 필요 시 `rm -rf .claude/tmp/` 으로 일괄 정리 가능.

## 성공 기준

| 상태 | 조건 |
|------|------|
| **양호** | 스키마 PASS AND Drift 이슈 0개 AND Export 커버리지 점수 90% 이상 |
| **수정 완료** | 이슈가 있었으나 issue-fixer가 모두 수정 완료 |
| **개선 권장** | 스키마 PASS AND (확인된 Drift 1-2개 OR Export 커버리지 점수 70-89%) AND 미수정 이슈 있음 |
| **개선 필요** | 스키마 FAIL OR 확인된 Drift 3개 이상 OR Export 커버리지 점수 70% 미만 AND 미수정 이슈 있음 |

## 출력 예시

```
/validate src/

CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 수정 완료: 1개
- 개선 필요: 1개 (일부 수정 실패)

| 디렉토리   | 스키마 | Drift | 확인됨 | 오탐 | 수정됨 | Export 커버리지 | 상태      |
|------------|--------|-------|--------|------|--------|---------------|-----------|
| src/auth   | PASS   | 0     | -      | -    | -      | 95%           | 양호      |
| src/utils  | PASS   | 3     | 2      | 1    | 2      | 78%→85%       | 수정 완료 |
| src/legacy | FAIL(1)| 5     | 4      | 1    | 3      | 45%→60%       | 개선 필요 |

상세 결과
---------

src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (수정 완료)
  스키마: PASS
  Drift: 3개 이슈 → 재검증: 2개 확인, 1개 오탐 → 2개 수정 완료
    - STALE: formatDate → 확인됨 → Exports에서 제거 ✓
    - MISSING: parseNumber → 확인됨 → Exports에 추가 ✓
    - MISSING: _internalHelper → 오탐 (private 헬퍼)
  Export 커버리지: 78%→85%

src/legacy (개선 필요)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Behavior → fix-schema로 수정 ✓
  Drift: 5개 이슈 → 재검증: 4개 확인, 1개 오탐 → 3개 수정 완료
    - UNCOVERED: 3개 파일 → 2개 확인, 1개 오탐 → 2개 Structure에 추가 ✓
    - MISMATCH: 2개 시그니처 → 모두 확인 → 1개 수정 ✓, 1개 수정 실패 ✗
  Export 커버리지: 45%→60%
```

## DO / DON'T

**DO:**
- Run validator/issue-verifier/issue-fixer agent tasks in batches of max 3 parallel tasks
- Append each batch result to `${TMP_DIR}validate-progress.jsonl` before proceeding to next batch
- Run schema validation via CLI before drift validation
- Report both schema, drift issues and export coverage metrics
- Check CLAUDE.md schema validity
- Use file-based progress accumulation for compact resilience
- Skip issue-verifier/issue-fixer for directories with 0 issues
- Run issue-verifier before issue-fixer (verify first, then fix)

**DON'T:**
- Ask user questions (validate runs non-interactively)
- Skip any drift category
- Launch all agent tasks in a single message (use batches of max 3)
- Run issue-fixer without issue-verifier (always verify before fixing)
- Modify files other than CLAUDE.md during fix phase (source code is out of scope)

## 참조 자료

- `references/validator-templates.md`: Drift 유형, Export 패턴, Result Template (validator agent가 런타임에 `cat`으로 로드)

## 관련 컴포넌트

- `agents/validator.md`: 코드-문서 일치 검증 및 Export 커버리지 (drift 검증만 담당)
- `agents/issue-verifier.md`: 검증 이슈 재검증 (false positive 필터링)
- `agents/issue-fixer.md`: 확인된 이슈 기반 CLAUDE.md 수정

## Examples

<example>
<user_request>/validate</user_request>
<assistant_response>
CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 수정 완료: 1개
- 개선 필요: 1개

| 디렉토리   | 스키마 | Drift | 확인됨 | 오탐 | 수정됨 | Export 커버리지 | 상태      |
|------------|--------|-------|--------|------|--------|---------------|-----------|
| src/auth   | PASS   | 0     | -      | -    | -      | 95%           | 양호      |
| src/utils  | PASS   | 2     | 2      | 0    | 2      | 78%→90%       | 수정 완료 |
| src/legacy | FAIL(1)| 5     | 4      | 1    | 3      | 45%→60%       | 개선 필요 |
</assistant_response>
</example>

<example>
<user_request>/validate src/</user_request>
<assistant_response>
CLAUDE.md 검증 보고서
=====================

상세 결과
---------
src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)
</assistant_response>
</example>
