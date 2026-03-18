## CLAUDE.md Is the Contract in This Project

**Source code is the sole Source of Truth.** CLAUDE.md defines the contract that code must satisfy:
- **CLAUDE.md** — interfaces, behavior, dependencies (Contract = WHAT code must do)
- **DEVELOPERS.md** — file relationships, decision rationale, operations (WHY)

When code differs from the contract:
- **Code may need fixing** — use `/compile` to regenerate code that satisfies the contract
- **Contract may need updating** — if requirements changed, update CLAUDE.md intentionally

To understand a module, read its CLAUDE.md first for the contract, then check source code for the actual implementation.

### No CLAUDE.md?
If the target directory has no CLAUDE.md, these rules do not apply. Work with source files normally.
