---
name: impl-review
version: 1.0.0
aliases: [review-impl, impl-quality, rate-impl]
description: |
  This skill should be used when the user asks to "review CLAUDE.md quality", "check impl result",
  "review spec", "check requirements coverage", or uses "/impl-review".
  Reviews CLAUDE.md + IMPLEMENTS.md quality across 4 dimensions: requirements coverage,
  document quality, planning quality, and cross-document consistency.
  Unlike /validate (code-document drift detection), this reviews document quality itself.
  Trigger keywords: 리뷰, 스펙 리뷰, impl 리뷰, 구현 계획 검토
user_invocable: true
allowed-tools: [Bash, Read, Write, Task, AskUserQuestion]
---

# /impl-review

/impl 결과물(CLAUDE.md + IMPLEMENTS.md)의 품질을 요구사항 커버리지와 계획 품질 관점에서 리뷰.
4차원 분석(D1-D4)과 점수 산출, 대화형 수정 제안을 수행.

## Triggers

- `/impl-review`
- `리뷰`, `스펙 리뷰`
- `impl 리뷰`, `구현 계획 검토`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 리뷰 대상 경로 (CLAUDE.md가 있는 디렉토리) |

## Calling Modes

### Mode A: Standalone

사용자가 직접 `/impl-review [path]`로 호출.

- path에서 CLAUDE.md + IMPLEMENTS.md 존재 확인
- AskUserQuestion으로 원본 요구사항 텍스트 수집 (선택)
- 요구사항 없으면 D1(요구사항 커버리지) 차원 스킵

### Mode B: Integrated

`/impl` SKILL.md에서 자동 호출.

- impl agent 결과의 `claude_md_file`, `implements_md_file` + 원본 `user_requirement` 전달
- 스키마 검증 스킵 (impl agent가 이미 검증 완료)

## Workflow

### 1. 인자 파싱 & 대상 해석

**Mode A (Standalone):**

path 인자에서 CLAUDE.md 위치 해석:
- path가 CLAUDE.md 파일이면 그대로 사용
- path가 디렉토리면 `{path}/CLAUDE.md` 사용
- path가 `.`이면 현재 디렉토리의 CLAUDE.md

```bash
# CLAUDE.md 존재 확인
if [ ! -f "{claude_md_path}" ]; then
  echo "ERROR: CLAUDE.md not found at {claude_md_path}"
  exit 1
fi
```

IMPLEMENTS.md: `{directory}/IMPLEMENTS.md` 존재 시 사용, 없으면 "N/A".

원본 요구사항 수집:
```
AskUserQuestion: "원본 요구사항이 있나요? (요구사항 커버리지 분석에 사용됩니다)"
옵션:
  - "요구사항 입력": 사용자가 텍스트로 입력
  - "건너뛰기": D1 분석 스킵
```

"건너뛰기" 선택 시 `user_requirement = "N/A"`.

**Mode B (Integrated):**

호출자가 전달한 값을 그대로 사용:
- `claude_md_path`, `implements_md_path`, `user_requirement` 직접 수신
- `schema_result` = "N/A" (impl agent가 이미 검증 완료)

### 2. 스키마 사전 검증 (CLI)

Mode A에서만 실행 (Mode B는 스킵).

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi

TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"

$CLI_PATH validate-schema --file {claude_md_path} 2>&1
```

실패해도 리뷰 진행 (finding으로 기록).

### 3. Task(impl-reviewer) 호출

```
Task(impl-reviewer):
  CLAUDE.md: {claude_md_path}
  IMPLEMENTS.md: {implements_md_path}
  원본 요구사항: {user_requirement | "N/A"}
  스키마 검증 결과: {schema_result 요약}
  결과 저장: ${TMP_DIR}impl-review-{dir-safe-name}.md
```

description은 "Review CLAUDE.md + IMPLEMENTS.md quality"입니다.

### 4. 최종 결과 보고

impl-reviewer agent의 결과 블록을 파싱하여 최종 보고:

```
=== /impl-review 완료 ===

리뷰 대상:
  - CLAUDE.md: {claude_md_path}
  - IMPLEMENTS.md: {implements_md_path}
  - 요구사항: {provided / N/A}

결과:
  - 전체 점수: {overall_score}/100 ({grade})
  - 이슈: {issues_count}개 (수정 적용: {fixes_applied}개)

상세 결과: {result_file}

---impl-review-result---
result_file: {result_file}
status: {status}
directory: {directory}
overall_score: {overall_score}
grade: {grade}
issues_count: {issues_count}
fixes_applied: {fixes_applied}
---end-impl-review-result---
```

## 참조 자료

- `references/impl-reviewer-templates.md`: 리뷰 차원(D1~D4), 점수 산출 공식, finding/fix 형식, 결과 템플릿, 품질 anti-pattern (impl-reviewer agent가 런타임에 로드)

## 관련 컴포넌트

- `agents/impl-reviewer.md`: 4차원 품질 리뷰 및 대화형 수정 제안 agent

## DO / DON'T

**DO:**
- Validate schema before review (Mode A)
- Collect original requirements when available
- Skip D1 when requirements are N/A
- Skip D3 when IMPLEMENTS.md is N/A
- Propose fixes interactively (only CRITICAL/WARNING)
- Apply fixes only after user approval

**DON'T:**
- Read source code files (review is document-only)
- Auto-apply fixes without user consent
- Skip D2/D4 (always applicable)
- Modify source code (use /compile after fixes)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | 에러 메시지 출력 후 종료 |
| IMPLEMENTS.md 없음 | D3 스킵, 나머지 차원만 리뷰 |
| CLI 빌드 실패 | 경고 출력, 스키마 검증 결과 "N/A"로 진행 |
| impl-reviewer agent 실패 | status: failed 보고 |

## Examples

<example>
<context>
사용자가 기존 CLAUDE.md를 리뷰하려고 합니다.
</context>
<user_request>/impl-review src/auth</user_request>
<assistant_response>
src/auth의 CLAUDE.md + IMPLEMENTS.md를 리뷰합니다...

=== /impl-review 완료 ===

리뷰 대상:
  - CLAUDE.md: src/auth/CLAUDE.md
  - IMPLEMENTS.md: src/auth/IMPLEMENTS.md
  - 요구사항: N/A

결과:
  - 전체 점수: 85/100 (Good)
  - 이슈: 3개 (수정 적용: 1개)

상세 결과: .claude/tmp/impl-review-src-auth.md
</assistant_response>
</example>
