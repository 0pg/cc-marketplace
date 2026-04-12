# Consult Templates

## Session File Format

```markdown
# Consult Session
type: consult | target: src/auth | project_root: .
dir_safe: src-auth

## Request
"OAuth 소셜 로그인 추가 가능한가?"

## [1] Current Spec

### CLAUDE.md
## Purpose
...

### DEVELOPERS.md
## Constraints
- CONST-1: ...
## Roadmap
### Short-term
- 사용자 프로필 이미지 캐싱 개선

## [2] Decision History

### diff-node-history (limit 5)
{"has_history": true, "commits": [...]}

### Agent Observations (structural, decision, improvement only)
### [structural] auth-utils import cycle
- anchor: REQ-2
- since: 2026-01-15
- refs: 2
- source: /dev tdd-coder
- ...

## [3] Strategic Direction

### Roadmap
### Short-term
- 사용자 프로필 이미지 캐싱 개선
### Long-term
- 외부 인증 provider 지원 검토
```

## Output Format

```
=== Consult: src/auth ===

Request: "OAuth 소셜 로그인 추가 가능한가?"

Verdict: partially_feasible

Constraints:
  CONST-3: auth token 30일 만료 — OAuth refresh token 정책과 충돌 가능

History:
  [2026-01-15] structural: auth-utils import cycle (REQ-2) — 인증 모듈 분리 시 참고 필요

Roadmap fit: aligned
  "Long-term '외부 인증 provider 지원 검토' 방향과 정렬됨"

Suggested path:
  Short: CONST-3 수정(refresh token 정책 별도 협의) 후 /sync → /dev
  Long:  Roadmap Long-term 작업으로 진행하면 자연스럽게 해결 가능

Downstream:
  - partially_feasible → CONST-3 수정 필요: /sync로 Constraints 업데이트 후 /dev
  - Roadmap 갱신 원하면 PM/PO가 ## Roadmap Long-term 직접 수정

===
```
