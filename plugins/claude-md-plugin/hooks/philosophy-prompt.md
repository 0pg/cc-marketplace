## MANDATORY: CLAUDE.md IS Your Source Code

In this project, CLAUDE.md files ARE the source code.
Source files (.ts, .py, .rs, etc.) are compiled output — do NOT read or edit them directly.

### Your Workflow for Every Request
1. **Read**: CLAUDE.md + IMPLEMENTS.md in the relevant directory — this is the code.
2. **Modify**: `/impl` (feature/spec change) or `/bugfix` (error/test failure) to update CLAUDE.md.
3. **Build**: `/compile` to regenerate source files from the updated spec.

### NEVER
- Edit source files (.ts, .py, .rs, .go, .java, etc.) directly.
- Search source files to understand behavior — read CLAUDE.md Exports instead.

### Exception: No CLAUDE.md
If the target directory has no CLAUDE.md, these rules do not apply. Work with source files directly as normal.

### Routing: Every Natural Language Request → Skill
Classify the user's request and invoke the matching skill BEFORE any code exploration:

**`/impl`** — the request describes something NEW or CHANGED:
- "Add login feature", "JWT 인증 모듈 필요", "change token expiry to 30 days"
- Signal words: add, create, need, change, update, improve, refactor

**`/bugfix`** — the request describes something BROKEN:
- "Login fails", "token validation 에러", "tests are failing", "TypeError at line 42"
- Signal words: fail, error, broken, crash, wrong, not working, fix

**Ambiguous** → ask user: "Is this a new feature (/impl) or a bug fix (/bugfix)?"
