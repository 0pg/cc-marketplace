#!/usr/bin/env python3
"""Build or bundle the local Memento runtime in this plugin installation."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


class InstallError(Exception):
    """A failed installation that leaves the previous runtime intact."""


def validate_binary(path):
    path = path.expanduser()
    if path.is_symlink():
        raise InstallError(f"The executable must be a regular file: {path}")
    if not path.is_file() or not os.access(path, os.X_OK):
        raise InstallError(f"The executable is missing or not executable: {path}")
    return path.resolve()


def build_binary(plugin):
    core = plugin / "core"
    command = [
        "cargo", "build", "--locked", "--release",
        "--manifest-path", str(core / "Cargo.toml"),
        "--message-format=json-render-diagnostics",
    ]
    print("Building the local memento CLI...", flush=True)
    try:
        result = subprocess.run(
            command, cwd=core, stdout=subprocess.PIPE,
            text=True, encoding="utf-8", errors="replace", check=False,
        )
    except FileNotFoundError as error:
        raise InstallError("Cargo was not found. Install Rust separately, or pass --binary /path/to/memento.") from error
    executable = None
    for line in result.stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict):
            continue
        if event.get("reason") == "compiler-message":
            message = event.get("message", {})
            if isinstance(message, dict) and message.get("rendered"):
                print(message["rendered"], file=sys.stderr, end="")
        target = event.get("target", {})
        if (event.get("reason") == "compiler-artifact"
                and isinstance(target, dict)
                and target.get("name") == "memento"
                and "bin" in target.get("kind", [])
                and isinstance(event.get("executable"), str)):
            executable = Path(event["executable"])
    if result.returncode != 0:
        raise InstallError(f"Cargo build failed (exit {result.returncode}); the previous runtime was not changed.")
    if executable is None:
        raise InstallError("Cargo did not report a memento executable artifact.")
    if not executable.is_absolute():
        executable = core / executable
    return validate_binary(executable)


def smoke_test(stage):
    environment = os.environ.copy()
    environment.pop("MEMENTO_BIN", None)
    result = subprocess.run(
        [sys.executable, str(stage / "scripts/memento.py"), "help"],
        cwd=stage, env=environment, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8", errors="replace", check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "no diagnostic output"
        raise InstallError(f"Bundled CLI smoke test failed (exit {result.returncode}): {detail}")
    try:
        help_result = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise InstallError("The executable did not return memento help JSON.") from error
    commands = help_result.get("commands") if isinstance(help_result, dict) else None
    if not isinstance(commands, list) or not all(name in commands for name in ("init", "note", "query", "compact")):
        raise InstallError("The executable does not provide the expected memento commands.")


def install(plugin, executable):
    skill = plugin / "skills/memento"
    launcher = skill / "scripts/memento.py"
    if not launcher.is_file():
        raise InstallError(f"The skill launcher is missing: {launcher}")
    binary_directory = skill / "bin"
    if binary_directory.is_symlink():
        raise InstallError(f"The runtime directory must not be a symbolic link: {binary_directory}")
    binary_directory.mkdir(exist_ok=True)
    destination = binary_directory / "memento"
    # Publish just the runtime, so stores and user-added files are left in place.
    # The candidate is verified through the distributed launcher before replacement.
    with tempfile.TemporaryDirectory(prefix=".memento-runtime-", dir=binary_directory) as workspace:
        stage = Path(workspace)
        (stage / "bin").mkdir()
        (stage / "scripts").mkdir()
        candidate = stage / "bin/memento"
        shutil.copy2(executable, candidate)
        shutil.copy2(launcher, stage / "scripts/memento.py")
        smoke_test(stage)
        os.replace(candidate, destination)
    return destination


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Bundle an existing memento executable instead of building with Cargo")
    args = parser.parse_args(argv)
    plugin = Path(__file__).resolve().parent.parent
    try:
        executable = validate_binary(args.binary) if args.binary is not None else build_binary(plugin)
        destination = install(plugin, executable)
    except (InstallError, OSError, shutil.Error) as error:
        print(f"Installation failed: {error}", file=sys.stderr)
        return 1
    print(f"Installed Memento runtime: {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
