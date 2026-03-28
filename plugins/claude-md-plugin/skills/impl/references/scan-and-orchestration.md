# /impl SKILL-level Orchestration Guide

## scan-claude-md 호출 패턴

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")

# CLI로 기존 CLAUDE.md 파일의 경량 인덱스 생성
mkdir -p .claude/extract-results
$CLI_PATH scan-claude-md --root {project_root} --output .claude/extract-results/claude-md-index.json
```

인덱스 출력 형식:
```json
{
  "root": "/path/to/project",
  "entries": [
    {
      "dir": "src/auth",
      "purpose": "JWT 토큰 검증 인증 모듈",
      "export_names": ["validateToken", "Claims", "TokenError"]
    }
  ]
}
```

## impl agent 워크플로우 요약

상세 Phase 0~7 워크플로우는 `impl-workflow.md`를 참조하세요.

| Phase | 요약 |
|-------|------|
| 0 | Scope Assessment — 3차원 증거 기반 완성도 분류 (D1: Purpose, D2: Requirements, D3: Domain Context) |
| 1 | Requirements Analysis — 4개 스펙 요소 추출 (Purpose, Requirements, Domain Context, Location) |
| 1.5 | dep-explorer 위임 — 의존성 탐색 |
| 2 | Tiered Clarification — 최대 2라운드, 라운드당 최대 4질문 |
| 3 | 대상 위치 결정 — 명시적 경로 > 모듈명 추론 > 사용자 선택 |
| 4 | 기존 CLAUDE.md 존재시 병합 — Purpose/Requirements/Domain Context smart merge |
| 5 | CLAUDE.md 생성 — v7 스키마 (Purpose, Requirements, Domain Context) |
| 5.25 | DEVELOPERS.md 생성 — Derived Spec (Constraints + Technical Context) |
| 5.5 | compile-context 생성 — Dependencies Direction, Implementation Approach |
| 6 | 스키마 검증 (1회) — validate-schema CLI |
| 6.5 | Plan Preview — 사용자 승인/범위조정/위치변경/취소 |
| 7 | 최종 저장 — CLAUDE.md + DEVELOPERS.md + compile-context |
