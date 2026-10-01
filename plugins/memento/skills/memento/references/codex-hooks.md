# Codex hooks and checkpoints

Use this reference when memento's plugin hooks supply capture scope or pending events. The hooks track lifecycle observations and ask the current Codex agent to save semantic context. They do not infer every decision from a transcript or run a separate model. Standalone skill use retains manual `note`/`record` capture without lifecycle gates.

## Package and enable

Install this plugin from the `jhk-plugins` marketplace, then prepare its local runtime as described in the plugin README. The portable root `plugin.json` declares `codex-hooks/hooks.json`; Claude Code keeps its `.claude-plugin/plugin.json` and does not automatically load these Codex-only hooks. The package contains Rust source and builds its runtime locally.

Plugin installation and hook trust are separate: review the current Codex hook definitions through the supported review UI (`/hooks` in the CLI). Changed definitions need review again. The runtime installer does not enable plugins or trust hooks. See [official packaging](https://developers.openai.com/plugins/build/plugins) and [hook trust](https://learn.chatgpt.com/docs/hooks#review-and-trust-hooks).

The skill allows implicit selection; its description names recording and retrieval triggers. A start/prompt hook can tell Codex to read the exact installed SKILL.md path when needed. This is model guidance, not a deterministic skill activation API. Use the skill selector for explicit invocation; a literal `$memento` in hook output is not equivalent to a user selector action.

## Explicit project scope

Configure the actual data directory supplied by Codex as `PLUGIN_DATA`, using the installed package's script. Use the directory reported by the hook's setup notice rather than guessing a cache/data path:

```sh
python3 /absolute/installed-plugin/codex-hooks/capture.py configure \
  --data-dir /absolute/plugin-data \
  --repository /absolute/project-repository \
  --store /absolute/context.sqlite \
  --project-id upload-app --work-id upload-429
```

The command updates `capture-config.json` in that selected data directory and records a canonical Git root with explicit store/project/work scope. There is one work mapping per working tree; change it explicitly when starting another work. Separate simultaneous works can use independently configured worktrees. Add `--initialize-store` only to initialize the selected store's journal source. Add `--policy /absolute/policy.json` to preserve an explicit literal-masking policy for checkpoint details, resolution reasons and initialization. Without it, the CLI's default masking applies. The command does not select an active task from the latest history. Hooks take the session and turn identity from Codex events. An unregistered repository does not receive a capture scope. Project records and receipts must match that exact scope.

## Capture and resolve

The examples abbreviate the installed `scripts/memento.py` launcher as `memento`. Hook context supplies the values below; reuse the event's IDs instead of inventing an event to satisfy a gate.

```sh
memento checkpoint --store /absolute/context.sqlite --input /tmp/checkpoint-status.json
```

`checkpoint-status.json`:

```json
{
  "operation": "status",
  "scope": {
    "project_id": "upload-app", "repository": "/absolute/project-repository",
    "work_id": "upload-429", "session_id": "codex-session-17", "turn_id": "turn-3"
  }
}
```

Status exposes the pending event identities in this project/repository/work/session. A continuation may have a different current turn; preserve an event's original turn identity when interpreting it. Status and stop checks cover that session's pending events, not only the latest turn. Hooks use `--summary true` for bounded ID samples and omitted counts; use the ordinary status request to inspect all retained events when a sample omits one.

Save new context with `note` or structured `record` and verify `durable` receipts. For each checkpoint inspect:

| Point | Context to save |
| --- | --- |
| Start/request | Requested outcome, constraints, open ambiguity before editing |
| Correction | Changed clauses, old decision being corrected, constraints still in force before acting |
| Decision/failure | Chosen and rejected approaches, observed adverse result, conditions; important output/patch before discard |
| Verification | Execution, code state, actual result, side effects, untested scope |
| Commit | Context for the actual staged changes, linked decisions and verification limitations |
| Final/handoff | New decisions and results, unresolved work, saved state and next action |

Use accessible public messages and outputs. Do not invent unavailable original dialogue, timestamps, or hidden reasoning. Record a reported explanation as `reported` and an inference as `inferred`. Read back the exact saved revision with a scoped `read` query when the write receipt does not include it. Use `receipt.sequence` from the write response (for `note`, `receipt` is nested inside the response), and `items[].entity.data.revision` from the matching record in the read response. The query's `checkpoint`, `query_snapshot`, and `rendered_revision` are different identities and must not replace the record revision or write sequence.

Resolve only with records captured after the event and explicitly assigned to the same work/session:

```json
{
  "operation": "resolve",
  "scope": {
    "project_id": "upload-app", "repository": "/absolute/project-repository",
    "work_id": "upload-429", "session_id": "codex-session-17", "turn_id": "turn-3"
  },
  "event_id": "UserPromptSubmit:turn-3",
  "resolution": {
    "kind": "records",
    "records": [
      {"source_id": "journal", "record_id": "correction-3", "revision": "exact-saved-revision", "sequence": 123}
    ]
  }
}
```

Pass this JSON to the same `checkpoint --input FILE` command (`--input -` accepts stdin). Replace the illustrative revision and sequence with actual values. References are checked against persisted accessible records, not accepted from a success sentence.

For a status-only request with no new context, use `{"kind":"no_new_context","reason":"Progress request only; no new decision or constraint."}`. A failed capture can be disclosed with `{"kind":"capture_incomplete","reason":"Required output was unavailable; the gap remains."}`; this reports a gap and does not turn it into successful capture. Stop feedback permits bounded repair rather than endless retries. Structural receipt checks do not certify semantic completeness.

## Commit boundary

If the optional Git gate is desired, enable it explicitly for the selected repository:

```sh
memento hooks-install --store /absolute/context.sqlite --project upload-app \
  --repository /absolute/project-repository --enforce-checkpoints true
```

Stage the authorized files first, then `checkpoint` with operation `prepare_commit`, the same explicit scope, a unique `event_id`, and a brief `detail`. This captures the actual parent HEAD and index tree. Write the staged change's semantic context after preparing, resolve that event with exact saved record references, then commit. `check_commit` with the same scope can validate readiness; a changed parent/index requires a new preparation and resolution. An unchanged-context resolution is not sufficient for a commit checkpoint.

Example prepare input:

```json
{
  "operation": "prepare_commit",
  "scope": {
    "project_id": "upload-app", "repository": "/absolute/project-repository",
    "work_id": "upload-429", "session_id": "codex-session-17", "turn_id": "turn-3"
  },
  "event_id": "commit-ready:upload-429-1",
  "detail": "Limit upload workers; retain retry behavior and verification limits."
}
```

Post-commit capture links the resulting SHA when the current reflog supplies the preceding HEAD (or the commit is initial). A missing or stale reflog leaves the actual Git result captured without inventing that association. It does not retroactively validate a test run against a different dirty tree. A Git hook observes a Git boundary, while the Codex hooks observe agent lifecycle events.

The command adapter recognizes direct commits and a small set of writes and test commands. Compound shell programs, aliases, script-internal changes, MCP tools, and later interactive input are outside its first-edit command gate. The actual Git gate must be enabled for commit-index enforcement at Git's `pre-commit` boundary; Git paths that bypass that hook remain outside its coverage. Handoff and important semantic decisions remain skill-guided; Stop validates the known obligations, not every sentence in the final answer. Large prompt/output details are truncated with a marker, and no full transcript is read. Setup or callback failures disclose `capture_incomplete`; recognized pre-edit/pre-commit validation failures return a deny decision. Codex's own handler failure/timeout behavior still applies.
