# /inspect --focus health

Project-level schema + pairing + drift + conventions snapshot.

## H.1 Scan modules

```bash
$CLI_PATH scan-claude-md --root {project_root}
```

Parse output into a module index (directories containing CLAUDE.md).

## H.2 Per-module checks

For each module:

```bash
$CLI_PATH validate-schema --file {claude_md_path} --dir {dir}
```

Check `{dir}/DEVELOPERS.md` existence (INV-3 pairing).

## H.3 Drift + conventions

```bash
$CLI_PATH diff-compile-targets --root {project_root}
$CLI_PATH validate-convention --project-root {project_root}
```

## H.4 Report

```
=== Project Health: {project_root} ===

Modules:       {N} total
Schema:        {pass}/{total} valid ({percentage}%)
DEVELOPERS.md: {paired}/{total} paired ({percentage}%)
Drift:         {summary or "N/A (not a git repository)"}
Conventions:   {complete | incomplete ({missing subsections})}

Module Details:
  {path}  schema:{pass|FAIL}  dev-md:{yes|no}  drift:{up-to-date|spec-newer|dev-pending}
  ...
===
```

`drift` values: `up-to-date` (no changes), `spec-newer` (CLAUDE.md changed
since last /dev), `dev-pending` (needs /dev run).

## Failure modes

| Situation | Response |
|-----------|----------|
| Non-git repo | Drift section = `N/A (not a git repository)`; continue |
| Individual module validation error | Report inline in Module Details; continue with next |
| scan-claude-md failure | Surface raw error; exit |
