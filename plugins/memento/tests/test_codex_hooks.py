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
    print(json.dumps({"protocol_version": 1, "package_version": "0.3.0",
        "build_identity": "a" * 64, "platform": {"os": {"darwin": "macos", "win32": "windows"}.get(sys.platform, sys.platform), "arch": {"arm64": "aarch64", "AMD64": "x86_64"}.get(__import__("platform").machine(), __import__("platform").machine())},
        "store_format": {"current": 2, "read": {"min": 0, "max": 2}, "write": {"min": 2, "max": 2}, "legacy": [0, 1]},
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
if sys.argv[1] == "query":
    state["calls"][-1]["_command"] = "query"
    store.write_text(json.dumps(state))
    if behavior == "query_fail":
        raise SystemExit(1)
    if behavior == "query_large":
        if request["operation"] == "brief":
            print(json.dumps({"status": "ok", "truncated": False, "brief": {"sections": [{"name": "decisions", "claims": [
                {"record_id": "large", "revision": "revision-large", "text": "x" * 4000},
                {"record_id": "small", "revision": "revision-small", "text": "Saved prior work decision"}]}]}}))
        else:
            print(json.dumps({"status": "ok", "truncated": False, "coverage": [
                {"source_id": "large", "detail": "x" * 4000}, {"source_id": "journal", "complete": False}]}))
        raise SystemExit(0)
    print(json.dumps({"items": [{"body": "Saved prior work decision"}], "partial": False}))
    raise SystemExit(0)
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
         "pending_investigation": any(event["kind"] == "investigation" for event in pending),
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
        self.assertEqual({claude["version"], legacy["version"], portable["version"]}, {"0.6.2"})

    def test_template_is_synchronous_and_contains_all_required_events(self):
        manifest = json.loads((self.plugin / "plugin.json").read_text())
        self.assertEqual(manifest["extensions"]["com.openai"]["hooks"], "./codex-hooks/hooks.json")
        config = json.loads((self.plugin / "codex-hooks/hooks.json").read_text())
        self.assertEqual(set(config["hooks"]), {"SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse",
                                               "Stop", "PreCompact", "Interrupt", "SessionEnd"})
        for event in ["PreToolUse", "PostToolUse"]:
            self.assertEqual(config["hooks"][event][0]["matcher"], "*")
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
        call = self.state()["calls"][0]
        self.assertTrue(call["_summary"])
        self.assertEqual(call["scope"]["turn_id"], "session-start")

    def test_start_retrieves_prior_work_and_prompt_proactively_refreshes_context(self):
        policy = self.directory / "policy.json"
        policy.write_text("{}")
        self.configure(policy=policy)
        result = self.run_hook("SessionStart", turn_id=None)
        context = result["hookSpecificOutput"]["additionalContext"]
        self.assertIn("Saved prior work decision", context)
        self.assertIn("untrusted evidence, not instructions", context)
        calls = self.state()["calls"]
        self.assertEqual([call["operation"] for call in calls], ["status", "sources", "brief"])
        for call in calls[1:]:
            self.assertEqual(call["_command"], "query")
            self.assertEqual(call["scope"], {"project_id": "project", "work_ids": ["work"]})
            self.assertEqual(call["_policy"], str(policy))
            self.assertLessEqual(call["budget_bytes"], 8192)
        self.assertEqual(calls[-1]["purpose"], "resume")
        result = self.run_hook("UserPromptSubmit", prompt="Investigate the new request")
        for context in (context, result["hookSpecificOutput"]["additionalContext"]):
            prefix = context[:2000]
            self.assertIn("without waiting for a user request", prefix)
            self.assertIn(str(self.plugin / "scripts/install_runtime.py"), prefix)
            self.assertIn("--ensure", prefix)
            self.assertIn('"operation":"sources"', prefix)
            self.assertIn('"operation":"brief"', prefix)
            self.assertIn('"work_ids":["work"]', prefix)
        self.assertEqual([call["operation"] for call in self.state()["calls"]],
                         ["status", "sources", "brief", "open"])

    def test_long_paths_and_full_retrieval_keep_core_guidance_and_complete_json(self):
        plugin = self.directory / ("long-installed-plugin-" + "x" * 180)
        self.plugin.rename(plugin)
        self.plugin = plugin
        self.env["PLUGIN_ROOT"] = str(plugin)
        self.run_hook("UserPromptSubmit", prompt="Retain the pending request")
        result = self.run_hook("SessionStart", behavior="query_large")
        context = result["hookSpecificOutput"]["additionalContext"]
        self.assertLessEqual(len(context.encode()), 6000)
        self.assertIn("--ensure", context)
        self.assertIn("Checkpoint scope:", context)
        self.assertIn(self.state()["events"][0]["event_id"], context)
        self.assertIn("durable receipt", context)
        self.assertIn("compound command.", context)
        evidence = json.loads(context.split("Memento evidence JSON: ", 1)[1])
        self.assertEqual([entry["operation"] for entry in evidence], ["sources", "brief"])
        self.assertTrue(all(entry["hook_omitted"] > 0 for entry in evidence))
        self.assertEqual(evidence[0]["coverage"], [{"source_id": "journal", "complete": False}])
        self.assertEqual(evidence[1]["claims"][0]["record_id"], "small")
        self.assertEqual(evidence[1]["claims"][0]["text"], "Saved prior work decision")

    def test_start_discloses_failed_retrieval_without_claiming_empty_history(self):
        result = self.run_hook("SessionStart", behavior="query_fail")
        context = result["hookSpecificOutput"]["additionalContext"]
        self.assertIn("sources retrieval unavailable", context)
        self.assertIn("Checkpoint result:", context)
        self.assertNotIn("Saved prior work decision", context)
        self.assertEqual([call["operation"] for call in self.state()["calls"]], ["status", "sources"])

    def assert_unprepared_startup(self):
        home = Path(self.env["MEMENTO_RUNTIME_HOME"])
        before = (home / "state.json").read_bytes() if (home / "state.json").exists() else None
        for event in ("SessionStart", "UserPromptSubmit"):
            result = self.run_hook(event, prompt="Continue the project")
            context = result["hookSpecificOutput"]["additionalContext"]
            self.assertIn(str(self.plugin / "scripts/install_runtime.py"), context[:2000])
            self.assertIn("--ensure", context[:2000])
            self.assertIn('"operation":"brief"', context[:2000])
            self.assertIn('"session_id":"session"', context)
            self.assertIn("capture_incomplete", result["systemMessage"])
        after = (home / "state.json").read_bytes() if (home / "state.json").exists() else None
        self.assertEqual(before, after)
        self.assertFalse(self.store.exists())

    def test_missing_runtime_start_and_request_give_bootstrap_without_installation(self):
        (self.plugin / "skills/memento/bin/memento").unlink()
        self.assert_unprepared_startup()
        self.assertFalse(Path(self.env["MEMENTO_RUNTIME_HOME"]).exists())

    def test_stale_runtime_start_and_request_give_bootstrap_without_installation(self):
        result = subprocess.run([sys.executable, str(self.plugin / "scripts/install_runtime.py"),
                                 "--ensure", "--embedding-model", "none"],
                                capture_output=True, env=self.env, check=False, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        core = self.plugin / "core"
        core.mkdir()
        (core / "Cargo.toml").write_text("changed source")
        (core / "Cargo.lock").write_text("changed lock")
        self.assert_unprepared_startup()

    def test_exact_runtime_preparation_command_remains_available_through_gate(self):
        (self.plugin / "skills/memento/bin/memento").unlink()
        installer = self.plugin / "scripts/install_runtime.py"
        for command in (f"python3 '{installer}' --ensure", f"'{installer}' --ensure"):
            for event in ("PreToolUse", "PostToolUse"):
                self.assertEqual(self.tool(event, command, response={"exit_code": 0}), {})
        self.assertFalse(self.store.exists())
        for command in (
            f"python3 '{installer}' --ensure; rg DBDUP src",
            f"python3 '{installer}' --ensure\nrg DBDUP src",
            f"python3 '{installer}' --ensure > result.txt",
            f"python3 '{installer}' --ensure --binary /tmp/unrelated",
            f"python3 '{self.repository / 'install_runtime.py'}' --ensure",
            f"python3 -c 'print(1)' '{installer}' --ensure",
        ):
            with self.subTest(command=command):
                result = self.tool("PreToolUse", command)
                self.assertEqual(result["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_missing_config_start_gives_actual_data_path_only_once_per_event(self):
        (self.data / "capture-config.json").unlink()
        result = self.run_hook("SessionStart")
        self.assertIn(str(self.data / "capture-config.json"), result["hookSpecificOutput"]["additionalContext"])
        self.assertIn(str(self.plugin / "codex-hooks/capture.py"), result["hookSpecificOutput"]["additionalContext"])
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

    def test_request_is_required_before_shell_file_and_mcp_investigation(self):
        self.run_hook("UserPromptSubmit", prompt="Investigate duplicate message handling.")
        for name, tool_input in [
            ("Bash", {"command": "rg DBDUP src"}),
            ("exec_command", {"cmd": "rg DBDUP src"}),
            ("Read", {"file_path": str(self.repository / "src.rs")}),
            ("Grep", {"pattern": "DBDUP"}),
            ("Glob", {"pattern": "**/*.rs"}),
            ("mcp__logs__search", {"transaction_id": "transaction-1"}),
        ]:
            with self.subTest(tool=name):
                result = self.run_hook("PreToolUse", tool_name=name, tool_input=tool_input)
                reason = result["hookSpecificOutput"]["permissionDecisionReason"]
                self.assertEqual(result["hookSpecificOutput"]["permissionDecision"], "deny")
                self.assertIn("--input -", reason)
        self.resolve_all()
        self.assertEqual(self.tool("PreToolUse", "rg DBDUP src"), {})

    def test_skill_reads_are_allowed_without_resolving_or_creating_checkpoints(self):
        references = self.plugin / "skills/memento/references"
        references.mkdir(exist_ok=True)
        reference = references / "atomic-claims.md"
        reference.write_text("# Capture instructions\n")
        skill = self.plugin / "skills/memento/SKILL.md"
        setup = self.plugin / "skills/setup-memento/SKILL.md"
        self.run_hook("UserPromptSubmit", prompt="Investigate the incident.")
        calls = len(self.state()["calls"])
        for name, tool_input in [
            ("Bash", {"command": f"cat '{skill}'"}),
            ("Bash", {"command": f"sed -n '1,200p' '{reference}'"}),
            ("Read", {"file_path": str(setup)}),
            ("mcp__filesystem__read_file", {"path": str(reference)}),
        ]:
            for event in ["PreToolUse", "PostToolUse"]:
                with self.subTest(tool=name, event=event):
                    self.assertEqual(self.run_hook(event, tool_name=name, tool_input=tool_input,
                                                  tool_response={"content": "instructions"}), {})
        self.assertEqual(len(self.state()["calls"]), calls)
        self.assertIsNone(self.state()["events"][0]["resolution"])

    def test_bootstrap_does_not_allow_writes_unrelated_reads_or_compound_commands(self):
        self.run_hook("UserPromptSubmit", prompt="Investigate the incident.")
        skill = self.plugin / "skills/memento/SKILL.md"
        for name, tool_input in [
            ("Write", {"file_path": str(skill), "content": "new text"}),
            ("Read", {"file_path": str(self.repository / "SKILL.md")}),
            ("Bash", {"command": f"cat '{skill}'; rg DBDUP src"}),
            ("Bash", {"command": f"cat '{skill}'\n'{skill}'"}),
            ("Bash", {"command": f"cat '{skill}' src.rs"}),
            ("Bash", {"command": f"sed -n '1p; e rg DBDUP src' '{skill}'"}),
        ]:
            with self.subTest(tool=name, input=tool_input):
                result = self.run_hook("PreToolUse", tool_name=name, tool_input=tool_input)
                self.assertEqual(result["hookSpecificOutput"]["permissionDecision"], "deny")

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

    def test_memento_operations_do_not_create_recursive_records(self):
        self.run_hook("UserPromptSubmit", prompt="Investigate the incident.")
        calls = len(self.state()["calls"])
        for command in ["memento record --input -", "memento query",
                        "python3 /tmp/memento.py checkpoint --input -",
                        "memento record --input - <<'JSON'\n{\"body\":\"a finding; $(literal) `literal`\"}\nJSON"]:
            self.assertEqual(self.tool("PostToolUse", command, response={"exit_code": 0}), {})
            self.assertEqual(self.tool("PreToolUse", command), {})
        self.assertEqual(len(self.state()["calls"]), calls)

    def test_completed_reads_require_review_before_next_supported_tool(self):
        for name, tool_input in [
            ("Bash", {"command": "ls"}),
            ("Bash", {"command": "git status"}),
            ("Bash", {"command": "cat file.rs"}),
            ("Bash", {"command": "rg DBDUP src; rg msgid src"}),
            ("Read", {"file_path": "file.rs"}),
            ("Grep", {"pattern": "DBDUP"}),
            ("Glob", {"pattern": "**/*.rs"}),
            ("mcp__logs__search", {"transaction_id": "transaction-1"}),
        ]:
            with self.subTest(tool=name, input=tool_input):
                self.resolve_all()
                result = self.run_hook("PostToolUse", tool_name=name, tool_input=tool_input,
                                       tool_use_id=f"result-{len(self.state()['events'])}",
                                       tool_response={"exit_code": 0, "output": "observed result"})
                self.assertIn("additionalContext", result["hookSpecificOutput"])
                self.assertEqual(self.state()["events"][-1]["kind"], "investigation")
                self.assertEqual(self.tool("PreToolUse", name="apply_patch")[
                    "hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("transaction-1", self.state()["events"][-1]["detail"])

    def test_no_new_context_review_releases_next_investigation(self):
        self.tool("PostToolUse", "git status", response={"exit_code": 0, "output": "clean"})
        self.assertEqual(self.tool("PreToolUse", "rg DBDUP src")["hookSpecificOutput"]["permissionDecision"], "deny")
        self.resolve_all("no_new_context")
        self.assertEqual(self.tool("PreToolUse", "rg DBDUP src"), {})

    def test_mcp_result_session_metadata_does_not_mean_running_shell(self):
        self.run_hook("PostToolUse", tool_name="mcp__logs__search", tool_input={"transaction_id": "transaction-1"},
                      tool_use_id="logs", tool_response={"session_id": "server-session", "content": "DBDUP observed"})
        self.assertEqual(self.state()["events"][-1]["kind"], "investigation")
        self.assertEqual(self.tool("PreToolUse", "cat src.rs")["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_investigation_outside_bounded_id_sample_still_blocks(self):
        for index in range(9):
            self.tool("PostToolUse", "touch file.rs", response={"exit_code": 0}, tool_id=f"mutation-{index}")
        self.tool("PostToolUse", "rg DBDUP src", response={"exit_code": 0}, tool_id="investigation")
        result = self.tool("PreToolUse", "cat src.rs")
        self.assertEqual(result["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("Additional pending IDs omitted: 2", result["hookSpecificOutput"]["permissionDecisionReason"])

    def test_compound_self_prefix_does_not_bypass_capture(self):
        self.run_hook("UserPromptSubmit", prompt="Investigate the incident.")
        for command in ["memento query; rg DBDUP src", "python3 /tmp/capture.py; echo ok",
                        "memento query && git status", "memento query\nrg DBDUP src",
                        "memento query $(rg DBDUP src)", "memento query <(rg DBDUP src)",
                        "memento query '<<' 'JSON' # <<'JSON'\nrg DBDUP src\nJSON",
                        "memento query # <<'JSON'\nrg DBDUP src\nJSON",
                        "memento query <<EXPAND <<'JSON'\n$(rg DBDUP src)\nJSON",
                        "memento record --input - <<'JSON'\n{}\nJSON\nrg DBDUP src",
                        "memento record --input - <<'JSON'\n{}\nJSON\nrg DBDUP src\nJSON"]:
            with self.subTest(command=command):
                self.assertEqual(self.tool("PreToolUse", command)["hookSpecificOutput"]["permissionDecision"], "deny")
                self.tool("PostToolUse", command, response={"exit_code": 0}, tool_id=command)
                self.assertEqual(self.state()["events"][-1]["kind"], "investigation")

    def test_unknown_administrative_tools_do_not_check_or_open_events(self):
        self.run_hook("UserPromptSubmit", prompt="Investigate the incident.")
        calls = len(self.state()["calls"])
        for name in ["update_plan", "request_user_input", "unknown_tool"]:
            for event in ["PreToolUse", "PostToolUse"]:
                self.assertEqual(self.run_hook(event, tool_name=name, tool_input={}, tool_response={"ok": True}), {})
        self.assertEqual(len(self.state()["calls"]), calls)

    def test_own_cli_failure_also_does_not_open_recursive_checkpoint(self):
        self.assertEqual(self.tool("PostToolUse", "memento checkpoint --input -", response={"exit_code": 1}), {})
        self.assertFalse(self.store.exists())

    def test_running_exec_is_not_completion_and_native_retry_is_idempotent(self):
        self.assertEqual(self.tool("PostToolUse", "cargo test", response={"session_id": 23, "exit_code": None}), {})
        self.assertEqual(self.tool("PostToolUse", "rg DBDUP src", response={"session_id": 24, "exit_code": None}), {})
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
        self.assertEqual(self.state()["calls"][-1]["operation"], "status")
        self.run_hook("UserPromptSubmit", prompt="Investigate the incident.")
        self.assertEqual(self.tool("PreToolUse", "git status;git commit -m x")[
            "hookSpecificOutput"]["permissionDecision"], "deny")

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

    def test_long_mcp_input_discloses_truncation_even_with_short_output(self):
        self.run_hook("PostToolUse", tool_name="mcp__logs__search", tool_use_id="long-filter",
                      tool_input={"filter": "x" * 1000, "transaction_id": "omitted-transaction"},
                      tool_response={"content": "short output"})
        saved = self.state()["events"][0]["detail"]
        self.assertIn("[capture detail truncated;", saved)
        self.assertIn("short output", saved)
        self.assertNotIn("omitted-transaction", saved)
        self.assertLessEqual(len(saved), 2048)

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

    def test_real_start_retrieves_decision_from_prior_session(self):
        body = "Keep the existing model selection during updates and read prior decisions before changing project code."
        self.cli("init", "--project", "project", "--work", "work", "--session", "previous-session",
                 "--title", "Prior work", "--goal", "Preserve prior decisions")
        self.cli("note", "--project", "project", "--work", "work", "--session", "previous-session",
                 "--input", "-", data={"id": "prior-decision", "kind": "decision", "body": body})
        result = self.hook("SessionStart", turn_id=None)
        context = result["hookSpecificOutput"]["additionalContext"]
        self.assertIn(body, context)
        self.assertNotIn("retrieval unavailable", context)
        evidence = json.loads(context.split("Memento evidence JSON: ", 1)[1])
        self.assertEqual([entry["operation"] for entry in evidence], ["sources", "brief"])
        self.assertTrue(evidence[0]["coverage"])
        claim = next(claim for claim in evidence[1]["claims"] if claim["record_id"] == "prior-decision")
        self.assertEqual(claim["text"], body)
        self.assertTrue(claim["revision"])

    def save_and_resolve(self, event_id, record_id, kind, bodies):
        event = next(value for value in self.checkpoint("status")["events"] if value["event_id"] == event_id)
        native = event["origin"]
        observation = self.cli("query", "--input", "-", data={
            "operation": "read", "scope": {"project_id": "project", "source_ids": [native["source_id"]]},
            "target": {"kind": "artifact", "record_id": native["record_id"], "revision": native["revision"]},
        })["items"][0]["entity"]["data"]
        self.assertEqual(observation["representation"], "evidence")
        self.assertEqual(observation["context_id"], event["context_id"])
        specs = [{"kind": kind, "body": bodies}] if isinstance(bodies, str) else bodies
        references = []
        for index, spec in enumerate(specs):
            identifier = record_id if len(specs) == 1 else f"{record_id}:{index + 1}"
            note = {"id": identifier, "kind": spec["kind"], "body": spec["body"],
                    "representation": "claim", "derived": True, "fidelity": "summary_only",
                    "nature": spec.get("nature", "reported"), "context_id": event["context_id"],
                    "evidence": [{"source_id": native["source_id"], "record_id": native["record_id"],
                                  "revision": native["revision"], "locator": f"checkpoint:{event_id}",
                                  "availability": observation["availability"], "purpose": "origin",
                                  "range": None,
                                  "span": {"start": 0, "end": len(observation["body"].encode("utf-8"))}}]}
            if spec["kind"] == "verification":
                note["verification_outcome"] = spec.get("verification_outcome", "unknown")
            receipt = self.cli("note", "--project", "project", "--work", "work", "--session", "session",
                               "--input", "-", data=note)["receipt"]
            self.assertTrue(receipt["durable"])
            saved = self.cli("query", "--input", "-", data={
                "operation": "read", "scope": {"project_id": "project", "work_ids": ["work"], "session_ids": ["session"]},
                "target": {"kind": "record", "id": identifier},
            })["items"][0]["entity"]["data"]
            self.assertEqual(saved["body"], spec["body"])
            self.assertEqual(saved["representation"], "claim")
            self.assertEqual(saved["context_id"], event["context_id"])
            self.assertEqual(saved["evidence"], note["evidence"])
            references.append({"source_id": saved["source_id"], "record_id": saved["id"],
                               "revision": saved["revision"], "sequence": receipt["sequence"]})
        resolved = self.checkpoint("resolve", event_id=event_id, resolution={"kind": "records", "records": references})
        self.assertTrue(resolved["durable"])

    def test_real_request_receipt_verification_limits_and_stop_continuation(self):
        self.hook("UserPromptSubmit", prompt="Reduce upload failures without slowing normal requests.")
        edit = {"tool_name": "apply_patch", "tool_input": {}, "tool_use_id": "first edit"}
        self.assertEqual(self.hook("PreToolUse", **edit)["hookSpecificOutput"]["permissionDecision"], "deny")
        event = self.checkpoint("status")["events"][0]
        self.save_and_resolve(event["event_id"], "request", "request", [
            {"kind": "request", "body": "Reduce upload failures."},
            {"kind": "constraint", "body": "Do not increase normal-request latency."},
        ])
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
        self.save_and_resolve(pending["event_id"], "verification-limits", "verification", [
            {"kind": "verification", "body": "40 tests passed.", "nature": "observed", "verification_outcome": "passed"},
            {"kind": "verification", "body": "Normal-request latency increased 40%.", "nature": "observed", "verification_outcome": "failed"},
        ])
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

    def test_real_investigation_decision_change_is_saved_before_interruption(self):
        self.hook("UserPromptSubmit", prompt="Investigate how DBDUP affects message processing.")
        search = {"tool_name": "mcp__logs__search", "tool_input": {"transaction_id": "transaction-1"}}
        self.assertEqual(self.hook("PreToolUse", **search)["hookSpecificOutput"]["permissionDecision"], "deny")
        request = self.checkpoint("status")["events"][0]
        self.save_and_resolve(request["event_id"], "incident-request", "request",
                              "Investigate how DBDUP affects message processing.")
        first = {**search, "tool_use_id": "initial-logs", "tool_response": {"content": "DBDUP was reported."}}
        self.assertEqual(self.hook("PreToolUse", **search), {})
        self.hook("PostToolUse", **first)
        self.assertEqual(self.hook("PreToolUse", **search)["hookSpecificOutput"]["permissionDecision"], "deny")
        initial = self.checkpoint("status")["events"][-1]
        self.assertEqual(initial["kind"], "investigation")
        self.save_and_resolve(initial["event_id"], "initial-decision", "decision",
                              "The initial interpretation was that DBDUP stops message processing.")
        second = {**search, "tool_use_id": "follow-up-logs", "tool_response": {
            "content": "The duplicate message is replaced with the existing msgid and processing continues."}}
        self.assertEqual(self.hook("PreToolUse", **search), {})
        self.hook("PostToolUse", **second)
        revised = self.checkpoint("status")["events"][-1]
        self.assertEqual(self.hook("PreToolUse", tool_name="Read", tool_input={"file_path": "src.rs"})[
            "hookSpecificOutput"]["permissionDecision"], "deny")
        self.save_and_resolve(revised["event_id"], "revised-decision", "decision",
                              "The initial DBDUP-stop interpretation is corrected: replace it with the existing msgid and continue processing.")
        self.assertFalse(self.checkpoint("status")["pending_investigation"])
        self.hook("Interrupt")
        status = self.checkpoint("status")
        self.assertEqual(status["events"][-1]["kind"], "interrupt")
        for record_id, expected in [
            ("initial-decision", "The initial interpretation was that DBDUP stops message processing."),
            ("revised-decision", "The initial DBDUP-stop interpretation is corrected: replace it with the existing msgid and continue processing."),
        ]:
            saved = self.cli("query", "--input", "-", data={
                "operation": "read", "scope": {"project_id": "project", "work_ids": ["work"], "session_ids": ["session"]},
                "target": {"kind": "record", "id": record_id},
            })["items"][0]["entity"]["data"]
            self.assertEqual(saved["body"], expected)

    def test_real_no_new_context_allows_status_request_and_uninformative_read(self):
        self.hook("UserPromptSubmit", prompt="What is the recording status?")
        request = self.checkpoint("status")["events"][0]
        self.checkpoint("resolve", event_id=request["event_id"], resolution={
            "kind": "no_new_context", "reason": "This is only a status question without new work context."})
        read = {"tool_name": "Bash", "tool_input": {"command": "git status"}, "tool_use_id": "status"}
        self.assertEqual(self.hook("PreToolUse", **read), {})
        self.hook("PostToolUse", **read, tool_response={"exit_code": 0, "output": "working tree clean"})
        investigation = self.checkpoint("status")["events"][-1]
        self.assertTrue(self.checkpoint("status")["pending_investigation"])
        self.checkpoint("resolve", event_id=investigation["event_id"], resolution={
            "kind": "no_new_context", "reason": "The working tree has no changes and no new investigation finding."})
        self.assertFalse(self.checkpoint("status")["pending_investigation"])
        self.assertEqual(self.hook("Stop"), {})

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

    def test_real_truncated_mcp_input_is_partial_source_evidence(self):
        self.hook("PostToolUse", tool_name="mcp__logs__search", tool_use_id="long-filter",
                  tool_input={"filter": "x" * 1000, "transaction_id": "omitted-transaction"},
                  tool_response={"content": "short output"})
        event = self.checkpoint("status")["events"][0]
        origin = event["origin"]
        saved = self.cli("query", "--input", "-", data={
            "operation": "read", "scope": {"project_id": "project", "source_ids": [origin["source_id"]]},
            "target": {"kind": "artifact", "record_id": origin["record_id"], "revision": origin["revision"]},
        })["items"][0]["entity"]["data"]
        self.assertTrue(saved["partial"])
        self.assertEqual(saved["fidelity"], "source_truncated")
        self.assertIn("[capture detail truncated;", saved["body"])
        self.assertTrue(self.checkpoint("status")["pending_investigation"])

    def test_real_missing_prepared_and_stale_commit_are_explicit_native_decisions(self):
        status = self.checkpoint("check_commit")
        self.assertTrue(status["durable"])
        self.assertEqual(status["decision"], "block")
        commit = {"tool_name": "Bash", "tool_input": {"command": "git commit -m capture"}}
        self.assertEqual(self.hook("PreToolUse", **commit)["hookSpecificOutput"]["permissionDecision"], "deny")
        prepared = self.checkpoint("prepare_commit", event_id="prepared-commit", detail="Commit the staged context capture.")
        self.assertTrue(prepared["durable"])
        self.save_and_resolve("prepared-commit", "commit-context", "decision", "Commit the staged context capture.")
        self.assertEqual(self.hook("PreToolUse", **commit), {})
        (self.repository / "changed.rs").write_text("staged state changed\n")
        subprocess.run(["git", "-C", str(self.repository), "add", "changed.rs"], check=True)
        self.assertEqual(self.checkpoint("check_commit")["decision"], "block")
        self.assertEqual(self.hook("PreToolUse", **commit)["hookSpecificOutput"]["permissionDecision"], "deny")


if __name__ == "__main__":
    unittest.main()
