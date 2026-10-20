# Bounded context retention

Use this when inspecting storage growth, preparing a manual compaction, recording important process evidence, or continuing after `history_compacted` / `storage_capacity_exceeded`.

## What the rules retain

The implementation evaluates fixed Datalog rules with the embedded Crepe engine. Rust converts typed captured facts into retention roots and dependency edges; Crepe computes their least fixed point. Only rows outside that closure are eligible for removal. It does not generate a prose summary or infer dependencies from words in a message.

For the same snapshot, policy and incoming-write pins, retained rows and reports are deterministic. Results are sorted, and a separate ordered traversal selects bounded explanations from roots, independently of the engine's internal iteration order. Duplicate dependency facts do not change the retained set; duplicate captured sequence IDs remain invalid input.

Retention roots are:

- Latest Source and Work metadata, including revoked sources.
- Latest active or unknown Session metadata.
- Latest Request, Constraint, Decision, Feedback and Verification records. Rejected and superseded decisions remain process evidence; a partial supersession does not erase unrelated clauses.
- Failed, abandoned, running or outcome-unknown attempts.
- Latest Deleted/Missing records, preserving deletion/replay semantics.
- The newest 128 captured entries by default, plus the sequence frontier.
- Every receipt's entry in the transaction currently being admitted, including duplicate deliveries.

Retained rows keep their precise source/record/revision evidence, latest same-entity revision, relevant work/session metadata, explicit code/commit references and strong relations. Native execution/attempt references retain matching attempt/result records. Exact evidence identities are source scoped; ambiguous source-less references preserve matching candidates rather than selecting an arbitrary one. Work membership alone does not pull every old log back into memory. `related_to` is not a retention dependency.

Earlier revisions of a retained record also stay when they contain evidence links to another record or source. These historical links propagate a later original deletion or source revocation even if the latest revision cites different evidence. Pure self-references do not force every old revision to stay.

Old unreferenced tool output, progress/status, successful attempts, findings and unnecessary revisions can therefore disappear. Choose a derived claim's kind from its proposition, not from the source record's kind. Before handoff, check that important reusable context has a truthful durable root or an actual retention dependency path from one; `finding`/`change`, work membership, `context_id` and `related_to` alone do not provide that path. Preserve an independently meaningful decision, constraint or verification boundary only when the source supports it; do not invent kinds or links solely to retain a log. Important failed output must be captured and linked; the collector cannot reconstruct a never-recorded cause.

## Limits and automatic behavior

Default policy:

```json
{"max_entries":10000,"max_payload_bytes":67108864,"recent_entries":128}
```

The limit covers the entire selected SQLite store, across its projects and sources. It counts retained entry rows and the actual UTF-8 JSON `payload` bytes, including one internal metadata row. SQLite indexes, free pages, filesystem allocation, WAL/journal and external source files are not included. It is not a promise that the SQLite file is exactly 64 MiB. Deleted space is available for SQLite reuse; this command does not run `VACUUM` or erase independent backups.

Each successful write checks the store's persisted policy. If it would exceed either cap, collection and admission happen in one transaction. No protected fact is truncated to force a fit. If the retained closure still exceeds a cap, `storage_capacity_exceeded` rolls back that write or batch. The previous state and policy remain intact. A batch cannot delete an entry for which it just returned a receipt, including duplicate deliveries; link its captured evidence in the same `record` array when appropriate.

Old stores without metadata use the default policy. Opening or querying such a store does not silently compact it; explicit compaction or a subsequent write evaluates retention. A collection epoch and aggregate removal count are updated in one metadata row, so the compaction journal itself does not grow with each run.

## Preview and apply

```sh
memento compact --store /absolute/context.sqlite
memento compact --store /absolute/context.sqlite --compaction-policy /absolute/retention.json
memento compact --store /absolute/context.sqlite --compaction-policy /absolute/retention.json --apply true
```

The first two commands are read-only previews for an existing store. `--apply true` recomputes the plan against the current transaction, removes eligible rows and persists the selected policy. A plan that does not fit cannot be applied. `--policy` still selects the separate redaction policy; it is not the retention policy.

The report includes before/after usage, protected usage, rule version, fit status, retention reasons with their supporting sequence, and a sample of removed entity identities/reasons. Explanation and removal lists each stop at 200 entries and disclose the omitted count. Counts cover all rows; samples are not the full plan. Run this maintenance operation only against the explicitly selected store, since it covers all its projects.

## Reading after collection

- Old page cursors fail with `stale_cursor`; old change checkpoints fail with `rescan_required`. Start a fresh scoped query.
- Every later response includes `history_compacted`. A missing match means no match in retained history, not that an event never occurred.
- Reading a removed artifact revision returns a partial/unavailable explanation; it never substitutes a different revision.
- Semantic candidates prepared before collection cannot be reused. Library callers bind preparation with `PreparedSearch::with_compaction_generation(corpus.compaction.generation)`; the CLI does this automatically.
- Exact evidence referenced by a retained fact stays available if originally retained. Deletion or source revocation still scrubs original and dependent text, including historical revisions.

There is no automatic enlargement of limits, archival spillover, external summary model, or lossless retention of an unlimited number of independent decisions. If protected facts alone fill the budget, report that capture has stopped and review the policy explicitly. Deleting a record removes its content but retains the tombstone, so it need not release an entry slot.
