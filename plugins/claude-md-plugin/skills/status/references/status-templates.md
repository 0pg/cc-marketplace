# Status Report Templates

## Full Report Template

```
=== Project Status: {project_root} ===

Modules:           {N} total

Schema Health:     {pass}/{total} valid ({percentage}%)
DEVELOPERS.md:     {paired}/{total} paired ({percentage}%)
Drift:             {drift_pending} pending dev, {spec_newer} spec-newer
Conventions:       complete

Module Details:
  src/auth          schema:pass  dev-md:yes  drift:up-to-date
  src/api           schema:pass  dev-md:no   drift:spec-newer
  src/utils         schema:FAIL  dev-md:yes  drift:up-to-date

===
```

## Non-Git Repository Template

```
=== Project Status: {project_root} ===

Modules:           {N} total

Schema Health:     {pass}/{total} valid ({percentage}%)
DEVELOPERS.md:     {paired}/{total} paired ({percentage}%)
Drift:             N/A (not a git repository)
Conventions:       {status}

Module Details:
  {path}  schema:{status}  dev-md:{status}  drift:N/A

===
```

## Drift Status Values

| Value | Meaning |
|-------|---------|
| `up-to-date` | No spec changes since last /dev commit |
| `spec-newer` | CLAUDE.md or DEVELOPERS.md changed, /dev not yet run |
| `dev-pending` | diff-compile-targets reports this module needs /dev |
| `N/A` | Not a git repository or git history unavailable |
