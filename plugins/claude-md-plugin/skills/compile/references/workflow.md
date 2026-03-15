# Compile Workflow Details

## Incremental Target Resolution

기본 동작은 git 상태 기반으로 변경된 CLAUDE.md만 선별하여 compile합니다.
`--all` 옵션으로 전체 compile을 수행할 수 있습니다.

1. `--all` 플래그가 있으면 `Glob("**/CLAUDE.md")`로 전체 CLAUDE.md를 수집하여 full rebuild를 수행한다.
2. `--all`이 없으면 (incremental 모드):
   1. `Bash`로 Rust CLI의 `diff-compile-targets` 서브커맨드를 실행하여 변경된 대상을 감지한다.
   2. 결과 JSON을 읽어 다음 분기를 처리한다:
      - **git 저장소가 아닌 경우** (warnings에 `no-git-repo`가 포함): `"⚠ Not a git repository. Falling back to full compilation."` 메시지를 출력하고 `Glob("**/CLAUDE.md")`로 전체를 대상으로 한다.
      - **변경된 대상이 없는 경우**: `"✓ All up-to-date. Use --all for full compile."` 메시지를 출력하고 종료한다.
      - **변경된 대상이 있는 경우**:
        1. 각 대상의 CLAUDE.md 경로를 compile 대상 목록으로 설정한다.
        2. 감지된 대상 수와 각 디렉토리/사유를 출력한다 (예: `"  ✓ src/auth — staged"`).
        3. 건너뛴 모듈(up-to-date)이 있으면 그 수를 출력한다.
        4. 의존성 경고(dependency warnings)가 있으면 각 메시지를 출력하고 `"  Use --all for full compilation."` 안내를 추가한다.

**Compile Target 판별 조건:**

| 조건 | 판별 방법 | reason |
|------|-----------|--------|
| Staging | `git diff --cached --name-only`에 포함 | `staged` |
| Modified | `git diff --name-only`에 포함 (unstaged 수정) | `modified` |
| Untracked | `git ls-files --others --exclude-standard`에 포함 | `untracked` |
| Spec이 Code보다 최신 | CLAUDE.md의 마지막 commit timestamp > 소스코드의 마지막 commit timestamp | `spec-newer` |
| 소스코드 없음 | 디렉토리에 소스 파일이 전혀 없음 (첫 compile) | `no-source-code` |

## 언어 자동 감지

각 CLAUDE.md가 있는 디렉토리의 언어를 감지합니다.

1. 대상 디렉토리에 있는 기존 소스 파일들의 확장자를 수집하여 언어를 추론한다. 성공하면 해당 언어를 반환한다.
2. 기존 파일이 없으면, 부모 디렉토리의 언어 정보를 참조한다. 성공하면 해당 언어를 반환한다.
3. 위 두 방법 모두 실패하면, 프로젝트에서 사용 중인 언어 목록을 옵션으로 구성하여 `AskUserQuestion`으로 사용자에게 질문한다.

## compile-context 확인 (optional)

1. compile 대상 CLAUDE.md 목록을 순회한다.
2. 각 CLAUDE.md에 대응하는 compile-context 파일이 `.claude/tmp/compile-context-{dir-hash}.md`에 존재하는지 확인한다.
3. compile-context가 있으면 참조용으로 사용한다 (Dependencies Direction, Implementation Approach, Technology Choices).
4. compile-context가 없어도 compile은 정상 진행한다 (CLAUDE.md만으로 충분).

## 테스트 프레임워크 감지

프로젝트에서 사용 중인 테스트 프레임워크를 감지합니다:

1. 프로젝트 CLAUDE.md `### Test Convention` 서브섹션이 있으면 그것을 사용합니다.
2. 없으면 프로젝트 설정 파일을 분석합니다:
   - `package.json`: jest, vitest, mocha 등
   - `pyproject.toml`: pytest, unittest 등
   - `Cargo.toml`: 기본 Rust test framework
   - `go.mod`: 기본 Go test framework
3. 감지 결과를 test-designer에게 전달합니다.

## 2-Agent 실행 (의존성 인식)

의존 모듈 간 순서를 보장하기 위해, depth 기반 leaf-first 실행을 수행합니다.
같은 depth의 독립 모듈은 병렬로 처리하되, 상위(부모) 모듈은 하위(자식) 모듈 compile 완료 후 실행합니다.

> **이유**: compiler Agent의 GREEN 워크플로우에서 테스트 실행 시 의존 모듈의 코드가 필요합니다.
> 병렬 실행하면 의존 모듈의 코드가 아직 생성되지 않아 import 실패가 발생할 수 있습니다.

**임시 디렉토리 초기화:**
```bash
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

1. compile 대상 파일들을 디렉토리 depth 기준으로 그룹화한다 (깊은 것부터 처리하는 leaf-first 순서).
2. 가장 깊은 depth 그룹부터 순서대로 처리한다:
   1. 같은 depth 그룹 내의 각 CLAUDE.md에 대해:
      1. 해당 디렉토리의 compile-context 경로(있으면)와 감지된 언어를 준비한다.
      2. `"  • {CLAUDE.md 경로} - 시작 (depth={depth})"` 메시지를 출력한다.
      3. **Step 1: test-designer 호출** — `Task`로 `test-designer` Agent를 실행한다:
         - 입력: CLAUDE.md 경로, compile-context 경로(optional), 대상 디렉토리, 감지된 언어, 테스트 프레임워크, 프로젝트 CLAUDE.md 경로, 모드 (full/incremental), 대상 exports, dependency CLAUDE.md 경로 목록
         - test-designer 결과에서 테스트 파일 목록을 추출한다.
      4. **Step 2: compiler 호출** — `Task`로 `compiler` Agent를 실행한다:
         - 입력: CLAUDE.md 경로, compile-context 경로(optional), 대상 디렉토리, 감지된 언어, 테스트 파일 목록, 충돌 처리 모드
         - 결과는 ${TMP_DIR}에 저장하고 경로만 반환하도록 지시한다.
      5. **피드백 루프 (compiler 실패 시):**
         - compiler가 3회 재시도 후 실패하면:
         - **Step 3**: test-designer를 에러 컨텍스트와 함께 재호출 (에러 메시지, 실패 테스트 정보 포함)
         - **Step 4**: compiler를 재호출
         - Step 4도 실패하면 사용자에게 실패 보고 (최대 1회 피드백 루프)
   2. 같은 depth 그룹의 모든 2-Agent 실행이 완료될 때까지 대기한 후, 다음(더 얕은) depth 그룹으로 진행한다.

**같은 depth 병렬 처리 시 주의:**
- 각 모듈의 test-designer → compiler는 순차 (dependency)
- 같은 depth의 독립 모듈은 병렬 가능 (각 모듈 내에서 순차)

## 결과 수집 및 보고

1. 모든 compiler Agent의 결과 파일을 수집한다.
2. 각 결과 파일에서 생성된 파일 수, 건너뛴 파일 수, 테스트 통과/실패 수를 누적한다.
3. 최종 요약을 출력한다:
   ```
   === 생성 완료 ===
   총 CLAUDE.md: {대상 수}개
   생성된 파일: {생성 수}개
   건너뛴 파일: {건너뛴 수}개
   테스트: {통과} passed, {실패} failed
   ```

## 파일 충돌 처리 로직

- **`--conflict skip` (기본)**: 대상 파일이 이미 존재하면 `"⏭ Skipped: {경로}"` 메시지를 출력하고 건너뛴 파일 목록에 추가한 뒤, 다음 파일로 넘어간다.
- **`--conflict overwrite`**: 대상 파일이 이미 존재해도 `"↻ Overwriting: {경로}"` 메시지를 출력하고 덮어쓴다.

## 내부 2-Agent TDD 워크플로우

사용자에게 노출되지 않는 내부 프로세스:

```
CLAUDE.md 파싱 (+ compile-context 참조, optional)
     │
     ▼
[RED] Task(test-designer)
     │   └─ CLAUDE.md Exports → Export Interface Tests (불변)
     │   └─ CLAUDE.md Behaviors → Behavior Tests
     │   └─ dependency CLAUDE.md → Mock 생성
     ▼
[GREEN] Task(compiler)
     │   └─ 구현 생성 + 테스트 통과 (최대 3회 재시도)
     │   └─ compile-context 참조 (있으면)
     │   └─ 테스트 파일 수정 금지 (INV-EXPORT)
     ▼
[REFACTOR] (compiler 내부)
     │   └─ Convention 섹션 기반 코드 정리
     │   └─ 회귀 테스트로 안전성 확인
     ▼
파일 충돌 처리
     │
     ▼
결과 반환
     │
     ▼ (실패 시)
[Feedback Loop] (최대 1회)
     │   └─ Task(test-designer) + 에러 컨텍스트 → 인프라 수정
     │   └─ Task(compiler) → 재시도
     ▼
최종 결과
```
