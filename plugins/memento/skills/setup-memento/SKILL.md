---
name: setup-memento
description: Prepare or diagnose this installed Memento plugin's local runtime and model when setting it up, updating it, or resolving a runtime setup error.
---

# Set up Memento

Use this workflow during project startup, after the installed plugin changes, or when the user starts Memento's plugin setup. Runtime checking and preparation do not require a separate update request. From this installed skill's location, the plugin root is two directories above. Run the installed package's script with absolute paths:

```sh
python3 /absolute/installed-plugin/scripts/install_runtime.py --ensure
```

This idempotently checks the installed package's executable and model identity, prepares an update when needed, then validates it before activation. A current runtime is reused. Fresh installations use E5. Updates preserve the recorded selection and custom semantic configuration. If an older installation's choice is unknown, ask whether to keep model setup skipped or prepare E5/MiniLM, then pass the chosen `--embedding-model`. Do not infer a choice from a missing configuration file.

Show the actual setup stage and result. If Cargo, uv, permissions, network access or disk space prevent setup, explain the concrete missing condition and keep the previous runtime. Do not silently install OS tools or modify Codex hook trust. Observe tool permissions for downloads and writes; the onboarding invitation does not bypass them. Use `runtime-status` through `skills/memento/scripts/memento.py` to inspect setup without starting an installation.

Runtime preparation does not create or search for project databases. For an explicitly selected existing store, run the prepared CLI's `store-status --store /absolute/context.sqlite`; report its format and compatibility. A real store operation or `migrate` performs only supported transactional legacy migration. Never recreate a nonempty database or restore an old snapshot to solve a compatibility error.

For project work, complete project initialization as well as runtime preparation. Read `skills/memento/references/codex-hooks.md` from the plugin root, reuse the explicit repository/store/project/work scope if known, and configure only that scope using the actual data directory in the hook notice. Obtain only missing scope values; do not choose a task from the latest history or scan unrelated stores. Use `--initialize-store` for a new selected store, then retrieve `sources` and a bounded `brief` across sessions and preserve the current request/constraints with durable receipts before ordinary investigation. If setup cannot complete, report the concrete condition instead of silently proceeding without context.

Codex requires review and trust of the current plugin hooks before lifecycle gates run. Do not change trust yourself or claim runtime readiness means hooks are trusted, project initialization is complete, or all project stores have been migrated. Standalone/manual capture remains available when hooks are unavailable; disclose that boundary.
