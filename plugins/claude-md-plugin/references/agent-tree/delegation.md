# Delegation Contract (v19 draft)

How a parent node agent hands off work to a child node agent, and how the
child returns results.

## Why Delegation

A single agent cannot hold the whole project in its context window, and even
if it could, a god-agent prompt would be mixed-concern and brittle. The tree
lets each agent run with a minimal, domain-focused prompt. Delegation is the
mechanism that lets a parent task flow down to the correct child without the
parent having to know the child's internals.

## Delegation Lifecycle

```
Parent agent receives a task
        │
        ▼
Parent classifies the task against its Responsibilities
        │
        ├── in-scope      → Parent executes using its own tools
        ├── out-of-scope  → Parent picks the child whose domain contains it
        └── ambiguous     → Parent clarifies with caller (don't guess)
        │
        ▼
Parent invokes child via Agent (Task)
        │
        ▼
Child runs in its own context, with its CLAUDE.md as system prompt
        │
        ▼
Child returns a result block matching the contract
        │
        ▼
Parent integrates the result and responds to its own caller
```

## Invocation Primitive

In Claude Code, the parent spawns a child agent using the `Agent` (Task) tool
with a node-scoped prompt. A concrete pattern:

```
Agent(
  description: "<short verb phrase>",
  subagent_type: "general-purpose",  # or a purpose-built type once step 4 lands
  prompt: <<~PROMPT
    You are the agent for node `<child path>`. Your system prompt is
    `<child path>/CLAUDE.md` — read it first, treat it as your contract.

    Task: <what the parent wants done>

    Inputs: <paths, arguments, prior results>

    Expected output: <format the parent will parse>
  PROMPT
)
```

The child's `CLAUDE.md` becomes its operating prompt; the parent's prompt is
the task message. The child sees its own ancestors' CLAUDE.md files via
Claude Code's hierarchical auto-load.

## Child Discovery

A parent learns about its children by:

1. Globbing for `CLAUDE.md` in direct subdirectories.
2. Reading the child's `## Identity` and `## Scope` to confirm the domain.
3. (Optional) Consulting its own `## Children` section, which is a cached
   one-liner map kept in sync when the child set changes.

Discovery is read-only. The parent must not modify a child's files directly —
that is the child agent's job.

## Task Message Shape

The parent's invocation message to a child should carry:

| Field | Purpose |
|-------|---------|
| **Task** | One-sentence goal. Imperative voice. |
| **Context** | Only the facts the child needs that are not in its own prompt or ancestors' prompts. |
| **Inputs** | Paths, prior results, arguments. Pass references, not verbatim blobs, when possible. |
| **Expected output** | The shape the parent will consume — a file, a JSON block, a verdict, a diff summary. |
| **Boundaries** | (Optional) explicit "do not touch X" when the default scope is insufficient. |

Keep the message as short as it can be. The child prompt already defines its
boundary and tools; the parent adds only the task-specific delta.

## Result Shape

The child returns a final message containing:

- A **summary** in prose (1–3 sentences).
- A **structured result block** matching the parent's expected output (JSON,
  file paths, verdicts). The parent should be able to parse it without
  heuristics.
- Any **follow-ups** the child recommends but did not do (belongs to sibling,
  parent, or user).

The child **does not** return its entire reasoning trace. Working notes stay
in the child's context; only the result crosses the boundary.

## Boundary Rules

1. **Parent reads child prompts, not child internals.** A parent inspects
   `<child>/CLAUDE.md` to decide where to delegate. It does not peek into a
   child's source to make judgments on the child's behalf.

2. **Children do not reach up.** A child must not `Read`/`Edit` files outside
   its own node. If it needs something from an ancestor or sibling, it
   returns a follow-up to the parent.

3. **Siblings do not cross-reference directly.** Cross-sibling work flows
   through the common ancestor, which delegates twice.

4. **System-wide scans are exempt.** Workflows that scan the whole tree
   (e.g. a future project-wide validator) operate read-only across nodes and
   do not count as boundary violations.

## Failure Modes

| Situation | Parent's move |
|-----------|---------------|
| No child's domain contains the task | Execute locally, or escalate to parent/user. |
| Multiple children plausibly match | Ask the caller (don't guess). |
| Child returns "out of scope" | Re-classify; delegate elsewhere or escalate. |
| Child fails (error / refusal) | Surface the failure with context; do not silently retry with the same inputs. |

## What This Contract Does Not Specify

- The exact grammar of the result block — that is set by each workflow's
  consumer (step 4 rebuild) and may vary by use case.
- Depth caps, retry counts, parallelism limits. A parent judges when to
  delegate, when to stop, and when to parallelize. Numeric limits are bug
  guards if and only if a concrete failure mode justifies them.
- How tools within a node are invoked — see `source-as-tool.md`.
