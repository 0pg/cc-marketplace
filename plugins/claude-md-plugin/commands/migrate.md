---
name: migrate
description: |
  claude-md-plugin 버전 업그레이드 시 기존 프로젝트를 새 버전에 맞게 마이그레이션합니다.
  v6→v7 스키마 전환 (Constraints→Requirements, DEVELOPERS.md 4섹션), 레거시 IMPLEMENTS.md 정리,
  CLAUDE.md 스키마 누락 섹션 추가, 불필요한 conditional "None" 섹션 제거, 검증 연계.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Edit, Write, Skill, AskUserQuestion]
---

# /migrate

기존 프로젝트를 현재 플러그인 버전에 맞게 마이그레이션합니다.

네 가지 마이그레이션을 자동 감지하여 처리합니다:
1. **레거시 정리**: IMPLEMENTS.md 삭제 (v2.x → v3.0+)
2. **스키마 업그레이드**: CLAUDE.md 누락 필수 섹션 추가 (v3.x → v4.0+)
3. **조건부 정리**: 불필요한 conditional "None" 섹션 제거 + Decision Log 언어 정규화 (v4.x → v5.0+)
4. **v6→v7 전환**: CLAUDE.md Constraints→Requirements, DEVELOPERS.md 4섹션 스키마, .claude/index.md 정리 (v6.x → v7.0+)

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
Glob("**/DEVELOPERS.md", path={project_root_path})
```

CLAUDE.md가 없으면:
> "CLAUDE.md 파일이 없습니다. 마이그레이션할 대상이 없습니다."

### 2. 마이그레이션 유형 자동 감지

| 감지 조건 | 마이그레이션 유형 | 설명 |
|-----------|-----------------|------|
| IMPLEMENTS.md 존재 | **LEGACY_CLEANUP** | 폐기된 IMPLEMENTS.md 삭제 |
| CLAUDE.md에 `## Constraints` 섹션 존재 | **V6_TO_V7** | v6→v7 스키마 전환 |
| CLAUDE.md 스키마 검증 FAIL (v7 기준) | **SCHEMA_UPGRADE** | 누락 필수 섹션 추가 |
| Protocol/Async Contract/Concurrency Model이 "None"이고 해당 패턴 미감지 | **CONDITIONAL_CLEANUP** | 불필요한 conditional "None" 섹션 제거 |
| Decision Log에 Korean 필드명(맥락/결정/근거) 사용 | **CONDITIONAL_CLEANUP** | 언어 정규화 |
| 위 해당 없음 | **UP_TO_DATE** | 마이그레이션 불필요 |

**V6_TO_V7 감지 로직:**

```bash
for claude_md in ${targets}; do
  content=$(cat "$claude_md")
  if echo "$content" | grep -q "^## Constraints"; then
    # v6 문서 — Constraints 섹션은 v7에서 Requirements로 변경됨
    v6_targets+=("$claude_md")
  fi
done
```

스키마 검증 (V6_TO_V7 이후에 실행):
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

  ⓘ IMPLEMENTS.md는 v3.0에서 폐기되었습니다.
    DEVELOPERS.md가 필요하면 마이그레이션 후 /decompile을 실행하세요.

[2] v6→v7 전환: {M}개 CLAUDE.md + {K}개 DEVELOPERS.md
| 파일 | 작업 |
|------|------|
| src/auth/CLAUDE.md | ## Constraints → ## Requirements |
| src/auth/DEVELOPERS.md | ## Domain Context → ## Technical Context, ## Invariants → ## Constraints, ## File Map 삭제 |

  ⓘ v7에서 CLAUDE.md는 Primary SSOT (PM 요구사항), DEVELOPERS.md는 Derived Spec (개발자 명세)입니다.

[3] 스키마 업그레이드: {P}개 파일
...

[4] 조건부 정리: {Q}개 파일
...
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

### 5. v6→v7 전환

V6_TO_V7가 감지된 경우 실행합니다.

#### 5.1. CLAUDE.md 결정적 변환

각 v6 CLAUDE.md에 대해:

```bash
# ## Constraints → ## Requirements 리네임
Edit: old_string="## Constraints", new_string="## Requirements"
```

#### 5.2. DEVELOPERS.md 결정적 변환

각 v6 DEVELOPERS.md에 대해:

1. `## Domain Context` → `## Technical Context` 리네임
2. `## Invariants` → `## Constraints` 리네임 (내용 이동)
3. `## File Map` 섹션 삭제

```bash
Edit: old_string="## Domain Context", new_string="## Technical Context"
Edit: old_string="## Invariants", new_string="## Constraints"
# ## File Map 섹션 전체 삭제 (헤더부터 다음 ## 또는 EOF까지)
```

#### 5.3. .claude/index.md 정리

```bash
# auto-generated 파일이므로 안전하게 삭제
for dir in ${project_dirs}; do
  if [ -f "$dir/.claude/index.md" ]; then
    rm "$dir/.claude/index.md"
  fi
done
```

#### 5.4. compile-context.md 정리

```bash
# ephemeral session file이므로 안전하게 삭제
for dir in ${project_dirs}; do
  if [ -f "$dir/compile-context.md" ]; then
    rm "$dir/compile-context.md"
  fi
done
```

#### 5.5. LLM 보조 분류 (선택적)

결정적 변환 후, 사용자에게 LLM 보조 분류를 제안합니다:

```
AskUserQuestion: "LLM 보조 분류를 실행하시겠습니까? (요구사항 수준 분류)"
옵션:
  - "실행": Requirements 내용을 PM-level과 developer-level로 분류
  - "건너뛰기": 결정적 변환만으로 완료
```

"실행" 선택 시:
1. CLAUDE.md Requirements에서 developer-level 항목(정확한 수치, I/O 계약 등)을 식별
2. 해당 항목을 DEVELOPERS.md Constraints로 이동 제안
3. CLAUDE.md Domain Context에서 기술 선택 항목을 Technical Context로 이동 제안
4. 각 이동에 대해 AskUserQuestion으로 확인

#### 5.6. 스키마 검증

```bash
for claude_md in ${v6_targets}; do
  $CLI_PATH validate-schema --file "$claude_md"
done
```

### 6. 스키마 업그레이드 (CLAUDE.md 섹션 추가)

SCHEMA_UPGRADE가 감지된 경우 실행합니다.

```bash
for claude_md in ${failed_targets}; do
  $CLI_PATH fix-schema --file "$claude_md"
done
```

### 6.5. 조건부 정리 (CONDITIONAL_CLEANUP)

CONDITIONAL_CLEANUP이 감지된 경우 실행합니다.

**6.5a. 불필요한 conditional "None" 섹션 제거:**

Protocol, Async Contract, Concurrency Model 섹션이 "None"이고 코드에 해당 패턴이 없는 경우 섹션을 제거합니다:

```bash
for claude_md in ${conditional_cleanup_targets}; do
  dir=$(dirname "$claude_md")
  analysis=$($CLI_PATH analyze-code --dir "$dir" --format json 2>&1)

  has_stateful=$(echo "$analysis" | grep -o '"has_stateful_patterns":[a-z]*' | cut -d: -f2)
  has_async=$(echo "$analysis" | grep -o '"has_async_patterns":[a-z]*' | cut -d: -f2)
  has_concurrency=$(echo "$analysis" | grep -o '"has_concurrency_patterns":[a-z]*' | cut -d: -f2)
done
```

각 conditional 섹션에 대해:
- 패턴 미감지 + 섹션 내용이 "None" → `Edit`으로 섹션 제거
- 패턴 감지 또는 내용이 "None"이 아님 → 유지

**6.5b. Decision Log 언어 정규화:**

DEVELOPERS.md의 Decision Log에서 Korean 필드명을 감지하고 alias 지원을 확인합니다:
- `맥락` → `Context|맥락` (양쪽 허용)
- `결정` → `Decision|결정` (양쪽 허용)
- `근거` → `Rationale|근거` (양쪽 허용)

### 7. 재검증 + Diff 표시

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
git diff -- "**/CLAUDE.md" "**/DEVELOPERS.md"
git status -- "**/IMPLEMENTS.md"
```

**전체 PASS:**
> "스키마 검증: {total}/{total} PASS"

**일부 FAIL:**
> "⚠ {fail_count}개 파일이 여전히 검증 실패합니다. 수동 확인이 필요합니다."

### 7.5. Conventions 부재 감지

v7에서 project/module root에 `## Conventions` (6개 필수 서브섹션)가 필수입니다.
마이그레이션만으로는 Conventions가 자동 생성되지 않으므로, 부재를 감지하고 안내합니다.

**7.5a. project root 검증:**

```bash
$CLI_PATH validate-convention --project-root {project_root_path}
```

실패 시 (Conventions 부재 또는 서브섹션 누락):

```
AskUserQuestion: "project root CLAUDE.md에 ## Conventions 섹션이 없거나 불완전합니다.
v7에서 이 섹션은 /compile의 REFACTOR 단계에 필요합니다.

/project-setup을 실행하여 기존 코드에서 컨벤션을 추출하시겠습니까?"
옵션: [실행, 나중에 수동 추가]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:project-setup", args: "{project_root_path}")
```

**7.5b. module root 검증 (선택):**

project root와 다른 module root가 존재하는 경우, 각 module root에서:
```
Grep: pattern="^## Conventions" path={module_root}/CLAUDE.md
```

module root에 override Conventions가 없으면 project root에서 상속하므로 **경고만** 출력:
> "ⓘ {module_root}/CLAUDE.md에 ## Conventions가 없습니다. project root에서 상속됩니다.
> override가 필요하면 /convention-update를 실행하세요."

성공 시 (Conventions 존재 + 6개 서브섹션 완전):
> 별도 출력 없이 다음 단계로 진행.

### 8. 후속 액션 안내

마이그레이션 결과에 따라 다음 단계를 안내합니다:

```
마이그레이션 결과
================

레거시 정리: {deleted}개 IMPLEMENTS.md 삭제
v6→v7 전환: {converted}개 CLAUDE.md + {dev_converted}개 DEVELOPERS.md 전환
스키마 업그레이드: {fixed}개 CLAUDE.md 섹션 추가
조건부 정리: {cleaned}개 파일 정리
스키마 검증: {pass}/{total} PASS

다음 단계:
```

IMPLEMENTS.md가 삭제된 경우:
```
  - DEVELOPERS.md 생성이 필요하면: /decompile
```

v6→v7 전환이 수행된 경우:
```
  - 품질 검증: /impl-review
  - 계약-코드 일치 확인: /validate
  - 코드 재생성이 필요하면: /compile --all --conflict overwrite
```

Conventions가 없거나 불완전한 경우:
```
  - Conventions 생성: /project-setup
  - Conventions 업데이트: /convention-update
```

### 9. 계약 검증 (선택)

```
AskUserQuestion: "계약-코드 일치 검증(/validate)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:validate")
```

### 10. 코드 재생성 (선택)

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
- v6→v7 결정적 변환은 안전한 리네임 (내용 보존)
- LLM 보조 분류는 선택적 (사용자 승인 필요)
- .claude/index.md, compile-context.md는 auto-generated이므로 안전하게 삭제
- /validate, /compile은 선택적 실행

**DON'T:**
- 사용자 승인 없이 파일 삭제
- IMPLEMENTS.md 내용을 DEVELOPERS.md로 변환 (구조/목적이 다름)
- 기존 CLAUDE.md 섹션 내용 변경 (헤더 리네임만)
- 파일마다 개별 승인 요청 (1회 승인으로 일괄 처리)
- LLM 보조 분류를 강제 (항상 선택적)

## Examples

<example>
<context>
v6에서 v7로 업그레이드하는 프로젝트
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  DEVELOPERS.md: 2개
  IMPLEMENTS.md: 0개

마이그레이션 유형 감지:
  [1] v6→v7 전환: 3개 CLAUDE.md + 2개 DEVELOPERS.md

마이그레이션 계획
================

[1] v6→v7 전환: 3개 CLAUDE.md + 2개 DEVELOPERS.md
| 파일 | 작업 |
|------|------|
| src/auth/CLAUDE.md | ## Constraints → ## Requirements |
| src/api/CLAUDE.md | ## Constraints → ## Requirements |
| src/utils/CLAUDE.md | ## Constraints → ## Requirements |
| src/auth/DEVELOPERS.md | ## Domain Context → ## Technical Context, ## Invariants → ## Constraints, ## File Map 삭제 |
| src/api/DEVELOPERS.md | ## Domain Context → ## Technical Context, ## Invariants → ## Constraints, ## File Map 삭제 |

  + .claude/index.md 삭제 (auto-generated)
  + compile-context.md 삭제 (ephemeral)

진행하시겠습니까? [진행/취소]
→ 진행

=== v6→v7 전환 ===
  ✓ src/auth/CLAUDE.md — Constraints → Requirements
  ✓ src/api/CLAUDE.md — Constraints → Requirements
  ✓ src/utils/CLAUDE.md — Constraints → Requirements
  ✓ src/auth/DEVELOPERS.md — 3개 섹션 전환
  ✓ src/api/DEVELOPERS.md — 3개 섹션 전환
  ✓ .claude/index.md 삭제

LLM 보조 분류를 실행하시겠습니까? [실행/건너뛰기]
→ 건너뛰기

스키마 검증: 3/3 PASS

마이그레이션 결과
================
v6→v7 전환: 3개 CLAUDE.md + 2개 DEVELOPERS.md
스키마 검증: 3/3 PASS

다음 단계:
  - 품질 검증: /impl-review
  - 계약-코드 일치 확인: /validate

계약 검증을 실행하시겠습니까? [실행/건너뛰기]
→ 건너뛰기

마이그레이션 완료.
</assistant_response>
</example>

<example>
<context>
v2.x에서 v7로 업그레이드하는 프로젝트 (IMPLEMENTS.md 존재 + v6 스키마)
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  DEVELOPERS.md: 0개
  IMPLEMENTS.md: 2개

마이그레이션 유형 감지:
  [1] 레거시 정리: 2개 IMPLEMENTS.md
  [2] v6→v7 전환: 3개 CLAUDE.md

마이그레이션 계획
================

[1] 레거시 정리 (IMPLEMENTS.md 삭제): 2개
  - src/auth/IMPLEMENTS.md
  - src/utils/IMPLEMENTS.md

  ⓘ IMPLEMENTS.md는 v3.0에서 폐기되었습니다.

[2] v6→v7 전환: 3개 CLAUDE.md
  - src/auth/CLAUDE.md — ## Constraints → ## Requirements
  - src/api/CLAUDE.md — ## Constraints → ## Requirements
  - src/utils/CLAUDE.md — ## Constraints → ## Requirements

진행하시겠습니까? [진행/취소]
→ 진행

=== 레거시 정리 ===
  ✓ 2개 IMPLEMENTS.md 삭제

=== v6→v7 전환 ===
  ✓ 3개 CLAUDE.md — Constraints → Requirements

스키마 검증: 3/3 PASS

마이그레이션 완료.
</assistant_response>
</example>

<example>
<context>
이미 v7 최신 버전인 프로젝트
</context>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK

문서 파일 수집:
  CLAUDE.md: 3개
  DEVELOPERS.md: 3개
  IMPLEMENTS.md: 0개

모든 문서가 현재 버전과 호환됩니다. 마이그레이션이 필요 없습니다.
</assistant_response>
</example>
