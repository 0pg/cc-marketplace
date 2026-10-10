# Codex hooks and checkpoints

Use this reference when memento's plugin hooks supply capture scope or pending events. The hooks track lifecycle observations and ask the current Codex agent to save semantic context. They do not infer every decision from a transcript or run a separate model. Standalone skill use retains manual `note`/`record` capture without lifecycle gates.

## Package and enable

Install this plugin from the `jhk-plugins` marketplace, then use its launcher or optional setup flow as described in the plugin README. The portable root `plugin.json` declares `codex-hooks/hooks.json`; Claude Code keeps its `.claude-plugin/plugin.json` and does not automatically load these Codex-only hooks. The package contains Rust source and builds its runtime locally.

Plugin installation and hook trust are separate: review the current Codex hook definitions through the supported review UI (`/hooks` in the CLI). Changed definitions need review again. The runtime installer does not enable plugins or trust hooks. See [official packaging](https://developers.openai.com/plugins/build/plugins) and [hook trust](https://learn.chatgpt.com/docs/hooks#review-and-trust-hooks).

The skill applies at project startup without an explicit Memento request. Start/prompt hooks provide the exact installed skill and preparation command, along with the selected scope. For a ready runtime, `SessionStart` also queries `sources` and a bounded `brief` for the configured work across prior sessions and supplies them as untrusted evidence. The agent reads relevant originals and refreshes context for the current request; hook queries do not establish that every relevant record was read. A literal `$memento` in hook output is not equivalent to a user selector action.

## Runtime readiness

At project startup the agent runs `scripts/install_runtime.py --ensure` for the installed package, using the same preparation routine as onboarding and ordinary launcher commands. This checks source, bundled binary and managed model setup identity, preserves the model selection, and prepares an update only when necessary. `SessionStart` checks readiness and retrieves context when ready; an unavailable or outdated runtime produces actionable preparation guidance. The bounded hook callback itself never runs Cargo or downloads models. Before configuring a source-only package with `--initialize-store`, prepare it through the same ensure command. Use `scripts/install_runtime.py --status` or the launcher `runtime-status` to inspect readiness without preparation. Runtime preparation targets the package already installed by the host; it does not fetch newer marketplace releases. The shared runtime home is separate from the hook-specific `PLUGIN_DATA` capture configuration.

Opening a selected known legacy store through the new CLI adds a version header transactionally. Runtime preparation alone does not open or migrate project stores. See [version and compatibility](interface.md#version-and-selected-store-compatibility) before using an older executable or responding to a compatibility error.

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

Before a supported investigation or edit tool runs, unresolved user requests and investigation results require resolution. The summary's `pending_user_prompt` and `pending_investigation` flags cover every retained event, including IDs omitted from the sample. Exact installed skill/reference reads, the installed `scripts/install_runtime.py --ensure` command, and standalone Memento calls remain available to complete preparation and capture. Use the launcher directly with `record --input -` or `checkpoint --input -` and supply JSON on stdin; creating an input file through a separate investigation/edit tool can itself encounter the gate. A compound command starting with Memento and continuing into another investigation is not exempt.

Completed ordinary shell/file reads and MCP results open `investigation` checkpoints. MCP inputs and results are bounded observations; the adapter does not infer whether an arbitrary MCP operation reads or writes. Before the next supported tool call, save new findings or changed decisions with their actual evidence, or resolve with a reasoned `no_new_context` when the result adds no meaningful context. Existing mutation, failure and verification checkpoints can still be recorded in a batch before Stop. A running shell process is not a completed investigation.

Save new context with `note` or structured `record` and verify `durable` receipts. For each checkpoint inspect:

| Point | Context to save |
| --- | --- |
| Start/request | Requested outcome, constraints, open ambiguity before editing |
| Correction | Changed clauses, old decision being corrected, constraints still in force before acting |
| Decision/failure | Chosen and rejected approaches, observed adverse result, conditions; important output/patch before discard |
| Investigation result | New findings and decision changes before the next tool; preserve the prior judgment and link the correction with its evidence and conditions |
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

An investigation's `records` resolution requires at least one finding or decision claim linked to that event's context and exact observed source. A raw output or status record alone is insufficient. For a status-only request or investigation result with no new context, use `{"kind":"no_new_context","reason":"Progress request only; no new decision or constraint."}`. A failed capture can be disclosed with `{"kind":"capture_incomplete","reason":"Required output was unavailable; the gap remains."}`; this reports a gap and does not turn it into successful capture. Stop feedback permits bounded repair rather than endless retries. Structural receipt checks do not certify semantic completeness.

## Commit boundary

If the optional Git gate is desired, enable it explicitly for the selected repository:

```sh
memento hooks-install --store /absolute/context.sqlite --project upload-app \
  --repository /absolute/project-repository --enforce-checkpoints true
```

When installed through the plugin launcher, newly generated Git hooks use the stable `<runtime-home>/git-memento` bridge. It resolves the active verified executable after plugin-cache replacement and owned artifact pruning, and never prepares software at a Git boundary. Inspect `hooks-status` after an update: managed hooks report their recorded `runtime_target` and target state; legacy hooks without that marker report `legacy_path_review_required`, and a removed target reports `missing`. Review/reinstall those hooks explicitly instead of assuming old paths are updated. Memento does not overwrite modified/user hooks or enable a gate while preparing runtime.

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

The adapter gates shell calls, file read/edit tools and MCP calls, and observes their completed results. It recognizes direct commits and a small set of writes and test commands. Compound shell programs and script-internal behavior receive ordinary investigation review when not otherwise classified; their embedded Git commits are not index-validated by this command adapter. Other administrative/local tools, hosted tools without lifecycle callbacks, aliases' internal behavior, and later interactive input remain outside that enforcement coverage. The actual Git gate must be enabled for commit-index enforcement at Git's `pre-commit` boundary; Git paths that bypass that hook remain outside its coverage. Handoff and important semantic decisions remain skill-guided; Stop validates the known obligations, not every sentence in the final answer. Large prompt/output details are truncated with a marker, and no full transcript is read. Setup or callback failures disclose `capture_incomplete`; supported pre-tool/pre-commit validation failures return a deny decision. Codex's own handler failure/timeout behavior still applies.
