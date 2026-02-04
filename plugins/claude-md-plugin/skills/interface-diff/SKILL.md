---
name: interface-diff
version: 1.1.0
description: |
  Detects interface changes by comparing CLAUDE.md Exports (spec) with source code exports (implementation).
  Invoked by compile skill after code generation to identify breaking changes and recommend dependent module recompilation.
allowed-tools: [Read, Write, Grep, Glob]
---

# Interface Diff Skill

## 목적

**스펙(CLAUDE.md)과 구현(소스 코드)의 일치 여부를 검증**하고, **인터페이스 변경(breaking change)**을 감지합니다.

핵심 역할:
1. **스펙 vs 구현 일치 검증**: CLAUDE.md Exports가 실제 소스 코드에 올바르게 구현되었는지 확인
2. **Breaking Change 감지**: compile 후 인터페이스 변경이 의존 모듈에 영향을 주는지 판정

## 입력

```
target_path: 분석 대상 디렉토리 경로
claude_md_path: CLAUDE.md 파일 경로
source_files: 소스 파일 경로 목록 (optional, 자동 감지)
output_name: 출력 파일명
```

## 출력

`.claude/incremental/{output_name}-interface-diff.json` 파일 생성

```json
{
  "path": "src/auth",
  "before": {
    "functions": ["validateToken(token: string): boolean"],
    "types": ["Claims"],
    "classes": ["AuthMiddleware"]
  },
  "after": {
    "functions": ["validateToken(token: string, options?: Options): Claims", "refreshToken(token: string): Claims"],
    "types": ["Claims", "Options"],
    "classes": ["AuthMiddleware"]
  },
  "changes": {
    "added": [
      {"kind": "function", "name": "refreshToken", "signature": "refreshToken(token: string): Claims"}
    ],
    "removed": [],
    "modified": [
      {
        "kind": "function",
        "name": "validateToken",
        "before": "validateToken(token: string): boolean",
        "after": "validateToken(token: string, options?: Options): Claims",
        "change_type": "signature_changed"
      }
    ]
  },
  "breaking_change": true,
  "breaking_reasons": [
    "Function 'validateToken' return type changed from 'boolean' to 'Claims'",
    "Function 'validateToken' added required parameter"
  ]
}
```

## Breaking Change 판정 기준

| 변경 유형 | Breaking? | 이유 |
|-----------|-----------|------|
| 함수/메서드 제거 | YES | 호출자가 없어진 함수 호출 시도 |
| 타입/클래스 제거 | YES | 참조하는 코드 컴파일 실패 |
| 필수 파라미터 추가 | YES | 기존 호출이 인자 부족으로 실패 |
| 반환 타입 변경 | YES | 반환값 처리 코드 호환성 깨짐 |
| 파라미터 타입 변경 | YES | 기존 인자가 타입 검사 실패 |
| 함수/타입 추가 | NO | 기존 코드에 영향 없음 |
| 선택적 파라미터 추가 | NO | 기존 호출 그대로 동작 |

## Before/After 상태의 의미

| 상태 | 소스 | 의미 |
|------|------|------|
| **Before** | CLAUDE.md Exports | 스펙에 정의된 의도된 인터페이스 |
| **After** | 소스 코드 exports | compile 후 실제 구현된 인터페이스 |

```
Before (CLAUDE.md)          After (Source Code)
──────────────────          ───────────────────
스펙 = 의도                  구현 = 결과
│                           │
├─ 어떤 함수가 있어야 하는가    ├─ 실제로 어떤 함수가 있는가
├─ 어떤 시그니처여야 하는가    ├─ 실제 시그니처는 무엇인가
└─ 어떤 타입을 export하는가   └─ 실제 export되는 타입은 무엇인가
```

## 워크플로우

### Step 1: CLAUDE.md에서 Before 상태 추출 (스펙 = 의도)

```
Read CLAUDE.md
Parse Exports 섹션:
- Functions: 함수명 + 시그니처 (의도된 API)
- Types: 타입명 (의도된 타입 정의)
- Classes: 클래스명 + public 메서드 (의도된 클래스 인터페이스)

Before = "CLAUDE.md가 정의한 인터페이스"
```

### Step 2: 소스 코드에서 After 상태 추출 (구현 = 결과)

소스 파일들을 분석하여 **실제로 구현된** exports 추출:

```
# TypeScript/JavaScript
export function ...    ← 실제 export된 함수
export const ...       ← 실제 export된 상수
export class ...       ← 실제 export된 클래스
export type ...        ← 실제 export된 타입
export interface ...   ← 실제 export된 인터페이스

# Python
def ... (모듈 레벨)    ← 실제 정의된 함수
class ...              ← 실제 정의된 클래스
__all__ = [...]        ← 명시적 export 목록

After = "소스 코드가 실제로 export하는 인터페이스"
```

### Step 3: Before/After 비교

```
added = after - before
removed = before - after
modified = intersect(before, after) where signature changed
```

### Step 4: Breaking Change 판정

```
breaking_change = (
  len(removed) > 0 or
  any(mod.is_breaking for mod in modified)
)
```

### Step 5: JSON 결과 생성 및 저장

```
Write → .claude/incremental/{output_name}-interface-diff.json
```

## 결과 반환

```
---interface-diff-result---
output_file: .claude/incremental/{output_name}-interface-diff.json
status: success
added_count: {추가된 export 수}
removed_count: {제거된 export 수}
modified_count: {변경된 export 수}
breaking_change: {true|false}
breaking_reasons: [{breaking 이유 목록}]
---end-interface-diff-result---
```

## 예시

### 시나리오: validateToken 시그니처 변경

**Before (CLAUDE.md Exports):**
```markdown
## Exports

### Functions
- `validateToken(token: string): boolean` - 토큰 검증
```

**After (실제 소스 코드):**
```typescript
export function validateToken(token: string, options?: ValidateOptions): Claims {
  // ...
}

export function refreshToken(token: string): Claims {
  // ...
}
```

**결과:**
```json
{
  "changes": {
    "added": [{"kind": "function", "name": "refreshToken", ...}],
    "removed": [],
    "modified": [{
      "kind": "function",
      "name": "validateToken",
      "before": "validateToken(token: string): boolean",
      "after": "validateToken(token: string, options?: ValidateOptions): Claims",
      "change_type": "signature_changed"
    }]
  },
  "breaking_change": true,
  "breaking_reasons": [
    "Function 'validateToken' return type changed from 'boolean' to 'Claims'"
  ]
}
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLAUDE.md 없음 | 에러 반환 |
| CLAUDE.md에 Exports 없음 | before를 빈 상태로 처리 |
| 소스 파일 없음 | after를 빈 상태로 처리 (모든 exports가 removed) |
| 파싱 실패 | 해당 항목 스킵, 경고 로그 |

## 참고

- 이 스킬은 compile 완료 후 호출됩니다.
- `dependency-tracker`와 연계하여 영향받는 모듈을 파악합니다.
- breaking change가 있으면 의존 모듈 재컴파일을 권장합니다.
