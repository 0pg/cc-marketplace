---
name: migrate
description: |
  claude-md-plugin 버전 업그레이드 시 기존 CLAUDE.md 파일을 새 스키마에 맞게 마이그레이션합니다.
  CLI(claude-md-core migrate)로 스키마 수정 + validate-schema 검증 후, /validate → /compile 연계.
argument-hint: "[project_root_path]"
allowed-tools: [Bash, Read, Glob, Grep, Skill, AskUserQuestion]
---

# /migrate

기존 프로젝트의 CLAUDE.md 파일을 현재 플러그인 버전의 스키마에 맞게 마이그레이션합니다.

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
```

CLI 빌드 실패 시 에러 출력 후 종료.

프로젝트 내 CLAUDE.md 파일 수집:
```
Glob("**/CLAUDE.md", path={project_root_path})
```

CLAUDE.md가 없으면:
> "CLAUDE.md 파일이 없습니다. 마이그레이션할 대상이 없습니다."

### 2. 현재 스키마 상태 진단

각 CLAUDE.md에 대해 스키마 검증을 실행하여 현재 상태를 파악합니다:

```bash
for claude_md in ${targets}; do
  $CLI_PATH validate-schema --file "$claude_md" 2>&1
done
```

결과를 집계합니다:
- PASS: 이미 현재 스키마 호환
- FAIL: 마이그레이션 필요 (누락 섹션 목록 추출)

**전체 PASS인 경우:**
> "모든 CLAUDE.md가 현재 스키마와 호환됩니다. 마이그레이션이 필요 없습니다."
> 종료.

### 3. 마이그레이션 계획 표시

마이그레이션이 필요한 파일과 변경 내용을 표시합니다:

```
마이그레이션 계획
================

대상: {N}개 CLAUDE.md (전체 {total}개 중)

| 파일 | 누락 섹션 |
|------|----------|
| src/auth/CLAUDE.md | Async Contract, Error Taxonomy, Concurrency Model |
| src/utils/CLAUDE.md | Async Contract, Concurrency Model |

변경 내용:
- 누락된 필수 섹션에 "None" 마커 자동 추가
- 기존 내용은 변경하지 않음
```

AskUserQuestion으로 확인:
```
AskUserQuestion: "위 마이그레이션을 진행하시겠습니까?"
옵션: [진행, 취소]
```

### 4. 스키마 마이그레이션 (CLI)

각 FAIL 파일에 대해 `fix-schema`를 실행합니다:

```bash
for claude_md in ${failed_targets}; do
  $CLI_PATH fix-schema --file "$claude_md"
done
```

### 5. 스키마 재검증

마이그레이션 후 전체 재검증:

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

**전체 PASS:**
> "스키마 마이그레이션 완료. {N}개 파일 수정됨."

**일부 FAIL:**
> "⚠ {fail_count}개 파일이 여전히 검증 실패합니다. 수동 확인이 필요합니다."
> 실패 파일 목록 표시 후 계속 여부 AskUserQuestion.

### 6. 변경사항 Diff 표시

```bash
git diff -- "**/CLAUDE.md"
```

### 7. 계약 검증 (선택)

AskUserQuestion:
```
AskUserQuestion: "계약-코드 일치 검증(/validate)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:validate")
```

### 8. 코드 재생성 (선택)

/validate에서 위반이 발견된 경우에만 질문:

```
AskUserQuestion: "위반이 발견되었습니다. 전체 재컴파일(/compile --all)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:compile", args: "--all --conflict overwrite")
```

### 9. 결과 보고

```
마이그레이션 결과
================

스키마 수정: {fixed_count}/{total}개 PASS
계약 검증: {실행됨/건너뜀} {결과}
코드 재생성: {실행됨/건너뜀} {결과}

마이그레이션 완료.
```

## DO / DON'T

**DO:**
- 마이그레이션 전 계획 표시 + 사용자 승인
- fix-schema CLI로 결정론적 섹션 추가
- 각 단계 결과를 명확히 표시
- /validate, /compile은 선택적 실행

**DON'T:**
- 사용자 승인 없이 파일 수정
- 기존 섹션 내용 변경 (누락 섹션 추가만)
- DEVELOPERS.md 수정 (CLAUDE.md 스키마만 대상)

## Examples

<example>
<user_request>/migrate</user_request>
<assistant_response>
CLI 빌드 확인... OK
CLAUDE.md 파일 5개 발견

현재 스키마 상태:
  PASS: 2개
  FAIL: 3개

마이그레이션 계획
================
대상: 3개 CLAUDE.md

| 파일 | 누락 섹션 |
|------|----------|
| src/auth/CLAUDE.md | Async Contract, Error Taxonomy, Concurrency Model |
| src/api/CLAUDE.md | Async Contract, Error Taxonomy, Concurrency Model |
| src/utils/CLAUDE.md | Error Taxonomy |

진행하시겠습니까? [진행/취소]
→ 진행

스키마 마이그레이션 실행 중...
  ✓ src/auth/CLAUDE.md — 3개 섹션 추가
  ✓ src/api/CLAUDE.md — 3개 섹션 추가
  ✓ src/utils/CLAUDE.md — 1개 섹션 추가

스키마 재검증: 5/5 PASS

계약 검증을 실행하시겠습니까? [실행/건너뛰기]
→ 실행

(... /validate 실행 ...)

마이그레이션 결과
================
스키마 수정: 3개 파일
계약 검증: 실행됨 — 위반 0개
코드 재생성: 건너뜀

마이그레이션 완료.
</assistant_response>
</example>
