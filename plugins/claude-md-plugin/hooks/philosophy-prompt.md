## CLAUDE.md Is the Primary Source of Truth in This Project

**CLAUDE.md is the Primary SSOT — PM의 요구사항 문서.** Source code is a derived artifact generated from CLAUDE.md specifications.
- **CLAUDE.md** — purpose, requirements, domain context (PM-level business requirements)
- **DEVELOPERS.md** — constraints, technical context, decision log, operations (developer-level derived spec)

When you encounter a CLAUDE.md:
- **Read it first** — it defines the authoritative requirements for the module
- **Source code should conform** to what CLAUDE.md specifies; if they disagree, the code should be regenerated

### No CLAUDE.md?
If the target directory has no CLAUDE.md, these rules do not apply. Work with source files normally.

### With a Process Plugin (e.g., superpowers)?

If a process discipline plugin (like superpowers) is active alongside claude-md-plugin:
- claude-md skills are **domain tools** within the process flow, not replacements for it
- **Planning phase**: Use `/impl` to formalize requirements into CLAUDE.md (not file-level code in plans)
- **Execution phase**: Use `/compile` to generate code — do NOT write source files directly
- **Verification phase**: Use `/compile --validate` or `/validate --strict` as completion evidence
- Plans should reference Skill invocations (`/impl`, `/compile`, `/validate`) as task steps, not file-level code blocks
