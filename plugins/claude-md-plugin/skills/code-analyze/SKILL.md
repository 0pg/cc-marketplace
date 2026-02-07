---
name: code-analyze
version: 1.0.0
description: |
  Analyzes source code to extract Exports, Dependencies, and Behavior patterns using regex-based parsing.
  Invoked by decompiler agent to generate structured analysis JSON for CLAUDE.md generation.
allowed-tools: [Read, Glob, Grep, Write]
---

# Code Analyze Skill

## 목적

소스 코드를 분석하여 구조화된 정보를 추출합니다:
- Exports: public 함수, 클래스, 타입
- Dependencies: 외부/내부 의존성
- Behavior: 주요 동작 패턴

순수 코드 분석만 수행하며, CLAUDE.md 생성은 하지 않습니다.
**Regex 기반 분석**으로 외부 의존성 없이 동작합니다.

## 입력

```
target_path: 분석 대상 디렉토리 경로
boundary_file: boundary-resolve 결과 파일 경로
output_name: 출력 파일명 (디렉토리명 기반)
```

## 출력

`.claude/tmp/{session-id}-analysis-{target}.json` 파일 생성

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
    "types": [...],
    "classes": [...]
  },
  "dependencies": {
    "external": ["jsonwebtoken"],
    "internal": ["./types", "../utils/crypto"]
  },
  "behaviors": [
    {
      "input": "유효한 JWT 토큰",
      "output": "Claims 객체 반환",
      "category": "success"
    }
  ],
  "analyzed_files": ["index.ts", "middleware.ts", "types.ts"]
}
```

## 워크플로우

### Step 1: Boundary 파일 읽기
```
Read boundary_file → JSON 파싱
files_to_analyze = boundary.direct_files
```

### Step 2: 파일 확장자로 언어 감지

| 확장자 | 언어 | 패턴 참조 |
|--------|------|----------|
| `.ts`, `.tsx` | TypeScript | `references/typescript-patterns.md` |
| `.js`, `.jsx` | JavaScript | `references/typescript-patterns.md` |
| `.py` | Python | `references/python-patterns.md` |
| `.go` | Go | `references/go-patterns.md` |
| `.rs` | Rust | `references/rust-patterns.md` |
| `.java` | Java | `references/java-patterns.md` |
| `.kt` | Kotlin | `references/kotlin-patterns.md` |

### Step 3: 언어별 Regex 패턴으로 분석

각 언어별 상세 패턴은 `references/` 디렉토리 참조:
- Exports 추출 패턴
- Dependencies 추출 패턴
- Behavior 추론 패턴

### Step 4: JSON 결과 생성 및 저장

```
mkdir -p .claude/tmp
Write → .claude/tmp/{session-id}-analysis-{target}.json
```

## 결과 반환

```
---code-analyze-result---
output_file: .claude/tmp/{session-id}-analysis-{target}.json
status: approve
exports_count: {함수 + 타입 + 클래스 수}
dependencies_count: {외부 + 내부 의존성 수}
behaviors_count: {동작 패턴 수}
files_analyzed: {분석된 파일 수}
---end-code-analyze-result---
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 읽기 실패 | 경고 로그, 해당 파일 스킵, analyzed_files에서 제외 |
| 시그니처 추출 실패 | `signature: "unknown"` 표시 |
| 빈 디렉토리 | 빈 분석 결과 반환 (exports/deps/behaviors 모두 빈 배열) |
| 지원하지 않는 확장자 | 해당 파일 스킵 |

## 테스트

테스트 fixtures와 expected 결과는 다음 위치에 있습니다:

```
plugins/claude-md-plugin/fixtures/
├── typescript/     # TypeScript 테스트 코드
├── python/         # Python 테스트 코드
├── go/             # Go 테스트 코드
├── rust/           # Rust 테스트 코드
├── java/           # Java 테스트 코드
├── kotlin/         # Kotlin 테스트 코드
└── expected/       # 예상 분석 결과 JSON
```

Gherkin 테스트: `skills/code-analyze/tests/code_analyze.feature`
