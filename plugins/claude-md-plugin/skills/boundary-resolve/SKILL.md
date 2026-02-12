---
name: boundary-resolve
version: 1.0.0
description: |
  (internal) This skill should be used when resolving the boundary of a single directory to identify direct files and subdirectories.
  단일 디렉토리의 바운더리(직접 파일, 하위 디렉토리) 결정
user_invocable: false
allowed-tools: [Bash, Read]
---

# Boundary Resolve Skill

## 목적

단일 디렉토리의 바운더리를 분석:
- 직접 소스 파일 목록
- 하위 디렉토리 목록

Rust CLI `claude-md-core resolve-boundary`를 래핑.

## 입력

```
target_path: 분석 대상 디렉토리 경로
claude_md: (optional) CLAUDE.md 파일 경로 - 제공 시 트리 구조 위반(INV-1) 검사 수행
output_name: 출력 파일명 (디렉토리명 기반)
```

## 출력

`.claude/extract-results/{output_name}-boundary.json` 파일 생성

```json
{
  "path": "src/auth",
  "direct_files": [
    {"name": "index.ts", "type": "typescript"},
    {"name": "types.ts", "type": "typescript"},
    {"name": "middleware.ts", "type": "typescript"}
  ],
  "subdirs": [
    {"name": "jwt", "has_claude_md": true},
    {"name": "session", "has_claude_md": false}
  ],
  "source_file_count": 3,
  "subdir_count": 2,
  "violations": [
    {"violation_type": "Parent", "reference": "../utils", "line_number": 15}
  ]
}
```

## 워크플로우

### 1. CLI 빌드 확인 및 실행

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd "$CORE_DIR" && cargo build --release
fi

mkdir -p .claude/extract-results

# claude_md가 제공된 경우 violation 검사 포함
if [ -n "{claude_md}" ]; then
    $CLI_PATH resolve-boundary \
      --path {target_path} \
      --claude-md {claude_md} \
      --output .claude/extract-results/{output_name}-boundary.json
else
    $CLI_PATH resolve-boundary \
      --path {target_path} \
      --output .claude/extract-results/{output_name}-boundary.json
fi
```

### 2. 결과 확인

```bash
if [ -f ".claude/extract-results/{output_name}-boundary.json" ]; then
    echo "Boundary resolution completed"
else
    echo "Error: Boundary resolution failed"
    exit 1
fi
```

## 결과 반환

```
---boundary-resolve-result---
output_file: .claude/extract-results/{output_name}-boundary.json
status: success
direct_files: {직접 파일 수}
subdirs: {하위 디렉토리 수}
---end-boundary-resolve-result---
```

## 출력 필드 설명

| 필드 | 설명 |
|------|------|
| direct_files | 해당 디렉토리에 직접 위치한 소스 파일 목록 |
| subdirs | 하위 디렉토리 목록 (CLAUDE.md 존재 여부 포함) |
| has_claude_md | 해당 하위 디렉토리에 CLAUDE.md가 이미 존재하는지 |
| violations | (Optional) claude_md 제공 시 검출된 트리 구조 위반 목록 (Parent/Sibling). claude_md 미제공 시 null |

## DO / DON'T

**DO:**
- CLI 빌드 상태 확인 후 실행
- 구조화된 결과 블록 (`---boundary-resolve-result---`) 반환
- 숨김 디렉토리 (`.`으로 시작) 제외

**DON'T:**
- 빌드 실패 시 진행하지 않음
- 하위 디렉토리 내부를 재귀 분석하지 않음 (단일 depth만)
- 결과 JSON을 직접 수정하지 않음

## 참조 자료

- `examples/boundary.json`: boundary-resolve 출력 JSON 예시
- `examples/input.json`: 입력 파라미터 예시

## 오류 처리

| 상황 | 대응 |
|------|------|
| 경로 없음 | 에러 메시지 출력, 실패 반환 |
| 권한 없음 | 에러 메시지 출력, 실패 반환 |
| CLI 실패 | CLI 에러 메시지 전달 |
