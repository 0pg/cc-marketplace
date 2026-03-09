# Bugfix Result Examples

## 성공 (Success)

```
/bugfix --error "TypeError: validateToken is not a function" --path src/auth

/bugfix 결과
=========

Root Cause: L1 - SPEC_EXPORT_MISMATCH
요약: CLAUDE.md exports validateToken as standalone but code defines it as class method

수정된 문서: [CLAUDE.md]
재현: REPRODUCED
Compile: PASS
검증: PASS (npx jest src/auth --no-coverage)

상세 결과: .claude/tmp/debug-src-auth.md
```

## 부분 성공 (Partial Success)

```
/bugfix --test "should refresh token on expiry" --path src/auth

/bugfix 결과
=========

Root Cause: MULTI - PLAN_ERROR_HANDLING_GAP + SPEC_BEHAVIOR_GAP
요약: Token refresh on expiry not specified in CLAUDE.md Behavior, not handled in IMPLEMENTS.md

수정된 문서: [CLAUDE.md, IMPLEMENTS.md]
재현: REPRODUCED
Compile: PASS
검증: FAIL (npx jest --testNamePattern "should refresh token on expiry")

⚠ 추가 `/bugfix`로 남은 문제를 진단하세요.

상세 결과: .claude/tmp/debug-src-auth.md
```

## 실패 (Failure)

```
/bugfix --error "RangeError: Maximum call stack size exceeded" --path src/utils

/bugfix 결과
=========

Root Cause: L2 - PLAN_ALGORITHM_ERROR
요약: IMPLEMENTS.md recursive traversal has no base case for circular references

수정된 문서: [IMPLEMENTS.md]
재현: REPRODUCED
Compile: FAIL

⚠ 수동 점검이 필요합니다. 상세 결과를 확인하세요.

상세 결과: .claude/tmp/debug-src-utils.md
```
