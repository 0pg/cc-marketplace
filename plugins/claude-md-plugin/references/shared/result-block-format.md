# Result Block Format

## 공통 상태 enum

| 상태 | 의미 |
|------|------|
| `approve` | 검증 통과 / 위반 없음 |
| `feedback` | 수정 필요 (피드백 포함) |
| `warning` | 경고 수준 이슈 (진행 가능) |
| `error` | 실패 (진행 불가) |

## 공통 필드

모든 Agent 결과 블록에 포함되는 필드:

| 필드 | 설명 |
|------|------|
| `status` | 상태 enum 값 |
| `result_file` | 결과 JSON 파일 경로 |

## Agent별 결과 블록

### compiler
```
---compiler-result---
status: approve | warning
result_file: .claude/tmp/{session-id}-compile-{target}.json
```
- 추가 필드: `phase`, `generated_files`, `skipped_files`, `tests_passed`, `tests_failed`, `implements_md_updated`

### spec-reviewer
```
---spec-reviewer-result---
status: approve | feedback
score: {0-100}
result_file: .claude/tmp/{session-id}-review-{target}.json
```
- 추가 필드: `checks`, `feedback`

### test-reviewer
```
---test-reviewer-result---
status: approve | feedback
score: {0-100}
result_file: .claude/tmp/{session-id}-test-review-{target}.json
```
- 추가 필드: `checks`, `feedback`

### code-reviewer
```
---code-reviewer-result---
status: approve | feedback | warning
result_file: .claude/tmp/{session-id}-convention-review-{target}.json
```
- 추가 필드: `directory`, `convention_score`, `violations_count`, `auto_fixed_count`

### decompiler
```
---decompiler-result---
status: approve | warning
claude_md_file: ...
implements_md_file: ...
```

### drift-validator / export-validator
```
---drift-validator-result---
status: approve | error
result_file: .claude/tmp/{session-id}-drift-{target}.md
```

### Internal Skills (공통)
```
---{skill-name}-result---
status: approve | warning | error
output_file: .claude/tmp/{session-id}-{prefix}-{target}.json
---end-{skill-name}-result---
```
- Internal Skill은 `approve | warning | error` 상태를 사용합니다.
- `feedback` 상태는 Agent 전용입니다.
