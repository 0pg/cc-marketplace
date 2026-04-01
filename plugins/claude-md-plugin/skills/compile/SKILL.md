---
name: compile
version: 1.0.0
aliases: [gen, generate, build]
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/compile". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Performs Inline TDD: compiler agent generates tests from DEVELOPERS.md Constraints, then implements code (GREEN+REFACTOR).
  Trigger keywords: 코드 생성, 컴파일, CLAUDE.md에서 코드
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# /compile

CLAUDE.md를 기반으로 소스코드를 생성합니다.

## Triggers

- `/compile`
- `코드 생성`
- `CLAUDE.md에서 코드`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `--path` | 아니오 | `.` | 대상 경로 |
| `--all` | 아니오 | false | 전체 CLAUDE.md 대상 (incremental 대신) |
| `--conflict` | 아니오 | `skip` | 파일 충돌 처리: `skip` \| `overwrite` |
| `--dry-run` | 아니오 | false | 실제 파일 생성 없이 대상만 표시 |
| `--validate` | 아니오 | false | 컴파일 후 /validate 자동 실행 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. Compile 대상 결정

**`--all` 모드:**
```
Glob("{path}/**/CLAUDE.md")
```

**Incremental 모드 (기본):**
```bash
$CLI_PATH diff-compile-targets --root {path}
```

결과 분기:
- git 저장소 아님 → 전체 대상으로 fallback
- 변경 없음 → "All up-to-date. Use --all for full compile." → 종료
- 변경 있음 → 대상 목록 + 사유 표시

대상이 없으면 종료.

### 2. 언어 자동 감지

각 대상 디렉토리의 파일 확장자를 분석하여 언어를 추론:
1. 디렉토리 내 소스 파일 확장자 → 언어 결정
2. 소스 파일 없으면 부모 디렉토리 참조
3. 모두 실패하면 `AskUserQuestion`으로 질문

### 3. compile-context 확인 (optional)

각 CLAUDE.md에 대응하는 `compile-context.md`가 같은 디렉토리에 있으면 참조용으로 사용.
없어도 정상 진행.

### 4. 의존성 순서 결정 (leaf-first)

디렉토리 depth 기준 정렬 (깊은 것부터).
같은 depth의 독립 모듈은 병렬 실행 가능 (최대 3개).

### 5. `--dry-run` 처리

대상 목록만 출력하고 종료:
```
Compile 대상:
  • src/auth/jwt (depth=3, typescript)
  • src/auth (depth=2, typescript)
  • src/utils (depth=2, typescript)
```

### 6. 세션 파일 생성

각 대상에 대해 CLAUDE.md + DEVELOPERS.md + Convention 계층을 읽고 세션 파일 생성:

0. (`--all`이 아닌 경우) impl 커밋 탐색 — 대상 디렉토리별로 실행:
   a. 마지막 compile 커밋 찾기:
      ```bash
      LAST_COMPILE=$(git log -1 --format="%H" --grep="^compile({path}):" 2>/dev/null || echo "")
      ```
   b. 그 이후의 impl 커밋 찾기:
      ```bash
      if [ -n "$LAST_COMPILE" ]; then
        IMPL_COMMITS=$(git log --format="%H" --grep="^impl({path}):" ${LAST_COMPILE}..HEAD 2>/dev/null)
      else
        IMPL_COMMITS=$(git log --format="%H" --grep="^impl({path}):" 2>/dev/null)
      fi
      ```
   c. 발견 시 — 각 impl 커밋의 diff + 메시지 추출:
      ```bash
      # diff 추출 (root commit 가드)
      PARENT=$(git rev-parse --verify {hash}~1 2>/dev/null || echo "")
      if [ -n "$PARENT" ]; then
        git diff {hash}~1..{hash} -- {path}/CLAUDE.md {path}/DEVELOPERS.md
      else
        git diff --root {hash} -- {path}/CLAUDE.md {path}/DEVELOPERS.md
      fi

      # 커밋 메시지 추출
      git log -1 --format="%B" {hash}
      ```
   d. 미발견 시: Spec Changes 섹션을 세션 파일에 포함하지 않음
1. 대상 CLAUDE.md Read → Requirements, Domain Context 추출
2. 대상 DEVELOPERS.md Read → Constraints, Technical Context 추출
3. Convention 계층 해소 (module > project > general)
4. compile-context.md Read (optional) → Dependencies, approach 추출
5. 세션 파일 Write → `${TMP_DIR}compile-session-{dir-safe}.md`
6. (sub-step 0에서 impl 커밋 발견 시) Spec Changes 섹션 추가:
   - 커밋 메시지 body에서 전환 맥락 추출 → `### Transition Context`
   - 커밋 메시지 Changes 섹션 파싱 → `### Added`, `### Modified`, `### Removed`
   - BREAKING 플래그 존재 시 → `breaking: true` 메타데이터 추가

세션 파일 형식:
```markdown
# Compile Task: {path}
type: compile | target: {path} | language: {lang} | conflict: {mode}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md
project_conventions: {project_root}/CLAUDE.md#Conventions

## Requirements (from CLAUDE.md)
{추출된 Requirements}

## Constraints (from DEVELOPERS.md)
{추출된 Constraints}

## Technical Context
{추출된 Technical Context}

## Conventions (resolved)
{계층 해소된 Conventions}

## Dependencies
{compile-context 또는 탐색 결과}

## Spec Changes (since compile({path}) @ {last_compile_hash})
breaking: {true|false}

### Transition Context
{impl 커밋 메시지 body에서 추출한 전환 맥락. 여러 impl 커밋이면 시간순으로 나열}

### Added
{추가된 Requirements/Constraints 목록}

### Modified
{변경된 Requirements/Constraints 목록}

### Removed
{삭제된 Requirements/Constraints 목록}
```

### 7. 컴파일 실행

각 대상에 대해 `Task(compiler)` 호출:
```
세션 파일: ${TMP_DIR}compile-session-{dir-safe}.md
대상 디렉토리: {path}
감지된 언어: {language}
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

compiler 결과 status 확인:
- `success`: 다음 모듈로
- `partial`: 경고 수집, 다음 모듈로
- `failed`: 에러 보고, 다음 모듈로

### 7.5. 빌드 검증

모든 모듈 컴파일 완료 후, 감지된 언어에 따라 타입 체크 실행:

| 언어 | 명령 |
|------|------|
| Rust | `cargo check --workspace 2>&1` |
| TypeScript/JavaScript | `tsc --noEmit 2>&1` (tsconfig.json 있을 때만) |
| Python | `python -m py_compile $(find src -name "*.py") 2>&1` |
| 기타 | 스킵 (경고만) |

성공: 계속 진행.

실패:
1. 에러 메시지에서 영향 파일 추출
2. 보고:
   ```
   [BUILD FAILED] {error summary}
   영향 파일: {file list}
   권장 조치: 해당 모듈 DEVELOPERS.md Constraints 검토 후 /compile 재실행
   ```
3. compile status = `failed` 반환, 이후 Step 건너뜀

> **한계**: 새 파일이 `mod.rs`/`lib.rs`에 선언되지 않은 경우 cargo check가 해당 파일을 검사하지 않음.
> compiler agent는 새 파일 생성 시 반드시 mod 선언을 함께 추가해야 함.

### 8. 변경사항 표시

```bash
git diff --stat
```

### 8.5. Compile 커밋 생성

컴파일이 성공적으로 완료된 경우 (status != failed), **각 대상 디렉토리별로 개별 커밋합니다** (통합 커밋 금지):

```bash
# 각 compile 대상에 대해 반복
git add {대상 디렉토리의 생성/수정된 파일들}
git commit -m "compile({path}): {summary}

{컴파일된 내용 요약 1-2문장}

Changes:
- compiled: {생성된 파일 목록}
- tests: {생성된 테스트 파일 목록}"
```

이 커밋이 `git log --grep="^compile({path}):"` 탐색의 기준점이 되므로,
path별 개별 커밋이 필수입니다.

### 9. Post-compile 검증 (optional)

`--validate` 플래그가 있으면:
```
Skill("claude-md-plugin:validate", args: "{path}")
```

### 10. 결과

```
---compile-result---
status: success | partial | failed
total: {n}
generated: {n}
skipped: {n}
tests: {passed} passed, {failed} failed
validate: {status} (--validate 사용 시)
---end-compile-result---
```

## DO / DON'T

**DO:**
- leaf-first 순서 준수
- 세션 파일 생성 시 Convention 계층 해소 완료
- compiler agent에게 세션 파일로 위임

**DON'T:**
- CLAUDE.md 수정 (읽기 전용)
- compiler agent에 CLAUDE.md 경로 직접 전달 (세션 파일로 전달)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| compiler agent 실패 (단일 모듈) | 경고, 나머지 계속 |
| 빌드 검증 실패 (Step 7.5) | 에러 보고, status=failed 반환 |
| 언어 감지 실패 | AskUserQuestion |
