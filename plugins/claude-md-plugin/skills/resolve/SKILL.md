---
name: resolve
description: |
  /validate 결과를 읽고 각 위반에 대해 해소 워크플로우를 실행합니다.
  3가지 선택지: Fix Code (/compile), Fix Contract (/decompile), Acknowledge (편차 인정).
argument-hint: "[path]"
allowed-tools: [Bash, Read, Glob, Grep, Edit, Write, Skill, AskUserQuestion]
---

> **DEPRECATED (v6.0.0)**: This skill depends on CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in a future version.

# /resolve

`/validate` 결과를 기반으로 계약-코드 불일치(drift)를 대화형으로 해소합니다.

## Triggers

- `/resolve`
- `/resolve src/auth`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 (특정 모듈 또는 프로젝트 루트) |

## Workflow

### 1. 사전 확인

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 2. /validate 결과 확인

최근 validate 결과 파일을 탐색합니다:
```
Glob("${TMP_DIR}violation-report-*.md")
```

결과가 없으면:
> "최근 /validate 결과가 없습니다. 먼저 /validate를 실행해주세요."
> 종료.

결과가 있으면 violation report를 읽고 위반 목록을 파싱합니다.

### 3. 위반별 해소

각 위반(violation)에 대해 AskUserQuestion으로 3가지 선택지를 제시합니다:

```
AskUserQuestion:
  "{module_path}: {violation_summary}"
  옵션:
    - "Fix Code": 코드를 계약에 맞게 수정 (/compile)
    - "Fix Contract": 계약을 코드에 맞게 수정 (/decompile)
    - "Acknowledge": 의도적 편차로 인정
```

#### "Fix Code" 선택 시
```
Skill("claude-md-plugin:compile", args: "--path {module_path}")
```

#### "Fix Contract" 선택 시
```
AskUserQuestion: "계약(CLAUDE.md)을 현재 코드에 맞게 업데이트합니다. 진행할까요?"
옵션: [진행, 취소]
```

"진행" 선택 시:
```
Skill("claude-md-plugin:decompile", args: "{module_path}")
```

#### "Acknowledge" 선택 시

CLAUDE.md에 `## Acknowledged Deviations` 섹션을 추가 (또는 기존 섹션에 항목 추가):

```markdown
## Acknowledged Deviations

- **{violation_id}**: {violation_summary}
  - Reason: {user가 입력한 사유}
```

사유를 AskUserQuestion으로 수집:
```
AskUserQuestion: "이 편차를 인정하는 사유를 입력해주세요."
```

### 4. 결과 요약

```
Resolve 결과
============

| 모듈 | 위반 | 해소 방법 |
|------|------|----------|
| src/auth | Export mismatch | Fix Code |
| src/utils | Missing behavior | Acknowledge |

총 위반: {total}
  - Fix Code: {n}
  - Fix Contract: {n}
  - Acknowledged: {n}
```

### 5. 재검증 (선택)

```
AskUserQuestion: "재검증(/validate)을 실행하시겠습니까?"
옵션: [실행, 건너뛰기]
```

"실행" 선택 시:
```
Skill("claude-md-plugin:validate")
```

## DO / DON'T

**DO:**
- 각 위반에 대해 3가지 선택지 제공
- Fix Contract 전 사용자 확인 (계약 변경은 의도적이어야 함)
- Acknowledged Deviations에 사유 기록 (날짜는 git blame으로 추적)
- 해소 후 재검증 제안

**DON'T:**
- 사용자 승인 없이 계약 수정
- Fix Code와 Fix Contract를 동시에 실행
- Acknowledge 사유 없이 편차 인정

## Examples

<example>
<context>
/validate에서 2개 위반이 발견된 상태
</context>
<user_request>/resolve</user_request>
<assistant_response>
최근 validate 결과를 확인합니다...

2개 위반 발견:

[1/2] src/auth: Export `validateToken` 시그니처 불일치
  - CLAUDE.md: `validateToken(token: string): Claims`
  - Code: `validateToken(token: string, options?: ValidateOptions): Claims`

해소 방법을 선택해주세요: [Fix Code / Fix Contract / Acknowledge]
→ Fix Contract

계약(CLAUDE.md)을 현재 코드에 맞게 업데이트합니다. 진행할까요? [진행/취소]
→ 진행

/decompile 실행 중... 완료.

[2/2] src/utils: Behavior 시나리오 누락 (에러 케이스)

해소 방법을 선택해주세요: [Fix Code / Fix Contract / Acknowledge]
→ Acknowledge

이 편차를 인정하는 사유를 입력해주세요.
→ 에러 케이스는 다음 스프린트에서 추가 예정

CLAUDE.md에 Acknowledged Deviations 추가.

Resolve 결과
============

| 모듈 | 위반 | 해소 방법 |
|------|------|----------|
| src/auth | Export 시그니처 불일치 | Fix Contract |
| src/utils | Behavior 누락 | Acknowledge |

총 위반: 2
  - Fix Contract: 1
  - Acknowledged: 1

재검증을 실행하시겠습니까? [실행/건너뛰기]
→ 건너뛰기
</assistant_response>
</example>
