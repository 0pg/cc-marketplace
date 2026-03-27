---
name: compile
version: 3.1.0
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
- 변경 있음 → 대상 목록 + 사유 표시 (staged, modified, spec-newer 등)

대상이 없으면 종료.

### 2. 언어 자동 감지

각 대상 디렉토리의 파일 확장자를 분석하여 언어를 추론:
1. 디렉토리 내 소스 파일 확장자 → 언어 결정
2. 소스 파일 없으면 부모 디렉토리 참조
3. 모두 실패하면 `AskUserQuestion`으로 질문

### 3. compile-context 확인 (optional)

각 CLAUDE.md에 대응하는 `compile-context.md`가 같은 디렉토리에 있으면 참조용으로 사용.
없어도 compile은 정상 진행 (CLAUDE.md만으로 충분).

### 4. 의존성 순서 결정 (leaf-first)

디렉토리 depth 기준 정렬 (깊은 것부터).
같은 depth의 독립 모듈은 병렬 실행 가능 (최대 3개).

### 5. `--dry-run` 처리

`--dry-run`이면 대상 목록만 출력하고 종료:
```
Compile 대상:
  • src/auth/jwt (depth=3, typescript)
  • src/auth (depth=2, typescript)
  • src/utils (depth=2, typescript)
```

### 6. 컴파일 실행

각 대상에 대해 `Task(compiler)` 호출:
```
CLAUDE.md 경로: {path}/CLAUDE.md
compile-context: {path}/compile-context.md (optional)
대상 디렉토리: {path}
감지된 언어: {language}
충돌 처리: {conflict_mode}
결과는 ${TMP_DIR}에 저장하고 경로만 반환
```

> **Inline TDD**: compiler agent가 DEVELOPERS.md Constraints에서 테스트를 생성하고
> 구현까지 단일 워크플로우로 수행합니다 (Phase 2: 테스트 생성 → Phase 3: GREEN → Phase 4: REFACTOR).

compiler 결과에서 status 확인:
- `success`: 다음 모듈로
- `warning`: 경고 수집, 다음 모듈로

### 7. 변경사항 표시

```bash
git diff --stat
```

### 7.5. Post-compile 검증 (optional)

`--validate` 플래그가 있으면 `/validate`를 자동 실행합니다:

```
Skill("claude-md-plugin:validate", args: "{path}")
```

검증 결과를 최종 보고에 포함합니다.

### 8. 최종 보고

```
=== Compile 완료 ===
총 CLAUDE.md: {total}개
생성된 파일: {generated}개
건너뛴 파일: {skipped}개
테스트: {passed} passed, {failed} failed
검증: {validate_status} (--validate 사용 시)
```

## DO / DON'T

**DO:**
- leaf-first 순서 준수 (의존 모듈 먼저)
- 언어 자동 감지 후 compile
- compiler agent에게 Inline TDD 위임

**DON'T:**
- CLAUDE.md 수정 (읽기 전용)
- 사용자에게 compiler agent 내부 진행 상황 중계
- 별도 테스트 생성 agent 호출 (compiler가 Inline TDD로 테스트 생성도 수행)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| compiler agent 실패 (단일 모듈) | 경고, 나머지 계속 |
| 언어 감지 실패 | AskUserQuestion |

## Examples

<example>
<user_request>/compile</user_request>
<assistant_response>
Incremental 대상 감지:
  • src/auth — staged
  • src/utils — spec-newer

Compile 진행:
  • src/auth (typescript) — 성공 (tests: 5 passed)
  • src/utils (typescript) — 성공 (tests: 3 passed)

=== Compile 완료 ===
총 CLAUDE.md: 2개
생성된 파일: 6개
건너뛴 파일: 0개
테스트: 8 passed, 0 failed
</assistant_response>
</example>

<example>
<user_request>/compile --all --conflict overwrite</user_request>
<assistant_response>
전체 CLAUDE.md 3개 수집.

Compile 진행:
  • src/auth/jwt (depth=3) — 성공
  • src/auth (depth=2) — 성공 (2 overwritten)
  • src/utils (depth=2) — 성공

=== Compile 완료 ===
총 CLAUDE.md: 3개
생성된 파일: 9개
건너뛴 파일: 0개
테스트: 12 passed, 0 failed
</assistant_response>
</example>

<example>
<user_request>/compile --validate --path src/auth</user_request>
<assistant_response>
Incremental 대상 감지:
  • src/auth — modified

Compile 진행:
  • src/auth (typescript) — 성공 (tests: 4 passed)

Post-compile 검증:
  • /validate src/auth — PASS (0 violations)

=== Compile 완료 ===
총 CLAUDE.md: 1개
생성된 파일: 3개
건너뛴 파일: 0개
테스트: 4 passed, 0 failed
검증: PASS (0 violations)
</assistant_response>
</example>
