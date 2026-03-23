---
name: validate
version: 2.0.0
aliases: [check, verify, lint]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "check export coverage", "lint documentation", or uses "/validate". Runs validator agent for comprehensive validation, then verifies confirmed issues via multi-agent pipeline and reports contract violations.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, 문서 린트, export 커버리지, 계약 위반
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task]
---

> **DEPRECATED (v6.0.0)**: This skill depends on CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in a future version.

# /validate

CLAUDE.md 계약(Contract)과 코드 간의 일치 여부를 검증하고 위반을 보고.

**Code-First + Spec-as-Contract 모델**: CLAUDE.md는 코드가 만족해야 할 계약입니다.
코드가 계약과 다르면 **코드가 수정되어야** 하며, 계약(CLAUDE.md) 자동 수정은 하지 않습니다.

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

**각 CLAUDE.md에 대해 CLI 실행 (`--strict` 모드로 DEVELOPERS.md도 함께 검증):**
```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")

for claude_md in ${targets}; do
  dir_safe=$(echo "$claude_md" | sed 's/\//-/g' | sed 's/\.//g')
  $CLI_PATH validate-schema \
    --file "$claude_md" --strict \
    --output "${TMP_DIR}schema-${dir_safe}.json"
done
```

> **`--strict` 모드**: CLAUDE.md 스키마 검증과 함께 DEVELOPERS.md 존재(INV-3) 및 스키마 검증을 수행합니다.

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

### 6. 위반 보고 (violation-reporter)

재검증에서 CONFIRMED된 이슈가 있는 디렉토리를 대상으로, violation-reporter agent를 통해 계약 위반 보고서를 생성합니다.

**Contract 모델 원칙:** CLAUDE.md(계약)를 코드에 맞게 수정하지 않습니다. 대신 코드가 계약을 위반하고 있음을 보고하고, 수정 방향을 추천합니다.

**보고 대상 선별:**
- Step 5에서 `confirmed_issues > 0`인 디렉토리만 추출

**배치 처리 규칙:**
- **최대 3개씩 배치 처리**
- 각 배치 내의 violation-reporter agent Task를 **단일 메시지에서 병렬로 호출**

**각 배치에서 violation-reporter agent 호출:**
```
Task(violation-reporter):
  보고 대상: {directory}
  재검증 결과 파일: ${TMP_DIR}verified-{dir-safe-name}.md
  CLAUDE.md: {directory}/CLAUDE.md
```

**결과 수집:**

violation-reporter agent는 구조화된 블록으로 결과를 반환:
```
---violation-reporter-result---
status: success | failed
result_file: ${TMP_DIR}violations-{dir-safe-name}.md
directory: {directory}
violation_count: {N}
critical: {N}
high: {N}
medium: {N}
low: {N}
---end-violation-reporter-result---
```

**진행 파일 업데이트:**
```bash
printf '{"directory":"%s","phase":"report","violation_count":%d,"critical":%d,"high":%d,"medium":%d,"low":%d,"result_file":"%s"}\n' \
  "$directory" "$violations" "$critical" "$high" "$medium" "$low" "${TMP_DIR}violations-${dir_safe}.md" \
  >> "${TMP_DIR}validate-progress.jsonl"
```

**스킵 조건:** CONFIRMED 이슈가 0개인 디렉토리는 보고를 스킵합니다.

### 7. 통합 보고서 생성

`${TMP_DIR}validate-progress.jsonl`을 Read하여 스키마 검증, Drift 검증, 재검증, 위반 보고 결과를 병합한 통합 보고서를 생성합니다.

**JSONL 파싱 방법:** 같은 파일에 phase별 라인이 혼재합니다. `directory` 필드 기준으로 그룹화:
- `phase` 필드 없음 → validate 결과 (`issues_count`, `export_coverage`)
- `"phase":"verify"` → verifier 결과 (`confirmed_issues`, `false_positives`)
- `"phase":"report"` → reporter 결과 (`violation_count`, `critical`, `high`, `medium`, `low`)

이슈가 없어 verify/report 단계를 스킵한 디렉토리는 phase 라인이 없으므로 `-` 로 표시.

**보고서 형식:**
```markdown
# CLAUDE.md 계약 검증 보고서

## 요약

| 디렉토리 | 스키마 | 위반 수 | 심각도 | Export 커버리지 | 상태 |
|----------|--------|---------|--------|---------------|------|
| src/auth | PASS | 0 | - | 95% | 양호 |
| src/utils | PASS | 2 | HIGH:1 MED:1 | 78% | 위반 발견 |
| src/legacy | FAIL (1) | 4 | CRIT:1 HIGH:2 MED:1 | 45% | 위반 발견 |

## 추천 액션

위반이 발견된 디렉토리별 추천 액션:
- `src/utils`: `/compile --path src/utils --conflict overwrite` (계약 기반 코드 재생성)
- `src/legacy`: 위반 보고서 검토 후 `/compile` 또는 수동 수정

## 상세 결과

### src/auth

#### 스키마 검증
- PASS

#### 계약 위반
- 없음

### src/utils

#### 스키마 검증
- PASS

#### 계약 위반 (2건)
- **HIGH** Exports STALE: `formatDate` — 계약에 정의되어 있으나 코드에 없음
  - 추천: `/compile` 재실행
- **MEDIUM** Structure UNCOVERED: `helper.ts` — 코드에 존재하나 계약에 미등록
  - 추천: 계약 Structure 섹션 업데이트 필요

### src/legacy

#### 스키마 검증
- FAIL (1): Missing required section: Behavior

#### 계약 위반 (4건)
- **CRITICAL** Exports MISMATCH: `validateToken` 시그니처 불일치
  - 추천: 코드를 계약에 맞게 수정 또는 계약 업데이트 (사용자 결정)
- ...
```

**중요:** context에 남아있는 결과가 아닌, 파일에 누적된 결과를 사용합니다.
- `${TMP_DIR}validate-progress.jsonl`: 요약 정보 (모든 phase)
- `${TMP_DIR}schema-*.json`: 스키마 검증 결과
- `${TMP_DIR}validate-*.md`: Drift 검증 상세 결과
- `${TMP_DIR}verified-*.md`: 재검증 상세 결과
- `${TMP_DIR}violations-*.md`: 위반 보고 상세 결과

### 8. 임시 파일 정리

`${TMP_DIR}` 내 임시 파일은 세션별로 격리되어 다른 세션과 충돌하지 않음. 필요 시 `rm -rf .claude/tmp/` 으로 일괄 정리 가능.

## 성공 기준

| 상태 | 조건 |
|------|------|
| **양호** | 스키마 PASS AND 위반 0개 AND Export 커버리지 점수 90% 이상 |
| **위반 발견** | 확인된 위반이 1개 이상 — 추천 액션 참조 |
| **개선 권장** | 스키마 PASS AND Export 커버리지 점수 70-89% AND 위반 없음 |
| **개선 필요** | 스키마 FAIL OR Export 커버리지 점수 70% 미만 |

## 출력 예시

```
/validate src/

CLAUDE.md 계약 검증 보고서
========================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 위반 발견: 2개

| 디렉토리   | 스키마 | 위반 수 | 심각도        | Export 커버리지 | 상태      |
|------------|--------|---------|---------------|---------------|-----------|
| src/auth   | PASS   | 0       | -             | 95%           | 양호      |
| src/utils  | PASS   | 2       | HIGH:1 MED:1  | 78%           | 위반 발견 |
| src/legacy | FAIL(1)| 4       | CRIT:1 HIGH:2 MED:1 | 45%      | 위반 발견 |

추천 액션
---------
- src/utils: `/compile --path src/utils --conflict overwrite`
- src/legacy: 위반 보고서 검토 후 결정
  - CRITICAL 위반 있음 — 시그니처 불일치 수동 확인 필요

상세 결과
---------

src/auth (양호)
  스키마: PASS
  위반: 0개
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (위반 발견)
  스키마: PASS
  위반: 2개 (확인됨 2, 오탐 1 제외)
    - HIGH Exports STALE: formatDate → 계약에 있으나 코드에 없음 → /compile 재실행
    - MEDIUM Structure UNCOVERED: helper.ts → 계약 Structure 업데이트 필요
  Export 커버리지: 78%

src/legacy (위반 발견)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Behavior → fix-schema로 수정 완료
  위반: 4개 (확인됨 4, 오탐 1 제외)
    - CRITICAL Exports MISMATCH: validateToken 시그니처 불일치
    - HIGH Exports STALE: 2개
    - MEDIUM Structure UNCOVERED: 1개
  Export 커버리지: 45%
```

## DO / DON'T

**DO:**
- Run validator/issue-verifier/violation-reporter agent tasks in batches of max 3 parallel tasks
- Append each batch result to `${TMP_DIR}validate-progress.jsonl` before proceeding to next batch
- Run schema validation via CLI before drift validation
- Report violations with severity, actionable recommendations, and /compile vs manual guidance
- Include Convention drift in validation (Convention is part of the contract)
- Check CLAUDE.md schema validity
- Use file-based progress accumulation for compact resilience
- Skip issue-verifier/violation-reporter for directories with 0 issues
- Run issue-verifier before violation-reporter (verify first, then report)

**DON'T:**
- **Modify CLAUDE.md** (contract is read-only during validation)
- Ask user questions (validate runs non-interactively)
- Skip any drift category
- Launch all agent tasks in a single message (use batches of max 3)
- Run violation-reporter without issue-verifier (always verify before reporting)

## 참조 자료

- `references/validator-templates.md`: Drift 유형, Export 패턴, Result Template (validator agent가 런타임에 `cat`으로 로드)

## 관련 컴포넌트

- `agents/validator.md`: 코드-문서 일치 검증 및 Export 커버리지 (drift 검증만 담당)
- `agents/issue-verifier.md`: 검증 이슈 재검증 (false positive 필터링)
- `agents/violation-reporter.md`: 확인된 이슈 기반 계약 위반 보고 (CLAUDE.md 수정 안 함)

## Examples

<example>
<user_request>/validate</user_request>
<assistant_response>
CLAUDE.md 계약 검증 보고서
========================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 위반 발견: 2개

| 디렉토리   | 스키마 | 위반 수 | 심각도        | Export 커버리지 | 상태      |
|------------|--------|---------|---------------|---------------|-----------|
| src/auth   | PASS   | 0       | -             | 95%           | 양호      |
| src/utils  | PASS   | 2       | HIGH:1 MED:1  | 78%           | 위반 발견 |
| src/legacy | FAIL(1)| 4       | CRIT:1 HIGH:2 MED:1 | 45%      | 위반 발견 |

추천 액션
---------
- src/utils: `/compile --path src/utils --conflict overwrite`
- src/legacy: 위반 보고서 검토 후 결정
</assistant_response>
</example>

<example>
<user_request>/validate src/</user_request>
<assistant_response>
CLAUDE.md 계약 검증 보고서
========================

상세 결과
---------
src/auth (양호)
  스키마: PASS
  위반: 0개
  Export 커버리지: 95% (18/19 예측 성공)
</assistant_response>
</example>
