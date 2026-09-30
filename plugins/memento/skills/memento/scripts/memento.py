#!/usr/bin/env python3
"""Run an already-built memento CLI without implicit installation."""
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys


def main() -> int:
    override = os.environ.get("MEMENTO_BIN")
    skill_directory = Path(__file__).resolve().parent.parent
    plugin_root = skill_directory.parent.parent
    candidates = [override] if override is not None else [
        str(skill_directory / "bin/memento"),
        shutil.which("memento"),
        str(plugin_root / "core/target/release/memento"),
        str(plugin_root / "core/target/debug/memento"),
    ]
    executable = next(
        (p for p in candidates if p and Path(p).is_file() and os.access(p, os.X_OK)),
        None,
    )
    if executable is None:
        print(
            f"Install this plugin runtime with python3 {shlex.quote(str(plugin_root / 'scripts/install_runtime.py'))}, "
            "or set MEMENTO_BIN to an executable.",
            file=sys.stderr,
        )
        return 2
    try:
        return subprocess.run([executable, *sys.argv[1:]], check=False).returncode
    except OSError as error:
        print(f"Cannot run memento: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
