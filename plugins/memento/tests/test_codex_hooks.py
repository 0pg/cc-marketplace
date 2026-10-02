#!/usr/bin/env python3
"""Subprocess scenarios for the native Codex adapter using a fake durable CLI.

These exercise the shipped callback protocol, not Codex's hook activation/trust
or the Rust backend's evidence validation (covered by its own tests).
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

REPOSITORY = Path(__file__).resolve().parent.parent
TEMPLATE = REPOSITORY
REAL_BINARY = Path(os.environ.get("MEMENTO_TEST_BIN", REPOSITORY / "core/target/debug/memento")).resolve()
FAKE_CLI = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys
import time

if sys.argv[1] == "version":
    print(json.dumps({"protocol_version": 1, "package_version": "0.2.0",
        "build_identity": "a" * 64, "platform": {"os": {"darwin": "macos", "win32": "windows"}.get(sys.platform, sys.platform), "arch": {"arm64": "aarch64", "AMD64": "x86_64"}.get(__import__("platform").machine(), __import__("platform").machine())},
        "store_format": {"current": 1, "read": {"min": 1, "max": 1}, "write": {"min": 1, "max": 1}, "legacy": [0]},
        "capabilities": ["checkpoint", "store-status", "migrate"]}))
    raise SystemExit(0)

store = Path(sys.argv[sys.argv.index("--store") + 1])
state = json.loads(store.read_text()) if store.exists() else {"events": [], "calls": [], "attempts": {}}
if sys.argv[1] == "init":
    state["initialized"] = True
    state["init_policy"] = sys.argv[sys.argv.index("--policy") + 1] if "--policy" in sys.argv else None
    store.write_text(json.dumps(state))
    print("{}")
    raise SystemExit(0)
request = json.load(sys.stdin)
request["_summary"] = "--summary" in sys.argv and sys.argv[sys.argv.index("--summary") + 1] == "true"
request["_policy"] = sys.argv[sys.argv.index("--policy") + 1] if "--policy" in sys.argv else None
state["calls"].append(request)
behavior = os.environ.get("MEMENTO_TEST_BEHAVIOR", "")
if behavior == "fail":
    print("secret stderr must not escape", file=sys.stderr)
    raise SystemExit(1)
if behavior == "timeout":
    time.sleep(15)
if behavior == "oversize":
    print("x" * (128 * 1024))
    raise SystemExit(0)
scope = request["scope"]
base = {key: value for key, value in scope.items() if key != "turn_id"}
key = json.dumps(base, sort_keys=True)
operation = request["operation"]
events = [event for event in state["events"] if event["base"] == base]
if operation == "open":
    existing = next((event for event in events if event["event_id"] == request["event_id"]), None)
    if existing is None:
        event = {"base": base, "event_id": request["event_id"], "kind": request["kind"],
                 "detail": request["detail"], "original_turn_id": scope["turn_id"], "resolution": None}
        state["events"].append(event)
        events.append(event)
elif operation == "resolve":
    for event in events:
        if event["event_id"] == request["event_id"]:
            event["resolution"] = request["resolution"]
pending = [event for event in events if event["resolution"] is None]
incomplete = [event for event in events if event["resolution"] and event["resolution"]["kind"] == "capture_incomplete"]
decision = "block" if pending else ("capture_incomplete" if incomplete else "allow")
if operation == "stop" and pending:
    attempts = state["attempts"].get(key, 0)
    if attempts < 2:
        state["attempts"][key] = attempts + 1
    else:
        for event in pending:
            event["resolution"] = {"kind": "capture_incomplete", "reason": "retry limit"}
        incomplete.extend(pending)
        pending = []
        decision = "capture_incomplete"
if operation == "check_commit":
    decision = "allow" if not pending and state.get("prepared_commit") else "block"
state["last_scope"] = scope
store.write_text(json.dumps(state))
reply = {"scope": scope, "events": [] if request["_summary"] else events,
         "pending_event_ids": [event["event_id"] for event in pending][:8] if request["_summary"] else [event["event_id"] for event in pending],
         "pending_user_prompt": any(event["kind"] == "user_prompt" for event in pending),
         "capture_incomplete_event_ids": [event["event_id"] for event in incomplete][:8] if request["_summary"] else [event["event_id"] for event in incomplete],
         "omitted_pending_event_ids": max(len(pending) - 8, 0),
         "omitted_capture_incomplete_event_ids": max(len(incomplete) - 8, 0),
         "decision": decision, "durable": behavior != "nondurable"}
print(json.dumps(reply))
'''


class CodexHooks(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="memento-codex-hooks-")
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name).resolve()
        self.plugin = self.directory / "plugin with spaces"
        shutil.copytree(TEMPLATE, self.plugin, ignore=shutil.ignore_patterns("target", "__pycache__", "*.pyc", ".memento-semantic", "bin"))
        shutil.rmtree(self.plugin / "core", ignore_errors=True)
        self.data = self.directory / "plugin data"
        self.data.mkdir()
        self.repository = self.make_repository("project one")
        self.store = self.directory / "store.json"
        binary = self.plugin / "skills/memento/bin/memento"
        binary.parent.mkdir(parents=True, exist_ok=True)
        binary.write_text(FAKE_CLI)
        binary.chmod(0o755)
        (self.plugin / "skills/memento/SKILL.md").write_text("# Fake skill for adapter tests\n")
        self.env = {**os.environ, "PLUGIN_ROOT": str(self.plugin), "PLUGIN_DATA": str(self.data),
                    "MEMENTO_RUNTIME_HOME": str(self.directory / "runtime-cache")}
        self.configure()

    def make_repository(self, name):
        repository = self.directory / name
        repository.mkdir()
        subprocess.run(["git", "init", "--quiet", str(repository)], check=True, capture_output=True)
        return repository.resolve()

    def configure(self, repository=None, store=None, project="project", work="work", initialize=False, policy=None):
        args = [sys.executable, str(self.plugin / "codex-hooks/capture.py"), "configure",
                "--data-dir", str(self.data), "--repository", str(repository or self.repository),
                "--store", str(store or self.store), "--project-id", project, "--work-id", work]
        if initialize:
            args.append("--initialize-store")
        if policy is not None:
            args.extend(["--policy", str(policy)])
        result = subprocess.run(args, capture_output=True, env=self.env, check=False, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        return json.loads(result.stdout)

    def payload(self, event, **fields):
        return {"hook_event_name": event, "cwd": str(self.repository), "session_id": "session",
                "turn_id": "turn", **fields}

    def run_hook(self, event, behavior="", raw=None, **fields):
        body = json.dumps(self.payload(event, **fields)).encode() if raw is None else raw
        result = subprocess.run([sys.executable, str(self.plugin / "codex-hooks/capture.py")],
                                input=body, capture_output=True, check=False, timeout=5,
                                env={**self.env, "MEMENTO_TEST_BEHAVIOR": behavior})
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertLess(len(result.stdout), 16 * 1024)
        self.assertEqual(result.stderr, b"")
        return json.loads(result.stdout)

    def state(self, store=None):
        selected = store or self.store
        return json.loads(selected.read_text()) if selected.exists() else {"events": [], "calls": []}

    def resolve_all(self, kind="records"):
        state = self.state()
        for event in state["events"]:
            event["resolution"] = {"kind": kind, "reason": "scenario evidence"}
        self.store.write_text(json.dumps(state))

    def tool(self, event, command=None, name="Bash", response=None, tool_id="tool", **fields):
        return self.run_hook(event, tool_name=name, tool_use_id=tool_id,
                             tool_input={"command": command} if command is not None else {},
                             tool_response=response, **fields)

    def test_codex_hooks_are_separate_from_claude_default_discovery(self):
        self.assertFalse((self.plugin / "hooks/hooks.json").exists())
        claude = json.loads((self.plugin / ".claude-plugin/plugin.json").read_text())
        legacy = json.loads((self.plugin / ".codex-plugin/plugin.json").read_text())
        portable = json.loads((self.plugin / "plugin.json").read_text())
        self.assertNotIn("hooks", claude)
        self.assertEqual(legacy["hooks"], "./codex-hooks/hooks.json")
        self.assertEqual({claude["version"], legacy["version"], portable["version"]}, {"0.5.0"})

    def test_template_is_synchronous_and_contains_all_required_events(self):
        manifest = json.loads((self.plugin / "plugin.json").read_text())
        self.assertEqual(manifest["extensions"]["com.openai"]["hooks"], "./codex-hooks/hooks.json")
        config = json.loads((self.plugin / "codex-hooks/hooks.json").read_text())
        self.assertEqual(set(config["hooks"]), {"SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse",
                                               "Stop", "PreCompact", "Interrupt", "SessionEnd"})
        for groups in config["hooks"].values():
            for group in groups:
                for hook in group["hooks"]:
                    self.assertEqual(hook["type"], "command")
                    self.assertFalse(hook.get("async", False))

    def test_configure_is_explicit_upsert_without_store_or_trust_changes(self):
        result = self.configure(work="next work")
        self.assertFalse(self.store.exists())
        self.assertFalse(result["hook_trust_modified"])
        projects = json.loads((self.data / "capture-config.json").read_text())["projects"]
        self.assertEqual(len(projects), 1)
        self.assertEqual(projects[0]["work_id"], "next work")
        self.configure(initialize=True)
        self.assertTrue(self.state()["initialized"])

    def test_configured_custom_policy_is_used_for_checkpoint_and_initialization(self):
        policy = self.directory / "redaction-policy.json"
        policy.write_text(json.dumps({"literal_secrets": ["fixture-private-literal"]}))
        self.configure(policy=policy, initialize=True)
        self.assertEqual(self.state()["init_policy"], str(policy))
        self.run_hook("UserPromptSubmit", prompt="Request")
        self.assertEqual(self.state()["calls"][-1]["_policy"], str(policy))
        self.configure(work="new-work")
        project = json.loads((self.data / "capture-config.json").read_text())["projects"][0]
        self.assertEqual(project["policy"], str(policy))

    def test_start_exposes_actual_skill_scope_and_compact_reply_request(self):
        result = self.run_hook("SessionStart", turn_id=None)
        context = result["hookSpecificOutput"]["additionalContext"]
        self.assertIn(str(self.plugin / "skills/memento/SKILL.md"), context)
        self.assertIn(str(self.repository), context)
        call = self.state()["calls"][-1]
        self.assertTrue(call["_summary"])
        self.assertEqual(call["scope"]["turn_id"], "session-start")

    def test_missing_config_start_gives_actual_data_path_only_once_per_event(self):
        (self.data / "capture-config.json").unlink()
        result = self.run_hook("SessionStart")
        self.assertIn(str(self.data / "capture-config.json"), result["hookSpecificOutput"]["additionalContext"])
        self.assertEqual(self.run_hook("UserPromptSubmit", prompt="unconfigured"), {})
        self.assertFalse(self.store.exists())

    def test_unregistered_repository_and_unknown_events_do_not_invoke_backend(self):
        other = self.make_repository("unregistered")
        self.assertEqual(self.run_hook("UserPromptSubmit", cwd=str(other), prompt="other project"), {})
        self.assertEqual(self.run_hook("UnknownEvent"), {})
        self.assertFalse(self.store.exists())

    def test_correction_requires_new_request_record_before_next_change(self):
        self.run_hook("UserPromptSubmit", prompt="Keep everything local; add deterministic capture.")
        denied = self.tool("PreToolUse", name="apply_patch")
        self.assertEqual(denied["hookSpecificOutput"]["permissionDecision"], "deny")
        self.resolve_all()
        self.assertEqual(self.tool("PreToolUse", name="apply_patch"), {})
        self.run_hook("UserPromptSubmit", prompt="Correction: do not import raw transcripts.", turn_id="correction")
        self.assertEqual(self.tool("PreToolUse", "touch source.rs")["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_same_active_turn_correction_is_new_but_repeat_delivery_is_idempotent(self):
        self.run_hook("UserPromptSubmit", prompt="First request")
        self.resolve_all()
        for _ in range(2):
            self.run_hook("UserPromptSubmit", prompt="Correction during this same active turn")
        self.assertEqual(len(self.state()["events"]), 2)
        self.assertEqual(self.state()["events"][0]["original_turn_id"], self.state()["events"][1]["original_turn_id"])
        self.assertEqual(self.tool("PreToolUse", name="apply_patch")["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_missing_turn_is_incomplete_except_for_session_start(self):
        result = self.run_hook("UserPromptSubmit", prompt="request", turn_id=None)
        self.assertIn("capture_incomplete", result["systemMessage"])
        self.assertFalse(self.store.exists())

    def test_mutations_batch_until_stop_without_per_patch_record_gate(self):
        self.run_hook("UserPromptSubmit", prompt="Implement capture.")
        self.resolve_all()
        result = self.tool("PostToolUse", name="apply_patch", response={"output": "changed source"})
        self.assertIn("additionalContext", result["hookSpecificOutput"])
        self.assertEqual(self.state()["events"][-1]["kind"], "mutation")
        self.assertEqual(self.tool("PreToolUse", name="apply_patch", tool_id="next edit"), {})
        self.assertEqual(self.run_hook("Stop")["decision"], "block")

    def test_many_checkpoints_show_bounded_ids_and_keep_full_prompt_gate(self):
        for index in range(10):
            self.tool("PostToolUse", "touch file.rs", response={"exit_code": 0}, tool_id=f"mutation {index}")
        result = self.run_hook("UserPromptSubmit", prompt="New requirement", turn_id="later request")
        self.assertIn("Additional pending IDs omitted: 3", result["hookSpecificOutput"]["additionalContext"])
        self.assertEqual(self.tool("PreToolUse", name="apply_patch")["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_failure_then_verification_preserves_both_observations(self):
        self.tool("PostToolUse", "cargo test", response={"exit_code": 101, "output": "failed test"})
        self.tool("PostToolUse", "cargo test", response={"exit_code": 0, "output": "passed"}, tool_id="fixed")
        self.assertEqual([event["kind"] for event in self.state()["events"]], ["tool_failure", "verification"])

    def test_bare_pytest_and_python_verification_are_recognized(self):
        self.tool("PostToolUse", "pytest", response={"exit_code": 0})
        self.tool("PostToolUse", "python3 -m unittest", response={"exit_code": 0}, tool_id="unittest")
        self.assertEqual([event["kind"] for event in self.state()["events"]], ["verification", "verification"])

    def test_reads_status_and_memento_operations_do_not_create_recursive_records(self):
        for command in ["ls", "git status", "cat file.rs", "memento record --input -", "memento query",
                        "python3 /tmp/memento.py checkpoint --input -", "memento query; git status",
                        "python3 /tmp/capture.py; echo ok"]:
            self.assertEqual(self.tool("PostToolUse", command, response={"exit_code": 0}), {})
            self.assertEqual(self.tool("PreToolUse", command), {})
        self.assertFalse(self.store.exists())

    def test_own_cli_failure_also_does_not_open_recursive_checkpoint(self):
        self.assertEqual(self.tool("PostToolUse", "memento checkpoint --input -", response={"exit_code": 1}), {})
        self.assertFalse(self.store.exists())

    def test_running_exec_is_not_completion_and_native_retry_is_idempotent(self):
        self.assertEqual(self.tool("PostToolUse", "cargo test", response={"session_id": 23, "exit_code": None}), {})
        self.assertFalse(self.store.exists())
        for _ in range(2):
            self.tool("PostToolUse", "cargo test", response={"exit_code": 0})
        self.assertEqual(len(self.state()["events"]), 1)

    def test_stop_continuations_preserve_original_turn_and_retry_budget(self):
        self.run_hook("UserPromptSubmit", prompt="Original request")
        for continuation in ["continuation one", "continuation two"]:
            result = self.run_hook("Stop", turn_id=continuation)
            self.assertEqual(result["decision"], "block")
            self.run_hook("UserPromptSubmit", prompt=result["reason"], turn_id=continuation)
        self.assertEqual(len(self.state()["events"]), 1)
        self.assertEqual(self.state()["events"][0]["original_turn_id"], "turn")
        result = self.run_hook("Stop", turn_id="continuation three")
        self.assertIn("capture_incomplete", result["systemMessage"])
        self.assertNotIn("decision", result)

    def test_precompact_only_checks_existing_records_and_does_not_generate(self):
        self.run_hook("UserPromptSubmit", prompt="Do not lose this constraint")
        result = self.run_hook("PreCompact")
        self.assertFalse(result["continue"])
        self.assertEqual(self.state()["calls"][-1]["operation"], "status")
        self.assertEqual(len(self.state()["events"]), 1)
        self.resolve_all()
        self.assertEqual(self.run_hook("PreCompact"), {})

    def test_acknowledged_capture_gap_warns_without_permanent_compaction_block(self):
        self.run_hook("UserPromptSubmit", prompt="request")
        self.resolve_all("capture_incomplete")
        result = self.run_hook("PreCompact")
        self.assertIn("capture_incomplete", result["systemMessage"])
        self.assertNotIn("continue", result)

    def test_commit_gate_delegates_binding_check_and_requires_prepared_commit(self):
        result = self.tool("PreToolUse", "git commit -m 'capture'")
        self.assertEqual(result["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertEqual(self.state()["calls"][-1]["operation"], "check_commit")
        state = self.state()
        state["prepared_commit"] = True
        self.store.write_text(json.dumps(state))
        self.assertEqual(self.tool("PreToolUse", f"git -C '{self.repository}' commit -m capture"), {})

    def test_commit_cannot_check_one_repository_then_commit_another(self):
        other = self.make_repository("other repository")
        commands = [f"git -C '{other}' commit -m wrong", "GIT_INDEX_FILE=/tmp/other-index git commit",
                    "git --git-dir=/tmp/other.git commit", "git -c core.worktree=/tmp commit",
                    f"git -C'{other}' commit", "git -ccore.worktree=/tmp commit"]
        for command in commands:
            result = self.tool("PreToolUse", command)
            self.assertEqual(result["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertFalse(self.store.exists())

    def test_recognized_commit_and_edit_gate_deny_when_backend_fails(self):
        for name, command in [("Bash", "git commit -m capture"), ("apply_patch", None)]:
            result = self.run_hook("PreToolUse", behavior="fail", tool_name=name,
                                   tool_input={"command": command} if command else {})
            output = result["hookSpecificOutput"]
            self.assertEqual(output["permissionDecision"], "deny")
            self.assertIn("capture_incomplete", output["permissionDecisionReason"])

    def test_compound_shell_is_outside_prefix_gate_coverage(self):
        self.assertEqual(self.tool("PreToolUse", "git status;git commit -m x"), {})
        self.assertFalse(self.store.exists())

    def test_projects_and_sessions_are_isolated_even_with_matching_work_ids(self):
        other = self.make_repository("registered project two")
        self.configure(repository=other, project="other", work="work")
        self.run_hook("UserPromptSubmit", prompt="First project")
        self.assertEqual(self.run_hook("Stop", cwd=str(other)), {})
        self.assertEqual(self.run_hook("Stop", session_id="other session"), {})
        self.run_hook("UserPromptSubmit", cwd=str(other), prompt="Second project")
        self.assertEqual(len(self.state()["events"]), 2)
        bases = [event["base"] for event in self.state()["events"]]
        self.assertNotEqual(bases[0]["project_id"], bases[1]["project_id"])
        self.assertNotEqual(bases[0]["repository"], bases[1]["repository"])

    def test_interrupt_and_session_end_are_advisory_without_model_generation(self):
        result = self.run_hook("Interrupt")
        self.assertIn("pending", result["systemMessage"])
        self.assertEqual(self.state()["events"][0]["kind"], "interrupt")
        result = self.run_hook("SessionEnd")
        self.assertIn("capture_incomplete", result["systemMessage"])
        self.assertNotIn("decision", result)

    def test_bounded_raw_input_and_detail_do_not_create_transcript_spools(self):
        result = self.run_hook("UserPromptSubmit", raw=b"x" * (256 * 1024 + 1))
        self.assertIn("capture_incomplete", result["systemMessage"])
        self.assertFalse(self.store.exists())
        self.tool("PostToolUse", "cargo test", response={"exit_code": 1, "output": "long output " * 5000})
        saved = self.state()["events"][0]["detail"]
        self.assertLessEqual(len(saved), 2048)
        self.assertIn("truncated", saved)
        self.assertEqual(sorted(path.name for path in self.data.iterdir()), ["capture-config.json"])

    def test_long_prompt_tail_affects_identity_and_truncation_is_disclosed(self):
        prefix = "long request " * 1000
        self.run_hook("UserPromptSubmit", prompt=prefix + "first")
        self.run_hook("UserPromptSubmit", prompt=prefix + "correction")
        events = self.state()["events"]
        self.assertEqual(len(events), 2)
        self.assertNotEqual(events[0]["event_id"], events[1]["event_id"])
        self.assertIn("prompt truncated", events[0]["detail"])
        self.assertLessEqual(len(events[0]["detail"]), 2048)

    def test_subprocess_failure_and_nondurable_receipt_are_explicit_incomplete(self):
        for behavior in ["fail", "nondurable"]:
            result = self.run_hook("SessionStart", behavior=behavior)
            self.assertIn("capture_incomplete", result["systemMessage"])
            self.assertNotIn("secret stderr", json.dumps(result))

    def test_oversize_output_and_timeout_are_bounded_failures(self):
        started = time.monotonic()
        result = self.run_hook("SessionStart", behavior="oversize")
        self.assertIn("size limit", result["systemMessage"])
        result = self.run_hook("Interrupt", behavior="timeout")
        self.assertIn("timed out", result["systemMessage"])
        self.assertLess(time.monotonic() - started, 4)

    def test_malformed_and_oversize_config_are_not_silently_accepted(self):
        config = self.data / "capture-config.json"
        for contents in ["{", json.dumps({"version": 1, "projects": [{"repository": "relative"}]}),
                         "x" * (64 * 1024 + 1)]:
            config.write_text(contents)
            result = self.run_hook("SessionStart")
            self.assertIn("capture_incomplete", result["systemMessage"])
            self.assertFalse(self.store.exists())


@unittest.skipUnless(REAL_BINARY.is_file(), "Build memento or set MEMENTO_TEST_BIN to run actual Rust CLI scenarios")
class RealCodexHooks(unittest.TestCase):
    """Real SQLite/CLI receipts; native callbacks are invoked as subprocesses."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="memento-real-codex-hooks-")
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name).resolve()
        self.repository = self.directory / "repository"
        self.repository.mkdir()
        subprocess.run(["git", "init", "--quiet", str(self.repository)], check=True)
        self.plugin = self.directory / "plugin"
        shutil.copytree(TEMPLATE, self.plugin, ignore=shutil.ignore_patterns("target", "__pycache__", "*.pyc", ".memento-semantic", "bin"))
        self.binary = self.plugin / "skills/memento/bin/memento"
        self.binary.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REAL_BINARY, self.binary)
        (self.plugin / "skills/memento/SKILL.md").write_text("# Native callback test skill\n")
        self.store = self.directory / "context.sqlite"
        self.data = self.directory / "data"
        self.env = {**os.environ, "PLUGIN_ROOT": str(self.plugin), "PLUGIN_DATA": str(self.data),
                    "MEMENTO_RUNTIME_HOME": str(self.directory / "runtime-cache")}
        self.scope = {"project_id": "project", "repository": str(self.repository), "work_id": "work",
                      "session_id": "session", "turn_id": "turn"}
        self.cli("init", "--project", "project", "--work", "work", "--session", "session",
                 "--title", "Capture", "--goal", "Preserve relevant context")
        configured = subprocess.run(
            [sys.executable, str(self.plugin / "codex-hooks/capture.py"), "configure", "--data-dir", str(self.data),
             "--repository", str(self.repository), "--store", str(self.store), "--project-id", "project", "--work-id", "work"],
            capture_output=True, check=False, env=self.env, timeout=5,
        )
        self.assertEqual(configured.returncode, 0, configured.stdout.decode())

    def cli(self, operation, *arguments, data=None):
        result = subprocess.run([str(self.binary), operation, "--store", str(self.store), *arguments],
                                input=json.dumps(data).encode() if data is not None else None,
                                capture_output=True, check=False, timeout=5)
        self.assertEqual(result.returncode, 0, result.stdout.decode() + result.stderr.decode())
        return json.loads(result.stdout)

    def checkpoint(self, operation, **fields):
        return self.cli("checkpoint", "--input", "-", data={"operation": operation, "scope": self.scope, **fields})

    def hook(self, event, **fields):
        payload = {"hook_event_name": event, "cwd": str(self.repository), "session_id": "session",
                   "turn_id": "turn", **fields}
        result = subprocess.run([sys.executable, str(self.plugin / "codex-hooks/capture.py")],
                                input=json.dumps(payload).encode(), capture_output=True,
                                check=False, env=self.env, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        return json.loads(result.stdout)

    def save_and_resolve(self, event_id, record_id, kind, body):
        receipt = self.cli("note", "--project", "project", "--work", "work", "--session", "session",
                           "--input", "-", data={"id": record_id, "kind": kind, "body": body})["receipt"]
        self.assertTrue(receipt["durable"])
        read = self.cli("query", "--input", "-", data={
            "operation": "read", "scope": {"project_id": "project", "work_ids": ["work"], "session_ids": ["session"]},
            "target": {"kind": "record", "id": record_id},
        })
        record = read["items"][0]["entity"]["data"]
        self.assertEqual(record["body"], body)
        reference = {"source_id": record["source_id"], "record_id": record["id"],
                     "revision": record["revision"], "sequence": receipt["sequence"]}
        resolved = self.checkpoint("resolve", event_id=event_id, resolution={"kind": "records", "records": [reference]})
        self.assertTrue(resolved["durable"])

    def test_real_request_receipt_verification_limits_and_stop_continuation(self):
        self.hook("UserPromptSubmit", prompt="Reduce upload failures without slowing normal requests.")
        edit = {"tool_name": "apply_patch", "tool_input": {}, "tool_use_id": "first edit"}
        self.assertEqual(self.hook("PreToolUse", **edit)["hookSpecificOutput"]["permissionDecision"], "deny")
        event = self.checkpoint("status")["events"][0]
        self.save_and_resolve(event["event_id"], "request", "request",
                              "Reduce upload failures; do not increase normal-request latency.")
        self.assertEqual(self.hook("PreToolUse", **edit), {})
        result = self.hook("PostToolUse", tool_name="Bash", tool_input={"command": "cargo test"},
                           tool_use_id="verification", tool_response={"exit_code": 0,
                           "output": "40 tests passed; normal-request latency increased 40%."})
        self.assertIn("additionalContext", result["hookSpecificOutput"])
        stop = self.hook("Stop")
        self.assertEqual(stop["decision"], "block")
        self.hook("UserPromptSubmit", turn_id="continuation", prompt=stop["reason"])
        status = self.checkpoint("status")
        self.assertEqual(len(status["events"]), 2)
        pending = next(event for event in status["events"] if event["kind"] == "verification")
        self.assertEqual(pending["original_turn_id"], "turn")
        self.save_and_resolve(pending["event_id"], "verification-limits", "verification",
                              "40 tests passed, but normal-request latency rose 40%; the latency requirement remains unmet.")
        self.assertEqual(self.hook("Stop", turn_id="continuation"), {})

    def test_real_same_active_turn_correction_and_exact_retry(self):
        self.hook("UserPromptSubmit", prompt="First requirement")
        self.hook("UserPromptSubmit", prompt="Correction in the same turn")
        self.hook("UserPromptSubmit", prompt="Correction in the same turn")
        status = self.checkpoint("status")
        self.assertEqual(len(status["events"]), 2)
        self.assertEqual(len(status["pending_event_ids"]), 2)
        self.assertEqual(self.hook("PreToolUse", tool_name="apply_patch", tool_input={})[
            "hookSpecificOutput"]["permissionDecision"], "deny")

    def test_real_custom_policy_masks_checkpoint_detail_before_persistence(self):
        policy = self.directory / "policy.json"
        policy.write_text(json.dumps({"literal_secrets": ["fixture-private-literal"]}))
        configured = subprocess.run(
            [sys.executable, str(self.plugin / "codex-hooks/capture.py"), "configure", "--data-dir", str(self.data),
             "--repository", str(self.repository), "--store", str(self.store), "--project-id", "project",
             "--work-id", "work", "--policy", str(policy)], capture_output=True, check=False, env=self.env, timeout=5,
        )
        self.assertEqual(configured.returncode, 0, configured.stdout.decode())
        result = self.hook("UserPromptSubmit", prompt="Keep fixture-private-literal out of retained history.")
        self.assertNotIn("fixture-private-literal", json.dumps(result))
        status = self.checkpoint("status")
        self.assertNotIn("fixture-private-literal", status["events"][0]["detail"])
        self.assertIn("[REDACTED]", status["events"][0]["detail"])

    def test_real_missing_prepared_and_stale_commit_are_explicit_native_decisions(self):
        status = self.checkpoint("check_commit")
        self.assertTrue(status["durable"])
        self.assertEqual(status["decision"], "block")
        commit = {"tool_name": "Bash", "tool_input": {"command": "git commit -m capture"}}
        self.assertEqual(self.hook("PreToolUse", **commit)["hookSpecificOutput"]["permissionDecision"], "deny")
        prepared = self.checkpoint("prepare_commit", event_id="prepared-commit", detail="Commit the staged context capture.")
        self.assertTrue(prepared["durable"])
        self.save_and_resolve("prepared-commit", "commit-context", "decision", "Preserve the context capture and verification limits.")
        self.assertEqual(self.hook("PreToolUse", **commit), {})
        (self.repository / "changed.rs").write_text("staged state changed\n")
        subprocess.run(["git", "-C", str(self.repository), "add", "changed.rs"], check=True)
        self.assertEqual(self.checkpoint("check_commit")["decision"], "block")
        self.assertEqual(self.hook("PreToolUse", **commit)["hookSpecificOutput"]["permissionDecision"], "deny")


if __name__ == "__main__":
    unittest.main()
