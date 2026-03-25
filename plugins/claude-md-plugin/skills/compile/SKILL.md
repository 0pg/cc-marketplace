---
name: compile
version: 2.0.0
aliases: [gen, generate, build]
description: |
  This skill should be used when the user asks to "compile CLAUDE.md to code", "generate code from CLAUDE.md", "implement CLAUDE.md",
  "create source files", or uses "/compile". Processes changed CLAUDE.md files in the target path (or all with --all flag).
  Performs 2-agent TDD workflow: test-designer (RED) → compiler (GREEN+REFACTOR) to ensure compiled code passes tests.
  Trigger keywords: 코드 생성, 컴파일, CLAUDE.md에서 코드
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write, Task, AskUserQuestion]
---

> **DEPRECATED (v6.0.0)**: This skill depends on `test-designer` agent and CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in Phase 2.

이 스킬은 현재 사용할 수 없습니다. `/impl` + `/bugfix`를 사용하세요.
