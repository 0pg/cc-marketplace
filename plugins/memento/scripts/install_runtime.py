#!/usr/bin/env python3
"""Install the local Memento runtime and selected embedding model."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


DEFAULT_EMBEDDING_MODEL = "e5-small"


class InstallError(Exception):
    """A failed installation that leaves the previous runtime intact."""


def install_embeddings(repository, destination, stage, model):
    scripts = repository / "core" / "scripts"
    requirements = scripts / "semantic-requirements.txt"
    setup = scripts / "setup_embeddings.py"
    # Keep virtual environments at stable paths outside the replaceable skill.
    # A different pinned setup gets a new runtime without changing the active one.
    version = hashlib.sha256(requirements.read_bytes() + setup.read_bytes()).hexdigest()[:16]
    root = destination.parent / ".memento-semantic"
    runtime = root / f"{model}-{version}"
    if root.is_symlink() or runtime.is_symlink():
        raise InstallError("The semantic runtime directory must not be a symbolic link.")
    python = runtime / "venv" / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    model_dir = runtime / "model"
    ready = runtime / "ready"
    if not ready.is_file():
        uv = shutil.which("uv")
        if uv is None:
            raise InstallError("uv was not found. Install uv separately to use --embedding-model.")
        runtime.mkdir(parents=True, exist_ok=True)
        steps = []
        if not python.is_file():
            steps.append(([uv, "venv", "--python", "3.12", str(runtime / "venv")], "Creating the semantic Python runtime"))
        steps.extend([
            ([uv, "pip", "install", "--python", str(python), "-r", str(requirements)], "Installing semantic dependencies"),
            ([str(python), str(setup), "--model", model, "--output", str(model_dir)], f"Installing the pinned {model} model"),
        ])
        for command, description in steps:
            print(f"{description}...", flush=True)
            result = subprocess.run(command, check=False)
            if result.returncode != 0:
                raise InstallError(f"{description} failed (exit {result.returncode}); the existing skill was not changed.")
    worker = stage / "scripts" / "local_embeddings.py"
    shutil.copy2(scripts / "local_embeddings.py", worker)
    try:
        manifest = json.loads((model_dir / "work-context-model.json").read_text())
    except (OSError, ValueError) as error:
        raise InstallError("The installed embedding model has no valid manifest.") from error
    if not isinstance(manifest, dict) or not all(
        isinstance(manifest.get(key), str) and manifest.get(key)
        for key in ("model_id", "model_revision")
    ):
        raise InstallError("The embedding model manifest is missing its identity or revision.")
    request = {
        "protocol": 1, "model_id": manifest["model_id"], "model_revision": manifest["model_revision"],
        "query": "installation check", "documents": ["installation check"],
    }
    print("Checking local embedding inference...", flush=True)
    try:
        result = subprocess.run(
            [str(python), str(worker), "--model-dir", str(model_dir)],
            input=json.dumps(request), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, check=False, timeout=120,
        )
    except subprocess.TimeoutExpired as error:
        raise InstallError("Local embedding inference exceeded 120 seconds; the existing skill was not changed.") from error
    if result.returncode != 0:
        raise InstallError(f"Local embedding inference failed (exit {result.returncode}); the existing skill was not changed.")
    try:
        response = json.loads(result.stdout)
    except ValueError as error:
        raise InstallError("The embedding worker did not return valid JSON.") from error
    if not isinstance(response, dict) or any(
        response.get(key) != request.get(key)
        for key in ("protocol", "model_id", "model_revision")
    ):
        raise InstallError("The embedding worker returned a different model identity or protocol.")
    query = response.get("query")
    documents = response.get("documents")
    if not isinstance(query, list) or not query or not isinstance(documents, list) or len(documents) != 1:
        raise InstallError("The embedding worker did not return query and document vectors.")
    document = next(iter(documents))
    if not isinstance(document, list) or len(document) != len(query) or any(
        type(value) not in (int, float) or not math.isfinite(value)
        for value in query + document
    ) or not any(query) or not any(document):
        raise InstallError("The embedding worker returned invalid vectors.")
    config = {
        "command": [str(python), str(destination / "scripts" / "local_embeddings.py"), "--model-dir", str(model_dir)],
        "model_id": manifest["model_id"], "model_revision": manifest["model_revision"],
    }
    (stage / "semantic-config.json").write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
    ready.write_text("Local embedding inference passed.\n", encoding="utf-8")


def reject_tree_links(directory):
    """Never follow a copied file into an unrelated location."""
    if directory.is_symlink():
        raise InstallError(f"Symbolic links are not copied: {directory}")
    for root, directories, files in os.walk(directory, followlinks=False):
        for name in directories + files:
            path = Path(root) / name
            if path.is_symlink():
                raise InstallError(f"Symbolic links are not copied: {path}")
            if not path.is_dir() and not path.is_file():
                raise InstallError(f"Only regular files and directories are copied: {path}")


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


def install(plugin, executable, embedding_model=DEFAULT_EMBEDDING_MODEL):
    destination = plugin / "skills/memento"
    launcher = destination / "scripts/memento.py"
    if not launcher.is_file():
        raise InstallError(f"The skill launcher is missing: {launcher}")
    reject_tree_links(destination)
    workspace = Path(tempfile.mkdtemp(prefix=".memento-install-", dir=destination.parent))
    stage = workspace / "stage"
    backup = workspace / "previous"
    try:
        # Preserve user-added files while staging the CLI and model configuration together.
        shutil.copytree(destination, stage, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        (stage / "bin").mkdir(exist_ok=True)
        shutil.copy2(executable, stage / "bin" / "memento")
        smoke_test(stage)
        if embedding_model is not None:
            install_embeddings(plugin, destination, stage, embedding_model)
        if destination.exists() or destination.is_symlink():
            destination.rename(backup)
        try:
            stage.rename(destination)
        except OSError as error:
            if backup.exists() or backup.is_symlink():
                try:
                    backup.rename(destination)
                except OSError as rollback_error:
                    raise InstallError(
                        f"Installation failed and the previous installation could not be restored. "
                        f"It is preserved at {backup}: {rollback_error}"
                    ) from error
            raise InstallError(f"Installation failed; the previous installation was restored: {error}") from error
        if backup.exists() or backup.is_symlink():
            try:
                shutil.rmtree(backup)
            except OSError as error:
                print(f"Warning: installed successfully, but the old backup remains at {backup}: {error}", file=sys.stderr)
    finally:
        # A failed rollback must never delete the only surviving old install.
        if not backup.exists() and not backup.is_symlink():
            shutil.rmtree(workspace, ignore_errors=True)
    return destination / "bin/memento"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Bundle an existing memento executable instead of building with Cargo")
    parser.add_argument("--embedding-model", choices=("e5-small", "minilm", "none"), default=DEFAULT_EMBEDDING_MODEL, help="Install the local model, Python runtime and semantic configuration (default: e5-small; requires uv); none skips model setup")
    args = parser.parse_args(argv)
    embedding_model = None if args.embedding_model == "none" else args.embedding_model
    plugin = Path(__file__).resolve().parent.parent
    try:
        executable = validate_binary(args.binary) if args.binary is not None else build_binary(plugin)
        destination = install(plugin, executable, embedding_model)
    except (InstallError, OSError, shutil.Error) as error:
        print(f"Installation failed: {error}", file=sys.stderr)
        return 1
    print(f"Installed Memento runtime: {destination}")
    if embedding_model is not None:
        print(f"Embedding model: {embedding_model}")
        print(f"Semantic config: {plugin / 'skills/memento/semantic-config.json'} (used automatically by the launcher)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
