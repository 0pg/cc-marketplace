---
name: dev
version: 1.0.0
aliases: [gen, generate, build, compile]
description: |
  This skill should be used when the user asks to "develop from CLAUDE.md", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/dev". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Performs 4-agent TDD pipeline: test-writer → test-reviewer loop → green-coder → refactorer.
  Trigger keywords: 코드 생성, 개발, CLAUDE.md에서 코드
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, Skill, AskUserQuestion]
---

# /dev

CLAUDE.md를 기반으로 소스코드를 생성합니다.

## Triggers

- `/dev`
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

### 1. Dev 대상 결정

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
- 변경 없음 → "All up-to-date. Use --all for full dev." → 종료
- 변경 있음 → 대상 목록 + 사유 표시

대상이 없으면 종료.

### 2. 언어 자동 감지

각 대상 디렉토리의 파일 확장자를 분석하여 언어를 추론:
1. 디렉토리 내 소스 파일 확장자 → 언어 결정
2. 소스 파일 없으면 부모 디렉토리 참조
3. 모두 실패하면 `AskUserQuestion`으로 질문

### 3. dev-context 확인 (optional)

각 CLAUDE.md에 대응하는 `dev-context.md`가 같은 디렉토리에 있으면 참조용으로 사용.
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

0. (`--all`이 아닌 경우) spec 커밋 탐색 — 대상 디렉토리별로 실행:
   a. 마지막 dev 커밋 찾기:
      ```bash
      LAST_DEV=$(git log -1 --format="%H" --grep="^dev({path}):" 2>/dev/null || echo "")
      ```
   b. 그 이후의 spec 커밋 찾기:
      ```bash
      if [ -n "$LAST_DEV" ]; then
        SPEC_COMMITS=$(git log --format="%H" --grep="^spec({path}):" ${LAST_DEV}..HEAD 2>/dev/null)
      else
        SPEC_COMMITS=$(git log --format="%H" --grep="^spec({path}):" 2>/dev/null)
      fi
      ```
   c. 발견 시 — 각 spec 커밋의 diff + 메시지 추출:
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
4. dev-context.md Read (optional) → Dependencies, approach 추출
5. 세션 파일 Write → `${TMP_DIR}dev-session-{dir-safe}.md`
6. (sub-step 0에서 spec 커밋 발견 시) Spec Changes 섹션 추가:
   - 커밋 메시지 body에서 전환 맥락 추출 → `### Transition Context`
   - 커밋 메시지 Changes 섹션 파싱 → `### Added`, `### Modified`, `### Removed`
   - BREAKING 플래그 존재 시 → `breaking: true` 메타데이터 추가

세션 파일 형식: dev-templates.md의 "Dev Session File Format" 참조.

**6e. Implementation Tasks 도출 (Spec Changes 있을 때만)**

세션 파일에 `## Spec Changes`가 포함된 경우:
1. Added → `[ADD]` 태스크: 새 Constraint/Requirement에 대한 테스트+구현 필요
2. Modified → `[MODIFY]` 태스크: 변경된 Constraint/Requirement에 맞게 테스트+구현 수정
3. Removed → `[DELETE]` 태스크: 삭제된 Constraint/Requirement 관련 코드+테스트 제거

`## Implementation Tasks` 섹션을 세션 파일에 추가:
```markdown
## Implementation Tasks (Spec Changes 있을 때만)
- [ADD] CONST-N: {설명}
- [MODIFY] CONST-N: {변경 내용}
- [DELETE] CONST-N: {삭제 대상}
```

**6f. [DELETE] 태스크 실행 (있을 때만)**

SKILL이 DELETE를 TDD 파이프라인 전에 직접 처리:

1. Grep으로 삭제 대상의 import/참조 검색
2. 참조하는 파일 목록 수집
3. 대상 파일/함수 삭제 (Bash rm 또는 Edit)
4. 참조 파일에서 import/호출 제거 (Edit)
5. 관련 테스트 파일 삭제
6. 회귀 테스트 실행 → 실패 시 경고 보고

### 7. Test Writing Loop (per target, 모듈별 순차)

`round = 1`, `max_safety = 5`

```
loop:
  7a. test-writer 세션 파일 생성:
      ${TMP_DIR}test-writer-session-{dir-safe}.md
      (형식: dev-templates.md의 "Test Writer Session File Format" mode=write 참조)

  7b. Task(test-writer) 디스패치:
      세션 파일: ${TMP_DIR}test-writer-session-{dir-safe}.md
      결과는 ${TMP_DIR}에 저장하고 경로만 반환

      result block에서 test_dir, mapping_file 추출.

  7c. test-reviewer 세션 파일 생성:
      ${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md:

      ```markdown
      # Test Review Session
      type: test-review | round: {round} | language: {lang}
      dir_safe: {dir-safe}
      mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
      test_dir: ${TMP_DIR}tests/{dir-safe}/
      spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
      ```

  7d. Task(test-reviewer) 디스패치:
      세션 파일: ${TMP_DIR}test-reviewer-session-{dir-safe}-v{round}.md
      결과는 ${TMP_DIR}에 저장하고 경로만 반환

      result block에서 verdict 추출.

  7e. if verdict == "approved":
        break → Step 7.5

  7f. if round >= max_safety:
        ⚠ Test review loop가 {max_safety}회 반복 후 종료됩니다.
          최선의 테스트로 진행합니다.
        break → Step 7.5

  7g. Revise 세션 파일 생성:
      ${TMP_DIR}test-writer-session-{dir-safe}.md (덮어쓰기):
      mode를 revise로 변경, round 증가, feedback_file 추가
      (형식: dev-templates.md의 "Test Writer Session File Format" mode=revise 참조)

  7h. Task(test-writer, mode=revise) 디스패치:
      세션 파일: ${TMP_DIR}test-writer-session-{dir-safe}.md
      결과는 ${TMP_DIR}에 저장하고 경로만 반환

  7i. round++ → 7c로 돌아감
```

### 7.5. TMP → target 복사 + Verify RED

```
7.5a. TMP/tests/{dir-safe}/ → target 디렉토리 복사
      mapping.json의 test_files 경로 기준으로 복사

7.5b. Verify RED (SKILL이 Bash로 직접 실행):
      언어별 테스트 실행:
      | 언어 | 명령 |
      | TypeScript | npx jest --passWithNoTests 2>&1 |
      | Rust | cargo test --no-run 2>&1 (컴파일만) |
      | Python | python -m pytest --collect-only 2>&1 |
      | Go | go test -run "^$" ./... 2>&1 (컴파일만) |

7.5c. 전부 실패 확인 → Step 8 진입
7.5d. 일부 통과 → 기존 구현 커버리지로 기록, Step 8 진입
7.5e. 컴파일 자체 실패 (import 오류 등) → green-coder에 위임 (import fix 허용)
```

### 8. Task(green-coder)

```
green-coder 세션 파일 생성:
${TMP_DIR}green-session-{dir-safe}.md
(형식: dev-templates.md의 "Green Coder Session File Format" 참조)

Task(green-coder) 디스패치:
  세션 파일: ${TMP_DIR}green-session-{dir-safe}.md
  대상 디렉토리: {path}
  감지된 언어: {language}
  결과는 ${TMP_DIR}에 저장하고 경로만 반환

green-result status 확인:
- success: Step 9로
- partial: 경고 수집, Step 9로
- failed: 에러 보고, 다음 모듈로
```

### 9. Task(refactorer)

```
refactorer 세션 파일 생성:
${TMP_DIR}refactor-session-{dir-safe}.md
(형식: dev-templates.md의 "Refactorer Session File Format" 참조)
Implementation Files: green-result의 implemented_files

Task(refactorer) 디스패치:
  세션 파일: ${TMP_DIR}refactor-session-{dir-safe}.md
  대상 디렉토리: {path}
  감지된 언어: {language}
  결과는 ${TMP_DIR}에 저장하고 경로만 반환

refactor-result status:
- success: 계속
- rolled_back: 경고 기록 (green-coder 결과는 유지)
- skipped: 계속
```

### 10. 빌드 검증

모든 모듈 완료 후, 감지된 언어에 따라 타입 체크 실행:

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
   권장 조치: 해당 모듈 DEVELOPERS.md Constraints 검토 후 /dev 재실행
   ```
3. dev status = `failed` 반환, 이후 Step 건너뜀

> **한계**: 새 파일이 `mod.rs`/`lib.rs`에 선언되지 않은 경우 cargo check가 해당 파일을 검사하지 않음.
> green-coder agent는 새 파일 생성 시 반드시 mod 선언을 함께 추가해야 함.

### 11. 변경사항 표시

```bash
git diff --stat
```

### 12. Dev 커밋 생성

컴파일이 성공적으로 완료된 경우 (status != failed), **각 대상 디렉토리별로 개별 커밋합니다** (통합 커밋 금지):

```bash
# 각 dev 대상에 대해 반복
git add {대상 디렉토리의 생성/수정된 파일들}
git commit -m "dev({path}): {summary}

{컴파일된 내용 요약 1-2문장}

Changes:
- compiled: {생성된 파일 목록}
- tests: {생성된 테스트 파일 목록}"
```

이 커밋이 `git log --grep="^dev({path}):"` 탐색의 기준점이 되므로,
path별 개별 커밋이 필수입니다.

### 13. Post-dev 검증 (optional)

`--validate` 플래그가 있으면:
```
Skill("claude-md-plugin:validate", args: "{path}")
```

### 14. 결과

```
---dev-result---
status: success | partial | failed
total: {n}
generated: {n}
skipped: {n}
tests: {passed} passed, {failed} failed
validate: {status} (--validate 사용 시)
---end-dev-result---
```

## DO / DON'T

**DO:**
- leaf-first 순서 준수
- 세션 파일 생성 시 Convention 계층 해소 완료
- test-writer → test-reviewer → green-coder → refactorer 순서 준수
- DELETE 태스크는 TDD 파이프라인 전에 SKILL이 직접 처리

**DON'T:**
- CLAUDE.md 수정 (읽기 전용)
- Agent에 CLAUDE.md 경로 직접 전달 (세션 파일로 전달)
- test-reviewer approve 없이 green-coder 진입
- compiler agent 사용 (폐기됨)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| test-writer 실패 | 경고, 나머지 계속 |
| test-reviewer max_safety 도달 | best-effort 진행, 경고 |
| green-coder 실패 (단일 모듈) | 경고, 나머지 계속 |
| refactorer 회귀 실패 | 롤백, 경고 |
| 빌드 검증 실패 (Step 10) | 에러 보고, status=failed |
| 언어 감지 실패 | AskUserQuestion |
