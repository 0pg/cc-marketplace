---
name: claude-md-parse
version: 1.0.0
description: |
  (internal) This skill should be used when parsing CLAUDE.md files into structured JSON spec.
  CLAUDE.md 파일을 파싱하여 구조화된 JSON 스펙으로 변환
user_invocable: false
allowed-tools: [Bash, Read]
---

# CLAUDE.md Parse Skill

## 목적

CLAUDE.md 파일을 파싱하여 코드 생성에 필요한 구조화된 JSON 스펙(ClaudeMdSpec)으로 변환.
Rust CLI `claude-md-core parse-claude-md`를 래핑.

## 입력

```
claude_md_path: CLAUDE.md 파일 경로
output_path: (선택) 출력 JSON 파일 경로 (기본: stdout)
```

## 출력

ClaudeMdSpec JSON:

```json
{
  "name": "auth-module",
  "purpose": "User authentication and token validation",
  "exports": {
    "functions": [
      {
        "name": "validateToken",
        "signature": "validateToken(token: string): Promise<Claims>",
        "is_async": true
      }
    ],
    "types": [
      {
        "name": "Claims",
        "definition": "Claims { userId: string, role: Role, exp: number }",
        "kind": "interface"
      }
    ],
    "classes": [],
    "enums": [],
    "variables": []
  },
  "dependencies": {
    "external": ["jsonwebtoken@9.0.0"],
    "internal": ["./types"]
  },
  "behaviors": [
    {
      "input": "valid JWT token",
      "output": "Claims object",
      "category": "success"
    },
    {
      "input": "expired token",
      "output": "TokenExpiredError",
      "category": "error"
    }
  ],
  "contracts": [
    {
      "function_name": "validateToken",
      "preconditions": ["token must be non-empty string"],
      "postconditions": ["returns Claims with valid userId"],
      "throws": ["InvalidTokenError"],
      "invariants": []
    }
  ],
  "protocol": {
    "states": ["Idle", "Loading", "Loaded", "Error"],
    "transitions": [
      { "from": "Idle", "trigger": "load()", "to": "Loading" }
    ],
    "lifecycle": [
      { "order": 1, "method": "init", "description": "Initialize resources" }
    ]
  },
  "structure": {
    "subdirs": [
      { "name": "jwt", "description": "JWT token handling" }
    ],
    "files": [
      { "name": "types.ts", "description": "Type definitions" }
    ]
  },
  "warnings": []
}
```

## 워크플로우

### 1. CLI 빌드 확인

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi
```

### 2. CLAUDE.md 파싱 실행

```bash
# output_path가 지정된 경우
$CLI_PATH parse-claude-md --file {claude_md_path} --output {output_path}

# stdout 출력
$CLI_PATH parse-claude-md --file {claude_md_path}
```

### 3. 결과 검증

- `warnings` 배열이 비어있는지 확인
- 필수 필드(purpose, exports, behaviors)가 존재하는지 확인

## 결과 반환

```
---claude-md-parse-result---
output_file: {output_path 또는 "stdout"}
status: success | warning | error
warnings: [에러 목록 (있는 경우)]
---end-claude-md-parse-result---
```

## DO / DON'T

**DO:**
- CLI 빌드 상태 확인 후 실행
- 구조화된 결과 블록 (`---claude-md-parse-result---`) 반환
- warnings 배열 확인하여 파싱 이슈 전달

**DON'T:**
- 빌드 실패 시 진행하지 않음
- CLAUDE.md 파일을 직접 수정하지 않음 (파싱만)
- 결과 JSON을 후처리하지 않음

## 참조 자료

- `examples/parse-output.json`: claude-md-parse 출력 JSON 예시

## 오류 처리

| 상황 | 대응 |
|------|------|
| 파일 없음 | ParseError 반환 |
| 필수 섹션 누락 | warnings에 기록 |
| 잘못된 형식 | warnings에 기록 |
