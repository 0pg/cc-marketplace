#!/usr/bin/env python3
"""Compare exported, actual query responses with a simple retained-log baseline.

First run tests with MEMENTO_EVALUATION_DIR set, then pass that directory.
Counts UTF-8 bytes, not model tokens. Does not include private or revoked text.
"""
import argparse
import json
import re
from pathlib import Path


def encoded(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def baseline(corpus):
    latest = {}
    for entry in corpus["entries"]:
        entity = entry["entity"]
        data = entity["data"]
        key = (entity["entity"], data["project_id"], data.get("source_id"), data["id"])
        latest[key] = entry
    sources = {
        (entry["entity"]["data"]["project_id"], entry["entity"]["data"]["id"]):
        entry["entity"]["data"]
        for entry in latest.values() if entry["entity"]["entity"] == "source"
    }
    lines = []
    for entry in sorted(latest.values(), key=lambda entry: entry["sequence"]):
        if entry["entity"]["entity"] != "record":
            continue
        data = entry["entity"]["data"]
        source = sources.get((data["project_id"], data["source_id"]), {})
        if not source.get("authorized") or data["availability"] not in ("available", "redacted"):
            continue
        # A deliberately small raw-log envelope, without query metadata or duplication.
        lines.append(encoded({key: data[key] for key in ("id", "revision", "kind", "body")}))
    return b"\n".join(lines), len(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    rows = []
    for path in sorted(args.directory.glob("SC*.json")):
        if not re.fullmatch(r"SC\d{2}-[AB]\d*\.json", path.name):
            continue
        case = json.loads(path.read_text())
        raw, count = baseline(case["corpus"])
        responses = b"\n".join(encoded(result) for result in case["results"])
        row = {
            "case": path.stem,
            "query_count": len(case["results"]),
            "retained_record_count": count,
            "full_log_bytes": len(raw),
            "last_32k_log_bytes": min(len(raw), 32768),
            "actual_response_bytes": len(responses),
            "response_to_full_log_ratio": round(len(responses) / len(raw), 4) if raw else None,
        }
        if path.stem == "SC14-A":
            original_marker = b"SC14_MATCH HTTP 429"
            row["original_evidence_in_full_log"] = original_marker in raw
            row["original_evidence_in_last_32k_log"] = original_marker in raw[-32768:]
            row["original_evidence_in_responses"] = original_marker in responses
        rows.append(row)
    report = {
        "unit": "UTF-8 bytes, not tokens",
        "baseline": "latest authorized available/redacted record bodies in capture order, id/revision/kind envelope",
        "responses": "all exported query results, including retries/errors/duplicate evidence; compact JSON plus newlines",
        "excluded": "shared question, skill instructions, query requests, model generation, reviewer extra queries",
        "cases": rows,
    }
    text = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
