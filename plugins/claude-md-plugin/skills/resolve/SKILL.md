---
name: resolve
version: 2.0.0
user_invocable: true
description: |
  This skill should be used when the user asks to "resolve drift", "fix validation issues",
  "handle validation results", or uses "/resolve".
  Reads /validate results and interactively resolves each drift issue.
  Trigger keywords: drift 해소, 위반 해소, validate 결과 처리
argument-hint: "[path]"
allowed-tools: [Bash, Read, Glob, Grep, Edit, Write, Skill, AskUserQuestion]
---

# /resolve

`/validate` 결과를 기반으로 문서-코드 불일치(drift)를 대화형으로 해소합니다.

## Triggers

- `/resolve`
- `drift 해소`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 (특정 모듈 또는 프로젝트 루트) |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. /validate 결과 확인

최근 validate 결과 파일을 탐색합니다:
```
Glob("${TMP_DIR}validate-*.md")
```

결과가 없으면:
> "최근 /validate 결과가 없습니다. 먼저 `/validate`를 실행해주세요."
> 종료.

결과가 있으면 validate report를 읽고 drift 이슈 목록을 파싱합니다.

### 2. 이슈별 대화형 해소

각 drift 이슈에 대해 AskUserQuestion으로 해소 옵션을 제시합니다:

| Drift 유형 | 해소 옵션 |
|------------|----------|
| **Constraints VIOLATED** | Fix Code (`/compile --conflict overwrite`), Fix Doc (CLAUDE.md 수정), Skip |
| **Constraints STALE** | Remove (제약 삭제), Keep (유지), Update (갱신) |
| **Domain Context STALE** | Update (갱신), Keep (유지) |
| **Convention 위반** | Fix Code (코드 수정), Update Convention (규칙 수정) |
| **DEVELOPERS.md MISSING** | Generate (`/decompile`), Skip |
| **Boundary 위반** | Fix reference (참조 수정) |

```
AskUserQuestion:
  "{module_path}: {drift_summary}"
  옵션: [위 해소 옵션 중 해당하는 것]
```

### 3. 선택에 따른 실행

#### Fix Code 선택 시
```
Skill("claude-md-plugin:compile", args: "--path {module_path} --conflict overwrite")
```

#### Fix Doc 선택 시
```
AskUserQuestion: "CLAUDE.md를 현재 코드에 맞게 업데이트합니다. 진행할까요?"
옵션: [진행, 취소]
```

"진행" 선택 시:
- Constraints VIOLATED → CLAUDE.md의 해당 Constraint를 Edit으로 수정
- Constraints STALE → 해당 Constraint 삭제
- Domain Context STALE → 해당 항목 업데이트/삭제

#### Generate DEVELOPERS.md 선택 시
```
Skill("claude-md-plugin:decompile", args: "{module_path}")
```

#### Fix reference 선택 시
CLAUDE.md의 위반 참조를 Edit으로 수정합니다.

#### Convention 위반 Fix Code 시
해당 코드를 Convention에 맞게 수정합니다.

### 4. 결과 요약

```
Resolve 결과
============

| 모듈 | Drift | 해소 방법 |
|------|-------|----------|
| src/auth | Constraints VIOLATED | Fix Code |
| src/utils | Domain Context STALE | Update |
| src/legacy | DEVELOPERS.md MISSING | Generate |

총 이슈: {total}
  - Fix Code: {n}
  - Fix Doc: {n}
  - Generate: {n}
  - Skip: {n}
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
- 각 이슈에 대해 drift 유형별 적절한 선택지 제공
- Fix Doc 전 사용자 확인 (문서 변경은 의도적이어야 함)
- 해소 후 재검증 제안

**DON'T:**
- 사용자 승인 없이 CLAUDE.md 수정
- Fix Code와 Fix Doc를 동시에 실행
- /validate 결과 없이 실행

## 오류 처리

| 상황 | 대응 |
|------|------|
| validate 결과 없음 | /validate 실행 안내 |
| /compile 실패 | 경고 출력, 다음 이슈로 |
| /decompile 실패 | 경고 출력, 다음 이슈로 |

## Examples

<example>
<user_request>/resolve</user_request>
<assistant_response>
최근 validate 결과를 확인합니다...

3개 이슈 발견:

[1/3] src/auth: Constraints VIOLATED — "토큰 만료 최대 7일" 제약이 코드에서 14일로 설정됨
  해소 방법: [Fix Code / Fix Doc / Skip]
→ Fix Code

/compile 실행 중... 완료.

[2/3] src/utils: Domain Context STALE — "Redis 캐시 사용" 맥락이 코드에서 미사용
  해소 방법: [Update / Keep]
→ Update

CLAUDE.md Domain Context 업데이트 완료.

[3/3] src/legacy: DEVELOPERS.md MISSING
  해소 방법: [Generate / Skip]
→ Skip

Resolve 결과
============
총 이슈: 3
  - Fix Code: 1
  - Update: 1
  - Skip: 1

재검증을 실행하시겠습니까? [실행/건너뛰기]
→ 건너뛰기
</assistant_response>
</example>
