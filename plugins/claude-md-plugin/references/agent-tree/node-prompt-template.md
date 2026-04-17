# Node Prompt Template (v19 draft)

A node's `CLAUDE.md` is the **system prompt** for that node's agent. This file
is the required shape.

> Guideline: every section should earn its place by containing information the
> agent cannot derive faster by reading the code or its children's prompts.
> Prompt bloat directly costs context window — treat each line as liability.

## Required Shape

```markdown
# <node name>

## Identity
One or two sentences: what this agent is.

## Scope
The files and responsibilities owned by this node. What is inside the boundary,
and — explicitly — what is *outside* (delegated to children, handled by
siblings via the parent, or simply out-of-plugin).

## Responsibilities
Bulleted list of the jobs this agent performs. Each item should be concrete
enough that the agent can decide "is this my job? or do I delegate?".

## Tools
The source files inside this node that the agent invokes as tools. For each:
- **<file or module>** — one-line purpose; how to invoke (Bash command, test,
  edit pattern); key contract (inputs/outputs) if non-obvious from the code.
Only list tools whose role isn't immediately clear from the filename.

## Children
Direct child nodes (subdirectories that own their own CLAUDE.md). For each:
- **<child path>** — one-line summary of the child's domain, so the parent
  knows when to delegate. The child's own CLAUDE.md remains the source of
  truth; this is a pointer, not a duplicate.
Omit the section if the node has no children.

## Interaction Contract
How this agent is invoked, and what it returns.
- **Invoked by**: parent node / external workflow / user command.
- **Input**: what the caller must supply (task description, paths, args).
- **Output**: what the agent returns (file changes, stdout JSON, decision,
  etc.). Match the format the caller expects.

## Invariants
Rules this agent must uphold while operating. Scoped narrowly to this node.
Prefer checkable invariants (schema, boundary, naming) over abstract ones.

## Domain Context
Business or technical background the agent needs for judgment — constraints,
historical decisions, regulations, legacy quirks. Only what cannot be inferred
from code or children.
```

## Section Rules

- **Identity, Scope, Responsibilities, Interaction Contract**: required at
  every node.
- **Tools**: required when the node contains source; omit for pure
  organizational nodes.
- **Children**: required when the node has child CLAUDE.md files.
- **Invariants, Domain Context**: optional; include only when the content
  materially changes agent behavior.

## Hierarchical Composition

Claude Code auto-loads every `CLAUDE.md` from the project root down to the
current working directory. This means:

- A child's agent implicitly sees its ancestors' prompts.
- A parent prompt should state project-wide conventions and domain context
  **once** at the appropriate level; descendants should not restate them.
- A child prompt writes only what differs from its parent's contract.

This mirrors the DRY principle from v18's Conventions section but applies to
the whole prompt rather than just a single subsection.

## Anti-Patterns

- **Restating code as prose**: if the agent can learn it from a 10-second
  read, don't prose it into the prompt.
- **SSOT hangover**: listing public types, data schemas, or I/O contracts in
  prose. Put those in code (as types) and reference the file in *Tools*.
- **Duplicated children summaries**: deep children details belong in the
  child's own prompt; the parent's *Children* entry is a one-liner pointer.
- **Speculative invariants**: an invariant that has never been violated and
  cannot be mechanically checked is decoration — omit.
