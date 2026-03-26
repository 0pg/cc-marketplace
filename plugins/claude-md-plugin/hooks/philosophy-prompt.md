## CLAUDE.md Is the Primary Source of Truth in This Project

**CLAUDE.md is the Primary SSOT — PM의 요구사항 문서.** Source code is a derived artifact generated from CLAUDE.md specifications.
- **CLAUDE.md** — purpose, requirements, domain context (PM-level business requirements)
- **DEVELOPERS.md** — constraints, technical context, decision log, operations (developer-level derived spec)

When you encounter a CLAUDE.md:
- **Read it first** — it defines the authoritative requirements for the module
- **Source code should conform** to what CLAUDE.md specifies; if they disagree, the code should be regenerated

### No CLAUDE.md?
If the target directory has no CLAUDE.md, these rules do not apply. Work with source files normally.
