# TypeScript Test Conventions

## Test Directory Structure

```
module/
├── src/
│   └── foo.ts
└── __tests__/
    ├── foo.test.ts           ← unit tests
    └── foo.acceptance.test.ts ← acceptance tests
```

Alternative (co-located):

```
module/
└── src/
    ├── foo.ts
    └── foo.test.ts
```

## Unit Tests

- Location: `__tests__/` directory alongside `src/`, or co-located with source
- Naming: `<module>.test.ts`
- Framework: Jest, Vitest

## Integration Tests

- Location: `__tests__/` or `tests/`
- Naming: `<module>.integration.test.ts`
- Framework: Jest, Vitest

## Acceptance Tests

- Location: `__tests__/`
- Naming: `<module>.acceptance.test.ts`
- Framework: Jest, Vitest (with describe/it blocks as Given-When-Then)

## File Naming

| Type | Pattern | Example |
|------|---------|---------|
| Unit test | `__tests__/<name>.test.ts` | `__tests__/parser.test.ts` |
| Integration test | `__tests__/<name>.integration.test.ts` | `__tests__/db.integration.test.ts` |
| Acceptance test | `__tests__/<name>.acceptance.test.ts` | `__tests__/auth.acceptance.test.ts` |

## Import Paths

- Relative to source: `import { foo } from '../src/foo';`
- With path aliases: `import { foo } from '@/foo';`
