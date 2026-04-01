# impl → compile Commit Hash Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** impl이 커밋 메시지 컨벤션으로 변경 맥락을 기록하고, compile이 이를 자동 탐색하여 incremental compile을 수행하는 핸드오프 구조 구현

**Architecture:** 커밋 메시지 컨벤션(`impl(path):`, `compile(path):`)이 유일한 인터페이스. compile SKILL이 git log --grep으로 impl 커밋을 탐색하고, diff를 추출하여 session file에 Spec Changes 섹션으로 포함. compiler AGENT는 Phase 0(writing-plans 기반 Task Definition)에서 구현 태스크를 도출하고, Phase 1(TDD)에서 태스크 단위로 실행.

**Tech Stack:** Markdown (SKILL/AGENT 파일), Bash (git commands), 기존 CLI (`parse-claude-md`)

**Spec:** `docs/superpowers/specs/2026-04-01-impl-compile-commit-hash-design.md`

---

### Task 1: impl SKILL 커밋 메시지 컨벤션 적용

**Files:**
- Modify: `skills/impl/SKILL.md:316-321`

- [ ] **Step 1: impl SKILL의 커밋 메시지 포맷 변경**

`skills/impl/SKILL.md` 316-321행의 기존 커밋 메시지:

```bash
# 기존
git add "{target_path}/CLAUDE.md" "{target_path}/DEVELOPERS.md"
git commit -m "feat({target_path}): {action} CLAUDE.md + DEVELOPERS.md

요구사항: {사용자 요구사항 텍스트 최초 150자}
workflow: .claude/workflows/{dir-safe}/state.json"
```

를 다음으로 교체:

```bash
# CLAUDE.md + DEVELOPERS.md만 커밋 (TMP 파일 및 workflow state 제외)
git add "{target_path}/CLAUDE.md" "{target_path}/DEVELOPERS.md"
git commit -m "impl({target_path}): [BREAKING] {summary}

{전환 맥락 — 어디서 어디로, 왜 이 변경을 하는가 1-2문장}

Changes:
- added: {추가된 Requirements/Constraints 목록}
- modified: {변경된 Requirements/Constraints 목록}
- removed: {삭제된 Requirements/Constraints 목록}"
```

커밋 메시지 생성 규칙 (impl agent에 지시 추가):
- `{summary}`: 변경의 핵심을 한 줄로 요약
- `[BREAKING]`: Requirements 삭제 또는 대규모 방향 전환 시에만 포함. 해당 없으면 생략
- 전환 맥락: 문서의 "현재 상태"와 달리 "어디서 어디로 전환하는가"를 기술
- Changes: impl agent가 자신이 수행한 변경을 before/after 비교하여 분류

- [ ] **Step 2: 커밋 메시지 지시를 SKILL 본문에 추가**

`6e-1` 섹션 상단에 impl agent가 커밋 메시지를 구성하는 방법을 설명하는 지시 블록 추가:

```markdown
**커밋 메시지 구성:**

impl agent는 Execute 완료 후 커밋 메시지를 다음 규칙으로 생성합니다:

1. **summary**: 이번 변경의 핵심을 한 줄로 (예: "OAuth2 인증 추가", "수수료 정책 변경")
2. **[BREAKING]** (선택): Requirements 삭제 또는 대규모 방향 전환이 있을 때만 포함
3. **전환 맥락**: 1-2문장. 문서는 "현재 상태"를 기술하지만, 커밋 메시지는 "어디서 어디로 전환하는가"를 기술
   - 좋은 예: "session 기반 인증에 OAuth2를 추가 경로로 도입. 레거시 클라이언트 지원을 위해 session 유지."
   - 나쁜 예: "인증 시스템 업데이트" (방향성 없음)
4. **Changes**: before/after 비교하여 added/modified/removed로 분류
   - 해당 없는 항목은 생략 (예: removed 없으면 removed 줄 생략)
```

- [ ] **Step 3: 변경 내용 검증**

impl SKILL.md를 읽어 커밋 메시지 포맷이 올바르게 교체되었는지 확인.
기존 `feat({target_path}):` 패턴이 남아있지 않은지 grep으로 검증:

```bash
grep -n "feat({target_path})" skills/impl/SKILL.md
```

Expected: 결과 없음

- [ ] **Step 4: 커밋**

```bash
git add skills/impl/SKILL.md
git commit -m "feat(impl-skill): 커밋 메시지 컨벤션 적용 (impl(path): 포맷)"
```

---

### Task 2: compile SKILL에 impl 커밋 탐색 로직 추가

**Files:**
- Modify: `skills/compile/SKILL.md:34-61` (Workflow 섹션, Step 0.5 삽입)

- [ ] **Step 1: Step 0.5 신규 섹션 작성**

`skills/compile/SKILL.md`의 `### 0. 초기화` 뒤, `### 1. Compile 대상 결정` 앞에 새 섹션 삽입:

```markdown
### 0.5. impl 커밋 탐색 (incremental compile 준비)

`--all` 모드이면 이 단계를 건너뜁니다.

각 대상 디렉토리({path})에 대해:

**Step 1: 마지막 compile 커밋 찾기**
```bash
LAST_COMPILE=$(git log -1 --format="%H" --grep="^compile({path}):" 2>/dev/null || echo "")
```

**Step 2: 그 이후의 impl 커밋 찾기**
```bash
if [ -n "$LAST_COMPILE" ]; then
  IMPL_COMMITS=$(git log --format="%H" --grep="^impl({path}):" ${LAST_COMPILE}..HEAD 2>/dev/null)
else
  IMPL_COMMITS=$(git log --format="%H" --grep="^impl({path}):" 2>/dev/null)
fi
```

**Step 3: impl 커밋 발견 시 — diff 추출 + 커밋 메시지 파싱**

각 impl 커밋에 대해:
```bash
# diff 추출
git diff {hash}~1..{hash} -- {path}/CLAUDE.md {path}/DEVELOPERS.md

# 커밋 메시지 추출
git log -1 --format="%B" {hash}
```

추출 결과를 변수에 누적하여 Step 6 세션 파일 생성 시 사용.

**Step 3-b: impl 커밋 미발견 시**

기존 `diff-compile-targets` fallback 동작 (Step 1로 진행).
이 경우 Spec Changes 섹션은 세션 파일에 포함하지 않음.
```

- [ ] **Step 2: 변경 내용 검증**

compile SKILL.md를 읽어 Step 0.5가 올바르게 삽입되었는지 확인.
`### 0. 초기화` → `### 0.5. impl 커밋 탐색` → `### 1. Compile 대상 결정` 순서 확인.

- [ ] **Step 3: 커밋**

```bash
git add skills/compile/SKILL.md
git commit -m "feat(compile-skill): impl 커밋 탐색 로직 추가 (Step 0.5)"
```

---

### Task 3: compile 세션 파일에 Spec Changes 섹션 추가

**Files:**
- Modify: `skills/compile/SKILL.md:90-125` (세션 파일 생성 섹션)
- Modify: `skills/compile/references/compiler-templates.md:5-36` (세션 파일 템플릿)

- [ ] **Step 1: compile SKILL의 세션 파일 생성 로직에 Spec Changes 포함**

`skills/compile/SKILL.md`의 `### 6. 세션 파일 생성` 섹션에서, 기존 5단계 뒤에 6단계 추가:

```markdown
6. (Step 0.5에서 impl 커밋 발견 시) Spec Changes 섹션 추가:
   - 커밋 메시지 body에서 전환 맥락 추출 → `### Transition Context`
   - 커밋 메시지 Changes 섹션 파싱 → `### Added`, `### Modified`, `### Removed`
   - BREAKING 플래그 존재 시 → `breaking: true` 메타데이터 추가
```

세션 파일 형식 템플릿에 다음 블록 추가 (`## Dependencies` 뒤):

```markdown
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

- [ ] **Step 2: compiler-templates.md 업데이트**

`skills/compile/references/compiler-templates.md`의 세션 파일 포맷에 동일한 Spec Changes 블록 추가.
`## Dependencies` 다음, `## Verification Contract` 이전에 삽입:

```markdown
## Spec Changes (optional — impl 커밋 발견 시에만 포함)
breaking: {true|false}

### Transition Context
{전환 맥락 — 어디서 어디로, 왜}

### Added
{추가된 Requirements/Constraints}

### Modified
{변경된 Requirements/Constraints}

### Removed
{삭제된 Requirements/Constraints}
```

- [ ] **Step 3: 변경 내용 검증**

두 파일 모두 읽어 Spec Changes 섹션이 올바르게 삽입되었는지 확인.

- [ ] **Step 4: 커밋**

```bash
git add skills/compile/SKILL.md skills/compile/references/compiler-templates.md
git commit -m "feat(compile-skill): 세션 파일에 Spec Changes 섹션 추가"
```

---

### Task 4: compile SKILL에 post-compile 커밋 메시지 컨벤션 적용

**Files:**
- Modify: `skills/compile/SKILL.md:168-179` (변경사항 표시 섹션 뒤)

- [ ] **Step 1: Step 8.5 신규 섹션 작성**

`### 8. 변경사항 표시` 뒤, `### 9. Post-compile 검증` 앞에 새 섹션 삽입:

```markdown
### 8.5. Compile 커밋 생성

컴파일이 성공적으로 완료된 경우 (status != failed), 생성된 코드를 커밋합니다:

```bash
git add {대상 디렉토리의 생성/수정된 파일들}
git commit -m "compile({path}): {summary}

{컴파일된 내용 요약 1-2문장}

Changes:
- compiled: {생성된 파일 목록}
- tests: {생성된 테스트 파일 목록}"
```

이 커밋은 다음 `/compile` 실행 시 `git log --grep="^compile({path}):"` 탐색의 기준점이 됩니다.
```

- [ ] **Step 2: 변경 내용 검증**

compile SKILL.md를 읽어 Step 8.5가 올바르게 삽입되었는지 확인.

- [ ] **Step 3: 커밋**

```bash
git add skills/compile/SKILL.md
git commit -m "feat(compile-skill): post-compile 커밋 메시지 컨벤션 적용 (compile(path): 포맷)"
```

---

### Task 5: compiler AGENT에 Phase 0 (Task Definition) 추가

**Files:**
- Modify: `agents/compiler.md:50-87` (Workflow 앞부분)

- [ ] **Step 1: compiler agent에 Phase 0 섹션 추가**

`agents/compiler.md`의 `## Workflow` 뒤, 기존 `### 1. Load superpowers:tdd` 앞에 새 섹션 삽입:

```markdown
### 0. Phase 0: Task Definition (Spec Changes 있을 때만)

세션 파일에 `## Spec Changes` 섹션이 **없으면** 이 Phase를 건너뛰고 Phase 1(기존 TDD)로 직행합니다.

**`## Spec Changes` 섹션이 있으면:**

⚡ **Skill("superpowers:writing-plans") 로드**

입력 분석:
1. `### Transition Context` 읽기 — 변경의 방향과 이유 파악
2. `### Added / Modified / Removed` 분류 읽기 — 변경 범위 파악
3. `breaking: true` 여부 확인 — BREAKING이면 `--conflict overwrite` 강제
4. 현재 소스코드 구조 탐색 — 기존 파일/함수 위치 파악

Implementation Tasks 도출:

| 변경 유형 | Task 유형 | 행동 |
|----------|----------|------|
| Added | [ADD] | 새 파일/함수 생성 — target path, approach 명시 |
| Modified | [MODIFY] | 기존 코드 수정 — 기존 파일 위치 + 변경 내용 |
| Removed | [DELETE] | 코드 제거 — 대상 파일 + 참조 정리 범위 |

특수 판단:
- Changes에 항목이 없거나 의미적 변경 없음 → **"할 일 없음" → compile 조기 종료**
  - `---compiler-result---` 블록에 `status: skipped`, `reason: no semantic changes` 반환
- Constraint만 변경 (Requirements 동일) → 기능 코드 미변경, 테스트/설정만 수정으로 명시
- BREAKING → 기존 코드 대량 변경 예상, conflict 모드 overwrite 적용

도출된 Tasks를 목록으로 정리한 후 Phase 1에서 Task 단위로 TDD를 실행합니다.
```

- [ ] **Step 2: 기존 TDD 섹션을 Phase 1로 리네이밍**

기존 `### 1. Load superpowers:tdd`를 `### 1. Phase 1: Load superpowers:tdd`로 변경.

기존 Step 2 설명에 다음 문구 추가:

```markdown
Phase 0에서 Implementation Tasks가 도출된 경우:
- Task 단위로 관련 Constraints를 매핑하여 RED-GREEN-REFACTOR 실행
- [ADD] 태스크: 새 Constraint → 테스트 생성 → 구현
- [MODIFY] 태스크: 변경된 Constraint → 기존 테스트 수정 → 코드 수정
- [DELETE] 태스크: 대상 코드 제거 → 참조 정리 → 관련 테스트 제거/수정

Phase 0 없이 진입한 경우 (Spec Changes 없음):
- 기존 동작 그대로 (전체 Constraints → 전체 TDD)
```

- [ ] **Step 3: DELETE 태스크 처리 방법을 Workflow에 추가**

기존 `### 8. File Conflicts` 앞에 새 섹션:

```markdown
### 7.5. DELETE 태스크 실행 (Phase 0 있을 때만)

[DELETE] 태스크는 TDD 사이클이 아닌 직접 실행:
1. 대상 파일/함수 삭제
2. import, 호출부 등 참조 정리
3. 관련 테스트 파일 삭제 또는 수정
4. 삭제 후 전체 테스트 실행 — 회귀 확인
```

- [ ] **Step 4: 변경 내용 검증**

compiler.md를 읽어:
- Phase 0 섹션이 Workflow 최상단에 있는지
- 기존 TDD 흐름이 Phase 1로 리네이밍되었는지
- DELETE 처리가 추가되었는지

- [ ] **Step 5: 커밋**

```bash
git add agents/compiler.md
git commit -m "feat(compiler-agent): Phase 0 Task Definition 추가 (writing-plans 조합)"
```

---

### Task 6: Gherkin feature 파일 작성

**Files:**
- Create: `core/tests/features/commit_hash_handoff.feature`

- [ ] **Step 1: feature 파일 작성**

```gherkin
Feature: impl → compile Commit Hash Handoff
  impl이 커밋 메시지 컨벤션으로 변경 맥락을 기록하고,
  compile이 이를 자동 탐색하여 incremental compile을 수행한다.

  Background:
    Given CLI가 설치되어 있다
    And git 저장소가 초기화되어 있다

  # --- 커밋 메시지 컨벤션 ---

  Scenario: impl 커밋 메시지가 컨벤션을 따른다
    When impl이 "src/auth"에 대해 커밋을 생성한다
    Then 커밋 메시지가 "impl(src/auth):" 로 시작한다
    And 커밋 메시지 body에 "Changes:" 섹션이 있다

  Scenario: BREAKING 태그가 필요할 때 포함된다
    Given Requirements 삭제가 포함된 impl 변경이 있다
    When impl이 커밋을 생성한다
    Then 커밋 메시지에 "[BREAKING]"이 포함된다

  Scenario: 전환 맥락이 커밋 메시지에 포함된다
    When impl이 기존 기능을 수정하는 커밋을 생성한다
    Then 커밋 메시지 body 첫 단락에 전환 맥락이 있다
    And 전환 맥락이 변경의 방향을 기술한다

  # --- compile의 impl 커밋 탐색 ---

  Scenario: compile이 마지막 compile 이후 impl 커밋을 탐색한다
    Given "compile(src/auth): 초기 코드 생성" 커밋이 있다
    And 그 이후 "impl(src/auth): OAuth2 추가" 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then compile은 impl 커밋의 diff를 추출한다
    And 세션 파일에 "## Spec Changes" 섹션이 포함된다

  Scenario: compile 커밋이 없으면 전체 히스토리에서 impl 탐색
    Given compile 커밋이 없다
    And "impl(src/auth): 초기 요구사항" 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then compile은 전체 히스토리에서 impl 커밋을 탐색한다

  Scenario: impl 커밋이 없으면 기존 diff-compile-targets fallback
    Given compile 커밋이 없다
    And impl 커밋도 없다
    When compile이 실행된다
    Then 기존 diff-compile-targets 동작으로 fallback한다
    And 세션 파일에 "## Spec Changes" 섹션이 없다

  Scenario: 수동 수정은 impl 탐색에 잡히지 않는다
    Given "compile(src/auth): 코드 생성" 커밋이 있다
    And 그 이후 수동 "fix: 타이포 수정" 커밋이 있다
    And 그 이후 "impl(src/auth): 기능 추가" 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then 수동 커밋은 무시되고 impl 커밋만 처리된다

  Scenario: 여러 impl 커밋의 diff가 합산된다
    Given "compile(src/auth): 코드 생성" 커밋이 있다
    And 그 이후 3개의 impl(src/auth) 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then 3개 impl 커밋의 diff가 모두 Spec Changes에 포함된다

  # --- Spec Changes 세션 파일 ---

  Scenario: Spec Changes에 Transition Context가 포함된다
    Given impl 커밋 메시지에 전환 맥락이 있다
    When compile이 세션 파일을 생성한다
    Then Spec Changes의 "### Transition Context"에 전환 맥락이 포함된다

  Scenario: Spec Changes에 Added/Modified/Removed가 분류된다
    Given impl 커밋 Changes에 added, modified, removed 항목이 있다
    When compile이 세션 파일을 생성한다
    Then Spec Changes에 "### Added", "### Modified", "### Removed" 섹션이 있다

  Scenario: BREAKING impl 커밋이면 breaking 메타데이터가 포함된다
    Given impl 커밋에 "[BREAKING]" 태그가 있다
    When compile이 세션 파일을 생성한다
    Then Spec Changes에 "breaking: true"가 포함된다

  # --- compiler agent Phase 0 ---

  Scenario: Spec Changes가 있으면 Phase 0 Task Definition 실행
    Given 세션 파일에 "## Spec Changes" 섹션이 있다
    When compiler agent가 실행된다
    Then Phase 0에서 Implementation Tasks를 도출한다
    And Phase 1에서 Task 단위로 TDD를 실행한다

  Scenario: Spec Changes가 없으면 Phase 0 건너뛰기
    Given 세션 파일에 "## Spec Changes" 섹션이 없다
    When compiler agent가 실행된다
    Then Phase 0을 건너뛰고 기존 TDD로 직행한다

  Scenario: 의미적 변경 없음 판단 시 compile 조기 종료
    Given Spec Changes의 Added, Modified, Removed가 모두 비어있다
    When compiler agent Phase 0이 실행된다
    Then "할 일 없음"으로 판단한다
    And status: skipped로 조기 종료한다

  Scenario: BREAKING 플래그 시 conflict overwrite 강제
    Given Spec Changes에 "breaking: true"가 있다
    When compiler agent Phase 0이 실행된다
    Then conflict 모드를 overwrite로 강제한다

  Scenario: DELETE 태스크가 코드 제거 + 참조 정리를 수행한다
    Given Phase 0에서 [DELETE] 태스크가 도출되었다
    When Phase 1에서 DELETE 태스크를 실행한다
    Then 대상 코드를 삭제한다
    And import/호출부 참조를 정리한다
    And 관련 테스트를 제거 또는 수정한다

  # --- post-compile 커밋 ---

  Scenario: compile 완료 후 커밋 메시지가 컨벤션을 따른다
    When compile이 성공적으로 완료된다
    Then 커밋 메시지가 "compile(src/auth):" 로 시작한다
```

- [ ] **Step 2: feature 파일 검증**

파일이 올바르게 생성되었는지 읽어서 확인.

- [ ] **Step 3: 커밋**

```bash
git add core/tests/features/commit_hash_handoff.feature
git commit -m "test: impl→compile commit hash handoff acceptance tests"
```

---

### Task 7: plugin.json 버전 bump

**Files:**
- Modify: `.claude-plugin/plugin.json`

- [ ] **Step 1: 현재 버전 확인**

```bash
cat .claude-plugin/plugin.json | grep version
```

- [ ] **Step 2: MINOR 버전 bump**

이 변경은 기능 추가이므로 MINOR 버전을 올립니다.

- [ ] **Step 3: marketplace.json 동기화**

프로젝트 루트의 `.claude-plugin/marketplace.json`에서 claude-md-plugin의 version도 동일하게 업데이트.

- [ ] **Step 4: 커밋**

```bash
git add .claude-plugin/plugin.json ../../.claude-plugin/marketplace.json
git commit -m "chore(claude-md-plugin): version bump for commit hash handoff feature"
```
