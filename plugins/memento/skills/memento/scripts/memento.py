#!/usr/bin/env python3
"""Run an already-built memento CLI without implicit installation."""
import os
from pathlib import Path
import shutil
import subprocess
import sys



def plugin_main(plugin, arguments):
    """Use the plugin lifecycle while standalone skill installs remain unchanged."""
    import json
    sys.path.insert(0, str(plugin / "scripts"))
    from runtime import RuntimeError, ensure_runtime, git_bridge, ready_runtime, runtime_status
    try:
        if arguments[:1] == ["runtime-status"]:
            print(json.dumps(runtime_status(plugin), indent=2))
            return 0
        passive = not arguments or arguments[:1] in (["help"], ["version"], ["--version"], ["store-status"], ["session-status"], ["semantic-config-check"], ["--help"], ["-h"])
        explicit = None
        if "--semantic-config" in arguments:
            index = arguments.index("--semantic-config") + 1
            if index >= len(arguments):
                raise RuntimeError("--semantic-config requires a path.")
            explicit = Path(arguments[index])
        prepared = ready_runtime(plugin) if passive else ensure_runtime(plugin, semantic_config=explicit)
        if arguments[:1] == ["query"] and explicit is None and prepared.semantic_config is not None:
            arguments.extend(["--semantic-config", str(prepared.semantic_config)])
        environment = os.environ.copy()
        if git_bridge().is_file():
            environment["MEMENTO_GIT_EXECUTABLE"] = str(git_bridge())
        return subprocess.run([str(prepared.executable), *arguments], env=environment, check=False).returncode
    except (RuntimeError, OSError) as error:
        print(f"Cannot run Memento: {error}", file=sys.stderr)
        return 2


def main() -> int:
    override = os.environ.get("MEMENTO_BIN")
    skill_directory = Path(__file__).resolve().parent.parent
    checkout = skill_directory.parent.parent
    if (checkout / "scripts/runtime.py").is_file():
        return plugin_main(checkout, sys.argv[1:])
    candidates = [override] if override is not None else [
        str(skill_directory / "bin/memento"),
        shutil.which("memento"),
        str(checkout / "target/release/memento"),
        str(checkout / "target/debug/memento"),
    ]
    executable = next(
        (p for p in candidates if p and Path(p).is_file() and os.access(p, os.X_OK)),
        None,
    )
    if executable is None:
        print(
            "Install from the implementation checkout with "
            "python3 scripts/install_memento.py, or set MEMENTO_BIN "
            "to an executable.",
            file=sys.stderr,
        )
        return 2
    arguments = sys.argv[1:]
    config = skill_directory / "semantic-config.json"
    if arguments[:1] == ["query"] and "--semantic-config" not in arguments and config.is_file():
        arguments.extend(["--semantic-config", str(config)])
    try:
        return subprocess.run([executable, *arguments], check=False).returncode
    except OSError as error:
        print(f"Cannot run memento: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
