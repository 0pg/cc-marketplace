## claude-md-plugin Core Philosophy (Post-Compaction Refresh)

### Compile/Decompile Paradigm
CLAUDE.md (.h, WHAT) + IMPLEMENTS.md (.c, HOW) --compile--> Source Code (Binary)
Source Code (Binary) --decompile--> CLAUDE.md + IMPLEMENTS.md
CLAUDE.md is the Source of Truth. Source code is a derived artifact.

### Command Responsibility
| Command | CLAUDE.md | IMPLEMENTS.md | Source Code |
|---------|-----------|---------------|-------------|
| /impl | Create/Update | Planning Section | - |
| /compile | Read-only | Implementation Section | Generate |
| /decompile | Create | Create (full) | Read-only |
| /bugfix | Fix (L1) | Fix (L2) | Regenerate via /compile |
| /validate | Verify | - | Verify |

### Auto-Routing (replaces /dev)
When user describes a need in natural language:
- Feature request / new module / spec definition → invoke /impl
- Error, bug, test failure, broken behavior → invoke /bugfix
- Ambiguous → ask user to clarify

### Key Invariants
- Tree dependency: parent -> child only (no sibling, no child -> parent)
- CLAUDE.md + IMPLEMENTS.md are always 1:1 pairs
- Exports = Interface Catalog (reference this, not source code)
- Domain Context = decisions for deterministic recompilation
- Always modify CLAUDE.md/IMPLEMENTS.md first, then /compile to regenerate
