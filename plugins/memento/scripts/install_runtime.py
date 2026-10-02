#!/usr/bin/env python3
"""Prepare the shared Memento runtime without touching project stores."""
import argparse
import json
from pathlib import Path
import sys

from runtime import RuntimeError as InstallError, ensure_runtime, runtime_status


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ensure", action="store_true", help="Prepare if needed and preserve the existing model selection")
    parser.add_argument("--status", action="store_true", help="Print preparation status without installing")
    parser.add_argument("--binary", type=Path, help="Use a compatible executable instead of building with Cargo")
    parser.add_argument("--embedding-model", choices=("e5-small", "minilm", "none"), default=None,
                        help="Select a model; a known new installation defaults to E5, updates preserve the existing selection")
    parser.add_argument("--semantic-config", type=Path, help="Preserve and verify an explicit custom semantic configuration")
    args = parser.parse_args(argv)
    plugin = Path(__file__).resolve().parent.parent
    try:
        if args.status:
            print(json.dumps(runtime_status(plugin), indent=2))
            return 0
        prepared = ensure_runtime(plugin, binary=args.binary, embedding_model=args.embedding_model,
                                  semantic_config=args.semantic_config)
    except (InstallError, OSError) as error:
        print(f"Preparation failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps({"status": "ready", "executable": str(prepared.executable),
                      "semantic_config": str(prepared.semantic_config) if prepared.semantic_config else None,
                      "runtime_home": runtime_status(plugin)["runtime_home"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
