"""Package update checks stay passive; preparation activates validated updates."""

import importlib.util
import json
import os
from pathlib import Path
import platform
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "memento_startup_runtime", ROOT / "scripts/runtime.py"
)
runtime = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = runtime
SPEC.loader.exec_module(runtime)


class RuntimePackageUpdateTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="memento-package-update-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.plugin = self.root / "plugin"
        self.home = self.root / "runtime"
        self.bundle = self.plugin / "skills/memento/bin/memento"
        self.bundle.parent.mkdir(parents=True)
        self.write_binary(self.bundle, "initial-build")
        (self.plugin / "plugin.json").write_text(json.dumps({"version": "1.0.0"}))
        (self.plugin / "runtime-selection.json").write_text(json.dumps({
            "format_version": 1, "embedding_selection": "none",
        }))
        self.scripts = self.plugin / "core/scripts"
        self.scripts.mkdir(parents=True)
        for name in ("semantic-requirements.txt", "setup_embeddings.py", "local_embeddings.py"):
            (self.scripts / name).write_text(f"# {name}\n")
        self.environment = patch.dict(os.environ, {"MEMENTO_RUNTIME_HOME": str(self.home)}, clear=True)
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def write_binary(self, path, build, *, protocol=1, extra=""):
        version = {
            "protocol_version": protocol, "build_identity": build,
            "platform": {
                "os": {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}.get(
                    platform.system(), platform.system().lower()
                ),
                "arch": {"arm64": "aarch64", "AMD64": "x86_64"}.get(
                    platform.machine(), platform.machine()
                ),
            },
            "store_format": {"current": 2, "read": {"min": 0, "max": 2}, "write": {"min": 2, "max": 2}},
            "capabilities": ["checkpoint"],
        }
        path.write_text(
            f"#!{sys.executable}\nimport json,sys\nversion={version!r}\n"
            "print(json.dumps(version if sys.argv[1:2] == ['version'] else {'valid': True}))\n"
            f"# {extra}\n"
        )
        path.chmod(0o755)

    def snapshot(self):
        return {str(path.relative_to(self.root)): (path.read_bytes(), path.stat().st_mtime_ns)
                for path in self.root.rglob("*") if path.is_file()}

    def install_model(self, plugin, home, selection):
        digest = runtime._model_digest(plugin, selection)
        key = f"{selection}-{digest[:20]}"
        model = home / "models" / key
        (model / "venv/bin").mkdir(parents=True)
        (model / "venv/bin/python").write_text("test Python")
        (model / "model").mkdir()
        worker = model / "local_embeddings.py"
        worker.write_bytes((plugin / "core/scripts/local_embeddings.py").read_bytes())
        config = {"command": [str(model / "venv/bin/python"), str(worker)],
                  "model_id": selection, "model_revision": digest}
        runtime._write(model / ".memento-owner.json", {"owner": "memento", "kind": "model", "format_version": 1})
        runtime._write(model / "model/work-context-model.json", config)
        runtime._write(model / "ready.json", {
            **config, "digest": digest,
            "worker_sha256": runtime.hashlib.sha256(worker.read_bytes()).hexdigest(),
        })
        return config, key

    def assert_stale_without_writes(self, message):
        before = self.snapshot()
        with self.assertRaisesRegex(runtime.RuntimeError, message):
            runtime.ready_runtime(self.plugin)
        status = runtime.runtime_status(self.plugin)
        self.assertFalse(status["ready_for_package"])
        self.assertIn(message, status["validation_error"])
        self.assertEqual(status["persisted_status"], "ready")
        self.assertEqual(self.snapshot(), before)

    def test_changed_bundle_is_detected_and_automatically_activated(self):
        previous = runtime.ensure_runtime(self.plugin)
        previous_bytes = previous.executable.read_bytes()
        # A build label alone cannot distinguish different delivered executable bytes.
        self.write_binary(self.bundle, "initial-build", extra="updated executable")
        self.assert_stale_without_writes("plugin binary")
        with patch.object(runtime, "_build") as build, patch.object(runtime, "install_embeddings") as model:
            prepared = runtime.ensure_runtime(self.plugin)
            self.assertEqual(runtime.ready_runtime(self.plugin).executable, prepared.executable)
            again = runtime.ensure_runtime(self.plugin)
        build.assert_not_called()
        model.assert_not_called()
        self.assertEqual(prepared.executable.read_bytes(), self.bundle.read_bytes())
        self.assertNotEqual(prepared.executable, previous.executable)
        self.assertEqual(prepared.state["previous"], previous.state["active"])
        self.assertEqual(prepared.state["active"]["embedding_selection"], "none")
        self.assertEqual(previous.executable.read_bytes(), previous_bytes)
        self.assertEqual(again.state["active"], prepared.state["active"])

    def test_plugin_version_alone_does_not_rebuild_the_runtime(self):
        previous = runtime.ensure_runtime(self.plugin)
        (self.plugin / "plugin.json").write_text(json.dumps({"version": "1.1.0"}))
        before = self.snapshot()
        with patch.object(runtime, "_build") as build, patch.object(runtime, "install_embeddings") as model:
            self.assertTrue(runtime.runtime_status(self.plugin)["ready_for_package"])
            prepared = runtime.ensure_runtime(self.plugin)
        build.assert_not_called()
        model.assert_not_called()
        self.assertEqual(prepared.state["active"], previous.state["active"])
        self.assertEqual(self.snapshot(), before)

    def test_changed_model_assets_update_the_existing_model_selection(self):
        with patch.object(runtime, "install_embeddings", side_effect=self.install_model) as install:
            previous = runtime.ensure_runtime(self.plugin, embedding_model="minilm")
            for name in ("semantic-requirements.txt", "setup_embeddings.py", "local_embeddings.py"):
                with self.subTest(asset=name):
                    script = self.scripts / name
                    script.write_text(script.read_text() + "# package update\n")
                    self.assert_stale_without_writes("plugin model setup")
                    prepared = runtime.ensure_runtime(self.plugin)
                    self.assertEqual(install.call_args.args[-1], "minilm")
                    self.assertEqual(prepared.state["active"]["embedding_selection"], "minilm")
                    self.assertNotEqual(prepared.state["active"]["model_key"], previous.state["active"]["model_key"])
                    self.assertEqual(prepared.executable.read_bytes(), previous.executable.read_bytes())
                    self.assertEqual(prepared.state["previous"], previous.state["active"])
                    self.assertTrue(previous.semantic_config.is_file())
                    self.assertTrue(runtime.runtime_status(self.plugin)["ready_for_package"])
                    previous = prepared

    def test_source_update_is_built_before_activation(self):
        core = self.plugin / "core"
        (core / "Cargo.toml").write_text("source package")
        (core / "Cargo.lock").write_text("locked dependencies")
        (core / "src").mkdir()
        source = core / "src/main.rs"
        source.write_text("initial source")

        def build(plugin, candidate, digest):
            self.write_binary(candidate / "memento", digest)

        with patch.object(runtime, "_build", side_effect=build) as compile_runtime:
            previous = runtime.ensure_runtime(self.plugin)
            source.write_text("updated source")
            self.assert_stale_without_writes("plugin build")
            prepared = runtime.ensure_runtime(self.plugin)
        self.assertEqual(compile_runtime.call_count, 2)
        self.assertEqual(prepared.state["active"]["build_identity"], runtime.source_digest(self.plugin))
        self.assertEqual(prepared.state["active"]["embedding_selection"], "none")
        self.assertEqual(prepared.state["previous"], previous.state["active"])
        self.assertTrue(runtime.runtime_status(self.plugin)["ready_for_package"])

    def test_failed_bundle_update_preserves_the_active_artifact(self):
        previous = runtime.ensure_runtime(self.plugin)
        previous_bytes = previous.executable.read_bytes()
        self.write_binary(self.bundle, "incompatible-update", protocol=2)
        with self.assertRaisesRegex(runtime.RuntimeError, "incompatible Memento protocol"):
            runtime.ensure_runtime(self.plugin)
        state = runtime._state(self.home)
        self.assertEqual(state["status"], "failed")
        self.assertEqual(state["active"], previous.state["active"])
        self.assertEqual(previous.executable.read_bytes(), previous_bytes)
        self.assertFalse((self.home / "candidate").exists())

    def test_failed_model_update_preserves_the_active_model(self):
        with patch.object(runtime, "install_embeddings", side_effect=self.install_model):
            previous = runtime.ensure_runtime(self.plugin, embedding_model="e5-small")
        previous_config = previous.semantic_config.read_bytes()
        (self.scripts / "setup_embeddings.py").write_text("# changed pinned revision\n")
        with patch.object(runtime, "install_embeddings", side_effect=runtime.RuntimeError("download failed")):
            with self.assertRaisesRegex(runtime.RuntimeError, "download failed"):
                runtime.ensure_runtime(self.plugin)
        state = runtime._state(self.home)
        self.assertEqual(state["active"], previous.state["active"])
        self.assertEqual(previous.semantic_config.read_bytes(), previous_config)
        self.assertEqual(runtime.ready_runtime(self.home / "bridge-context").executable, previous.executable)

    def test_explicit_environment_binary_takes_precedence_over_the_package(self):
        runtime.ensure_runtime(self.plugin)
        explicit = self.root / "explicit-memento"
        self.write_binary(explicit, "selected-explicit-build")
        self.write_binary(self.bundle, "updated-bundled-build")
        with patch.dict(os.environ, {"MEMENTO_BIN": str(explicit)}):
            self.assertEqual(runtime.ready_runtime(self.plugin).executable, explicit)
            self.assertTrue(runtime.runtime_status(self.plugin)["ready_for_package"])
            prepared = runtime.ensure_runtime(self.plugin)
        self.assertEqual(prepared.state["active"]["origin"], "explicit")
        self.assertEqual(prepared.executable.read_bytes(), explicit.read_bytes())


if __name__ == "__main__":
    unittest.main()
