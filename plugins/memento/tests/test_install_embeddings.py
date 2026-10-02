"""Runtime install contracts; package downloads and model inference are stubbed."""
import contextlib
import io
import json
import os
import platform
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
import install_runtime as installer
import runtime


class EmbeddingInstallationTests(unittest.TestCase):
    def setUp(self):
        workspace = tempfile.TemporaryDirectory(prefix="memento semantic install ")
        self.addCleanup(workspace.cleanup)
        self.root = Path(workspace.name).resolve()
        distributed = Path(installer.__file__).resolve().parent.parent
        self.plugin = self.root / "independent plugin"
        self.target = self.plugin / "skills/memento"
        shutil.copytree(distributed / "skills/memento", self.target)
        shutil.copytree(distributed / "core/scripts", self.plugin / "core/scripts")
        (self.plugin / "scripts").mkdir()
        shutil.copy2(distributed / "scripts/install_runtime.py", self.plugin / "scripts/install_runtime.py")
        shutil.copy2(distributed / "scripts/runtime.py", self.plugin / "scripts/runtime.py")
        self.home = self.root / "external runtime"
        self.addCleanup(patch.stopall)
        patch.dict(os.environ, {"MEMENTO_RUNTIME_HOME": str(self.home), "PATH": ""}).start()
        patch.object(installer, "__file__", str(self.plugin / "scripts/install_runtime.py")).start()
        self.binary = self.root / "test cli"
        version = {"platform": {"os": {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}.get(platform.system()),
                                "arch": {"arm64": "aarch64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())},
                   "package_version": "0.2.0", "build_identity": "test-verified-build", "protocol_version": 1,
                   "store_format": {"current": 1, "read": {"min": 1, "max": 1}, "write": {"min": 1, "max": 1}},
                   "capabilities": ["checkpoint"]}
        self.binary.write_text(f"#!{sys.executable}\nimport json,sys\nprint(json.dumps({version!r} if sys.argv[1:2] == ['version'] else {{'valid':True}}))\n", encoding="utf-8")
        self.binary.chmod(0o755)
        self.models = runpy.run_path(str(self.plugin / "core/scripts/setup_embeddings.py"))["MODELS"]
        self.calls = []
        self.failure = None
        self.response_change = {}
        self.uv = "/stub/uv"
        original = runtime.run_command

        def external(command, **kwargs):
            phase = Path(command[1]).name if len(command) > 1 else ""
            if command[0] != self.uv and phase not in ("setup_embeddings.py", "local_embeddings.py"):
                return original(command, **kwargs)
            self.calls.append((phase, command))
            if phase == self.failure:
                return subprocess.CompletedProcess(command, 1, "", "test setup failure")
            if phase == "venv":
                python = Path(command[-1]) / "bin/python"
                python.parent.mkdir(parents=True)
                python.write_text("test Python runtime")
            elif phase == "setup_embeddings.py":
                options = iter(command[2:])
                args = dict(zip(options, options))
                model_dir = Path(args["--output"])
                model_dir.mkdir(parents=True, exist_ok=True)
                model_id, revision = self.models[args["--model"]]
                (model_dir / "work-context-model.json").write_text(json.dumps({"model_id": model_id, "model_revision": revision}))
            elif phase == "local_embeddings.py":
                request = json.loads(kwargs["input"])
                response = {"protocol": request["protocol"], "model_id": request["model_id"], "model_revision": request["model_revision"],
                            "query": [1.0, 0.0], "documents": [[1.0, 0.0]], "metrics": {}}
                response.update(self.response_change)
                return subprocess.CompletedProcess(command, 0, json.dumps(response), "")
            return subprocess.CompletedProcess(command, 0, "", "")

        patch.object(runtime, "run_command", side_effect=external).start()
        patch.object(runtime.shutil, "which", side_effect=lambda name: self.uv if name == "uv" else None).start()

    def install(self, model=None):
        arguments = ["--binary", str(self.binary), "--ensure"]
        if model is not None:
            arguments.extend(["--embedding-model", model])
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return installer.main(arguments)

    def state(self):
        return json.loads((self.home / "state.json").read_text())

    def config(self):
        directory = self.state()["active"]["directory"]
        return json.loads((self.home / "artifacts" / directory / "semantic-config.json").read_text())

    def test_selection_installs_each_model_and_creates_package_independent_config(self):
        for model, expected_id in (("e5-small", "intfloat/multilingual-e5-small"),
                                   ("minilm", "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")):
            with self.subTest(model=model):
                self.calls.clear()
                self.assertEqual(self.install(model), 0)
                config = self.config()
                self.assertEqual(config["model_id"], expected_id)
                self.assertEqual(config["model_revision"], self.models[model][1])
                python, worker, flag, model_dir = config["command"]
                self.assertTrue(Path(python).is_file())
                self.assertTrue(Path(worker).is_relative_to(self.home / "models"))
                self.assertEqual(Path(worker).read_bytes(), (self.plugin / "core/scripts/local_embeddings.py").read_bytes())
                self.assertEqual(flag, "--model-dir")
                self.assertTrue(Path(model_dir).is_dir())
                self.assertEqual([phase for phase, _ in self.calls], ["venv", "pip", "setup_embeddings.py", "local_embeddings.py"])

    def test_new_install_defaults_e5_updates_preserve_minilm_and_none(self):
        self.assertEqual(self.install(), 0)
        self.assertEqual(self.config()["model_id"], self.models["e5-small"][0])
        self.assertEqual(self.install("minilm"), 0)
        self.calls.clear()
        self.assertEqual(self.install(), 0)
        self.assertEqual(self.config()["model_id"], self.models["minilm"][0])
        self.assertEqual(self.calls, [])
        prior = self.config()
        self.assertEqual(self.install("none"), 0)
        self.calls.clear()
        self.assertEqual(self.install(), 0)
        self.assertEqual(self.config(), prior)
        self.assertEqual(self.calls, [])
        self.assertEqual(self.state()["active"]["embedding_selection"], "none")

    def test_copied_package_keeps_runtime_paths_after_original_package_removed(self):
        self.assertEqual(self.install("e5-small"), 0)
        config = self.config()
        copied = self.root / "cache copy"
        shutil.copytree(self.plugin, copied)
        shutil.rmtree(self.plugin)
        for path in (config["command"][0], config["command"][1], config["command"][3]):
            self.assertTrue(Path(path).exists())
        self.assertEqual(runtime.ready_runtime(copied).semantic_config.read_text(), json.dumps(config, indent=2) + "\n")

    def test_none_preserves_legacy_custom_selection_without_model_download(self):
        config = {"command": [sys.executable, "external-custom-worker"], "model_id": "custom", "model_revision": "fixed"}
        (self.target / "semantic-config.json").write_text(json.dumps(config))
        self.assertEqual(self.install("none"), 0)
        self.assertEqual(self.config(), config)
        self.assertEqual(self.calls, [])

    def test_model_failures_preserve_active_runtime_and_database(self):
        self.assertEqual(self.install("e5-small"), 0)
        database = self.target / "context.sqlite"
        database.write_bytes(b"existing project store")
        active = self.state()["active"]
        config = self.config()
        for phase in ("pip", "setup_embeddings.py", "local_embeddings.py"):
            with self.subTest(phase=phase):
                self.failure = phase
                self.assertEqual(self.install("minilm"), 1)
                self.assertEqual(self.state()["active"], active)
                self.assertEqual(self.config(), config)
                self.assertEqual(database.read_bytes(), b"existing project store")
                self.assertFalse((self.home / "candidate").exists())
        self.failure = None
        self.assertEqual(self.install("minilm"), 0)

    def test_unowned_generated_model_directory_is_never_claimed_or_deleted(self):
        self.assertEqual(self.install("none"), 0)
        active = self.state()["active"]
        key = f"e5-small-{runtime._model_digest(self.plugin, 'e5-small')[:20]}"
        directory = self.home / "models" / key
        directory.mkdir(parents=True)
        user_note = directory / "user notes.txt"
        user_note.write_text("keep the user's existing files")
        self.failure = "pip"
        self.assertEqual(self.install("e5-small"), 1)
        self.assertEqual(user_note.read_text(), "keep the user's existing files")
        self.assertFalse((directory / ".memento-owner.json").exists())
        self.assertEqual(self.state()["active"], active)
        self.assertEqual(self.calls, [])

    def test_missing_uv_preserves_existing_runtime(self):
        self.assertEqual(self.install("none"), 0)
        active = self.state()["active"]
        self.uv = None
        self.assertEqual(self.install("e5-small"), 1)
        self.assertEqual(self.state()["active"], active)
        self.assertIn("uv was not found", self.state()["failure"])

    def test_invalid_worker_responses_do_not_activate_model(self):
        self.assertEqual(self.install("none"), 0)
        active = self.state()["active"]
        for response in ({"model_id": "wrong"}, {"documents": []}, {"documents": [[float("nan"), 0.0]]}, {"query": [0.0, 0.0]}):
            with self.subTest(response=response):
                self.response_change = response
                self.assertEqual(self.install("e5-small"), 1)
                self.assertEqual(self.state()["active"], active)

    def test_worker_requirements_change_prepares_only_new_model_runtime(self):
        self.assertEqual(self.install("minilm"), 0)
        old = self.state()["active"]
        path = self.plugin / "core/scripts/semantic-requirements.txt"
        path.write_text(path.read_text() + "\n# changed pinned environment\n")
        self.calls.clear()
        prepared = runtime.ensure_runtime(self.plugin)
        self.assertEqual(prepared.state["active"]["embedding_selection"], "minilm")
        self.assertNotEqual(prepared.state["active"]["model_key"], old["model_key"])
        self.assertEqual(prepared.state["active"]["binary_sha256"], old["binary_sha256"])
        self.assertEqual([phase for phase, _ in self.calls], ["venv", "pip", "setup_embeddings.py", "local_embeddings.py"])


if __name__ == "__main__":
    unittest.main()
