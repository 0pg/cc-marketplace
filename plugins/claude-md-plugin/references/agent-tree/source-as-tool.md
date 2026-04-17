# Source Code as Tools (v19 draft)

A node's source files are the agent's **tools**, in the same spirit as Claude
Code Skills or MCP tools: named capabilities the agent selects, invokes, and
composes to do work.

Source code is not a derivative of a spec. It is the capability itself. The
agent's job is to **use** and **evolve** those tools, not to reconstruct them
from prose.

## What Counts as a Tool

A tool is any source artifact inside the node that has a distinct externally
observable effect. Examples:

- A CLI subcommand (Rust binary, Python script, shell script).
- A library function with a well-defined public signature.
- A test suite whose "run" answers a question (does X hold?).
- A config or schema file that downstream code reads.

Not every file is a tool. Build manifests, lockfiles, fixtures, and private
helpers are *infrastructure* — the agent may read them, but they do not
appear in the `## Tools` list of the prompt.

## Tool Invocation Modes

An agent invokes a tool through Claude Code's framework primitives:

| Mode | Framework primitive | Typical use |
|------|---------------------|-------------|
| **Execute** | Bash | Run the tool and observe stdout/stderr/exit. CLI subcommands, scripts, test runs, build commands. |
| **Inspect** | Read, Glob, Grep | Understand a tool's signature, find callers, check invariants. |
| **Modify** | Edit, Write | Evolve the tool in place. Prefer Edit; use Write only for new files or complete rewrites. |
| **Verify** | Bash (tests / linters) | After Modify, confirm behavior via the node's test suite or the project's lint/typecheck. |

The agent is expected to chain these modes (inspect → modify → verify) as a
normal development cycle, not as a ceremonial workflow.

## Declaring Tools in the Prompt

The `## Tools` section in a node's CLAUDE.md lists tools the agent cannot
identify at a glance. For each, write:

- **Name** — the file path or symbol.
- **Purpose** — one line; what question does invoking it answer, or what
  effect does it have?
- **Invocation** — the exact shape: `<binary> <args>`, `pytest tests/foo.py`,
  `cargo run --bin x -- <args>`, etc.
- **Contract** — only if the code alone doesn't make it obvious: input
  preconditions, output format, side effects, failure modes.

Example:

```markdown
## Tools

- **`src/validate_schema.rs`** (CLI: `claude-md-core validate-schema <path>`)
  — verifies a CLAUDE.md against the v19 prompt shape.
  Stdout: `{"ok": bool, "errors": [...]}` on success; non-zero exit on I/O
  failure.

- **`tests/features/delegation.feature`** (run: `cargo test --test cucumber`)
  — BDD coverage for the delegation contract. Invoke when editing anything
  in `src/delegation/`.
```

## Tools vs Children

A tool lives inside the node; a child node is itself an agent with its own
tools. Use a child when the domain has enough cohesion to warrant its own
prompt, invariants, and responsibilities. Use a tool when the capability is
small enough that the parent agent can hold the whole picture.

Rule of thumb: if invoking the capability requires the agent to reason about
a distinct domain (different vocabulary, different users, different failure
modes), that's a child. Otherwise, it's a tool.

## Composition

The agent may chain tools in service of a single user task. The prompt does
not prescribe pipelines — the agent composes at runtime. Prompt authors
should describe tools as individual capabilities, not as scripted
choreographies.
