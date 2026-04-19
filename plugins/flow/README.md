# flow — DAG-based task execution engine

A Claude Code plugin that turns a user request into a directed acyclic graph of atomic subtasks, executes independent nodes in parallel under git-worktree isolation, and performs merge+validate as an **independent, cascading step** — so integration failures cannot silently corrupt main.

## Why this exists

Most multi-agent workflows treat merge as a trivial byproduct of parallel work. When two agents edit the same file, results get concatenated and "it compiled" is declared success. `flow` makes the merge a named node with its own validation cascade and retry surface:

1. **git merge** (conflict-free?) →
2. **project tests** (`spec.test_cmd` passes?) →
3. **LLM semantic review** (reviewer agrees merge is coherent?)

First pass wins and the remaining steps are skipped. Fail all three → merger retries (up to `MAX_RETRIES`) and then halts with full context.

## Install

This plugin is published in the `jhk-plugins` marketplace.

```
/plugin marketplace add 0pg/cc-marketplace
/plugin install flow@jhk-plugins
```

### Build the core binary

`flow` ships a small Rust CLI at `core/` that enforces DAG structural invariants (acyclicity, referential integrity, R3/R5, enums) deterministically. Build it once after install:

```
cd "$CLAUDE_PLUGIN_ROOT/core"
cargo build --release
```

This produces `core/target/release/flow-core`. The SKILL shells out to this binary before accepting any `dag.json` from the planner. If the binary is missing, the SKILL halts with a `core-not-built` reason and prints the build instruction.

## Commands

| Command | Purpose |
|---------|---------|
| `/flow "<request>"` | Full pipeline: intake → interview → plan → approve → execute → report |
| `/flow-status [task-id]` | Show state summary (one task or all) |
| `/flow-resume <task-id>` | Resume a halted or interrupted task |
| `/flow-graph <task-id>` | Render the DAG as mermaid, annotated with status |

### Flags

- `--no-ask`: suppress interactive confirmation prompts (planner's self-review substitutes for user approval)
- `--max-retries N`: override per-node retry cap (default 3)

## Example

```
$ /flow "Add /health and /ready endpoints to the HTTP server, each with a unit test"
```

The pipeline then:

1. **Interviewer** asks follow-ups (framework? response body shape? `test_cmd`?), writes `spec.md`.
2. **Planner** produces `dag.json` with two parallel `work` nodes (`health-endpoint`, `ready-endpoint`) and one `merge` node with both as parents. Shows a mermaid preview.
3. You approve (or `--no-ask` accepts planner's self-review).
4. Orchestrator dispatches the two `work` nodes in parallel — each in its own git worktree on branch `flow/{task-id}/{node}`.
5. Each worker produces code + test, commits to its branch, returns success.
6. Orchestrator dispatches the `merge` node. Merger runs the cascade:
   - `git merge health-endpoint ready-endpoint` → no conflict → **step 1 PASS** → valid, break.
7. `/flow-status` shows all nodes `complete`. Merged branch is at `flow/{task-id}/merge-1`.

## State layout

```
.claude/workflows/flow/{task-id}/
├── spec.md                       ← finalized requirements
├── dag.json                      ← immutable DAG
├── state.json                    ← runtime per-node status
├── nodes/{node-id}/{meta,output,validator}.json/md
├── merges/{merge-id}/validators.json
└── worktrees/{node-id}/          ← git worktree (ephemeral)
```

`/flow-resume <task-id>` re-reads `state.json` and retries any `failed` or `running` nodes.

## Invariants

- **INV-F1**: `state.json[n].status = running` is persisted before `Task(agent)` dispatch. No orphaned in-flight work after interruption.
- **INV-F2**: Every fan-in (≥2 parents) is an explicit `merge` node. No implicit multi-parent workers.
- **INV-F3**: Merge cascade: step 1 → step 2 → step 4 (build-only is excluded; include build in `test_cmd` if needed). First PASS wins, break.
- **INV-F4**: Each work node runs in its own git worktree on its own branch.
- **INV-F5**: `dag.json` is immutable once execution begins. Re-planning = new task.
- **INV-F6**: `MAX_RETRIES=3` is a bug-guard; exhausted retries halt and surface full failure context.

## License

MIT. See the repository root.
