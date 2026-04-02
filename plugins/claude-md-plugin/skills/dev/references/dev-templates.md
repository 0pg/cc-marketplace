# Dev Templates

## Dev Session File Format

````markdown
# Dev Task: {path}
type: dev | target: {path} | language: {lang} | conflict: {mode}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md
project_conventions: {project_root}/CLAUDE.md#Conventions

## Requirements (from CLAUDE.md)
{Requirements 섹션 전체}

## Constraints (from DEVELOPERS.md)
{Constraints 섹션 전체 — 테스트 생성 원천}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션 — 타입 참조용, 테스트 생성 원천 아님}

## Technical Context
{Technical Context 섹션 전체}

## Conventions (resolved)
{계층 해소된 Conventions}

## Dependencies
{dev-context 또는 탐색 결과}

## Implementation Tasks (Spec Changes 있을 때만)
- [ADD] CONST-N: {설명}
- [MODIFY] CONST-N: {변경 내용}
- [DELETE] CONST-N: {삭제 대상}

## Spec Changes (optional — spec 커밋 발견 시에만 포함)
breaking: {true|false}

### Transition Context
{전환 맥락 — 어디서 어디로, 왜}

### Added
{추가된 Requirements/Constraints}

### Modified
{변경된 Requirements/Constraints}

### Removed
{삭제된 Requirements/Constraints}

## Verification Contract
- All Constraints → corresponding tests exist
- All Requirements → corresponding acceptance tests exist
- All tests pass
- /validate --strict {path}
````

## Test Writer Session File Format

### mode=write

````markdown
# Test Writer Session
type: test-writer | mode: write | target: {path} | language: {lang}
test_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Requirements (from CLAUDE.md)
{Requirements 섹션 전체}

## Constraints (from DEVELOPERS.md)
{Constraints 섹션 전체}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션}

## Technical Context
{Technical Context}

## Conventions (resolved)
{계층 해소된 Conventions}

## Implementation Tasks (Spec Changes 있을 때만)
- [ADD] CONST-N: {설명}
- [MODIFY] CONST-N: {변경 내용}

## Existing Test Directory (Incremental 모드, 기존 테스트 있을 때만)
existing_test_dir: {path}/{detected_test_dir}/

## Dependencies
{dev-context 또는 탐색 결과}
````

### mode=revise

````markdown
# Test Writer Session
type: test-writer | mode: revise | round: {N} | target: {path} | language: {lang}
test_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_output: ${TMP_DIR}test-mapping-{dir-safe}.json
feedback_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{N-1}.md

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Requirements (from CLAUDE.md)
{Requirements 섹션 전체}

## Constraints (from DEVELOPERS.md)
{Constraints 섹션 전체}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션}

## Technical Context
{Technical Context}

## Conventions (resolved)
{계층 해소된 Conventions}

## Implementation Tasks (Spec Changes 있을 때만)
- [ADD] CONST-N: {설명}
- [MODIFY] CONST-N: {변경 내용}

## Existing Test Directory (Incremental 모드, 기존 테스트 있을 때만)
existing_test_dir: {path}/{detected_test_dir}/

## Dependencies
{dev-context 또는 탐색 결과}
````

## Test Reviewer Session File Format

````markdown
# Test Review Session
type: test-review | round: {N} | language: {lang}
dir_safe: {dir-safe}
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
test_dir: ${TMP_DIR}tests/{dir-safe}/
spec_session_file: ${TMP_DIR}dev-session-{dir-safe}.md
````

## Green Coder Session File Format

````markdown
# Green Coder Session
type: green | target: {path} | language: {lang} | conflict: {mode}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md

## Requirements (from CLAUDE.md)
{Requirements}

## Constraints (from DEVELOPERS.md)
{Constraints}

## Technical Context
{Technical Context}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션 — 타입 참조용}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Tasks (Spec Changes 있을 때만)
{[ADD]/[MODIFY] 태스크만 — DELETE는 SKILL이 이미 처리}

## Dependencies
{dependencies}
````

## Refactorer Session File Format

````markdown
# Refactorer Session
type: refactor | target: {path} | language: {lang}

## Conventions (resolved)
{계층 해소된 Conventions}

## Approved Tests
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json

## Implementation Files
{green-coder result에서 추출한 파일 목록}
````

## Mapping JSON Format

```json
{
  "target_path": "src/auth",
  "test_files": ["src/auth/__tests__/auth.test.ts", "src/auth/__tests__/auth.acceptance.test.ts"],
  "constraints": [
    {
      "id": "CONST-1",
      "text": "authenticate(token: string) → User | AuthError",
      "tests": ["auth.test.ts::should return User for valid token", "auth.test.ts::should throw AuthError for expired token"]
    }
  ],
  "requirements": [
    {
      "id": "REQ-1",
      "text": "유효한 토큰으로 사용자 인증 가능",
      "acceptance_tests": ["auth.acceptance.test.ts::Given valid token When authenticate Then return user"]
    }
  ],
  "unmapped_constraints": [],
  "unmapped_requirements": []
}
```

## Result Formats

### test-writer-result

```
---test-writer-result---
result_file: ${TMP_DIR}test-writer-result-{dir-safe}.json
status: success | partial
test_dir: ${TMP_DIR}tests/{dir-safe}/
mapping_file: ${TMP_DIR}test-mapping-{dir-safe}.json
tests_generated: N
unmapped_constraints: N
unmapped_requirements: N
---end-test-writer-result---
```

### test-reviewer-result

```
---test-reviewer-result---
result_file: ${TMP_DIR}test-reviewer-result-{dir-safe}-v{round}.md
verdict: approved | rejected
round: {N}
---end-test-reviewer-result---
```

### green-result

```
---green-result---
result_file: ${TMP_DIR}green-result-{dir-safe}.json
status: success | partial | failed
implemented_files: [...]
tests_passed: N
tests_failed: N
---end-green-result---
```

### refactor-result

```
---refactor-result---
result_file: ${TMP_DIR}refactor-result-{dir-safe}.json
status: success | rolled_back | skipped
refactored_files: [...]
tests_passed: N
tests_failed: N
---end-refactor-result---
```

## Error Handling

| 상황 | 대응 |
|------|------|
| 세션 파일 파싱 실패 | Agent 실패 반환 |
| test-writer unmapped > 0 | partial 상태 반환 |
| test-reviewer max_safety 도달 | best-effort 진행, 경고 |
| Verify RED 컴파일 실패 | green-coder에 위임 (import fix 허용) |
| GREEN 3회 실패 | partial 상태 반환 |
| REFACTOR 회귀 실패 | 롤백, rolled_back 상태 반환 |
| 파일 쓰기 실패 | 해당 파일 건너뛰기 |
