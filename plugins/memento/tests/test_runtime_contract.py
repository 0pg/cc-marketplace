"""Validate the plugin/runtime contract without installing models or stores."""

import copy
import importlib.util
import json
import platform
from pathlib import Path
import os
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "memento_runtime_contract", ROOT / "scripts/runtime.py"
)
runtime = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = runtime
SPEC.loader.exec_module(runtime)


class RuntimeContractTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="memento-runtime-contract-")
        self.addCleanup(self.directory.cleanup)
        self.binary = Path(self.directory.name) / "memento"
        self.version = {
            "protocol_version": 1,
            "package_version": "0.3.0",
            "build_identity": "b" * 64,
            "platform": {
                "os": {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}.get(
                    platform.system(), platform.system().lower()
                ),
                "arch": {"arm64": "aarch64", "AMD64": "x86_64"}.get(
                    platform.machine(), platform.machine()
                ),
            },
            "store_format": {
                "current": 2,
                "read": {"min": 0, "max": 2},
                "write": {"min": 2, "max": 2},
                "legacy": [0, 1],
            },
            "capabilities": ["checkpoint", "migrate"],
        }

    def handshake(self, version):
        self.binary.write_text(
            f"#!{sys.executable}\nimport json\nprint(json.dumps({version!r}))\n",
            encoding="utf-8",
        )
        self.binary.chmod(0o755)
        return runtime.handshake(self.binary)

    def test_current_runtime_accepts_format_two_and_preserves_configuration_format(self):
        self.assertEqual(self.handshake(self.version), self.version)
        self.assertEqual(runtime.FORMAT_VERSION, 1)
        self.assertEqual(runtime.STORE_FORMAT_VERSION, 2)

    def test_format_one_binary_is_rejected_before_any_store_is_opened(self):
        old = copy.deepcopy(self.version)
        old["store_format"] = {
            "current": 1,
            "read": {"min": 0, "max": 1},
            "write": {"min": 1, "max": 1},
        }
        with self.assertRaisesRegex(runtime.RuntimeError, "current store format"):
            self.handshake(old)
        self.assertEqual(list(Path(self.directory.name).iterdir()), [self.binary])

    def test_incompatible_or_malformed_access_bounds_are_rejected(self):
        for access in ("read", "write"):
            for bounds in (
                {"min": 0, "max": 1},
                {"min": 3, "max": 3},
                {"min": True, "max": 2},
                {"min": 0, "max": "2"},
                {"min": 3, "max": 0},
            ):
                with self.subTest(access=access, bounds=bounds):
                    version = copy.deepcopy(self.version)
                    version["store_format"][access] = bounds
                    with self.assertRaisesRegex(runtime.RuntimeError, "current store format"):
                        self.handshake(version)

    def test_unsupported_current_format_is_rejected_even_with_broad_ranges(self):
        for current in (1, 3, True, "2", None):
            with self.subTest(current=current):
                version = copy.deepcopy(self.version)
                version["store_format"]["current"] = current
                version["store_format"]["read"] = {"min": 0, "max": 3}
                version["store_format"]["write"] = {"min": 0, "max": 3}
                with self.assertRaisesRegex(runtime.RuntimeError, "current store format"):
                    self.handshake(version)

    def legacy_runtime(self):
        root = Path(self.directory.name)
        plugin = root / "plugin"
        shutil.copytree(ROOT, plugin, ignore=shutil.ignore_patterns("target", "__pycache__"))
        home = root / "runtime"
        artifact = home / "artifacts/legacy"
        artifact.mkdir(parents=True)
        old = copy.deepcopy(self.version)
        old["package_version"] = "0.2.0"
        old["store_format"] = {
            "current": 1, "read": {"min": 0, "max": 1},
            "write": {"min": 1, "max": 1}, "legacy": [0],
        }
        self.binary.write_text(
            f"#!{sys.executable}\nimport json\nprint(json.dumps({old!r}))\n",
            encoding="utf-8",
        )
        self.binary.chmod(0o755)
        shutil.copy2(self.binary, artifact / "memento")
        runtime._write(artifact / ".memento-owner.json",
                       {"owner": "memento", "kind": "artifact", "format_version": 1})
        active = {
            "directory": "legacy", "build_identity": old["build_identity"],
            "binary_sha256": runtime.hashlib.sha256(self.binary.read_bytes()).hexdigest(),
            "version": old, "embedding_selection": "none", "model_key": None,
            "requested_source_digest": None, "model_digest": None,
            "semantic_config_sha256": None,
            "os": platform.system(), "architecture": platform.machine(),
        }
        runtime._write(home / "state.json", {
            "format_version": 1, "status": "ready", "active": active, "previous": None,
        })
        return plugin, home, active

    def test_store_format_upgrade_preserves_known_no_model_selection(self):
        plugin, home, previous = self.legacy_runtime()
        old_bytes = (home / "artifacts/legacy/memento").read_bytes()
        self.handshake(self.version)
        with patch.dict(os.environ, {"MEMENTO_RUNTIME_HOME": str(home)}):
            prepared = runtime.ensure_runtime(plugin, binary=self.binary)
        self.assertEqual(prepared.state["format_version"], 1)
        self.assertEqual(prepared.state["active"]["version"]["store_format"]["current"], 2)
        self.assertEqual(prepared.state["active"]["embedding_selection"], "none")
        self.assertEqual(prepared.state["previous"], previous)
        self.assertIsNone(prepared.semantic_config)
        self.assertEqual((home / "artifacts/legacy/memento").read_bytes(), old_bytes)
        self.assertFalse((home / "models").exists())
        self.assertEqual(list(home.rglob("*.sqlite")), [])

    def test_incompatible_update_does_not_replace_the_existing_artifact(self):
        plugin, home, previous = self.legacy_runtime()
        old_bytes = (home / "artifacts/legacy/memento").read_bytes()
        with patch.dict(os.environ, {"MEMENTO_RUNTIME_HOME": str(home)}):
            with self.assertRaisesRegex(runtime.RuntimeError, "current store format"):
                runtime.ensure_runtime(plugin, binary=self.binary)
        state = json.loads((home / "state.json").read_text(encoding="utf-8"))
        self.assertEqual(state["status"], "failed")
        self.assertEqual(state["active"], previous)
        self.assertEqual((home / "artifacts/legacy/memento").read_bytes(), old_bytes)
        self.assertFalse((home / "candidate").exists())


if __name__ == "__main__":
    unittest.main()
