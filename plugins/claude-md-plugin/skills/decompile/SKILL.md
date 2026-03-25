---
name: decompile
version: 1.1.0
aliases: [decom]
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", or uses "/decompile" or "/decom".
  Analyzes existing source code (binary) and creates CLAUDE.md (source) documentation for each directory.
  Trigger keywords: 디컴파일, 코드에서 문서 추출, 기존 코드 문서화
user_invocable: true
allowed-tools: [Bash, Read, Write, Glob, Task, Skill]
---

> **DEPRECATED (v6.0.0)**: This skill's workflow assumes v5 CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in Phase 2.

이 스킬은 현재 사용할 수 없습니다. Phase 2에서 v6 스키마 기반으로 재설계 예정입니다.
