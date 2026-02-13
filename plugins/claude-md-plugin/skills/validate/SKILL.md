---
name: validate
version: 1.1.0
aliases: [check, verify, lint]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify spec matches implementation", "check for drift", "lint documentation", or uses "/validate". Runs validator agent for comprehensive validation.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, 문서 린트
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task]
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
mkdir -p .claude/tmp
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
    --output ".claude/tmp/schema-${dir_safe}.json"
done
```

**결과 JSON (~500bytes/file):**
```json
{"file": "src/auth/CLAUDE.md", "valid": false, "errors": [{"error_type": "MissingSection", "message": "Missing required section: Behavior", "section": "Behavior"}], "warnings": []}
```

> **참고:** `schema-validate` internal skill은 `.claude/extract-results/`를 사용하지만,
> validate는 세션 임시 결과이므로 `.claude/tmp/`에 저장합니다.
> schema-validate는 decompile/impl 등 다른 workflow에서 영구 결과로 사용되는 반면,
> validate의 스키마 결과는 보고서 생성 후 폐기됩니다.

validate SKILL이 각 JSON을 Read하여 스키마 이슈를 수집합니다.
- `valid: true` → 스키마 통과, drift 검증 진행
- `valid: false` → 스키마 이슈를 기록, drift 검증은 여전히 진행 (스키마 문제와 drift는 독립적)

### 2. 배치 Drift 검증

validator agent를 **최대 3개씩 배치 처리**하여 context 폭발을 방지합니다.

**배치 처리 규칙:**
- 대상 CLAUDE.md 목록을 최대 3개씩 나누어 배치 생성
- 각 배치 내의 validator agent Task를 **단일 메시지에서 병렬로 호출**
- 배치 완료 후 다음 배치 진행

**진행 파일 초기화:**
```bash
: > .claude/tmp/validate-progress.jsonl
```

**각 배치 완료 후, 결과를 `.claude/tmp/validate-progress.jsonl` 파일에 append:**

validator agent의 결과 블록을 파싱하여 아래 형식으로 append합니다:
```bash
echo '{"directory":"src/auth","status":"success","issues_count":0,"export_coverage":95,"result_file":".claude/tmp/validate-src-auth.md"}' >> .claude/tmp/validate-progress.jsonl
echo '{"directory":"src/utils","status":"success","issues_count":2,"export_coverage":72,"result_file":".claude/tmp/validate-src-utils.md"}' >> .claude/tmp/validate-progress.jsonl
```

**compact 대비:**
- compact이 발생해도 `.claude/tmp/validate-progress.jsonl`에 이전 배치 결과가 보존됨
- 최종 보고서 생성 시 context가 아닌 이 파일을 읽어서 생성
- validator agent의 상세 결과도 개별 `.claude/tmp/validate-*.md` 파일에 저장되어 있음
- **compact 후 재개:** `validate-progress.jsonl`을 Read하여 이미 완료된 directory 목록을 확인하고, 나머지 대상만 다음 배치로 처리. 중복 실행 방지를 위해 JSONL의 `directory` 필드와 대상 목록을 대조.

### 3. 결과 수집

validator agent는 구조화된 블록으로 결과를 반환:

```
---validate-result---
status: success | failed
result_file: .claude/tmp/validate-{dir-safe-name}.md
directory: {directory}
issues_count: {N}
export_coverage: {0-100}
---end-validate-result---
```

### 4. 통합 보고서 생성

`.claude/tmp/validate-progress.jsonl`을 Read하여 스키마 검증 결과와 Drift 검증 결과를 병합한 통합 보고서를 생성합니다.

**보고서 형식:**
```markdown
# CLAUDE.md 검증 보고서

## 요약

| 디렉토리 | 스키마 | Drift 이슈 | Export 커버리지 | 상태 |
|----------|--------|-----------|---------------|------|
| src/auth | PASS | 0 | 95% | 양호 |
| src/utils | FAIL (1) | 2 | 72% | 개선 필요 |

## 상세 결과

### src/auth

#### 스키마 검증
- PASS

#### Drift 검증
(validator agent 결과 - drift 이슈 + export 커버리지)

### src/utils

#### 스키마 검증
- [MissingSection] Missing required section: Behavior

#### Drift 검증
- STALE: formatDate export가 코드에 없음
- MISSING: parseNumber export가 문서에 없음
```

**중요:** context에 남아있는 결과가 아닌, 파일에 누적된 결과를 사용합니다.
- `.claude/tmp/validate-progress.jsonl`: 요약 정보 (모든 배치)
- `.claude/tmp/schema-*.json`: 스키마 검증 결과
- `.claude/tmp/validate-*.md`: Drift 검증 상세 결과

### 5. 임시 파일 정리

`.claude/tmp/`의 임시 파일은 세션 종료 시 자동으로 정리됨.

## 성공 기준

| 상태 | 조건 |
|------|------|
| **양호** | 스키마 PASS AND Drift 이슈 0개 AND Export 커버리지 점수 90% 이상 |
| **개선 권장** | 스키마 PASS AND (Drift 1-2개 OR Export 커버리지 점수 70-89%) |
| **개선 필요** | 스키마 FAIL OR Drift 3개 이상 OR Export 커버리지 점수 70% 미만 |

## 출력 예시

```
/validate src/

CLAUDE.md 검증 보고서
=====================

요약
----
검증 대상: 3개 디렉토리
- 양호: 1개
- 개선 권장: 1개
- 개선 필요: 1개

상세 결과
---------

src/auth (양호)
  스키마: PASS
  Drift: 0개 이슈
  Export 커버리지: 95% (18/19 예측 성공)

src/utils (개선 권장)
  스키마: PASS
  Drift: 2개 이슈
    - STALE: formatDate export가 코드에 없음
    - MISSING: parseNumber export가 문서에 없음
  Export 커버리지: 78% (14/18 예측 성공)

src/legacy (개선 필요)
  스키마: FAIL (1)
    - [MissingSection] Missing required section: Behavior
  Drift: 5개 이슈
    - UNCOVERED: 3개 파일이 Structure에 없음
    - MISMATCH: 2개 시그니처 불일치
  Export 커버리지: 45% (9/20 예측 성공)
```

## DO / DON'T

**DO:**
- Run validator agent tasks in batches of max 3 parallel tasks
- Append each batch result to `.claude/tmp/validate-progress.jsonl` before proceeding to next batch
- Run schema validation via CLI before drift validation
- Report both schema, drift issues and export coverage metrics
- Include IMPLEMENTS.md presence check (INV-3)
- Use file-based progress accumulation for compact resilience

**DON'T:**
- Modify any files (validate is read-only, except `.claude/tmp/` for results)
- Ask user questions (validate runs non-interactively)
- Skip any drift category
- Launch all validator agents in a single message (use batches of max 3)

## 참조 자료

- `references/drift-types.md`: 4가지 Drift 유형 상세 설명
- `examples/validation-report.md`: 검증 결과 보고서 예시

## 관련 컴포넌트

- `agents/validator.md`: 코드-문서 일치 검증 및 Export 커버리지 (drift 검증만 담당)
- `skills/schema-validate/SKILL.md`: 스키마 검증 CLI 래핑

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
- 개선 권장: 1개
- 개선 필요: 1개
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
