# Impact Analysis Templates

## Full Impact Report

```
=== Impact Analysis: {path} ===

Changed:
  + REQ-4: Auto-refresh token on access     [ADD]
  ~ REQ-2: Maximum login devices             [MODIFY]
  - CONST-1: Token validation contract       [DELETE]

Direct dependents:
  src/api
    CONST-2, CONST-5 may need update
    Files: handler.ts, middleware.ts

Transitive dependents:
  src/gateway
    CONST-1 may need update
    Files: proxy.ts

Summary: 2 modules, 3 constraints, 3 source files potentially affected
===
```

## No Dependents Report

```
=== Impact Analysis: {path} ===

Changed:
  ~ REQ-1: {text}              [MODIFY]

No downstream dependents found.

===
```

## Change Type Markers

| Marker | Meaning |
|--------|---------|
| `+` | Added (new REQ/CONST) |
| `~` | Modified (changed REQ/CONST) |
| `-` | Deleted (removed REQ/CONST) |

## Constraint Advisory Label

All constraint references in dependent modules use the label "may need update" because:
- Detection is Grep-based (string matching on module paths)
- False positives are possible
- Actual impact requires human/agent judgment
