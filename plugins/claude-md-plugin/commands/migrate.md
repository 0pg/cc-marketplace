---
name: migrate
description: |
  claude-md-plugin 버전 업그레이드 시 기존 프로젝트를 새 버전에 맞게 마이그레이션합니다.
  레거시 IMPLEMENTS.md 정리, CLAUDE.md 스키마 누락 섹션 추가, 검증 연계.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Skill, AskUserQuestion]
---

# /migrate

기존 프로젝트를 현재 플러그인 버전에 맞게 마이그레이션합니다.

세 가지 마이그레이션을 자동 감지하여 처리합니다:
1. **레거시 정리**: IMPLEMENTS.md 삭제 (v2.x → v3.0+)
2. **스키마 업그레이드**: CLAUDE.md 누락 필수 섹션 추가 (v3.x → v4.0+)
3. **조건부 정리**: 불필요한 conditional "None" 섹션 제거 + Decision Log 언어 정규화 (v4.x → v5.0+)

## Triggers

- `/migrate`
- `마이그레이션`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `project_root_path` | 아니오 | `.` | 프로젝트 루트 경로 |

## Workflow

### 1. 사전 확인

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

CLI 빌드 실패 시 에러 출력 후 종료.

프로젝트 내 문서 파일 수집:
```
Glob("**/CLAUDE.md", path={project_root_path})
Glob("**/IMPLEMENTS.md", path={project_root_path})
```

CLAUDE.md가 없으면:
> "CLAUDE.md 파일이 없습니다. 마이그레이션할 대상이 없습니다."

### 2. 마이그레이션 유형 자동 감지

| 감지 조건 | 마이그레이션 유형 | 설명 |
|-----------|-----------------|------|
| IMPLEMENTS.md 존재 | **LEGACY_CLEANUP** | 폐기된 IMPLEMENTS.md 삭제 |
| CLAUDE.md 스키마 검증 FAIL | **SCHEMA_UPGRADE** | 누락 필수 섹션 추가 |
| Protocol/Async Contract/Concurrency Model이 "None"이고 해당 패턴 미감지 | **CONDITIONAL_CLEANUP** | 불필요한 conditional "None" 섹션 제거 |
| Decision Log에 Korean 필드명(맥락/결정/근거) 사용 | **CONDITIONAL_CLEANUP** | 언어 정규화 (Context/Decision/Rationale alias 지원 추가) |
| 위 해당 없음 | **UP_TO_DATE** | 마이그레이션 불필요 |

스키마 검증:
```bash
for claude_md in ${targets}; do
  $CLI_PATH validate-schema --file "$claude_md" 2>&1
done
```

**UP_TO_DATE인 경우:**
> "모든 문서가 현재 버전과 호환됩니다. 마이그레이션이 필요 없습니다."
> 종료.

### 3. 마이그레이션 계획 표시 + 승인

감지된 항목을 표시하고 **1회만** 승인을 요청합니다:

```
마이그레이션 계획
================

[1] 레거시 정리 (IMPLEMENTS.md 삭제): {N}개 파일
| 파일 | 작업 |
|------|------|
| src/auth/IMPLEMENTS.md | 삭제 |
| src/utils/IMPLEMENTS.md | 삭제 |

  ⓘ IMPLEMENTS.md는 v3.0에서 폐기되었습니다.
    DEVELOPERS.md가 필요하면 마이그레이션 후 /decompile을 실행하세요.

[2] 스키마 업그레이드 (누락 섹션 추가): {M}개 파일
| 파일 | 누락 섹션 |
|------|----------|
| src/auth/CLAUDE.md | Async Contract, Error Taxonomy, Concurrency Model |

[3] 조건부 정리 (v4→v5): {K}개 파일
| 파일 | 작업 |
|------|------|
| src/utils/CLAUDE.md | Protocol(None) 제거, Concurrency Model(None) 제거 |
| src/utils/DEVELOPERS.md | Decision Log 필드명 alias 정규화 |
```

```
AskUserQuestion: "위 마이그레이션을 진행하시겠습니까?"
옵션: [진행, 취소]
```

### 4. 레거시 정리 (IMPLEMENTS.md 삭제)

LEGACY_CLEANUP이 감지된 경우 실행합니다.

```bash
for impl_md in ${implements_files}; do
  git rm "$impl_md" 2>/dev/null || rm "$impl_md"
done
```

삭제 결과 출력:
```
  ✓ src/auth/IMPLEMENTS.md 삭제
  ✓ src/utils/IMPLEMENTS.md 삭제
```

### 5. 스키마 업그레이드 (CLAUDE.md 섹션 추가)

SCHEMA_UPGRADE가 감지된 경우 실행합니다.

```bash
for claude_md in ${failed_targets}; do
  $CLI_PATH fix-schema --file "$claude_md"
done
```

### 5.5. 조건부 정리 (CONDITIONAL_CLEANUP)

CONDITIONAL_CLEANUP이 감지된 경우 실행합니다.

**5.5a. 불필요한 conditional "None" 섹션 제거:**

Protocol, Async Contract, Concurrency Model 섹션이 "None"이고 코드에 해당 패턴이 없는 경우 섹션을 제거합니다:

```bash
for claude_md in ${conditional_cleanup_targets}; do
  # 코드 패턴 감지 (analyze-code 또는 --dir 기반)
  # has_stateful_patterns == false && Protocol == "None" → 섹션 제거
  # has_async_patterns == false && Async Contract == "None" → 섹션 제거
  # has_concurrency_patterns == false && Concurrency Model == "None" → 섹션 제거
done
```

**5.5b. Decision Log 언어 정규화:**

DEVELOPERS.md의 Decision Log에서 Korean 필드명을 감지하고 alias 지원을 확인합니다:
- `맥락` → `Context|맥락` (양쪽 허용)
- `결정` → `Decision|결정` (양쪽 허용)
- `근거` → `Rationale|근거` (양쪽 허용)

> 기존 Korean 필드명은 유효한 alias로 인정되므로 강제 변환하지 않습니다.
> 단, 검증 시 양쪽 모두 인식하도록 정규화합니다.

### 6. 재검증 + Diff 표시

```bash
fail_count=0
for claude_md in ${targets}; do
  result=$($CLI_PATH validate-schema --file "$claude_md" --strict 2>&1)
  if echo "$result" | grep -q '"valid":false'; then
    fail_count=$((fail_count + 1))
    echo "FAIL: $claude_md"
  fi
done
```

```bash
git diff -- "**/CLAUDE.md"
git status -- "**/IMPLEMENTS.md"
```

**전체 PASS:**
> "스키마 검증: {total}/{total} PASS"

**일부 FAIL:**
> "⚠ {fail_count}개 파일이 여전히 검증 실패합니다. 수동 확인이 필요합니다."

### 7. 후속 액션 안내

마이그레이션 결과에 따라 다음 단계를 안내합니다:

```
마이그레이션 결과
================

레거시 정리: {deleted}개 IMPLEMENTS.md 삭제
스키마 업그레이드: {fixed}개 CLAUDE.md 섹션 추가
조건부 정리: {cleaned}개 파일 정리 (conditional None 제거 + 언어 정규화)
스키마 검증: {pass}/{total} PASS

다음 단계:
```

IMPLEMENTS.md가 삭제된 경우:
```
  - DEVELOPERS.md 생성이 필요하면: /decompile
```

스키마 업그레이드가 수행된 경우:
```
  - 계약-코드 일치 확인: /validate
  - 코드 재생성이 필요하면: /compile --all --conflict overwrite
```

### 8. 계약 검증 (선택)

```
AskUserQuestion: "계약-코드 일치 검증(/validate)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:validate")
```

### 9. 코드 재생성 (선택)

/validate에서 위반이 발견된 경우에만 질문:

```
AskUserQuestion: "위반이 발견되었습니다. 전체 재컴파일(/compile --all)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:compile", args: "--all --conflict overwrite")
```

## DO / DON'T

**DO:**
- 마이그레이션 유형을 자동 감지
- 계획 표시 후 1회 승인으로 전체 진행
- IMPLEMENTS.md는 삭제만 (DEVELOPERS.md 생성은 /decompile 책임)
- fix-schema CLI로 결정론적 섹션 추가
- /validate, /compile은 선택적 실행

**DON'T:**
- 사용자 승인 없이 파일 삭제
- IMPLEMENTS.md 내용을 DEVELOPERS.md로 변환 (구조/목적이 다름, INV-4 위반)
- 기존 CLAUDE.md 섹션 내용 변경 (누락 섹션 추가만)
- 파일마다 개별 승인 요청 (1회 승인으로 일괄 처리)

## Examples

<example>
<context>
v2.x에서 v4.1로 업그레이드하는 프로젝트 (IMPLEMENTS.md 존재 + 스키마 누락)
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  IMPLEMENTS.md: 2개

마이그레이션 유형 감지:
  [1] 레거시 정리: 2개 IMPLEMENTS.md
  [2] 스키마 업그레이드: 3개 CLAUDE.md

마이그레이션 계획
================

[1] 레거시 정리 (IMPLEMENTS.md 삭제): 2개
  - src/auth/IMPLEMENTS.md
  - src/utils/IMPLEMENTS.md

  ⓘ IMPLEMENTS.md는 v3.0에서 폐기되었습니다.
    DEVELOPERS.md가 필요하면 마이그레이션 후 /decompile을 실행하세요.

[2] 스키마 업그레이드: 3개
  - src/auth/CLAUDE.md — Async Contract, Error Taxonomy, Concurrency Model
  - src/api/CLAUDE.md — Async Contract, Error Taxonomy, Concurrency Model
  - src/utils/CLAUDE.md — Async Contract, Error Taxonomy, Concurrency Model

진행하시겠습니까? [진행/취소]
→ 진행

=== 레거시 정리 ===
  ✓ src/auth/IMPLEMENTS.md 삭제
  ✓ src/utils/IMPLEMENTS.md 삭제

=== 스키마 업그레이드 ===
  ✓ src/auth/CLAUDE.md — 3개 섹션 추가
  ✓ src/api/CLAUDE.md — 3개 섹션 추가
  ✓ src/utils/CLAUDE.md — 3개 섹션 추가

스키마 검증: 3/3 PASS

마이그레이션 결과
================
레거시 정리: 2개 삭제
스키마 업그레이드: 3개 파일
스키마 검증: 3/3 PASS

다음 단계:
  - DEVELOPERS.md 생성: /decompile
  - 계약-코드 일치 확인: /validate

계약 검증을 실행하시겠습니까? [실행/건너뛰기]
→ 건너뛰기

마이그레이션 완료.
</assistant_response>
</example>

<example>
<context>
v3.1에서 v4.1로 업그레이드하는 프로젝트 (스키마 누락만)
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 5개
  IMPLEMENTS.md: 0개

마이그레이션 유형 감지:
  [1] 레거시 정리: 해당 없음
  [2] 스키마 업그레이드: 5개

진행하시겠습니까? [진행/취소]
→ 진행

스키마 업그레이드:
  ✓ 5개 파일 — 각 3개 섹션 추가

스키마 검증: 5/5 PASS

마이그레이션 완료.
</assistant_response>
</example>

<example>
<context>
이미 최신 버전인 프로젝트
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  IMPLEMENTS.md: 0개

모든 문서가 현재 버전과 호환됩니다. 마이그레이션이 필요 없습니다.
</assistant_response>
</example>
