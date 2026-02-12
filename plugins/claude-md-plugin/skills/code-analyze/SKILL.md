---
name: code-analyze
version: 2.0.0
description: |
  (internal) This skill should be used when analyzing source code to extract Exports, Dependencies, and Behavior.
  소스 코드를 분석하여 Exports, Dependencies, Behavior 추출.
  --tree-result 옵션으로 tree-parse 결과를 전달하면 internal dependencies를 CLAUDE.md 경로로 resolve함.
user_invocable: false
allowed-tools: [Bash, Read, Write]
---

# Code Analyze Skill

## 목적

소스 코드를 분석하여 구조화된 정보를 추출:
- Exports: public 함수, 클래스, 타입
- Dependencies: 외부/내부 의존성
- Behavior: 주요 동작 패턴

순수 코드 분석만 수행하며, CLAUDE.md 생성은 하지 않음.
**CLI 네이티브 분석** (`claude-md-core analyze-code`)으로 ms 단위 고속 처리.

## 입력

```
target_path: 분석 대상 디렉토리 경로
output_name: 출력 파일명 (디렉토리명 기반)
tree_result_file: (선택) tree-parse 결과 JSON 파일 경로 — 있으면 internal deps를 CLAUDE.md 경로로 resolve
files: (선택) 분석 대상 파일명 목록 — boundary-resolve 결과의 direct_files로 필터링 시 사용
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
    "types": [...],
    "classes": [...]
  },
  "dependencies": {
    "external": ["jsonwebtoken"],
    "internal_raw": ["./types", "../utils/crypto"],
    "internal": [
      {
        "raw_import": "./types",
        "resolved_dir": "src/auth/types",
        "claude_md_path": "src/auth/types/CLAUDE.md",
        "resolution": "Exact"
      }
    ]
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

**Note:** `internal` 필드는 `--tree-result`가 주어졌을 때만 채워짐. 없으면 `internal_raw`만 존재.

## 워크플로우

### Step 1: CLI 빌드 확인 및 코드 분석 실행

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi

mkdir -p .claude/extract-results

# tree-result가 있으면 --tree-result 전달 (internal deps resolved)
$CLI_PATH analyze-code \
  --path {target_path} \
  --tree-result {tree_result_file} \
  --output .claude/extract-results/{output_name}-analysis.json
```

`--tree-result`가 없으면 생략 (internal_raw만 출력):
```bash
$CLI_PATH analyze-code --path {target_path} --output .claude/extract-results/{output_name}-analysis.json
```

boundary-resolve 결과의 `direct_files`로 필터링 시 `--files` 옵션 사용:
```bash
$CLI_PATH analyze-code \
  --path {target_path} \
  --files {comma_separated_filenames} \
  --tree-result {tree_result_file} \
  --output .claude/extract-results/{output_name}-analysis.json
```

### Step 2: 결과 확인

```
Read → .claude/extract-results/{output_name}-analysis.json
```

JSON 결과에서 필요한 정보를 확인.

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

## DO / DON'T

**DO:**
- CLI 빌드 상태 확인 후 실행
- 구조화된 결과 블록 (`---code-analyze-result---`) 반환
- `--tree-result` 옵션으로 dependency resolution 활용
- `--files` 옵션으로 분석 대상 필터링

**DON'T:**
- 빌드 실패 시 진행하지 않음
- 결과 JSON을 직접 수정하지 않음
- CLAUDE.md 생성하지 않음 (순수 분석만)

## 참조 자료

- `examples/analysis-output.json`: code-analyze 출력 JSON 예시

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 실행 실패 | 에러 메시지 반환, status: error |
| 빈 디렉토리 | 빈 분석 결과 반환 (exports/deps/behaviors 모두 빈 배열) |
| 지원하지 않는 확장자 | 해당 파일 스킵 |

## 테스트

테스트 fixtures와 expected 결과는 다음 위치에 있음:

```
${CLAUDE_PLUGIN_ROOT}/core/tests/fixtures/
├── typescript/     # TypeScript 테스트 코드
├── python/         # Python 테스트 코드
├── go/             # Go 테스트 코드
├── rust/           # Rust 테스트 코드
├── java/           # Java 테스트 코드
├── kotlin/         # Kotlin 테스트 코드
└── expected/       # 예상 분석 결과 JSON
```

Gherkin 테스트: `${CLAUDE_PLUGIN_ROOT}/core/tests/features/code_analyze.feature`

## Language-Specific Patterns

| Language | Reference File |
|----------|---------------|
| TypeScript | `references/typescript-patterns.md` |
| Python | `references/python-patterns.md` |
| Go | `references/go-patterns.md` |
| Rust | `references/rust-patterns.md` |
| Java | `references/java-patterns.md` |
| Kotlin | `references/kotlin-patterns.md` |
