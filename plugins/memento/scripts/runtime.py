#!/usr/bin/env python3
"""Shared, local-only runtime lifecycle for launchers, installers and hooks."""
from __future__ import annotations
from contextlib import contextmanager
from dataclasses import dataclass
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import runpy
import re
import shutil
import signal
import subprocess
import sys
import time


FORMAT_VERSION = 1
PROTOCOL_VERSION = 1
STORE_FORMAT_VERSION = 2


class RuntimeError(Exception):
    """A preparation failure; the active runtime and stores remain untouched."""


@dataclass(frozen=True)
class Runtime:
    executable: Path
    semantic_config: Path | None
    state: dict


def runtime_home():
    override = os.environ.get("MEMENTO_RUNTIME_HOME")
    base = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex")))
    return Path(override).expanduser().absolute() if override else base / "memento/runtime"


def _json(path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise RuntimeError(f"Cannot read runtime state {path}: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"Expected a JSON object: {path}")
    return value


def _write(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    if temporary.is_symlink() or path.is_symlink():
        raise RuntimeError(f"Runtime state symbolic links are not followed: {path}")
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    temporary.chmod(0o600)
    temporary.replace(path)


def _state(home):
    path = home / "state.json"
    if not path.exists():
        return {"format_version": FORMAT_VERSION, "status": "not_installed", "active": None, "previous": None}
    if path.is_symlink():
        raise RuntimeError("Runtime configuration must not be a symbolic link.")
    state = _json(path)
    if type(state.get("format_version")) is not int or state.get("format_version") != FORMAT_VERSION:
        raise RuntimeError("Unsupported runtime configuration format; initialization was not attempted.")
    if state.get("status") not in ("not_installed", "preparing", "ready", "failed"):
        raise RuntimeError("Unknown runtime initialization state.")
    return state


def _regular(path):
    path = Path(path).expanduser().absolute()
    if path.is_symlink() or not path.is_file() or not os.access(path, os.X_OK):
        raise RuntimeError(f"The executable must be a regular executable file: {path}")
    return path


def run_command(command, *, timeout=120, **kwargs):
    """Bound execution and terminate the process group, including Cargo children."""
    input_value = kwargs.pop("input", None)
    if input_value is not None:
        kwargs["stdin"] = subprocess.PIPE
    try:
        process = subprocess.Popen(command, start_new_session=True, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, text=True, **kwargs)
    except FileNotFoundError as error:
        raise RuntimeError(f"Required executable was not found: {command[0]}") from error
    try:
        stdout, stderr = process.communicate(input=input_value, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate()
        raise RuntimeError(f"Command exceeded {timeout} seconds: {command[0]}") from error
    return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)


def handshake(executable):
    result = run_command([str(_regular(executable)), "version"], timeout=15)
    if result.returncode:
        raise RuntimeError(f"Runtime handshake failed (exit {result.returncode}): {result.stderr.strip()}")
    try:
        value = json.loads(result.stdout)
    except ValueError as error:
        raise RuntimeError("Runtime handshake did not return version JSON.") from error
    if not isinstance(value, dict) or value.get("protocol_version") != PROTOCOL_VERSION:
        raise RuntimeError("The executable has an incompatible Memento protocol.")
    capabilities = value.get("capabilities", [])
    store = value.get("store_format", {})
    if not isinstance(capabilities, list) or "checkpoint" not in capabilities or not isinstance(store, dict):
        raise RuntimeError("The executable does not provide the required checkpoint capability.")
    if type(store.get("current")) is not int or store["current"] != STORE_FORMAT_VERSION:
        raise RuntimeError("The executable has an incompatible current store format.")
    for access in ("read", "write"):
        bounds = store.get(access, {})
        if (not isinstance(bounds, dict) or type(bounds.get("min")) is not int
                or type(bounds.get("max")) is not int
                or not bounds["min"] <= STORE_FORMAT_VERSION <= bounds["max"]):
            raise RuntimeError("The executable does not support the current store format.")
    reported = value.get("platform")
    allowed_os = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}.get(platform.system(), platform.system().lower())
    allowed_arch = {"arm64": "aarch64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())
    if not isinstance(reported, dict) or reported.get("os") != allowed_os or reported.get("arch") != allowed_arch:
        raise RuntimeError("The executable belongs to another operating system or architecture.")
    if not isinstance(value.get("build_identity"), str) or not value["build_identity"]:
        raise RuntimeError("The executable did not declare its build identity.")
    return value


def source_digest(plugin):
    core = Path(plugin) / "core"
    paths = [core / "Cargo.toml", core / "Cargo.lock"]
    if not all(path.is_file() for path in paths):
        return None
    if (core / "build.rs").is_file():
        paths.append(core / "build.rs")
    paths.extend(sorted((core / "src").rglob("*.rs")))
    digest = hashlib.sha256()
    for path in paths:
        if path.is_symlink():
            raise RuntimeError(f"Source symbolic links are not followed: {path}")
        digest.update(path.relative_to(core).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _package_version(plugin):
    path = Path(plugin) / "plugin.json"
    return _json(path).get("version") if path.is_file() else None


def _artifact(home, value):
    if not isinstance(value, dict) or not isinstance(value.get("directory"), str):
        raise RuntimeError("No active runtime is installed. Run scripts/install_runtime.py --ensure.")
    path = home / "artifacts" / value["directory"]
    if path.parent != home / "artifacts" or path.is_symlink():
        raise RuntimeError("The runtime artifact path is invalid.")
    return path


def runtime_status(plugin):
    home = runtime_home()
    state = _state(home)
    status = {**state, "runtime_home": str(home), "plugin_version": _package_version(plugin),
              "desired_build": source_digest(plugin), "protocol_version": PROTOCOL_VERSION, "ready_for_package": False}
    if state.get("active"):
        try:
            ready_runtime(plugin)
            status["ready_for_package"] = True
        except RuntimeError as error:
            status["validation_error"] = str(error)
            if status["status"] == "ready":
                status.update(status="failed", persisted_status="ready")
    return status


def ready_runtime(plugin, override=None):
    """Resolve a validated runtime quickly; never prepare or change a store."""
    plugin = Path(plugin)
    home = runtime_home()
    state = _state(home)
    selected = override or os.environ.get("MEMENTO_BIN")
    if selected:
        executable = _regular(selected)
        version = handshake(executable)
        active_config = (_artifact(home, state["active"]) / "semantic-config.json") if state.get("active") else Path(plugin) / "skills/memento/semantic-config.json"
        if active_config.is_file():
            _validate_semantic_config(executable, active_config)
        return Runtime(executable, active_config if active_config.is_file() else None, {**state, "origin": "override", "version": version})
    if not state.get("active"):
        bundled = Path(plugin) / "skills/memento/bin/memento"
        version = handshake(bundled)
        config = Path(plugin) / "skills/memento/semantic-config.json"
        if config.is_file():
            _validate_semantic_config(bundled, config)
        return Runtime(bundled, config if config.is_file() else None, {**state, "origin": "bundled", "version": version})
    active = _artifact(home, state.get("active"))
    executable = _regular(active / "memento")
    version = handshake(executable)
    if state["active"].get("os") != platform.system() or state["active"].get("architecture") != platform.machine():
        raise RuntimeError("The active runtime belongs to another operating system or architecture.")
    expected = state["active"].get("build_identity")
    if version["build_identity"] != expected:
        raise RuntimeError("The active executable build identity changed; initialization is required.")
    if hashlib.sha256(executable.read_bytes()).hexdigest() != state["active"].get("binary_sha256"):
        raise RuntimeError("The active executable digest changed; initialization is required.")
    desired = source_digest(plugin)
    if desired is not None and state["active"].get("requested_source_digest") != desired:
        raise RuntimeError("The installed runtime requires an update for this plugin build.")
    bundled = plugin / "skills/memento/bin/memento"
    if (desired is None and bundled.is_file()
            and hashlib.sha256(_regular(bundled).read_bytes()).hexdigest() != state["active"].get("binary_sha256")):
        raise RuntimeError("The installed runtime requires an update for this plugin binary.")
    selection = state["active"].get("embedding_selection")
    # The stable Git bridge has no package assets; it validates the active files.
    if selection in ("e5-small", "minilm") and (plugin / "core/scripts").is_dir():
        if state["active"].get("model_digest") != _model_digest(plugin, selection):
            raise RuntimeError("The installed semantic runtime requires an update for this plugin model setup.")
    config = active / "semantic-config.json"
    actual_config_digest = hashlib.sha256(config.read_bytes()).hexdigest() if config.is_file() else None
    if actual_config_digest != state["active"].get("semantic_config_sha256"):
        raise RuntimeError("The active semantic configuration digest changed; preparation is required.")
    model_key = state["active"].get("model_key")
    if model_key:
        model_root = home / "models"
        model = model_root / model_key
        if model_root.is_symlink() or model.is_symlink() or model.parent != model_root:
            raise RuntimeError("The active semantic runtime path is invalid.")
        marker = _json(model / "ready.json")
        if not config.is_file():
            raise RuntimeError("The active semantic configuration is missing.")
        configured = _config(config)
        manifest = _json(model / "model/work-context-model.json")
        if any(manifest.get(key) != configured[key] or marker.get(key) != configured[key] for key in ("model_id", "model_revision")):
            raise RuntimeError("The active semantic model identity changed.")
        worker = model / "local_embeddings.py"
        if not worker.is_file() or hashlib.sha256(worker.read_bytes()).hexdigest() != marker.get("worker_sha256"):
            raise RuntimeError("The active semantic worker digest changed.")
        if not (model / "venv/bin/python").is_file():
            raise RuntimeError("The active semantic Python executable is missing.")
    return Runtime(executable, config if config.is_file() else None, state)


@contextmanager
def _lock(home):
    if home.is_symlink():
        raise RuntimeError("The runtime directory must not be a symbolic link.")
    home.mkdir(parents=True, exist_ok=True, mode=0o700)
    path = home / "install.lock"
    if path.is_symlink():
        raise RuntimeError("The runtime lock must not be a symbolic link.")
    with path.open("a+") as stream:
        deadline = time.monotonic() + 120
        while True:
            try:
                fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise RuntimeError("Another runtime preparation is still running; retry later.")
                time.sleep(0.05)
        try:
            yield
        finally:
            fcntl.flock(stream, fcntl.LOCK_UN)


def _build(plugin, candidate, digest):
    core = plugin / "core"
    environment = os.environ.copy()
    environment["MEMENTO_BUILD_ID"] = digest
    environment["CARGO_TARGET_DIR"] = str(candidate / "target")
    command = ["cargo", "build", "--locked", "--release", "--manifest-path", str(core / "Cargo.toml"),
               "--message-format=json-render-diagnostics"]
    print("Building the local Memento runtime...", file=sys.stderr, flush=True)
    result = run_command(command, timeout=1800, cwd=core, env=environment)
    if result.stderr:
        print(result.stderr, file=sys.stderr, end="")
    executable = None
    for line in result.stdout.splitlines():
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if not isinstance(event, dict):
            continue
        target = event.get("target", {})
        if (event.get("reason") == "compiler-artifact" and isinstance(target, dict)
                and target.get("name") == "memento" and "bin" in target.get("kind", [])
                and isinstance(event.get("executable"), str)):
            executable = Path(event["executable"])
    if result.returncode or executable is None:
        raise RuntimeError(f"Cargo build failed (exit {result.returncode}); the previous runtime was preserved.")
    shutil.copy2(_regular(executable), candidate / "memento")
    shutil.rmtree(candidate / "target", ignore_errors=True)


def _config(path):
    value = _json(path)
    if (not isinstance(value.get("command"), list) or not value["command"]
            or not all(isinstance(item, str) and item for item in value["command"])
            or not all(isinstance(value.get(key), str) and value[key] for key in ("model_id", "model_revision"))):
        raise RuntimeError("The semantic configuration has no valid command/model identity.")
    return value


def _validate_semantic_config(executable, path):
    """Rust owns the complete configuration schema and compatibility contract."""
    result = run_command([str(executable), "semantic-config-check", "--semantic-config", str(path)], timeout=15)
    if result.returncode:
        raise RuntimeError(f"Semantic configuration is incompatible with the executable: {result.stderr.strip()}")
    try:
        response = json.loads(result.stdout)
    except ValueError as error:
        raise RuntimeError("Semantic configuration validation did not return JSON.") from error
    if not isinstance(response, dict) or response.get("valid") is not True:
        raise RuntimeError("The executable did not confirm a valid semantic configuration.")


def _model_digest(plugin, model):
    scripts = plugin / "core/scripts"
    digest = hashlib.sha256()
    for path in (scripts / "semantic-requirements.txt", scripts / "setup_embeddings.py", scripts / "local_embeddings.py"):
        digest.update(path.read_bytes())
        digest.update(b"\0")
    digest.update(json.dumps({"model": model, "os": platform.system(), "arch": platform.machine(),
                              "python_abi": "cpython-312", "host_python": sys.implementation.cache_tag}, sort_keys=True).encode())
    return digest.hexdigest()


def _inference(config):
    request = {"protocol": 1, "model_id": config["model_id"], "model_revision": config["model_revision"],
               "query": "installation check", "documents": ["installation check"]}
    result = run_command(config["command"], input=json.dumps(request), timeout=120)
    if result.returncode:
        raise RuntimeError(f"Local embedding inference failed (exit {result.returncode}).")
    try:
        response = json.loads(result.stdout)
    except ValueError as error:
        raise RuntimeError("The embedding worker did not return JSON.") from error
    if not isinstance(response, dict) or any(response.get(key) != request[key] for key in ("protocol", "model_id", "model_revision")):
        raise RuntimeError("The embedding worker returned a different model identity or protocol.")
    query, documents = response.get("query"), response.get("documents")
    if not isinstance(query, list) or not query or not isinstance(documents, list) or len(documents) != 1:
        raise RuntimeError("The embedding worker did not return query and document vectors.")
    document = documents[0]
    if (not isinstance(document, list) or len(document) != len(query)
            or any(type(value) not in (int, float) or not math.isfinite(value) for value in query + document)
            or not any(query) or not any(document)):
        raise RuntimeError("The embedding worker returned invalid vectors.")


def install_embeddings(plugin, home, model):
    scripts = plugin / "core/scripts"
    key = f"{model}-{_model_digest(plugin, model)[:20]}"
    directory = home / "models" / key
    if (home / "models").is_symlink():
        raise RuntimeError("The model runtime root must not be a symbolic link.")
    if directory.is_symlink():
        raise RuntimeError("The model runtime must not be a symbolic link.")
    if directory.exists():
        owner = directory / ".memento-owner.json"
        if (not directory.is_dir() or owner.is_symlink() or not owner.is_file()
                or _json(owner) != {"owner": "memento", "kind": "model", "format_version": 1}):
            raise RuntimeError("The model runtime directory contains unrecognized user files; it was left untouched.")
    python = directory / "venv/bin/python"
    worker = directory / "local_embeddings.py"
    model_dir = directory / "model"
    if not (directory / "ready.json").is_file():
        uv = shutil.which("uv")
        if not uv:
            raise RuntimeError("uv was not found. Install uv separately or select --embedding-model none.")
        directory.mkdir(parents=True, exist_ok=True)
        _write(directory / ".memento-owner.json", {"owner": "memento", "kind": "model", "format_version": 1})
        commands = [([uv, "venv", "--python", "3.12", str(directory / "venv")], "Creating semantic Python runtime"),
                    ([uv, "pip", "install", "--python", str(python), "-r", str(scripts / "semantic-requirements.txt")], "Installing semantic dependencies"),
                    ([str(python), str(scripts / "setup_embeddings.py"), "--model", model, "--output", str(model_dir)], "Installing pinned embedding model")]
        for command, step in commands:
            print(f"{step}...", file=sys.stderr, flush=True)
            result = run_command(command, timeout=900)
            if result.returncode:
                raise RuntimeError(f"{step} failed (exit {result.returncode}).")
        shutil.copy2(scripts / "local_embeddings.py", worker)
    manifest = _json(model_dir / "work-context-model.json")
    expected = runpy.run_path(str(scripts / "setup_embeddings.py"))["MODELS"].get(model)
    if expected is None or (manifest.get("model_id"), manifest.get("model_revision")) != expected:
        raise RuntimeError("The installed model does not match the pinned model identity/revision.")
    if worker.read_bytes() != (scripts / "local_embeddings.py").read_bytes():
        raise RuntimeError("The installed semantic worker digest changed.")
    if not all(isinstance(manifest.get(key), str) and manifest[key] for key in ("model_id", "model_revision")):
        raise RuntimeError("The embedding manifest is missing its identity or revision.")
    config = {"command": [str(python), str(worker), "--model-dir", str(model_dir)],
              "model_id": manifest["model_id"], "model_revision": manifest["model_revision"]}
    _inference(config)
    _write(directory / "ready.json", {"format_version": 1, "model": model, "digest": _model_digest(plugin, model),
                                       "model_id": config["model_id"], "model_revision": config["model_revision"],
                                       "worker_sha256": hashlib.sha256(worker.read_bytes()).hexdigest()})
    return config, key


def _prune(home, state):
    artifacts = home / "artifacts"
    keep = {entry.get("directory") for entry in (state.get("active"), state.get("previous")) if isinstance(entry, dict)}
    model_keep = {entry.get("model_key") for entry in (state.get("active"), state.get("previous")) if isinstance(entry, dict)}
    for directory, keys, kind, pattern in ((artifacts, keep, "artifact", r"[0-9a-f]{24}(?:-[0-9a-f]{12})?"),
                                           (home / "models", model_keep, "model", r"(?:e5-small|minilm)-[0-9a-f]{20}")):
        if directory.is_symlink():
            raise RuntimeError(f"Runtime artifact roots must not be symbolic links: {directory}")
        if directory.is_dir():
            for path in directory.iterdir():
                if path.name in keys or path.is_symlink() or not path.is_dir() or not re.fullmatch(pattern, path.name):
                    continue
                receipt = path / ".memento-owner.json"
                if receipt.is_symlink() or not receipt.is_file():
                    continue
                try:
                    owner = _json(receipt)
                except RuntimeError:
                    continue
                if owner == {"owner": "memento", "kind": kind, "format_version": 1}:
                    shutil.rmtree(path)


def git_bridge():
    return runtime_home() / "git-memento"


def _install_bridge(home):
    # Native Git hooks keep this stable path while immutable build artifacts rotate.
    helper = home / "runtime.py"
    if helper.is_symlink() or (home / "git-memento").is_symlink():
        raise RuntimeError("Git runtime bridge files must not be symbolic links.")
    source = Path(__file__)
    if source.absolute() != helper.absolute() and (not helper.is_file() or source.read_bytes() != helper.read_bytes()):
        temporary_helper = home / "runtime.py.tmp"
        if temporary_helper.is_symlink():
            raise RuntimeError("The Git runtime helper staging file must not be a symbolic link.")
        try:
            shutil.copy2(source, temporary_helper)
            temporary_helper.chmod(0o600)
            temporary_helper.replace(helper)
        except OSError:
            try:
                temporary_helper.unlink(missing_ok=True)
            except OSError:
                pass
            raise
    bridge = home / "git-memento"
    program = (f"#!{sys.executable}\n"
               "import os, pathlib, sys\n"
               "from runtime import RuntimeError, ready_runtime\n"
               f"os.environ['MEMENTO_RUNTIME_HOME'] = {str(home)!r}\n"
               "os.environ.pop('MEMENTO_BIN', None)\n"
               "try:\n"
               "    selected = ready_runtime(pathlib.Path(__file__).parent / 'bridge-context')\n"
               "    os.execv(str(selected.executable), [str(selected.executable), *sys.argv[1:]])\n"
               "except (RuntimeError, OSError) as error:\n"
               "    print(f'Memento Git runtime unavailable: {error}', file=sys.stderr)\n"
               "    raise SystemExit(2)\n")
    if bridge.is_file() and bridge.read_text(encoding="utf-8") == program:
        return
    temporary = home / "git-memento.tmp"
    if temporary.is_symlink():
        raise RuntimeError("The Git runtime bridge staging file must not be a symbolic link.")
    temporary.write_text(program, encoding="utf-8")
    temporary.chmod(0o700)
    temporary.replace(bridge)


def ensure_runtime(plugin, *, binary=None, embedding_model=None, semantic_config=None):
    """Prepare once under a process lock, preserve known selections, then activate."""
    plugin = Path(plugin).absolute()
    home = runtime_home()
    with _lock(home):
        state = _state(home)
        desired = source_digest(plugin)
        previous_active = state.get("active")
        previous_previous = state.get("previous")
        active_dir = _artifact(home, previous_active) if previous_active else None
        legacy_config = plugin / "skills/memento/semantic-config.json"
        selected = embedding_model
        hint = plugin / "runtime-selection.json"
        if selected is None and previous_active is None and hint.is_file():
            selection = _json(hint)
            if type(selection.get("format_version")) is not int or selection.get("format_version") != 1 or selection.get("embedding_selection") not in ("none", "custom"):
                raise RuntimeError("Unsupported bundled runtime selection format.")
            selected = selection["embedding_selection"]
        custom = None
        if semantic_config is not None:
            custom = _config(Path(semantic_config))
            selected = "custom"
        elif selected is None and previous_active:
            selected = previous_active.get("embedding_selection")
            if selected in ("custom", "none") and active_dir and (active_dir / "semantic-config.json").is_file():
                custom = _config(active_dir / "semantic-config.json")
        elif selected in (None, "custom") and legacy_config.is_file():
            custom = _config(legacy_config)
            selected = "custom"
        elif selected is None:
            legacy_binary = plugin / "skills/memento/bin/memento"
            if legacy_binary.exists() or os.environ.get("MEMENTO_BIN") or shutil.which("memento"):
                raise RuntimeError("selection_required: legacy model selection is unknown; choose --embedding-model e5-small, minilm or none.")
            selected = "e5-small"
        if selected == "custom" and custom is None:
            raise RuntimeError("selection_required: the selected custom model has no semantic configuration.")
        if selected not in ("e5-small", "minilm", "none", "custom"):
            raise RuntimeError("selection_required: runtime has no known model selection.")
        if selected == "none" and custom is None:
            config_path = active_dir / "semantic-config.json" if active_dir else legacy_config
            if config_path.is_file():
                custom = _config(config_path)
        desired_model_digest = _model_digest(plugin, selected) if selected in ("e5-small", "minilm") else None
        external_binary = binary or os.environ.get("MEMENTO_BIN")
        origin = "explicit" if external_binary else "source_build"
        bundled = plugin / "skills/memento/bin/memento"
        if external_binary is None and desired is None and bundled.is_file():
            external_binary = bundled
            origin = "bundled"
        external_matches = external_binary is None or (previous_active is not None
            and hashlib.sha256(_regular(external_binary).read_bytes()).hexdigest() == previous_active.get("binary_sha256"))
        # Revalidate ready artifacts. A failed prior update may still have a good active build.
        if (previous_active and previous_active.get("embedding_selection") == selected
                and (desired is None or previous_active.get("requested_source_digest") == desired)
                and previous_active.get("model_digest") == desired_model_digest
                and external_matches
                and (semantic_config is None or (active_dir / "semantic-config.json").is_file()
                     and _config(active_dir / "semantic-config.json") == custom)):
            try:
                prepared = ready_runtime(plugin)
                if state["status"] != "ready":
                    state.update(status="ready", stage=None, failure=None)
                    _write(home / "state.json", state)
                _install_bridge(home)
                _prune(home, state)
                if state.pop("cleanup_failure", None) is not None:
                    _write(home / "state.json", state)
                return Runtime(prepared.executable, prepared.semantic_config, state)
            except RuntimeError:
                pass
        _prune(home, state)
        candidate = home / "candidate"
        if candidate.exists() or candidate.is_symlink():
            receipt = candidate / ".memento-owner.json"
            if (candidate.is_symlink() or receipt.is_symlink() or not receipt.is_file()
                    or _json(receipt) != {"owner": "memento", "kind": "candidate", "format_version": 1}):
                raise RuntimeError("The runtime candidate contains unrecognized files; it was left untouched.")
            shutil.rmtree(candidate)
        candidate.mkdir(mode=0o700)
        _write(candidate / ".memento-owner.json", {"owner": "memento", "kind": "candidate", "format_version": 1})
        state.update(status="preparing", desired_build=desired, desired_model=selected, stage="binary", failure=None)
        _write(home / "state.json", state)
        try:
            source_built = False
            if external_binary:
                shutil.copy2(_regular(external_binary), candidate / "memento")
            elif (previous_active and previous_active.get("requested_source_digest") == desired
                  and hashlib.sha256((active_dir / "memento").read_bytes()).hexdigest() == previous_active.get("binary_sha256")):
                shutil.copy2(active_dir / "memento", candidate / "memento")
                origin = previous_active.get("origin", "validated_reuse")
            elif desired is not None:
                _build(plugin, candidate, desired)
                source_built = True
            else:
                raise RuntimeError("Cargo sources are unavailable; pass --binary to a compatible executable.")
            version = handshake(candidate / "memento")
            if source_built and version["build_identity"] != desired:
                raise RuntimeError("The executable build identity does not match this plugin source.")
            config, model_key = custom, previous_active.get("model_key") if previous_active else None
            if selected == "custom" and (active_dir is None or not (active_dir / "semantic-config.json").is_file()
                    or _config(active_dir / "semantic-config.json") != custom):
                model_key = None
            state["stage"] = "model"
            _write(home / "state.json", state)
            if selected in ("e5-small", "minilm"):
                config, model_key = install_embeddings(plugin, home, selected)
            if config:
                _write(candidate / "semantic-config.json", config)
                _validate_semantic_config(candidate / "memento", candidate / "semantic-config.json")
                if selected == "custom":
                    _inference(config)
            digest = hashlib.sha256((candidate / "memento").read_bytes()).hexdigest()
            identity = hashlib.sha256((version["build_identity"] + selected + json.dumps(config, sort_keys=True)).encode()).hexdigest()[:24]
            artifact = home / "artifacts" / identity
            artifact.parent.mkdir(exist_ok=True, mode=0o700)
            candidate_config_digest = hashlib.sha256((candidate / "semantic-config.json").read_bytes()).hexdigest() if config else None
            if artifact.exists():
                owner = artifact / ".memento-owner.json"
                if (artifact.is_symlink() or owner.is_symlink() or not owner.is_file()
                        or _json(owner) != {"owner": "memento", "kind": "artifact", "format_version": 1}):
                    raise RuntimeError("The runtime artifact contains unrecognized user files; it was left untouched.")
                existing_config = artifact / "semantic-config.json"
                existing_config_digest = hashlib.sha256(existing_config.read_bytes()).hexdigest() if existing_config.is_file() else None
                # Reuse only the same validated executable AND config bytes.
                if (hashlib.sha256((artifact / "memento").read_bytes()).hexdigest() != digest
                        or existing_config_digest != candidate_config_digest):
                    variant = hashlib.sha256((digest + str(candidate_config_digest)).encode()).hexdigest()[:12]
                    identity += "-" + variant
                    artifact = artifact.parent / identity
                    if artifact.exists():
                        raise RuntimeError("A conflicting runtime variant exists; it was left untouched.")
                else:
                    shutil.rmtree(candidate)
            if candidate.exists():
                _write(candidate / ".memento-owner.json", {"owner": "memento", "kind": "artifact", "format_version": 1})
                candidate.rename(artifact)
            active = {"directory": identity, "build_identity": version["build_identity"], "binary_sha256": digest,
                      "version": version, "embedding_selection": selected, "model_key": model_key,
                      "requested_source_digest": desired, "origin": origin,
                      "model_digest": desired_model_digest,
                      "semantic_config_sha256": candidate_config_digest,
                      "os": platform.system(), "architecture": platform.machine(), "python_abi": sys.implementation.cache_tag}
            _install_bridge(home)
            state.update(status="ready", stage=None, failure=None, active=active,
                         previous=previous_active if previous_active and previous_active.get("directory") != identity else state.get("previous"))
            _write(home / "state.json", state)
            try:
                _prune(home, state)
            except (RuntimeError, OSError) as cleanup_error:
                state["cleanup_failure"] = str(cleanup_error)
                try:
                    _write(home / "state.json", state)
                except (RuntimeError, OSError) as state_error:
                    print(f"Cannot persist deferred cleanup diagnostic: {state_error}", file=sys.stderr)
                print(f"Runtime activated; deferred cleanup failed: {cleanup_error}", file=sys.stderr)
            active_config = artifact / "semantic-config.json"
            return Runtime(artifact / "memento", active_config if active_config.is_file() else None, state)
        except (RuntimeError, OSError, shutil.Error) as error:
            state.update(status="failed", stage=state.get("stage"), failure=str(error),
                         active=previous_active, previous=previous_previous)
            _write(home / "state.json", state)
            shutil.rmtree(candidate, ignore_errors=True)
            _prune(home, state)
            if isinstance(error, RuntimeError):
                raise
            raise RuntimeError(str(error)) from error
