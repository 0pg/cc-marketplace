"""First-use, concurrent, interrupted and updated plugin installations."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

PLUGIN = Path(__file__).resolve().parent.parent


class RuntimeLifecycleTests(unittest.TestCase):
    def setUp(self):
        workspace = tempfile.TemporaryDirectory(prefix="memento lifecycle ")
        self.addCleanup(workspace.cleanup)
        self.root = Path(workspace.name).resolve()
        self.plugin = self.root / "plugin package"
        shutil.copytree(PLUGIN / "scripts", self.plugin / "scripts", ignore=shutil.ignore_patterns("__pycache__"))
        shutil.copytree(PLUGIN / "skills/memento", self.plugin / "skills/memento", ignore=shutil.ignore_patterns("__pycache__", "bin"))
        shutil.copytree(PLUGIN / "core/scripts", self.plugin / "core/scripts")
        (self.plugin / "core/src").mkdir()
        (self.plugin / "core/Cargo.toml").write_text('[package]\nname="memento"\nversion="0.3.0"\n')
        (self.plugin / "core/Cargo.lock").write_text("lock fixture\n")
        (self.plugin / "core/src/main.rs").write_text("initial source\n")
        self.home = self.root / "stable runtime"
        self.environment = os.environ.copy()
        self.environment.pop("MEMENTO_BIN", None)
        self.environment["MEMENTO_RUNTIME_HOME"] = str(self.home)
        self.environment["FAKE_BUILD_LOG"] = str(self.root / "build log")
        self.bin = self.root / "tools"
        self.bin.mkdir()
        self.environment["PATH"] = str(self.bin)
        self.cargo = self.bin / "cargo"
        self.cargo.write_text(
            f"#!{sys.executable}\n"
            "import json, os, pathlib, platform, sys, time\n"
            "log=pathlib.Path(os.environ['FAKE_BUILD_LOG'])\n"
            "with log.open('a') as f: f.write('build\\n')\n"
            "if os.environ.get('FAKE_BUILD_SLEEP'): time.sleep(float(os.environ['FAKE_BUILD_SLEEP']))\n"
            "if os.environ.get('FAKE_BUILD_FAIL'): raise SystemExit(17)\n"
            "output=pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'release/memento'\n"
            "output.parent.mkdir(parents=True,exist_ok=True)\n"
            "version={'platform':{'os':{'Darwin':'macos','Linux':'linux','Windows':'windows'}.get(platform.system()),'arch':{'arm64':'aarch64','AMD64':'x86_64'}.get(platform.machine(),platform.machine())},'package_version':'0.3.0','build_identity':os.environ['MEMENTO_BUILD_ID'],'protocol_version':1,"
            "'store_format':{'current':2,'read':{'min':0,'max':2},'write':{'min':2,'max':2}},'capabilities':['checkpoint']}\n"
            f"program='#!{sys.executable}\\nimport json,sys\\nversion='+repr(version)+'\\nprint(json.dumps(version if sys.argv[1:2]==[\"version\"] else ({{\"valid\":True}} if sys.argv[1:2]==[\"semantic-config-check\"] else {{\"executed\":sys.argv[1:2]}})))\\n'\n"
            "output.write_text(program); output.chmod(0o755)\n"
            "print(json.dumps({'reason':'compiler-artifact','target':{'name':'memento','kind':['bin']},'executable':str(output)}))\n",
            encoding="utf-8")
        self.cargo.chmod(0o755)

    def run_install(self, *arguments, environment=None):
        return subprocess.run([sys.executable, str(self.plugin / "scripts/install_runtime.py"), "--ensure", *arguments],
                              env=environment or self.environment, text=True, capture_output=True, check=False)

    def run_launcher(self, *arguments):
        return subprocess.run([sys.executable, str(self.plugin / "skills/memento/scripts/memento.py"), *arguments],
                              env=self.environment, text=True, capture_output=True, check=False)

    def state(self):
        return json.loads((self.home / "state.json").read_text())

    def assert_success(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_passive_status_help_and_version_never_prepare(self):
        status = self.run_launcher("runtime-status")
        self.assert_success(status)
        self.assertEqual(json.loads(status.stdout)["status"], "not_installed")
        self.assertNotEqual(self.run_launcher("help").returncode, 0)
        self.assertNotEqual(self.run_launcher("version").returncode, 0)
        self.assertNotEqual(self.run_launcher("--version").returncode, 0)
        self.assertNotEqual(self.run_launcher("store-status").returncode, 0)
        self.assertNotEqual(self.run_launcher("semantic-config-check").returncode, 0)
        self.assertFalse(self.home.exists())
        self.assertFalse((self.root / "build log").exists())

    def test_skip_onboarding_first_use_prepares_and_continues_json_command(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        self.assert_success(self.run_launcher("query"))
        self.assertEqual(json.loads(self.run_launcher("query").stdout), {"executed": ["query"]})
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"])
        self.assertFalse((self.plugin / "skills/memento/bin").exists())

    def test_first_real_command_prepares_without_onboarding_with_explicit_custom_model(self):
        worker = self.root / "custom worker.py"
        worker.write_text("import json,sys\nrequest=json.load(sys.stdin)\nprint(json.dumps({**request,'query':[1.0,0.0],'documents':[[1.0,0.0]]}))\n")
        config = self.root / "selected custom model.json"
        config.write_text(json.dumps({"command": [sys.executable, str(worker)], "model_id": "custom-model", "model_revision": "fixed"}))
        result = self.run_launcher("query", "--semantic-config", str(config))
        self.assert_success(result)
        self.assertEqual(json.loads(result.stdout), {"executed": ["query"]})
        self.assertEqual(self.state()["active"]["embedding_selection"], "custom")
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"])
        self.assertFalse((self.home / "models").exists())

    def test_stable_git_bridge_survives_pruning_old_artifacts(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        bridge = self.home / "git-memento"
        first = self.home / "artifacts" / self.state()["active"]["directory"]
        for number in range(3):
            (self.plugin / "core/src/main.rs").write_text(f"bridge revision {number}\n")
            self.assert_success(self.run_install())
        self.assertFalse(first.exists())
        result = subprocess.run([str(bridge), "query"], env=self.environment, text=True, capture_output=True, check=False)
        self.assert_success(result)
        self.assertEqual(json.loads(result.stdout), {"executed": ["query"]})

    def test_two_processes_install_once_and_reuse_verified_result(self):
        environment = {**self.environment, "FAKE_BUILD_SLEEP": "0.3"}
        command = [sys.executable, str(self.plugin / "scripts/install_runtime.py"), "--ensure", "--embedding-model", "none"]
        processes = [subprocess.Popen(command, env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2)]
        for process in processes:
            output, errors = process.communicate(timeout=10)
            self.assertEqual(process.returncode, 0, output + errors)
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"])
        self.assertEqual(self.state()["status"], "ready")

    def test_three_source_updates_bound_artifacts_and_preserve_selection(self):
        for number in range(4):
            (self.plugin / "core/src/main.rs").write_text(f"source revision {number}\n")
            self.assert_success(self.run_install("--embedding-model", "none") if number == 0 else self.run_install())
            self.assertEqual(self.state()["active"]["embedding_selection"], "none")
        self.assertEqual(len(list((self.home / "artifacts").iterdir())), 2)
        self.assertFalse((self.home / "candidate").exists())
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"] * 4)

    def test_failed_update_preserves_active_then_retry_recovers(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        before = self.state()["active"]
        (self.plugin / "core/src/main.rs").write_text("next source\n")
        result = self.run_install(environment={**self.environment, "FAKE_BUILD_FAIL": "1"})
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.state()["status"], "failed")
        self.assertEqual(self.state()["active"], before)
        self.assertFalse((self.home / "candidate").exists())
        self.assertNotEqual(self.run_launcher("help").returncode, 0)
        self.assert_success(self.run_install())
        self.assertNotEqual(self.state()["active"]["build_identity"], before["build_identity"])

    def test_interrupted_prepare_is_not_ready_and_can_be_retried(self):
        command = [sys.executable, str(self.plugin / "scripts/install_runtime.py"), "--ensure", "--embedding-model", "none"]
        process = subprocess.Popen(command, env={**self.environment, "FAKE_BUILD_SLEEP": "3"},
                                   text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        deadline = time.monotonic() + 5
        while not (self.root / "build log").exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        process.kill()
        process.communicate(timeout=5)
        self.assertEqual(self.state()["status"], "preparing")
        self.assertIsNone(self.state()["active"])
        self.assertNotEqual(self.run_launcher("version").returncode, 0)
        # The externally interrupted Cargo is still bounded; wait for its fixture to finish.
        time.sleep(3.1)
        self.assert_success(self.run_install("--embedding-model", "none"))
        self.assertEqual(self.state()["status"], "ready")
        self.assertFalse((self.home / "candidate").exists())

    def test_cache_package_replacement_reuses_external_binary(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        active = self.state()["active"]
        copied = self.root / "new plugin cache"
        shutil.copytree(self.plugin, copied)
        shutil.rmtree(self.plugin)
        self.plugin = copied
        self.assert_success(self.run_launcher("query"))
        self.assertEqual(self.state()["active"], active)
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"])

    def test_legacy_binary_without_recorded_selection_requires_explicit_choice(self):
        bundled = self.plugin / "skills/memento/bin"
        bundled.mkdir()
        (bundled / "memento").write_text("legacy executable")
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("selection_required", result.stderr)
        self.assertFalse((self.root / "build log").exists())
        self.assert_success(self.run_install("--embedding-model", "none"))

    def test_unknown_files_are_preserved_when_owned_artifacts_are_pruned(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        unrelated = self.home / "artifacts" / ("f" * 24)
        unrelated.mkdir()
        document = unrelated / "user notes.txt"
        document.write_text("keep this user file")
        for number in range(3):
            (self.plugin / "core/src/main.rs").write_text(f"owned revision {number}\n")
            self.assert_success(self.run_install())
        self.assertEqual(document.read_text(), "keep this user file")
        owned = [path for path in (self.home / "artifacts").iterdir() if (path / ".memento-owner.json").is_file()]
        self.assertEqual(len(owned), 2)

    def test_model_root_symlink_never_traverses_or_deletes_external_files(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        external = self.root / "unrelated models"
        external.mkdir()
        saved = external / "user data.txt"
        saved.write_text("keep external data")
        (self.home / "models").symlink_to(external, target_is_directory=True)
        result = self.run_install("--embedding-model", "minilm")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("symbolic links", result.stderr)
        self.assertEqual(saved.read_text(), "keep external data")

    def test_prebuilt_package_hint_imports_compatible_binary_without_cargo_sources(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        active = self.home / "artifacts" / self.state()["active"]["directory"] / "memento"
        bundled = self.plugin / "skills/memento/bin"
        bundled.mkdir()
        shutil.copy2(active, bundled / "memento")
        shutil.rmtree(self.plugin / "core")
        (self.plugin / "runtime-selection.json").write_text(json.dumps({"format_version": 1, "embedding_selection": "none"}))
        shutil.rmtree(self.home)
        self.assert_success(self.run_launcher("query"))
        self.assertEqual(self.state()["active"]["origin"], "bundled")
        self.assertIsNone(self.state()["active"]["requested_source_digest"])
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"])

    def test_source_package_preserves_explicit_binary_origin_when_model_changes(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        first = self.state()["active"]
        external = self.root / "provided executable"
        shutil.copy2(self.home / "artifacts" / first["directory"] / "memento", external)
        external.write_text(external.read_text().replace(first["build_identity"], "c" * 64))
        self.assert_success(self.run_install("--binary", str(external), "--embedding-model", "none"))
        explicit = self.state()["active"]
        self.assertEqual(explicit["origin"], "explicit")
        worker = self.root / "custom worker.py"
        worker.write_text("import json,sys\nrequest=json.load(sys.stdin)\nprint(json.dumps({**request,'query':[1.0,0.0],'documents':[[1.0,0.0]]}))\n")
        config = self.root / "custom config.json"
        config.write_text(json.dumps({"command": [sys.executable, str(worker)], "model_id": "custom", "model_revision": "fixed"}))
        self.assert_success(self.run_install("--semantic-config", str(config)))
        active = self.state()["active"]
        self.assertEqual(active["origin"], "explicit")
        self.assertEqual(active["build_identity"], "c" * 64)
        self.assertEqual(active["binary_sha256"], explicit["binary_sha256"])
        self.assertEqual(active["embedding_selection"], "custom")
        self.assertEqual((self.root / "build log").read_text().splitlines(), ["build"])

    def test_restored_custom_config_never_reuses_tampered_artifact_config(self):
        worker = self.root / "custom worker.py"
        worker.write_text("import json,sys\nrequest=json.load(sys.stdin)\nprint(json.dumps({**request,'query':[1.0,0.0],'documents':[[1.0,0.0]]}))\n")
        selected = {"command": [sys.executable, str(worker)], "model_id": "custom", "model_revision": "fixed"}
        original = self.root / "selected config.json"
        original.write_text(json.dumps(selected))
        self.assert_success(self.run_launcher("query", "--semantic-config", str(original)))
        previous = self.state()["active"]["directory"]
        previous_config = self.home / "artifacts" / previous / "semantic-config.json"
        previous_config.write_text(json.dumps({**selected, "unverified_future_field": True}))
        self.assert_success(self.run_launcher("query", "--semantic-config", str(original)))
        active = self.state()["active"]
        self.assertNotEqual(active["directory"], previous)
        actual = self.home / "artifacts" / active["directory"] / "semantic-config.json"
        self.assertEqual(json.loads(actual.read_text()), selected)
        self.assertEqual(hashlib.sha256(actual.read_bytes()).hexdigest(), active["semantic_config_sha256"])

    def test_bridge_conflict_preserves_active_pointer_during_update(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        previous = self.state()["active"]
        external = self.root / "user bridge"
        external.write_text("preserve external user bridge")
        bridge = self.home / "git-memento"
        bridge.unlink()
        bridge.symlink_to(external)
        (self.plugin / "core/src/main.rs").write_text("updated source before bridge conflict\n")
        result = self.run_install()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.state()["active"], previous)
        self.assertEqual(self.state()["status"], "failed")
        self.assertEqual(external.read_text(), "preserve external user bridge")
        self.assertTrue(bridge.is_symlink())
        self.assertFalse((self.home / "candidate").exists())

    def test_partial_helper_copy_failure_preserves_existing_git_bridge(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        original = self.state()["active"]
        helper = self.home / "runtime.py"
        before = helper.read_bytes()
        (self.plugin / "scripts/runtime.py").write_text((self.plugin / "scripts/runtime.py").read_text() + "\n# helper upgrade\n")
        (self.plugin / "core/src/main.rs").write_text("source update with failed helper publication\n")
        sys.path.insert(0, str(PLUGIN / "scripts"))
        import runtime
        copy = runtime.shutil.copy2

        def partial_copy(source, destination, **kwargs):
            if Path(source).name == "runtime.py" and Path(destination).name == "runtime.py.tmp":
                Path(destination).write_bytes(b"partial helper copy")
                raise OSError("injected helper copy failure")
            return copy(source, destination, **kwargs)

        with patch.dict(os.environ, self.environment), patch.object(runtime, "__file__", str(self.plugin / "scripts/runtime.py")), patch.object(runtime.shutil, "copy2", side_effect=partial_copy):
            with self.assertRaises(runtime.RuntimeError):
                runtime.ensure_runtime(self.plugin)
        self.assertEqual(helper.read_bytes(), before)
        self.assertEqual(self.state()["active"], original)
        self.assertEqual(self.state()["status"], "failed")
        self.assertFalse((self.home / "runtime.py.tmp").exists())
        self.assertFalse((self.home / "candidate").exists())
        result = subprocess.run([str(self.home / "git-memento"), "query"], env=self.environment,
                                text=True, capture_output=True, check=False)
        self.assert_success(result)
        self.assertEqual(json.loads(result.stdout), {"executed": ["query"]})

    def test_repeated_prebuilt_use_reuses_state_and_override_change_refreshes_binary(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        active = self.home / "artifacts" / self.state()["active"]["directory"] / "memento"
        bundled = self.plugin / "skills/memento/bin"
        bundled.mkdir()
        shutil.copy2(active, bundled / "memento")
        shutil.rmtree(self.plugin / "core")
        (self.plugin / "runtime-selection.json").write_text(json.dumps({"format_version": 1, "embedding_selection": "none"}))
        shutil.rmtree(self.home)
        self.assert_success(self.run_launcher("query"))
        before = (self.home / "state.json").stat().st_mtime_ns
        original = self.state()["active"]
        self.assert_success(self.run_launcher("query"))
        self.assertEqual((self.home / "state.json").stat().st_mtime_ns, before)
        replacement = self.root / "provided override"
        shutil.copy2(bundled / "memento", replacement)
        replacement.write_text(replacement.read_text() + "\n# explicitly selected updated binary\n")
        self.environment["MEMENTO_BIN"] = str(replacement)
        self.assert_success(self.run_launcher("query"))
        self.assertNotEqual(self.state()["active"]["binary_sha256"], original["binary_sha256"])
        before = (self.home / "state.json").stat().st_mtime_ns
        self.assert_success(self.run_launcher("query"))
        self.assertEqual((self.home / "state.json").stat().st_mtime_ns, before)

    def test_timeout_terminates_worker_child_process_group(self):
        sys.path.insert(0, str(PLUGIN / "scripts"))
        import runtime
        delayed = self.root / "orphan wrote data"
        child = f"import pathlib,time;time.sleep(1);pathlib.Path({str(delayed)!r}).write_text('orphan')"
        parent = f"import subprocess,sys,time;subprocess.Popen([sys.executable,'-c',{child!r}]);time.sleep(10)"
        with self.assertRaises(runtime.RuntimeError):
            runtime.run_command([sys.executable, "-c", parent], timeout=0.2)
        time.sleep(1.1)
        self.assertFalse(delayed.exists())

    def test_unknown_configuration_version_never_builds_or_overwrites(self):
        self.home.mkdir()
        state = {"format_version": 99, "status": "ready"}
        (self.home / "state.json").write_text(json.dumps(state))
        before = (self.home / "state.json").read_bytes()
        result = self.run_install("--embedding-model", "none")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unsupported runtime configuration", result.stderr)
        self.assertEqual((self.home / "state.json").read_bytes(), before)
        self.assertFalse((self.root / "build log").exists())

    def test_tampered_active_binary_is_rebuilt_before_use(self):
        self.assert_success(self.run_install("--embedding-model", "none"))
        active = self.home / "artifacts" / self.state()["active"]["directory"] / "memento"
        active.write_text(active.read_text() + "\n# changed bytes\n")
        self.assertNotEqual(self.run_launcher("help").returncode, 0)
        self.assert_success(self.run_install())
        self.assertEqual(self.state()["active"]["binary_sha256"], hashlib.sha256((self.home / "artifacts" / self.state()["active"]["directory"] / "memento").read_bytes()).hexdigest())


if __name__ == "__main__":
    unittest.main()
