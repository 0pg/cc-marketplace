# Memento local interface

Invoke the skill as `$memento`. It uses the `memento` Rust package and CLI; `MEMENTO_BIN` can select an explicit executable.

## Setup and storage

Install the plugin from `jhk-plugins`. At project startup, run the installed package's `scripts/install_runtime.py --ensure` without waiting for an explicit update request, then retrieve the selected work's `sources` and bounded `brief` with purpose `resume`. Use fresh hook-provided evidence without repeating completed queries. Use `python3 <installed-skill>/scripts/memento.py COMMAND ...` for commands. Ordinary launcher commands call the same preparation routine as startup and the setup installer, then continue with the original command and its explicit scope. Codex may offer the optional `setup-memento` onboarding skill; skipping that conversation is supported. Installation alone is not an arbitrary post-install script or hook-trust operation.

From the installed plugin directory, explicit preparation and inspection are available:

```sh
python3 scripts/install_runtime.py --ensure
python3 scripts/install_runtime.py --status
python3 scripts/install_runtime.py --binary /absolute/built/memento --embedding-model none
python3 scripts/install_runtime.py --embedding-model minilm
python3 skills/memento/scripts/memento.py runtime-status
```

Source-only packages require Python 3.9+, Rust/Cargo 1.94+ and `uv` for the supplied embedding models. A compatible `--binary` skips Cargo; `--embedding-model none` skips model setup. A known new installation defaults to E5. Updates preserve the existing E5/MiniLM/custom selection. `none` also preserves an existing semantic configuration; it does not disable or delete that configuration. If a legacy installation has a binary but no recorded model selection or config, preparation returns `selection_required`; choose E5, MiniLM or `none` explicitly. A newly generated prebuilt package includes a format-1 selection hint to avoid this ambiguity.

The runtime directory is `${CODEX_HOME:-~/.codex}/memento/runtime`, overridden by `MEMENTO_RUNTIME_HOME`. State, verified executable artifacts, models and the stable Git bridge live here rather than in the replaceable plugin cache. `runtime-status` and installer `--status` inspect this location without preparing software or opening project stores. Help and version commands also never prepare; on an unprepared source-only package they can report that no executable is available. Readiness detects changed source, bundled executable or managed model setup identity; the startup ensure step or next real command prepares the update. A plugin version label change alone does not rebuild an unchanged runtime. An explicit `MEMENTO_BIN` remains the selected executable. This checks the installed package; fetching a new marketplace package remains the host's plugin update operation.

Preparation uses a process lock, validates protocol/capabilities, platform and build identity, and activates only a candidate that passes its checks and configured local inference. Memento retains at most the active and previous owned executable artifact and their model references, plus one preparation candidate. It prunes only Memento-owned directories and preserves unrelated files. The state distinguishes `not_installed`, `preparing`, `ready`, `failed`; binary existence alone is insufficient. Missing tools, offline dependency/model downloads, timeouts and failed inference surface errors; prerequisite tools are not silently installed.

For `query`, the launcher supplies the active semantic config unless `--semantic-config` is explicit. `MEMENTO_BIN` selects a compatible explicit executable and still undergoes the version handshake. Use the launcher to preserve selection/update checks rather than guessing an artifact filename. Direct executable calls require the semantic-config flag for semantic search. The examples below abbreviate the launcher as `memento`.

Codex hooks use `codex-hooks/hooks.json` via root `plugin.json`. Review and trust the current definitions in Codex and configure explicit project/store/work scope separately. Runtime preparation does not change trust, global settings or enable Git hooks. SessionStart checks readiness and retrieves `sources` plus a bounded `brief` for the configured work across prior sessions, or gives actionable setup guidance; the callback itself never builds or downloads a model. See [Codex hooks and checkpoints](codex-hooks.md).

### Version and selected-store compatibility

The current release separates plugin version `0.6.2`, Rust core version `0.3.0`, CLI protocol `1`, runtime configuration format `1`, and store format `2`. Inspect them independently:

```sh
memento version
memento store-status --store /absolute/context.sqlite
memento migrate --store /absolute/context.sqlite
```

The plugin launcher treats `store-status` as passive too: it requires an available compatible executable but does not prepare software. `version` returns build identity, OS/architecture, capabilities and store read/write ranges without opening a DB. `store-status` neither creates a missing store nor marks a legacy one; it returns `missing`, `migration_required` (known formats `0` and `1`) or `compatible`. Invalid schema/payload and future versions return compatibility errors instead of a success status. `migrate` only operates on the selected existing DB. Normal commands also upgrade a known legacy store when opening it; other stores are not scanned.

Check a supplied semantic configuration without opening a store or running its worker/model:

```sh
memento semantic-config-check --semantic-config /absolute/semantic-config.json
```

This passive command uses the Rust configuration schema to reject unknown fields, invalid types/ranges and an empty command, returning `{"valid": true}` only for a valid configuration. It requires an available compatible executable. Runtime preparation calls the checker through the candidate executable before activating a configured artifact; local inference is a separate check.

Migration from format `0` or `1` to `2` changes only the existing reserved metadata row in one SQLite transaction. Format `0` adds `store_format: {version, db_identity, migration_epoch}`; format `1` preserves `db_identity` and increments `migration_epoch`. Existing entity payloads, IDs, revisions, sequences, timestamps, evidence, policy, compaction and capture state are preserved; an absent legacy capture field becomes an empty default. It does not export/reinsert records, change the SQL schema, compact history or increase a policy limit. If metadata growth exceeds the configured limit, the migration fails. SQLite's transaction journal supplies rollback; no external plaintext DB snapshot is retained.

Writers coordinate using the empty `<store>.memento-lock` sidecar and recheck identity/epoch inside each transaction. Already open readers recheck the header as well. Busy, malformed, fenced or future stores fail without being replaced. The tested format `1` executable rejects the upgraded format `2` store, including writes from a handle opened before upgrade. The current plugin also rejects a format `1` executable during its runtime handshake before activation or selected-store access. These checks do not cover arbitrary older binaries or direct SQL access.

Runtime preparation does not restore DB snapshots. A failed update preserves the previous active runtime and returns an error; it does not execute the original command with an unchecked older binary. Switching a runtime cannot remove records written after a migration. The compatibility gate must succeed before any chosen executable accesses the selected DB.

Use an absolute SQLite path outside tracked source files, a stable project ID, and explicit source/work/session IDs. Toasty SQLite stores retained revisions locally; no remote service or LLM is required. Protect the store as project history. Default masking covers common credentials; pass the same `--policy /absolute/policy.json` on every capture that needs additional literal masking. When installing hooks, supply the same `--policy` option; the hook retains that explicit policy path for subsequent captures. Policy JSON: `{"literal_secrets":["a known sensitive value"]}`. Original raw exports remain at their selected source; this tool does not alter them.

Stores default to at most 10,000 stored entries and 64 MiB of retained JSON payload, including internal compaction metadata. Writes collect eligible history transactionally when either limit is exceeded; protected facts that do not fit reject the write. `compact` previews the rules and removal candidates; `compact --apply true` applies them. See [compaction](compaction.md) for policy, retained evidence, cursor invalidation and the distinction between payload usage and SQLite file size.

```sh
memento init --store /tmp/context.sqlite --project demo --work W1 --session S1 --title 'Retry behavior' --goal 'Avoid 429 without slowing normal requests'
memento note --store /tmp/context.sqlite --project demo --work W1 --session S1 --input /tmp/note.json
```

`note.json`:

```json
{"id":"U1","kind":"request","body":"Avoid 429 without increasing normal-request latency.","nature":"reported","actor":{"kind":"human","name":"user"}}
```

`note` requires `id`, `kind`, `body`. Optional fields use the Record schema below; unprovided nature is `reported`, occurrence time is unknown, association is explicit only when `--work` is supplied. Receipt `sequence/entity_id/duplicate/durable` is the persistence result. Do not invent timestamps. Unknown fields are rejected. Query failures return structured `error.code` with a nonzero exit code.

An automatic collection adds `compaction: {generation, removed_entries, remaining}` to the receipt. `record` arrays and each import batch commit atomically: their new entries are protected during admission and the entire batch fails if they cannot fit. A failed multi-command workflow can still have earlier successful receipts; report the actual saved portion.

## Query JSON

```sh
memento query --store /tmp/context.sqlite --input /tmp/query.json
```

Minimal query:

```json
{"operation":"brief","scope":{"project_id":"demo","work_ids":["W1"]},"purpose":"resume","limit":20,"budget_bytes":32768}
```

Eight operations:

| Operation | Additional fields | Result/use |
| --- | --- | --- |
| `sources` | scope | capabilities, coverage gaps, freshness |
| `list_work` | filters/query | known works and observed status |
| `search` | query, filters | matches, excerpts, matching fields and one-based `match_ranges` |
| `read` | target, optional range/context_lines | retained source record or exact artifact revision |
| `timeline` | scope, filters | source/session order; no inferred cross-session causality |
| `trace` | target, direction, relations, max_depth | evidence graph, default depth 3, maximum 32 |
| `compare` | from/to or since_checkpoint | history/code comparison or newly captured/revised evidence |
| `brief` | purpose, optional target | evidence package for resume/explain/investigate_failure/review |

Every query requires `scope.project_id`. Optional scope arrays: `work_ids`, `session_ids`, `source_ids`, `worktree_ids`. Different fields combine with AND; members of each array combine with OR. Limit is 1–100 (default 20), default serialized response budget is 64 KiB. Small impossible budgets return `budget_too_small`. `sort` is `oldest`, `newest`, or `relevance`; timeline uses source order. `query`: `{"text":"HTTP 429","mode":"literal"}` or case-insensitive all-token `tokens` mode.

Filters: `record_kinds`, `work_statuses`, `session_statuses`, `decision_statuses`, `attempt_outcomes`, `verification_outcomes`, `actors`, `paths`, `commit_shas`, `natures`. Each has `exclude_` counterpart. String path filters are exact repository-relative paths. Time fields: `occurred_from`, `occurred_to`, `as_of` (RFC3339), `time_unknown` boolean. Unknown occurrence times do not match a dated interval. A failed-attempt filter does not also match a linked verification; follow `trace` for it. Unsupported filters produce an error instead of silently broadening results.

Targets:

```json
{"kind":"record","id":"D1"}
{"kind":"work","id":"W1"}
{"kind":"session","id":"S1"}
{"kind":"artifact","record_id":"doc:document","revision":"opaque-source-revision","range":{"start_line":10,"end_line":25}}
{"kind":"code","state_id":"returned-state-id","path":"src/retry.rs","range":{"start_line":10,"end_line":25}}
{"kind":"commit","repository_id":"demo","commit_sha":"full-sha"}
```

For `code` targets, omit `state_id` only to inspect retained location candidates: `location.status` distinguishes exact, ambiguous, missing and unavailable. Multiple candidates require explicit selection; no current checkout is guessed. For explicit two-state movement tracking, add `code_mapping` as shown below; the original target remains unchanged.

For a text search hit, pass the returned body `match_ranges[].range` into `read.range` and add `context_lines` as needed; do not recompute a position from the excerpt.

Use one-based inclusive `range`; `context_lines` expands it, or requests neighboring records when no text range is supplied. A record's `revision` identifies its source revision; `rendered_revision` identifies the entire retained/masked text projection, even in a partial read. Never substitute a changed current document for the revision originally cited. `response_truncated` can continue; `source_truncated` means the source never retained the missing text.

`trace.direction`: `incoming`, `outgoing`, `both`; `relations` is an optional list of relation kinds. `compare.from/to` each accept `occurred_at`, `session_last_records`, and/or `code_state_id`. Code-only points cannot reconstruct historical requirements. `since_checkpoint` is an opaque token returned only after the final page; do not synthesize it. Keep occurrence and capture timestamps distinct.

## Code movement between retained states

```json
{"operation":"trace","scope":{"project_id":"demo"},"target":{"kind":"code","state_id":"before-state","path":"src/retry.rs","range":{"start_line":10,"end_line":25}},"code_mapping":{"target_state_id":"after-state","paths":["src/network/backoff.rs"]},"budget_bytes":65536}
```

Optionally select `code_mapping.target_source_id` within the query's accessible source scope. Observe both states and requested paths before querying; the mapping query does no Git checkout or file read. Only `read`/`trace` with explicit original/destination state IDs are accepted. `location.status` resolves the historical address; `mapping_status` reports `mapped`, `ambiguous`, `missing`, or `unavailable` separately. A failed mapping leaves the original read/trace useful.

Candidates include destination `CodeRef`, comparison method/version, supporting ranges, same-layer SHA-256 digests, symbols when available, and limitations. Symbol `byte_range` uses zero-based, end-exclusive offsets in the retained file, so distinct functions sharing one line remain distinguishable. Supported methods compare complete content, moved exact lines, Rust function token structure, and shared Rust statements. Rust function renames/moves are supported; macro expansion, type/binding analysis, arbitrary languages, and behavior equivalence are not. Copies or partial correspondences stay ambiguous. Absence means only absence within fully observed requested paths; unobserved paths, hash-only/masked content, parse errors, or unstable observation cannot prove deletion.

The comparison is bounded to 128 paths, 1 MiB per file, 8 MiB total text, 32,768 lines per file, and bounded AST candidates. Returned issues disclose incomplete comparison. A response with excessive mapping metadata may require narrower paths/ranges or a larger response budget. No mapping table or new causal/verification relationship is persisted.

For optional local semantic search, see [model setup, query, and lifecycle](semantic.md).

## Full records and links

`record --input FILE` accepts an Entity or array, encoded `{"entity":"record","data":{...}}`. Entity kinds: source/work/session/record/relation/code_state/commit. For new atomic capture, use a `record` batch for source evidence and its claims when the retained projection is known. `note` remains available for individual records; use actual returned/queryable revisions in links.

Record fields beyond the minimal note: `title`, `occurred_at`, `source_order`, `actor`, `work_ids`, `association` (`explicit/candidate/unassigned`), `session_id`, `worktree_id`, `paths`, `code_refs`, `commit_shas`, `evidence`, `decision_status`, `attempt_outcome`, `verification_outcome`, `attempt_id`, `execution`, `applies_to`, `alternatives`, `derived`, `partial`, `fidelity`, `availability`, `representation`, `context_id`. A revision may be supplied from the original source. `code_refs` carry `state_id/path/range`; evidence carries `source_id/record_id/revision/locator/availability/range`, optional `purpose` (`unspecified/origin/support`) and optional `span` (`start/end` UTF-8 byte offsets, end exclusive).

### Atomic source and claim batch

New capture separates source `representation: evidence` from semantic `representation: claim`. `legacy` is the default for existing records, not a way to certify new compound bodies. A claim is derived and needs an exact available `origin` into evidence or legacy source text in the same project. An origin cannot target another claim; `support` may reference earlier claims. A normalized claim uses `fidelity: summary_only`; a literal atomic excerpt can use `original`. Read [atomic capture](atomic-claims.md) before splitting conditions, corrections or verification scope.

After initializing the selected journal/work/session, `record --input FILE` accepts this batch. The context ID below is illustrative; for a pending checkpoint use the event's actual returned `context_id` and frozen origin instead of inventing them.

```json
[
  {"entity":"record","data":{"id":"U1","project_id":"demo","source_id":"journal","revision":"v1","kind":"status","nature":"reported","fidelity":"original","availability":"available","title":"Public user request","body":"Retry at most 3 times.\nDo not change token refresh.","actor":{"kind":"human","name":"user"},"work_ids":["W1"],"association":"explicit","session_id":"S1","representation":"evidence","context_id":"capture-transition-1"}},
  {"entity":"record","data":{"id":"C1","project_id":"demo","source_id":"journal","revision":"v1","kind":"constraint","nature":"reported","fidelity":"summary_only","availability":"available","title":"Retry bound","body":"Requests may be retried at most three times.","work_ids":["W1"],"association":"explicit","session_id":"S1","applies_to":["retry-limit"],"derived":true,"representation":"claim","context_id":"capture-transition-1","evidence":[{"source_id":"journal","record_id":"U1","revision":"v1","locator":"message:U1","availability":"available","purpose":"origin","span":{"start":0,"end":21}}]}},
  {"entity":"record","data":{"id":"C2","project_id":"demo","source_id":"journal","revision":"v1","kind":"constraint","nature":"reported","fidelity":"summary_only","availability":"available","title":"Token refresh scope","body":"Token refresh must remain unchanged.","work_ids":["W1"],"association":"explicit","session_id":"S1","applies_to":["token-refresh"],"derived":true,"representation":"claim","context_id":"capture-transition-1","evidence":[{"source_id":"journal","record_id":"U1","revision":"v1","locator":"message:U1","availability":"available","purpose":"origin","span":{"start":22,"end":50}}]}}
]
```

Measure spans against the saved body with `len(text.encode("utf-8"))`, not character counts. Every claim origin/support needs `span` or a one-based inclusive line `range`; whole-source references use the full retained range. When redaction can change the body, save/read the evidence first and measure the exact sanitized revision before admitting claims. Inspect durable receipts and read the exact saved revisions. Structural acceptance does not prove that all clauses were captured or that an inferred relationship is correct. Several claims may share one origin; changing one does not invalidate the others.

Record kinds: request/constraint/finding/decision/attempt/tool_result/change/verification/feedback/status/git_event. Nature: observed/reported/inferred. Fidelity: original/summary_only/source_truncated. Availability: available/redacted/missing/deleted/unsupported. Decision status: proposed/accepted/superseded/rejected/unknown. Attempt: succeeded/failed/abandoned/running/unknown. Verification: passed/failed/running/skipped/unknown.

### Typed execution metadata

An execution is an object, not a copy of arbitrary native metadata. The typed `execution` object is normalized metadata, not the retained native original; keep exact public field evidence or exact field spans in a faithfully retained original container when a claim depends on those fields, as described in [atomic capture](atomic-claims.md). `environment` is an object with `os`, `toolchain`, `profile` and `dependencies`; preserve a native environment label such as `local-debug` in `environment.profile`. A supplied `code_state` label maps to `before_state` or `after_state` according to when that state was observed. Do not add an unsupported `code_state` field or infer that a later state was verified. If state-observation timing is unknown, leave `before_state` and `after_state` unset and disclose the timing gap. Preserve the supplied label through faithfully retained source evidence with accurate provenance; do not add it to the original message body, invent a state observation, or repurpose an unrelated execution field to hold it.

This illustrative `execution` object shows all fields for a public command result whose source explicitly supplies its execution ID, command, completion, exit code, environment label and state observed at completion:

```json
{"id":"run-1","command":"cargo test parser","tool_name":null,"tool_input":null,"cwd":null,"started_at":null,"ended_at":null,"exit_code":0,"last_observed_state":"completed","observed_at":null,"liveness":"unknown","before_state":null,"after_state":"state-observed-1","scope":["parser tests"],"environment":{"os":null,"toolchain":null,"profile":"local-debug","dependencies":[]}}
```

Only `id`, `command` and `last_observed_state` are nonoptional strings. Do not invent an ID or command to fill this object; if either is unavailable, omit `execution`, preserve the supplied output and disclose the metadata gap. Preserve native execution IDs separately across retries. `tool_name` and `tool_input` are optional strings; serialize an accessible structured native input to a JSON string rather than supplying an object. `scope` and `dependencies` are string arrays. Optional fields use null or omission when the source does not supply them. Do not invent cwd, timestamps, toolchain, dependencies or current liveness; `liveness` is `running/stopped/unknown`, with `unknown` for historical results unless current liveness was actually observed. A missing completion/result uses `last_observed_state: "unknown"` and `exit_code: null`. Metadata preserves the observed execution scope; a separate verification claim still needs its own supported result and limitations.

A relation example (replace revisions with actual receipts/query evidence):

```json
{"entity":"relation","data":{"id":"D1-support","project_id":"demo","source_id":"journal","from":{"kind":"record","id":"E1"},"to":{"kind":"record","id":"D1"},"kind":"supports","nature":"reported","evidence":[{"source_id":"journal","record_id":"E1","revision":"actual-revision","locator":"record:E1","availability":"available"}],"applies_to":["normal-request latency"]}}
```

Relations: responds_to/supports/contradicts/supersedes/attempt_of/verifies/changes/related_to/forked_from/integrated_into/derived_from/reverts. Direction of `supersedes`: new → old; `derived_from`: new → origin; `reverts`: revert → reversed change. Treat relationship provenance as evidence, not automatic causality. `applies_to` scopes partial correction, contribution or verification.

## Import and sync

```sh
memento import --store /tmp/context.sqlite --project demo --source export-S1 --format codex --file /absolute/selected.jsonl --work W1
memento import --store /tmp/context.sqlite --project demo --source design --format document --file /absolute/design.md
memento sync --store /tmp/context.sqlite --project demo --source export-S1
```

Supported imports are explicitly selected UTF-8 files: Codex rollout JSONL (documented type/payload records), a text document, or portable journal JSONL (`--format journal`). No home-directory search. Imports preserve native message/call/ordinal IDs where present, plus explicitly provided parent/fork session metadata. A parent relationship does not imply completion or integration. Unresolved parent IDs remain labeled gaps. Fallback line IDs are only stable for append-only files; gaps disclose limitations. Private analysis/reasoning, encrypted payloads, and unsupported fields are excluded.

`import --completeness full_snapshot|partial|delta` defaults to `full_snapshot` for backward compatibility. Choose the mode explicitly for pages or incremental exports. Partial/delta input requires stable native record IDs: document fragments and Codex line-derived identities are rejected before storage. Delta means merge-only input; no deletion-event or incremental API protocol is implemented. Switching back to full_snapshot asserts that the selected file contains the full source history. Unsupported or malformed content prevents disappearance reconciliation. New product-specific adapters remain deferred.

Imported execution records preserve `tool_name` and `tool_input` separately from the compatibility `command`; session metadata preserves the actual `started_at` and `working_directory` when provided. Session ancestry is resolved within the same source kind, and missing parents remain unresolved.

Portable journal line: `{"id":"E1","kind":"tool_result","text":"HTTP 429","execution_id":"X1","work_id":"W1","session_id":"S1","nature":"observed"}`. Optional `occurred_at`, `tool`, `references`, `partial`, `source_truncated`, `evidence_level`. Use `record` for richer typed relationships. Plain agent prose is not automatically promoted to an accepted decision or verified success.

Sync re-reads selected known sources and preserves the source’s selected completeness mode. Source edits produce revisions. Only a sufficiently parsed `full_snapshot` reconciles absent records as missing; `partial`/`delta` retain absent history and disclose incomplete coverage. Past explicit work assignments survive replay; newly imported records with no explicit work remain unassigned. Provide `--work` only when the selected source range belongs to that work. Query revalidates registered paths: changed files are stale until sync; confirmed removal or access denial revokes cached content too.

## Commands and Git

```sh
memento observe --store /tmp/context.sqlite --project demo --repository /absolute/repo --path src/retry.rs
memento run --store /tmp/context.sqlite --project demo --work W1 --session S1 --id X1 --repository /absolute/repo --path src/retry.rs -- cargo test retry
memento git-sync --store /tmp/context.sqlite --project demo --repository /absolute/repo --ref HEAD --limit 20
memento hooks-status --store /tmp/context.sqlite --repository /absolute/repo
memento hooks-install --store /tmp/context.sqlite --project demo --repository /absolute/repo
```

`run` saves a started event before launching the explicitly requested argv, then output and before/after code observations. No shell is implied; use a new execution ID for a retry. Successful command execution is not automatically a verification claim. If killed before completion, last persisted running is historical, and current liveness is unknown. Only observations saved before a crash are guaranteed.

`observe` retains the selected regular text files and their code-state metadata without staging or changing branches. It rechecks the content digest before retention and masks text before storage and output. Ignored files and symlinks retain metadata only. Head/index object IDs and raw working SHA256 use different hash layers; compare like layers only. `git-sync` is bounded (1–100 commits, at most 32 refs); it cannot recover missed rewrite mappings or vanished conversations. Commit paths are first-parent-relative for merges. An origin worktree is recorded only when observed by the originating hook; reconciliation location is a separate observation.

Hooks coexist with existing executable hooks/core.hooksPath and preserve their output/exit behavior. The new capture step is local, bounded to five seconds, and reports failure; an already-created commit survives capture failure. Check installation and observed persistence separately.

`hooks-install --enforce-checkpoints true` additionally enables the optional Git pre-commit checkpoint gate. Its prerequisite is a fresh resolved commit checkpoint for the actual parent and staged tree; see [the commit checkpoint procedure](codex-hooks.md#commit-boundary). The default remains result capture without this gate. Codex lifecycle hooks are distributed by the plugin and are configured separately from these Git hooks.

`source-access --project demo --source ID --allow false` revokes and purges cached original/derived text. `delete-record --project demo --id ID` also prevents replay from resurrecting the deleted ID; explicitly record a new ID for a new replacement. Use `--allow true` plus explicit reimport only for an authorized source restored later. These commands require the same `--store` option.
