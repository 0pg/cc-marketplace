#!/usr/bin/env python3
"""Resolve a verified shared runtime, preparing it on the first real command."""
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    plugin = Path(__file__).resolve().parents[3]
    sys.path.insert(0, str(plugin / "scripts"))
    from runtime import RuntimeError, ensure_runtime, git_bridge, ready_runtime, runtime_status
    arguments = sys.argv[1:]
    try:
        if arguments[:1] == ["runtime-status"]:
            print(json.dumps(runtime_status(plugin), indent=2))
            return 0
        passive = not arguments or arguments[:1] in (["help"], ["version"], ["--version"], ["store-status"], ["semantic-config-check"], ["--help"], ["-h"])
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


if __name__ == "__main__":
    raise SystemExit(main())
