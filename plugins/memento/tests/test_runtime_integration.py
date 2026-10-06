"""Actual Rust CLI across installer, launcher, Codex callbacks and Git hooks.

Rotating builds use synthetic build identities with the actual Rust program;
they do not simulate three released versions or Codex's hook trust UI.
"""

import hashlib
from contextlib import closing
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest


PLUGIN = Path(__file__).resolve().parent.parent
PROJECT = "runtime-integration"
WORK = "upload-fix"


class RuntimeIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build_directory = tempfile.TemporaryDirectory(prefix="memento real build identities ")
        cls.addClassCleanup(cls.build_directory.cleanup)
        root = Path(cls.build_directory.name).resolve()
        selected = Path(os.environ.get("MEMENTO_TEST_BINARY", PLUGIN / "core/target/debug/memento"))
        if not selected.is_file():
            raise RuntimeError("Build core or set MEMENTO_TEST_BINARY to the actual Rust CLI.")
        cls.binaries = [root / "original-memento"]
        shutil.copy2(selected, cls.binaries[0])
        cls.versions = [cls.binary_version(cls.binaries[0])]
        for index in (2, 3):
            environment = os.environ.copy()
            environment.pop("CARGO_TARGET_DIR", None)
            environment["MEMENTO_BUILD_ID"] = hashlib.sha256(
                f"{cls.versions[0]['build_identity']}:actual-runtime-integration-build-{index}".encode()
            ).hexdigest()
            result = subprocess.run(
                ["cargo", "build", "--locked", "--manifest-path", str(PLUGIN / "core/Cargo.toml")],
                env=environment, text=True, capture_output=True, timeout=180, check=False,
            )
            if result.returncode:
                raise RuntimeError(result.stderr + result.stdout)
            destination = root / f"actual-memento-{index}"
            shutil.copy2(PLUGIN / "core/target/debug/memento", destination)
            cls.binaries.append(destination)
            cls.versions.append(cls.binary_version(destination))
        if len({version["build_identity"] for version in cls.versions}) != 3:
            raise RuntimeError("Expected three distinct actual Rust build identities.")

    @staticmethod
    def binary_version(binary):
        result = subprocess.run([str(binary), "version"], text=True, capture_output=True,
                                check=True, timeout=15)
        version = json.loads(result.stdout)
        if version.get("protocol_version") != 1 or "checkpoint" not in version.get("capabilities", []):
            raise RuntimeError("Integration tests require the real compatible Memento runtime.")
        return version

    def setUp(self):
        workspace = tempfile.TemporaryDirectory(prefix="memento cross layer ")
        self.addCleanup(workspace.cleanup)
        self.root = Path(workspace.name).resolve()
        self.plugin = self.root / "plugin cache first"
        self.plugin.mkdir()
        for name in ("scripts", "skills", "codex-hooks", "core"):
            shutil.copytree(PLUGIN / name, self.plugin / name,
                            ignore=shutil.ignore_patterns("__pycache__", "bin", "target"))
        shutil.copy2(PLUGIN / "plugin.json", self.plugin / "plugin.json")
        self.runtime = self.root / "persistent runtime"
        self.data = self.root / "persistent plugin data"
        self.database = self.root / "selected project data/context.sqlite"
        self.outside = self.root / "project outside plugin cache"
        self.outside.mkdir()
        self.environment = os.environ.copy()
        for name in ("MEMENTO_BIN", "MEMENTO_GIT_EXECUTABLE", "PLUGIN_ROOT", "PLUGIN_DATA"):
            self.environment.pop(name, None)
        self.environment.update(HOME=str(self.root / "isolated home"),
                                CODEX_HOME=str(self.root / "isolated codex"),
                                MEMENTO_RUNTIME_HOME=str(self.runtime))
        self.git("init", "--quiet", "--initial-branch=main")
        self.git("config", "user.name", "Memento Integration")
        self.git("config", "user.email", "memento@example.invalid")
        self.git("config", "commit.gpgsign", "false")

    def run_process(self, command, value=None, environment=None):
        result = subprocess.run(
            [str(argument) for argument in command],
            input=json.dumps(value, ensure_ascii=False) if value is not None else None,
            cwd=self.outside, env=environment or self.environment, text=True,
            capture_output=True, timeout=30, check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        return result

    def git(self, *arguments):
        result = self.run_process(["git", "-C", self.outside, *arguments])
        self.assertNotIn("local context capture failed", result.stderr)
        return result.stdout.strip()

    def install(self, index=0):
        response = self.run_process([
            sys.executable, self.plugin / "scripts/install_runtime.py", "--ensure",
            "--binary", self.binaries[index], "--embedding-model", "none",
        ])
        result = json.loads(response.stdout)
        self.assertEqual(result["status"], "ready")
        self.assertIsNone(result["semantic_config"])
        self.assertFalse((self.plugin / "skills/memento/bin/memento").exists())
        state = self.state()
        self.assertIsNotNone(state["active"]["requested_source_digest"])
        return state

    def state(self):
        return json.loads((self.runtime / "state.json").read_text())

    def cli(self, operation, value=None, options=()):
        arguments = [sys.executable, self.plugin / "skills/memento/scripts/memento.py",
                     operation, "--store", self.database, *options]
        if operation in ("init", "note", "hooks-install", "hooks-status"):
            arguments += ["--project", PROJECT]
        result = self.run_process(arguments, value)
        return json.loads(result.stdout)

    def init(self):
        self.cli("init", options=("--work", WORK, "--session", "session-1",
                                  "--title", "Upload correction", "--goal", "Preserve public process"))

    def scope(self):
        return {"project_id": PROJECT, "repository": str(self.outside),
                "work_id": WORK, "session_id": "session-1", "turn_id": "turn-1"}

    def note(self, identifier, kind, body):
        response = self.cli("note", {"id": identifier, "revision": "v1",
                                     "kind": kind, "body": body},
                            ("--work", WORK, "--session", "session-1"))
        self.assertTrue(response["receipt"]["durable"])
        return response["receipt"]

    def assert_note(self, identifier, body):
        result = self.cli("query", {"operation": "read", "scope": {"project_id": PROJECT},
                                    "target": {"kind": "record", "id": identifier}})
        self.assertEqual(result["items"][0]["entity"]["data"]["body"], body)

    def callback(self, event, **fields):
        value = {"hook_event_name": event, "cwd": str(self.outside),
                 "session_id": "session-1", "turn_id": "turn-1", **fields}
        result = self.run_process(
            [sys.executable, self.plugin / "codex-hooks/capture.py"], value,
            {**self.environment, "PLUGIN_ROOT": str(self.plugin), "PLUGIN_DATA": str(self.data)},
        )
        return json.loads(result.stdout)

    def test_shared_runtime_handles_real_note_query_and_configured_codex_without_bundled_bin(self):
        self.install()
        self.init()
        body = "재시도 횟수는 유지하고 업로드 작업자 수만 4개로 제한한다."
        self.note("decision", "decision", body)
        self.assert_note("decision", body)
        configured = self.run_process([
            sys.executable, self.plugin / "codex-hooks/capture.py", "configure",
            "--data-dir", self.data, "--repository", self.outside, "--store", self.database,
            "--project-id", PROJECT, "--work-id", WORK,
        ])
        self.assertFalse(json.loads(configured.stdout)["hook_trust_modified"])
        started = self.callback("SessionStart")
        self.assertIn(str(self.plugin / "skills/memento/SKILL.md"),
                      started["hookSpecificOutput"]["additionalContext"])
        self.callback("UserPromptSubmit", prompt="정정: 토큰 갱신 코드는 변경하지 마.")
        request = {"operation": "status", "scope": self.scope()}
        pending = self.cli("checkpoint", request)
        self.assertTrue(pending["pending_user_prompt"])
        for event in pending["events"]:
            if event["event_id"] not in pending["pending_event_ids"]:
                continue
            native = event["origin"]
            origin = self.cli("query", {
                "operation": "read", "scope": {"project_id": PROJECT},
                "target": {"kind": "artifact", "record_id": native["record_id"],
                           "revision": native["revision"]},
            })["items"][0]["entity"]["data"]
            receipt = self.cli("note", {
                "id": "correction", "revision": "v1", "kind": "constraint",
                "body": "토큰 갱신은 변경하지 않는다.", "representation": "claim",
                "derived": True, "fidelity": "summary_only", "nature": "reported",
                "context_id": event["context_id"],
                "evidence": [{"source_id": native["source_id"],
                              "record_id": native["record_id"], "revision": native["revision"],
                              "locator": f"checkpoint:{event['event_id']}",
                              "availability": origin["availability"], "purpose": "origin",
                              "span": {"start": 0, "end": len(origin["body"].encode("utf-8"))}}],
            }, ("--work", WORK, "--session", "session-1"))["receipt"]
            self.assertTrue(receipt["durable"])
            self.cli("checkpoint", {"operation": "resolve", "scope": self.scope(),
                                    "event_id": event["event_id"], "resolution": {"kind": "records", "records": [{
                                        "source_id": "journal", "record_id": "correction",
                                        "revision": "v1", "sequence": receipt["sequence"],
                                    }]}})
        self.assertEqual(self.callback("Stop"), {})
        self.assert_note("correction", "토큰 갱신은 변경하지 않는다.")

    def test_native_git_bridge_survives_cache_replacement_and_real_build_artifact_pruning(self):
        original_state = self.install()
        self.init()
        original_artifact = self.runtime / "artifacts" / original_state["active"]["directory"]
        preserved_log = self.outside / "user-hook-invocations"
        original_hook = self.outside / ".git/hooks/post-commit"
        original_hook.write_text("#!/bin/sh\nprintf '%s\\n' user-hook >> user-hook-invocations\n")
        original_hook.chmod(0o755)
        installed = self.cli("hooks-install", options=("--repository", self.outside))
        bridge = self.runtime / "git-memento"
        self.assertTrue(bridge.is_file())
        self.assertTrue(all(entry["runtime_target"] == str(bridge) for entry in installed["hooks"]))
        before_hook = original_hook.read_bytes()
        self.assertTrue((self.outside / ".git/hooks/post-commit.memento-original").is_file())
        for index in (1, 2):
            replaced = self.root / f"plugin cache update {index}"
            shutil.copytree(self.plugin, replaced)
            shutil.rmtree(self.plugin)
            self.plugin = replaced
            self.install(index)
        self.assertFalse(original_artifact.exists())
        artifacts = list((self.runtime / "artifacts").iterdir())
        self.assertEqual(len(artifacts), 2)
        self.assertEqual(original_hook.read_bytes(), before_hook)
        shutil.rmtree(self.plugin)
        (self.outside / "upload.txt").write_text("parallel=4 retries=3\n")
        self.git("add", "upload.txt")
        self.git("commit", "--quiet", "-m", "Keep upload retry policy after runtime updates")
        sha = self.git("rev-parse", "HEAD")
        self.assertEqual(preserved_log.read_text().splitlines(), ["user-hook"])
        with closing(sqlite3.connect(f"{self.database.as_uri()}?mode=ro", uri=True)) as database:
            stored = [json.loads(row[0]) for row in database.execute(
                "SELECT payload FROM stored_entries WHERE entity_key != '__work_context_compaction_v1__'"
            ).fetchall()]
        commits = [entity["data"] for entity in stored if entity["entity"] == "commit"]
        self.assertTrue(any(commit["sha"] == sha for commit in commits))
        commit = next(commit for commit in commits if commit["sha"] == sha)
        direct = self.run_process([bridge, "query", "--store", self.database],
                                  {"operation": "read", "scope": {"project_id": PROJECT},
                                   "target": {"kind": "commit", "repository_id": commit["repository_id"],
                                              "commit_sha": sha}, "budget_bytes": 64000})
        self.assertEqual(json.loads(direct.stdout)["items"][0]["entity"]["data"]["sha"], sha)
        self.assertEqual(self.binary_version(bridge)["build_identity"], self.versions[2]["build_identity"])

    def test_real_config_validation_rejects_future_fields_and_invalid_limits_preserving_good_state(self):
        self.install()
        self.init()
        body = "검증된 기존 설정과 프로젝트 데이터는 잘못된 업데이트로 바꾸지 않는다."
        self.note("preserved-context", "decision", body)
        worker = self.root / "valid fixture vector worker.py"
        calls = self.root / "fixture inference calls"
        worker.write_text(
            "import json, pathlib, sys\n"
            "request = json.load(sys.stdin)\n"
            f"with pathlib.Path({str(calls)!r}).open('a') as stream: stream.write('inference\\n')\n"
            "json.dump({'protocol': 1, 'model_id': request['model_id'], "
            "'model_revision': request['model_revision'], 'query': [1.0, 0.0], "
            "'documents': [[1.0, 0.0] for _ in request['documents']], 'metrics': {}}, sys.stdout)\n"
        )
        valid = {"command": [sys.executable, str(worker)],
                 "model_id": "fixture-config-contract", "model_revision": "fixed-1"}
        selected = self.root / "valid semantic configuration.json"
        selected.write_text(json.dumps(valid))
        cwd_before = set(self.outside.rglob("*"))
        checked = self.run_process([self.binaries[0], "semantic-config-check",
                                    "--semantic-config", selected])
        self.assertEqual(json.loads(checked.stdout), {"valid": True})
        self.assertEqual(set(self.outside.rglob("*")), cwd_before)
        prepared = self.run_process([sys.executable, self.plugin / "scripts/install_runtime.py",
                                     "--ensure", "--semantic-config", selected])
        self.assertEqual(json.loads(prepared.stdout)["status"], "ready")
        prior = self.state()["active"]
        retained_config = self.runtime / "artifacts" / prior["directory"] / "semantic-config.json"
        retained_bytes = retained_config.read_bytes()
        database_digest = hashlib.sha256(self.database.read_bytes()).hexdigest()
        inference_calls = calls.read_bytes()
        for label, extra in (("future-key", {"unsupported_future_option": True}),
                             ("invalid-limit", {"chunk_chars": 0})):
            with self.subTest(configuration=label):
                incompatible = self.root / f"{label}.json"
                incompatible.write_text(json.dumps({**valid, **extra}))
                result = subprocess.run(
                    [sys.executable, str(self.plugin / "scripts/install_runtime.py"),
                     "--ensure", "--semantic-config", str(incompatible)],
                    cwd=self.outside, env=self.environment, text=True,
                    capture_output=True, check=False, timeout=30,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Semantic configuration is incompatible", result.stderr)
                failed = self.state()
                self.assertEqual(failed["status"], "failed")
                self.assertEqual(failed["active"], prior)
                self.assertEqual(retained_config.read_bytes(), retained_bytes)
                self.assertEqual(hashlib.sha256(self.database.read_bytes()).hexdigest(), database_digest)
                self.assertEqual(calls.read_bytes(), inference_calls)
                self.assertFalse((self.runtime / "candidate").exists())
        self.assert_note("preserved-context", body)


if __name__ == "__main__":
    unittest.main()
