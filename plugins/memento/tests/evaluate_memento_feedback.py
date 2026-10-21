#!/usr/bin/env python3
"""Closed CLI replays for the Memento feedback changes, using synthetic history."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


PROJECT = "feedback-replay"
WORK = "feedback"
SESSION = "replay"
FOCUS = "GC collection source conditions"
CONDITIONS = ("gc-enabled", "gc-cycle")


class ReplayFailure(Exception):
    pass


def require(condition, message):
    if not condition:
        raise ReplayFailure(message)


class Cli:
    def __init__(self, binary, store):
        self.binary = str(binary)
        self.store = str(store)
        self.calls = []

    def call(self, operation, *arguments, data=None, succeeds=True):
        call = {"operation": operation, "returncode": None}
        self.calls.append(call)
        result = subprocess.run(
            [self.binary, operation, "--store", self.store, *arguments],
            input=json.dumps(data).encode() if data is not None else None,
            capture_output=True, check=False, timeout=30,
        )
        call["returncode"] = result.returncode
        try:
            reply = json.loads(result.stdout)
        except (ValueError, UnicodeError) as error:
            raise ReplayFailure(f"{operation} returned invalid JSON: {result.stderr.decode(errors='replace')}") from error
        require((result.returncode == 0) == succeeds, f"{operation}: unexpected exit {result.returncode}: {reply}")
        return reply

    def query(self, operation, **fields):
        return self.call("query", "--input", "-", data={
            "operation": operation, "scope": {"project_id": PROJECT, "work_ids": [WORK]},
            "limit": 100, "budget_bytes": 65536, **fields,
        })

    def checkpoint(self, operation, scope, **fields):
        return self.call("checkpoint", "--input", "-", data={"operation": operation, "scope": scope, **fields})


def record(record_id, kind, body, **fields):
    return {
        "id": record_id, "project_id": PROJECT, "source_id": "journal", "revision": "v1",
        "kind": kind, "nature": "reported", "fidelity": "original", "availability": "available",
        "title": record_id, "body": body, "work_ids": [WORK], "association": "explicit",
        "session_id": SESSION, **fields,
    }


def evidence_ref(record_id, revision="v1", purpose="origin"):
    return {"source_id": "journal", "record_id": record_id, "revision": revision,
            "locator": "fixture:" + record_id, "availability": "available", "purpose": purpose,
            "range": {"start_line": 1, "end_line": 1}}


def seed_history():
    rows = [
        ("old-request", "request", "Review the old dashboard layout.", "2026-01-01T00:00:00Z"),
        ("old-dashboard", "decision", "Approved the old GC dashboard colors.", "2026-01-02T00:00:00Z"),
        ("important-requirement", "constraint", "Preserve exact GC evidence origins.", "2026-02-01T00:00:00Z"),
        ("important-decision", "decision", "Use retained source records for GC inspection.", "2026-02-02T00:00:00Z"),
        ("important-verification", "verification", "The fixture GC source trace was verified.", "2026-02-03T00:00:00Z"),
        ("gc-enabled", "finding", "GC collection source conditions require the collector to be enabled.", "2026-03-01T00:00:00Z"),
        ("gc-cycle", "finding", "GC collection source conditions require a completed cycle.", "2026-03-02T00:00:00Z"),
        ("gc-deprecation", "finding", "GC has not been deprecated.", "2026-03-03T00:00:00Z"),
        ("recent-state", "status", "The current GC collection source conditions review is open.", "2026-10-20T00:00:00Z"),
    ]
    entities = []
    expected = {}
    for record_id, kind, body, occurred_at in rows:
        original = record(record_id + "-origin", "status", body, representation="evidence")
        claim = record(record_id, kind, body, representation="claim", derived=True,
                       fidelity="summary_only", context_id="fixture:" + record_id,
                       occurred_at=occurred_at, evidence=[evidence_ref(original["id"])])
        if kind == "decision":
            claim["decision_status"] = "accepted"
        if kind == "verification":
            claim["verification_outcome"] = "passed"
        entities.extend([{"entity": "record", "data": original}, {"entity": "record", "data": claim}])
        expected[record_id] = claim
    return entities, expected


def records_in(reply):
    return [item["entity"]["data"] for item in reply.get("items", [])
            if item.get("entity", {}).get("entity") == "record"]


def refs(value):
    return [{key: entry.get(key) for key in
             ("source_id", "record_id", "revision", "purpose", "range", "span", "availability", "locator")}
            for entry in value.get("evidence", [])]


def check_record(actual, expected):
    require(actual["body"] == expected["body"], f"body changed for {expected['id']}")
    require(actual["revision"] == expected["revision"], f"revision changed for {expected['id']}")
    require(refs(actual) == refs(expected), f"origin references changed for {expected['id']}")


def audit(cli, expected):
    reply = cli.query("timeline", filters={"record_kinds": ["constraint", "decision", "verification"]})
    saved = {row["id"]: row for row in records_in(reply)}
    for record_id in ("important-requirement", "important-decision", "important-verification"):
        require(record_id in saved, f"protected record missing: {record_id}")
        check_record(saved[record_id], expected[record_id])


def metrics(cli, workflow_calls):
    return {
        "workflow_cli_calls": workflow_calls,
        "audit_cli_calls": len(cli.calls) - workflow_calls,
        "total_cli_calls": len(cli.calls),
        "commands": dict(Counter(call["operation"] for call in cli.calls)),
        "failed_cli_calls": sum(call["returncode"] != 0 for call in cli.calls),
    }


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def clone_pair(base, directory, baseline, candidate):
    directory.mkdir()
    before = digest(base)
    clients = []
    for label, binary in (("baseline", baseline), ("candidate", candidate)):
        store = directory / (label + ".sqlite")
        shutil.copy2(base, store)
        require(digest(store) == before, "initial SQLite clone differs")
        clients.append(Cli(binary, store))
    return clients, before


def open_base(seed, path, baseline, scope, event_id, detail):
    shutil.copy2(seed, path)
    setup = Cli(baseline, path)
    reply = setup.checkpoint("open", scope, event_id=event_id, kind="user_prompt", detail=detail)
    event = next((event for event in reply["events"] if event["event_id"] == event_id), None)
    require(event is not None, "checkpoint event missing")
    return event, len(setup.calls)


def event_claim(event, body, record_id):
    origin = event["origin"]
    return record(
        record_id, "request", body, representation="claim", derived=True, fidelity="summary_only",
        context_id=event["context_id"], evidence=[{
            "source_id": origin["source_id"], "record_id": origin["record_id"], "revision": origin["revision"],
            "locator": "checkpoint:" + event["event_id"], "availability": "available", "purpose": "origin",
            "range": {"start_line": 1, "end_line": 1},
        }],
    )


def persist(cli, claim, event, scope, atomic):
    entity = {"entity": "record", "data": claim}
    if atomic:
        reply = cli.call("record", "--input", "-", data={
            "records": [entity], "resolve": {"scope": scope, "event_id": event["event_id"],
                                            "records": [{"source_id": "journal", "record_id": claim["id"]}]},
        })
        require(all(receipt["durable"] for receipt in reply["receipts"]), "non-durable atomic receipt")
        selected = next((row for row in reply["records"] if row["id"] == claim["id"]), None)
        require(selected is not None, "normalized atomic record missing")
        check_record(selected, claim)
        checkpoint = reply["checkpoint"]
    else:
        written = cli.call("record", "--input", "-", data=[entity])
        require(all(receipt["durable"] for receipt in written["receipts"]), "non-durable legacy receipt")
        selected = records_in(cli.query("read", target={"kind": "record", "id": claim["id"]}))
        require(len(selected) == 1, "legacy readback missing")
        check_record(selected[0], claim)
        receipt = next(receipt for receipt in written["receipts"] if receipt["entity_id"] == claim["id"])
        checkpoint = cli.checkpoint("resolve", scope, event_id=event["event_id"], resolution={
            "kind": "records", "records": [{"source_id": "journal", "record_id": claim["id"],
                                           "revision": selected[0]["revision"], "sequence": receipt["sequence"]}],
        })
    require(event["event_id"] not in checkpoint["pending_event_ids"], "checkpoint still pending")
    require(checkpoint["durable"], "checkpoint acknowledgment is not durable")


def replay_capture(seed, directory, baseline, candidate, scope, expected):
    body = "Use a thirty-second GC collection interval."
    base = directory / "capture-base.sqlite"
    event, setup_calls = open_base(seed, base, baseline, scope, "capture-update", body)
    clients, initial_hash = clone_pair(base, directory / "capture", baseline, candidate)
    results = {}
    for label, cli, atomic in zip(("baseline", "candidate"), clients, (False, True)):
        persist(cli, event_claim(event, body, "updated-request"), event, scope, atomic)
        measured = len(cli.calls)
        audit(cli, expected)
        results[label] = metrics(cli, measured)
    require(results["candidate"]["workflow_cli_calls"] < results["baseline"]["workflow_cli_calls"],
            "atomic capture did not reduce workflow calls")
    return {"initial_store_sha256": initial_hash, "setup_cli_calls": setup_calls, **results}


def replay_followup(seed, directory, baseline, candidate, scope, expected):
    body = "GC has not been deprecated, correct?"
    base = directory / "followup-base.sqlite"
    event, setup_calls = open_base(seed, base, baseline, scope, "explanation", body)
    checked = Cli(baseline, base)
    original = records_in(checked.query("read", target={"kind": "record", "id": "gc-deprecation-origin"}))
    require(len(original) == 1 and original[0]["body"] == expected["gc-deprecation"]["body"], "prior checked origin missing")
    setup_calls += len(checked.calls)
    clients, initial_hash = clone_pair(base, directory / "followup", baseline, candidate)
    results = {}
    for label, cli in zip(("baseline", "candidate"), clients):
        if label == "baseline":
            persist(cli, event_claim(event, body, "redundant-followup-request"), event, scope, False)
        else:
            reply = cli.checkpoint("resolve", scope, event_id=event["event_id"], resolution={
                "kind": "no_new_context",
                "reason": "Re-explain gc-deprecation@v1 and its checked origin; no requirement, decision or verification changed.",
            })
            require(event["event_id"] not in reply["pending_event_ids"], "explanation checkpoint still pending")
        measured = len(cli.calls)
        saved = records_in(cli.query("read", target={"kind": "record", "id": "gc-deprecation"}))
        require(len(saved) == 1, "reused claim missing")
        check_record(saved[0], expected["gc-deprecation"])
        audit(cli, expected)
        results[label] = metrics(cli, measured)
    require(results["candidate"]["workflow_cli_calls"] < results["baseline"]["workflow_cli_calls"],
            "explicit no-new-context policy did not reduce calls")
    return {"initial_store_sha256": initial_hash, "setup_cli_calls": setup_calls,
            "comparison": "Legacy unnecessary capture policy versus explicit reuse; baseline CLI already supports no_new_context.",
            **results}


def replay_brief(seed, directory, baseline, candidate, expected):
    clients, initial_hash = clone_pair(seed, directory / "brief", baseline, candidate)
    results = {}
    for label, cli in zip(("baseline", "candidate"), clients):
        cli.query("sources", limit=4, budget_bytes=8192)
        fields = {"purpose": "resume", "limit": 4, "budget_bytes": 8192}
        if label == "candidate":
            fields["focus"] = FOCUS
        reply = cli.query("brief", **fields)
        visible = {claim["record_id"] for section in reply["brief"]["sections"] for claim in section["claims"]}
        additional_start = len(cli.calls)
        if not set(CONDITIONS).issubset(visible):
            searched = cli.query("search", query={"text": FOCUS, "mode": "tokens"},
                                 filters={"record_kinds": ["finding"]})
            visible.update(row["id"] for row in records_in(searched))
        require(set(CONDITIONS).issubset(visible), "collection conditions were not reached")
        for record_id in CONDITIONS:
            original = records_in(cli.query("read", target={"kind": "record", "id": record_id + "-origin"}))
            require(len(original) == 1, "collection original missing")
            require(original[0]["body"] == expected[record_id]["body"], "collection original changed")
        additional = len(cli.calls) - additional_start
        measured = len(cli.calls)
        audit(cli, expected)
        results[label] = {**metrics(cli, measured), "additional_queries_to_conditions_and_originals": additional,
                          "conditions_in_initial_brief": sorted(set(CONDITIONS) & {
                              claim["record_id"] for section in reply["brief"]["sections"] for claim in section["claims"]})}
    require(results["candidate"]["additional_queries_to_conditions_and_originals"] <
            results["baseline"]["additional_queries_to_conditions_and_originals"],
            "focused brief did not reduce extra queries in this fixture")
    return {"initial_store_sha256": initial_hash, "retrieval_query_inputs": "Hook-shaped CLI requests; native hooks not executed.",
            **results}


def replay_invalid(seed, directory, baseline, candidate, expected):
    clients, initial_hash = clone_pair(seed, directory / "invalid", baseline, candidate)
    results = {}
    for label, cli in zip(("baseline", "candidate"), clients):
        first = record("schema-first", "status", "First fixture message.", representation="evidence")
        second = record("schema-second", "status", "Second fixture message.", representation="evidence")
        second.pop("title")
        batch = [{"entity": "record", "data": first}, {"entity": "record", "data": second}]
        rejected = cli.call("record", "--input", "-", data=batch, succeeds=False)
        if label == "candidate":
            error = rejected["error"]
            require(error["code"] == "invalid_record_schema", "schema diagnostic code missing")
            require(error["record_index"] == 1 and error["record_id"] == "schema-second", "record location missing")
            require(error["field_path"] == "/1/data/title" and error["hint"], "field path or repair hint missing")
        second["title"] = "schema-second"
        fixed = cli.call("record", "--input", "-", data=batch)
        require(all(receipt["durable"] for receipt in fixed["receipts"]), "corrected batch not durable")
        measured = len(cli.calls)
        audit(cli, expected)
        results[label] = {**metrics(cli, measured), "correction_retries": 1,
                          "repair_diagnostic_checked": label == "candidate"}
    return {"initial_store_sha256": initial_hash, "comparison": "Both versions make one invalid attempt and one correction; retry reduction is not claimed.",
            **results}


def evaluate(baseline, candidate):
    require(baseline.is_file() and candidate.is_file(), "both explicit binaries must exist")
    with tempfile.TemporaryDirectory(prefix="memento-feedback-replay-") as temporary:
        directory = Path(temporary).resolve()
        repository = directory / "repository"
        repository.mkdir()
        subprocess.run(["git", "init", "--quiet", str(repository)], check=True, capture_output=True)
        seed = directory / "seed.sqlite"
        setup = Cli(baseline, seed)
        setup.call("init", "--project", PROJECT, "--work", WORK, "--session", SESSION,
                   "--title", "Synthetic feedback replay", "--goal", "Replay feedback with retained originals")
        entities, expected = seed_history()
        setup.call("record", "--input", "-", data=entities)
        scope = {"project_id": PROJECT, "repository": str(repository), "work_id": WORK,
                 "session_id": SESSION, "turn_id": "replay-turn"}
        return {
            "status": "passed", "fixture": "synthetic", "baseline_binary": str(baseline),
            "candidate_binary": str(candidate), "initial_setup_cli_calls": len(setup.calls),
            "hook_internal_cli_calls": None,
            "notes": ["Counts are actual subprocess CLI calls, with audit calls reported separately.",
                      "Native hook callbacks and their internal subprocess counts are not measured.",
                      "The user's six observed calls are not treated as a minimum or a universal baseline.",
                      "Preserved requirements, decisions, verifications and origin references are asserted in every replay."],
            "capture": replay_capture(seed, directory, baseline, candidate, scope, expected),
            "explanatory_followup": replay_followup(seed, directory, baseline, candidate, scope, expected),
            "brief": replay_brief(seed, directory, baseline, candidate, expected),
            "invalid_batch": replay_invalid(seed, directory, baseline, candidate, expected),
        }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-bin", type=Path, required=True)
    parser.add_argument("--candidate-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    exit_code = 0
    try:
        result = evaluate(args.baseline_bin.resolve(), args.candidate_bin.resolve())
    except (ReplayFailure, OSError, subprocess.SubprocessError, ValueError) as error:
        result = {"status": "failed", "error": str(error)}
        exit_code = 1
    rendered = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    print(rendered, end="")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
