#!/usr/bin/env python3
"""Integrity checks for the synthetic feedback replay's success conditions."""

import copy
import json
import tempfile
from pathlib import Path
import subprocess
import unittest
from unittest import mock

if __package__:
    from . import evaluate_memento_feedback as replay
else:
    import evaluate_memento_feedback as replay


class FeedbackReplayIntegrity(unittest.TestCase):
    def test_failed_cli_invocations_are_counted(self):
        cli = replay.Cli(Path("/fixture/memento"), Path("/fixture/context.sqlite"))
        failed = subprocess.CompletedProcess([], 1, stdout=b'{"error":"bad input"}', stderr=b"")
        with mock.patch.object(replay.subprocess, "run", return_value=failed) as execute:
            cli.call("record", "--input", "-", data=[], succeeds=False)
        execute.assert_called_once()
        self.assertEqual(len(cli.calls), 1)
        self.assertEqual(cli.calls[0]["returncode"], 1)
        self.assertEqual(replay.metrics(cli, 1)["failed_cli_calls"], 1)

    def test_missing_protected_history_cannot_report_success(self):
        _, expected = replay.seed_history()
        cli = mock.Mock()
        cli.query.return_value = {"items": []}
        with self.assertRaisesRegex(replay.ReplayFailure, "protected record missing"):
            replay.audit(cli, expected)

    def test_changed_original_reference_cannot_report_success(self):
        _, expected = replay.seed_history()
        saved = [copy.deepcopy(expected[key]) for key in
                 ("important-requirement", "important-decision", "important-verification")]
        saved[1]["evidence"][0]["revision"] = "wrong-origin-revision"
        cli = mock.Mock()
        cli.query.return_value = {"items": [{"entity": {"entity": "record", "data": row}} for row in saved]}
        with self.assertRaisesRegex(replay.ReplayFailure, "origin references changed"):
            replay.audit(cli, expected)

    def test_failed_replay_replaces_a_stale_success_artifact(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "metrics.json"
            output.write_text('{"status":"passed"}')
            arguments = ["evaluate", "--baseline-bin", "/fixture/base", "--candidate-bin", "/fixture/candidate",
                         "--output", str(output)]
            with mock.patch("sys.argv", arguments), mock.patch.object(
                    replay, "evaluate", side_effect=replay.ReplayFailure("fixture failed")), mock.patch("builtins.print"):
                self.assertEqual(replay.main(), 1)
            self.assertEqual(json.loads(output.read_text()), {"status": "failed", "error": "fixture failed"})

    def test_atomic_receipt_with_pending_event_cannot_report_success(self):
        claim = replay.record("request", "request", "Retain this request.", evidence=[])
        cli = mock.Mock()
        cli.call.return_value = {
            "receipts": [{"durable": True}], "records": [claim],
            "checkpoint": {"durable": True, "pending_event_ids": ["pending"]},
        }
        with self.assertRaisesRegex(replay.ReplayFailure, "checkpoint still pending"):
            replay.persist(cli, claim, {"event_id": "pending"}, {}, True)


if __name__ == "__main__":
    unittest.main()
