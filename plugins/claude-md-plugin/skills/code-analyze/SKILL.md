---
name: code-analyze
description: (internal) 소스 코드를 분석하여 Exports, Dependencies, Behavior 추출
allowed-tools: [Read, Glob, Grep]
---

# Code Analyze Skill

## 목적

소스 코드를 분석하여 구조화된 정보를 추출합니다:
- Exports: public 함수, 클래스, 타입
- Dependencies: 외부/내부 의존성
- Behavior: 주요 동작 패턴

순수 코드 분석만 수행하며, CLAUDE.md 생성은 하지 않습니다.

## 입력

```
target_path: 분석 대상 디렉토리 경로
boundary_file: boundary-resolve 결과 파일 경로
output_name: 출력 파일명 (디렉토리명 기반)
```

## 출력

`.claude/extract-results/{output_name}-analysis.json` 파일 생성

```json
{
  "path": "src/auth",
  "exports": {
    "functions": [
      {
        "name": "validateToken",
        "signature": "validateToken(token: string): Promise<Claims>",
        "description": "JWT 토큰 검증"
      }
    ],
    "types": [
      {
        "name": "Claims",
        "definition": "Claims { userId: string, role: Role, exp: number }",
        "description": "토큰 클레임"
      }
    ],
    "classes": []
  },
  "dependencies": {
    "external": ["jsonwebtoken@9.0.0"],
    "internal": ["./types", "../utils/crypto"]
  },
  "behaviors": [
    {
      "input": "유효한 JWT 토큰",
      "output": "Claims 객체 반환",
      "category": "success"
    },
    {
      "input": "만료된 토큰",
      "output": "TokenExpiredError",
      "category": "error"
    }
  ],
  "analyzed_files": ["index.ts", "middleware.ts", "types.ts"]
}
```

## 워크플로우

### 1. Boundary 파일 읽기

```python
boundary = read_json(boundary_file)
files_to_analyze = boundary["direct_files"]
```

### 2. 각 파일 분석

```python
for file in files_to_analyze:
    # Symbol overview 먼저 확인 (전체 파일 읽기 최소화)
    symbols = get_symbols_overview(file.path)

    # Public exports 추출
    for symbol in symbols:
        if is_public(symbol):
            exports.append(extract_signature(symbol))

    # Import/require 문 분석
    deps = extract_dependencies(file.path)

    # 주요 동작 패턴 파악
    behaviors = infer_behaviors(symbols)
```

### 3. 분석 전략

**효율적인 분석**:
- 전체 파일을 읽기 전에 symbol overview 확인
- public interface만 상세 분석
- 필요한 함수 본문만 선택적 읽기

**언어별 패턴**:
| 언어 | Export 키워드 | Import 패턴 |
|------|-------------|------------|
| TypeScript | `export`, `export default` | `import ... from` |
| Python | `__all__`, 최상위 정의 | `import`, `from ... import` |
| Go | 대문자 시작 | `import` |
| Rust | `pub` | `use`, `mod` |
| Java | `public` | `import` |

### 4. 결과 저장

```bash
# JSON 형태로 분석 결과 저장
write_json(".claude/extract-results/{output_name}-analysis.json", analysis_result)
```

## 결과 반환

```
---code-analyze-result---
output_file: .claude/extract-results/{output_name}-analysis.json
status: success
exports_count: {함수 + 타입 + 클래스 수}
dependencies_count: {외부 + 내부 의존성 수}
behaviors_count: {동작 패턴 수}
files_analyzed: {분석된 파일 수}
---end-code-analyze-result---
```

## 분석 가이드라인

### Exports 추출

**정확한 시그니처 추출**:
```typescript
// 소스 코드
export async function validateToken(token: string): Promise<Claims> { ... }

// 추출 결과
{
  "name": "validateToken",
  "signature": "validateToken(token: string): Promise<Claims>",
  "description": "추론 또는 JSDoc에서 추출"
}
```

### Behavior 추론

코드 패턴에서 동작 추론:
```typescript
// try-catch 패턴에서 에러 케이스 추론
try {
    const claims = jwt.verify(token, secret);
    return claims;  // → 유효한 토큰 → Claims 반환
} catch (e) {
    if (e instanceof TokenExpiredError) {
        // → 만료된 토큰 → TokenExpiredError
    }
}
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 읽기 실패 | 경고 로그, 해당 파일 스킵 |
| 시그니처 추출 실패 | `signature: "unknown"` 표시 |
| 빈 디렉토리 | 빈 분석 결과 반환 |
