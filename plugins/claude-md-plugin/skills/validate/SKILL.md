---
name: validate
version: 2.0.0
aliases: [check, verify, lint]
description: |
  This skill should be used when the user asks to "validate CLAUDE.md", "check documentation-code consistency",
  "verify specification matches implementation", "check for drift", "check export coverage", "lint documentation", or uses "/validate". Runs validator agent for comprehensive validation.
  Trigger keywords: CLAUDE.md 검증, 문서 검증, drift 검사, 문서 린트, export 커버리지
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Edit, Task]
---

> **DEPRECATED (v6.0.0)**: This skill depends on `issue-verifier`/`violation-reporter` agents and CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. `validator` agent alone is v6-compatible. Full pipeline will be redesigned in Phase 2.

이 스킬은 현재 사용할 수 없습니다. `validator` agent 단독 실행은 가능하지만, 전체 파이프라인은 Phase 2에서 재설계 예정입니다.
