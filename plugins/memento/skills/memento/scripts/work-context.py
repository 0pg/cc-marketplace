#!/usr/bin/env python3
"""Run an already-built work-context CLI without implicit installation."""
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys


def main() -> int:
    override = os.environ.get("WORK_CONTEXT_BIN")
    skill_directory = Path(__file__).resolve().parent.parent
    plugin_root = skill_directory.parent.parent
    candidates = [override] if override is not None else [
        str(skill_directory / "bin/work-context"),
        shutil.which("work-context"),
        str(plugin_root / "core/target/release/work-context"),
        str(plugin_root / "core/target/debug/work-context"),
    ]
    executable = next(
        (p for p in candidates if p and Path(p).is_file() and os.access(p, os.X_OK)),
        None,
    )
    if executable is None:
        print(
            f"Install this plugin runtime with python3 {shlex.quote(str(plugin_root / 'scripts/install_runtime.py'))}, "
            "or set WORK_CONTEXT_BIN to an executable.",
            file=sys.stderr,
        )
        return 2
    try:
        return subprocess.run([executable, *sys.argv[1:]], check=False).returncode
    except OSError as error:
        print(f"Cannot run work-context: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
