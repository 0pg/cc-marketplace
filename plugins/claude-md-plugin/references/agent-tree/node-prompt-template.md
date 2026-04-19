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

## Conventions
Tree-wide policies and design rules that apply to this node **and every
descendant**. Required at the project-root when the project has any
cross-cutting policy (almost always). Optional at non-root nodes — use it
only to *override* or *refine* an inherited rule.

Free-form structure; use whichever subsections match what your project
actually has. Common examples:
- Design Principles (layering, DI, functional core / imperative shell)
- Coding Rules (language-specific, beyond what linters catch)
- Naming (variables / functions / modules / files)
- Testing Strategy (TDD? BDD? coverage floor? runner? location?)
- Architectural Patterns (repository / hexagonal / event-driven)
- Shared Contracts (API response envelope, error format, logging format)

Inheritance and override:
- Descendants **inherit** the root's Conventions automatically via Claude
  Code's hierarchical auto-load.
- A child node may deviate. When it does, its own Conventions section must
  explicitly state **"overrides X from <ancestor> because Y"** — default is
  silent inheritance; deviation is documented.

Escape hatch when Conventions grows large:
- Factor subsections into sibling files at the project root (e.g.
  `CONVENTIONS.md`, `DESIGN-PRINCIPLES.md`, `CONTRACTS.md`). The root's
  `## Conventions` section then keeps a summary + pointers. All node agents
  may Read these sibling files (extension of the existing "project-root
  CLAUDE.md as shared contract" boundary allowance).

## Workspace Provisioning
Project-root nodes should declare who owns tree-wide build/test
infrastructure (`package.json`, `tsconfig.json`, `pnpm-workspace.yaml`,
`Cargo.toml`, lockfiles, etc.). Two patterns:

- **Root-owned**: the root node itself owns these files as tools. List
  them in `## Tools` and declare the commands needed to install
  dependencies and run tree-wide verification (e.g. `pnpm install`,
  `pnpm -w test`). Under this pattern, when a descendant executor
  reports `environment prerequisite unmet`, main ctx may auto-recover
  by dispatching a root-level executor to scaffold the missing
  artifact.
- **Out-of-tree**: the user, CI, or an external provisioning system
  owns these files. Declare the expected location and the reproduction
  command (e.g. "run `./scripts/bootstrap.sh` from repo root"). Under
  this pattern, `environment prerequisite unmet` surfaces directly to
  the user — main ctx does not attempt cross-boundary setup.

Omit this section at non-root nodes; provisioning is inherited from
the root by default.

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
- **Conventions**: required at project-root whenever the project has any
  tree-wide policy; optional at other nodes, and used only to override or
  refine an inherited rule.
- **Workspace Provisioning**: recommended at project-root when the tree
  has any shared build/test infrastructure (almost always); omit at
  non-root nodes.
- **Invariants, Domain Context**: optional; include only when the content
  materially changes agent behavior.

## Hierarchical Composition

Claude Code auto-loads every `CLAUDE.md` from the project root down to the
current working directory. This means:

- A child's agent implicitly sees its ancestors' prompts — including the
  root's `## Conventions` section.
- A parent prompt should state project-wide conventions and domain context
  **once** at the appropriate level; descendants should not restate them.
- A child prompt writes only what differs from its parent's contract, and
  documents any deviation in its own Conventions section.

The auto-load mechanism runs on every session re-initialization — startup,
resume, `/clear`, `/compact` — so tree-wide policies survive context resets
without additional machinery.

## Anti-Patterns

- **Restating code as prose**: if the agent can learn it from a 10-second
  read, don't prose it into the prompt.
- **SSOT hangover**: listing public types, data schemas, or I/O contracts in
  prose. Put those in code (as types) and reference the file in *Tools*.
- **Duplicated children summaries**: deep children details belong in the
  child's own prompt; the parent's *Children* entry is a one-liner pointer.
- **Speculative invariants**: an invariant that has never been violated and
  cannot be mechanically checked is decoration — omit.
