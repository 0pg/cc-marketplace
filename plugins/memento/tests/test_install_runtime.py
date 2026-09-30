"""Exercise the distributed runtime in independent plugin installations.

Build core first, then run with MEMENTO_TEST_BINARY=/absolute/path/work-context.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


PLUGIN = Path(__file__).resolve().parent.parent
NOTE = "다른 작업 폴더에서 Memento 런타임으로 결정 맥락을 기록합니다."


class RuntimeInstallationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        selected = os.environ.get("MEMENTO_TEST_BINARY")
        cls.binary = Path(selected).resolve() if selected else PLUGIN / "core/target/debug/work-context"
        if not cls.binary.is_file() or not os.access(cls.binary, os.X_OK):
            raise RuntimeError("Build core first, or set MEMENTO_TEST_BINARY to its real executable.")

    def setUp(self):
        self.workspace = tempfile.TemporaryDirectory(prefix="memento runtime installation ")
        self.addCleanup(self.workspace.cleanup)
        self.root = Path(self.workspace.name)
        self.plugin = self.root / "independent plugin"
        self.skill = self.plugin / "skills/memento"
        self.outside = self.root / "another project"
        self.outside.mkdir()
        self.empty_path = self.root / "empty executable path"
        self.empty_path.mkdir()
        (self.plugin / "scripts").mkdir(parents=True)
        (self.skill / "scripts").mkdir(parents=True)
        shutil.copy2(PLUGIN / "scripts/install_runtime.py", self.plugin / "scripts/install_runtime.py")
        shutil.copy2(PLUGIN / "skills/memento/scripts/work-context.py", self.skill / "scripts/work-context.py")
        self.environment = os.environ.copy()
        self.environment.pop("WORK_CONTEXT_BIN", None)
        self.environment["PATH"] = str(self.empty_path)
        self.database = self.root / "selected local data/context.sqlite"
        self.database.parent.mkdir()

    def install(self, binary=None):
        return subprocess.run(
            [sys.executable, str(self.plugin / "scripts/install_runtime.py"), "--binary", str(binary or self.binary)],
            cwd=self.outside, env=self.environment, text=True, capture_output=True, check=False,
        )

    def assert_success(self, result):
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def cli(self, operation, value=None):
        command = [sys.executable, str(self.skill / "scripts/work-context.py"), operation, "--store", str(self.database)]
        if operation in ("init", "note"):
            command.extend(["--project", "installation-tests"])
        result = subprocess.run(
            command, input=json.dumps(value) if value is not None else None,
            cwd=self.outside, env=self.environment, text=True, capture_output=True, check=False,
        )
        self.assert_success(result)
        return json.loads(result.stdout)

    def create_note(self):
        self.cli("init")
        receipt = self.cli("note", {"id": "install-note", "kind": "decision", "body": NOTE})
        self.assertTrue(receipt["receipt"]["durable"])
        self.assert_note()

    def assert_note(self):
        response = self.cli("query", {
            "operation": "read", "scope": {"project_id": "installation-tests"},
            "target": {"kind": "record", "id": "install-note"},
        })
        records = [item["entity"]["data"] for item in response["items"]
                   if item["entity"]["entity"] == "record"]
        self.assertTrue(any(record["id"] == "install-note" and record["body"] == NOTE
                            for record in records))

    def runtime_hash(self):
        return hashlib.sha256((self.skill / "bin/work-context").read_bytes()).hexdigest()

    def test_independent_install_runs_outside_checkout_with_path_spaces_and_empty_path(self):
        self.assert_success(self.install())
        self.assertEqual(self.runtime_hash(), hashlib.sha256(self.binary.read_bytes()).hexdigest())
        self.assertFalse((self.plugin / "core").exists())
        self.create_note()

    def test_reinstall_preserves_selected_database_and_user_files(self):
        self.assert_success(self.install())
        self.create_note()
        custom = self.skill / "personal notes.txt"
        custom.write_text("사용자가 선택한 파일", encoding="utf-8")
        database_before = self.database.read_bytes()
        self.assert_success(self.install())
        self.assertEqual(self.database.read_bytes(), database_before)
        self.assertEqual(custom.read_text(encoding="utf-8"), "사용자가 선택한 파일")
        self.assert_note()

    def test_failing_executable_leaves_prior_runtime_and_database_intact(self):
        self.assert_success(self.install())
        self.create_note()
        runtime_before = self.runtime_hash()
        database_before = self.database.read_bytes()
        failing = self.root / "failing executable"
        failing.write_text(f"#!{sys.executable}\nraise SystemExit(19)\n", encoding="utf-8")
        failing.chmod(0o755)
        result = self.install(failing)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("smoke test failed", result.stderr)
        self.assertEqual(self.runtime_hash(), runtime_before)
        self.assertEqual(self.database.read_bytes(), database_before)
        self.assert_note()


if __name__ == "__main__":
    unittest.main()
