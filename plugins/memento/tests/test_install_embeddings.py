"""Installer contracts; only package downloads and model inference are stubbed."""

import contextlib
import io
import json
import os
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


class EmbeddingInstallationTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="memento semantic install ")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        distributed = Path(installer.__file__).resolve().parent.parent
        self.plugin = self.root / "independent plugin"
        self.skills = self.plugin / "skills"
        self.target = self.skills / "memento"
        shutil.copytree(distributed / "skills/memento", self.target)
        shutil.copytree(distributed / "core/scripts", self.plugin / "core/scripts")
        (self.plugin / "scripts").mkdir()
        shutil.copy2(distributed / "scripts/install_runtime.py", self.plugin / "scripts/install_runtime.py")
        self.addCleanup(patch.stopall)
        patch.object(installer, "__file__", str(self.plugin / "scripts/install_runtime.py")).start()
        self.binary = self.root / "test cli"
        self.binary.write_text(
            f"#!{sys.executable}\n"
            'print(\'{"commands":["init","note","query","compact","checkpoint"]}\')\n',
            encoding="utf-8",
        )
        self.binary.chmod(0o755)
        repository = Path(installer.__file__).resolve().parent.parent
        self.models = runpy.run_path(str(repository / "core/scripts/setup_embeddings.py"))["MODELS"]
        self.calls = []
        self.failure = None
        self.response_change = {}
        self.uv = "/stub/uv"
        real_run = subprocess.run

        def external_run(command, **kwargs):
            phase = Path(command[1]).name if len(command) > 1 else ""
            if command[0] != self.uv and phase not in ("setup_embeddings.py", "local_embeddings.py"):
                return real_run(command, **kwargs)
            self.calls.append((phase, command))
            if phase == self.failure:
                return subprocess.CompletedProcess(command, 1, "", "test setup failure")
            if phase == "venv":
                environment = Path(command[-1])
                python = environment / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
                python.parent.mkdir(parents=True)
                python.write_text("test Python runtime", encoding="utf-8")
            elif phase == "setup_embeddings.py":
                options = iter(command[2:])
                args = dict(zip(options, options))
                model_dir = Path(args["--output"])
                model_dir.mkdir(parents=True, exist_ok=True)
                model_id, revision = self.models[args["--model"]]
                (model_dir / "work-context-model.json").write_text(
                    json.dumps({"model_id": model_id, "model_revision": revision}), encoding="utf-8",
                )
            elif phase == "local_embeddings.py":
                request = json.loads(kwargs["input"])
                response = {
                    "protocol": request["protocol"], "model_id": request["model_id"],
                    "model_revision": request["model_revision"], "query": [1.0, 0.0],
                    "documents": [[1.0, 0.0]], "metrics": {},
                }
                response.update(self.response_change)
                return subprocess.CompletedProcess(command, 0, json.dumps(response), "")
            return subprocess.CompletedProcess(command, 0)

        patch.object(installer.subprocess, "run", side_effect=external_run).start()
        patch.object(installer.shutil, "which", side_effect=lambda _: self.uv).start()

    def install(self, model=None):
        arguments = ["--binary", str(self.binary)]
        if model is not None:
            arguments.extend(["--embedding-model", model])
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return installer.main(arguments)

    def config(self):
        return json.loads((self.target / "semantic-config.json").read_text(encoding="utf-8"))

    def snapshot(self):
        return {str(path.relative_to(self.target)): path.read_bytes() for path in self.target.rglob("*") if path.is_file()}

    def test_selection_installs_each_model_and_creates_repository_independent_config(self):
        for model, expected_id in (
            ("e5-small", "intfloat/multilingual-e5-small"),
            ("minilm", "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2"),
        ):
            with self.subTest(model=model):
                self.calls.clear()
                self.assertEqual(self.install(model), 0)
                config = self.config()
                self.assertEqual(config["model_id"], expected_id)
                self.assertEqual(config["model_revision"], self.models[model][1])
                python, worker, flag, model_dir = config["command"]
                self.assertTrue(Path(python).is_file())
                self.assertTrue(Path(worker).is_relative_to(self.plugin.parent / ".memento-semantic"))
                self.assertTrue(Path(worker).is_file())
                self.assertEqual(flag, "--model-dir")
                self.assertTrue(Path(model_dir).is_dir())
                self.assertTrue(Path(model_dir).is_relative_to(self.plugin.parent / ".memento-semantic"))
                self.assertTrue((Path(model_dir).parent / "ready").is_file())
                self.assertEqual([phase for phase, _ in self.calls], ["venv", "pip", "setup_embeddings.py", "local_embeddings.py"])

    def test_default_installs_e5_small_and_explicit_model_overrides_it(self):
        self.assertEqual(self.install(), 0)
        self.assertEqual(self.config()["model_id"], "intfloat/multilingual-e5-small")
        self.assertEqual([phase for phase, _ in self.calls], ["venv", "pip", "setup_embeddings.py", "local_embeddings.py"])
        self.assertEqual(self.install("minilm"), 0)
        self.assertEqual(self.config()["model_id"], "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")
        self.assertEqual(self.install("none"), 0)
        self.assertEqual(self.config()["model_id"], "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")
        self.calls.clear()
        self.assertEqual(self.install(), 0)
        self.assertEqual(self.config()["model_id"], "intfloat/multilingual-e5-small")
        self.assertEqual([phase for phase, _ in self.calls], ["local_embeddings.py"])

    def test_copied_package_keeps_runtime_paths_after_original_package_is_removed(self):
        self.assertEqual(self.install("e5-small"), 0)
        config = self.config()
        copied = self.root / "cache copy"
        shutil.copytree(self.plugin, copied)
        shutil.rmtree(self.plugin)
        copied_config = json.loads((copied / "skills/memento/semantic-config.json").read_text())
        self.assertEqual(copied_config, config)
        python, worker, _, model_dir = copied_config["command"]
        for path in (python, worker, model_dir):
            self.assertTrue(Path(path).exists())
            self.assertFalse(Path(path).is_relative_to(self.plugin))
        self.assertEqual(Path(worker).read_bytes(), (copied / "core/scripts/local_embeddings.py").read_bytes())

    def test_reinstall_reuses_runtime_and_none_preserves_existing_selection(self):
        self.assertEqual(self.install("e5-small"), 0)
        config = self.config()
        data = self.target / "context.sqlite"
        data.write_bytes(b"existing context store")
        before = self.snapshot()
        self.calls.clear()
        self.assertEqual(self.install("e5-small"), 0)
        self.assertEqual([phase for phase, _ in self.calls], ["local_embeddings.py"])
        self.assertEqual(self.config(), config)
        self.assertEqual(self.snapshot(), before)
        self.calls.clear()
        self.assertEqual(self.install(), 0)
        self.assertEqual([phase for phase, _ in self.calls], ["local_embeddings.py"])
        self.assertEqual(self.snapshot(), before)
        self.calls.clear()
        self.assertEqual(self.install("none"), 0)
        self.assertEqual(self.calls, [])
        self.assertEqual(self.snapshot(), before)

    def test_package_model_and_inference_failures_preserve_existing_installation(self):
        self.assertEqual(self.install("e5-small"), 0)
        (self.target / "context.sqlite").write_bytes(b"existing context store")
        before = self.snapshot()
        for phase in ("pip", "setup_embeddings.py", "local_embeddings.py"):
            with self.subTest(phase=phase):
                self.failure = phase
                self.assertEqual(self.install("minilm"), 1)
                self.assertEqual(self.snapshot(), before)
        self.failure = None
        self.assertEqual(self.install("minilm"), 0)
        self.assertEqual(self.config()["model_id"], "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2")
        self.assertEqual((self.target / "context.sqlite").read_bytes(), b"existing context store")

    def test_missing_uv_leaves_existing_installation_intact(self):
        self.assertEqual(self.install("none"), 0)
        before = self.snapshot()
        self.uv = None
        self.assertEqual(self.install(), 1)
        self.assertEqual(self.snapshot(), before)

    def test_invalid_worker_responses_do_not_activate_a_model(self):
        self.assertEqual(self.install("none"), 0)
        before = self.snapshot()
        for response in (
            {"model_id": "different model"}, {"documents": []},
            {"documents": [[float("nan"), 0.0]]}, {"query": [0.0, 0.0]},
        ):
            with self.subTest(response=response):
                self.response_change = response
                self.assertEqual(self.install("e5-small"), 1)
                self.assertEqual(self.snapshot(), before)


if __name__ == "__main__":
    unittest.main()
