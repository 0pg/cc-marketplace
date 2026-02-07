# Temp File Patterns

## 경로 규칙

**경로:** `.claude/tmp/{session-id}-{prefix}-{target}.{ext}`

| 요소 | 설명 |
|------|------|
| `{session-id}` | 세션 ID (8자) - 세션별 격리 |
| `{prefix}` | Agent/Skill 고유 접두사 |
| `{target}` | 대상 식별자 (경로의 `/`를 `-`로 변환) |

## 파일명 패턴 테이블

| Component | 파일명 패턴 |
|-----------|-----------|
| drift-validator | `{session-id}-drift-{target}.md` |
| export-validator | `{session-id}-export-{target}.md` |
| decompiler | `{session-id}-decompile-{target}-claude.md`, `{session-id}-decompile-{target}-implements.md` |
| compiler | `{session-id}-compile-{target}.json` |
| test-reviewer | `{session-id}-test-review-{target}.json` |
| audit CLI | `{session-id}-audit-result.json` |
| boundary-resolve | `{session-id}-boundary-{target}.json` |
| code-analyze | `{session-id}-analysis-{target}.json` |
| schema-validate | `{session-id}-validation-{target}.json` |
| spec-agent (state) | `{session-id}-spec-state-{target}.json` |
| spec-reviewer | `{session-id}-review-{target}.json` |
| code-reviewer | `{session-id}-convention-review-{target}.json` |

## `{target}` 변환 예시

`src/auth` → `src-auth`
`src/utils/crypto` → `src-utils-crypto`

## 정리

세션 종료 시 해당 session-id 접두사의 파일들은 자동 정리됩니다.
