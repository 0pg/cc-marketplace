## CLAUDE.md Is the Primary Code in This Project

Every module's CLAUDE.md is the primary source of truth — read it first to understand any module:
- **CLAUDE.md** — interfaces, behavior, dependencies (WHAT)
- **IMPLEMENTS.md** — algorithm, constants, error handling (HOW)

Source files (.ts, .py, .rs, .go, .java) are generated from these documents.
To understand a module, read its CLAUDE.md first — not the source files.

### No CLAUDE.md?
If the target directory has no CLAUDE.md, these rules do not apply. Work with source files normally.
