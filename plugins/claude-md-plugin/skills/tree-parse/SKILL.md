---
name: tree-parse
version: 1.0.0
description: (internal) 프로젝트 디렉토리 구조를 파싱하여 CLAUDE.md가 필요한 위치 식별
allowed-tools: [Bash, Read]
---

# Tree Parse Skill

## 목적

프로젝트 디렉토리 구조를 분석하여 CLAUDE.md가 필요한 위치를 식별합니다.
Rust CLI `claude-md-core parse-tree`를 래핑합니다.

## 입력

```
root_path: 프로젝트 루트 경로 (기본: 현재 디렉토리)
```

## 출력

`.claude/extract-tree.json` 파일 생성

```json
{
  "root": "/path/to/project",
  "needs_claude_md": [
    {
      "path": "src",
      "source_file_count": 2,
      "subdir_count": 3,
      "reason": "2 source files and 3 subdirectories",
      "depth": 1
    },
    {
      "path": "src/auth",
      "source_file_count": 4,
      "subdir_count": 1,
      "reason": "4 source files",
      "depth": 2
    }
  ],
  "excluded": ["node_modules", "target", "dist"]
}
```

## 워크플로우

### 1. CLI 빌드 확인

```bash
CLI_PATH="plugins/claude-md-plugin/core/target/release/claude-md-core"
if [ ! -f "$CLI_PATH" ]; then
    echo "Building claude-md-core..."
    cd plugins/claude-md-plugin/core && cargo build --release
fi
```

### 2. 트리 파싱 실행

```bash
mkdir -p .claude
claude-md-core parse-tree --root {root_path} --output .claude/extract-tree.json
```

### 3. 결과 확인

```bash
# 파일 존재 및 유효성 확인
if [ -f ".claude/extract-tree.json" ]; then
    echo "Tree parsing completed successfully"
    echo "Output: .claude/extract-tree.json"
else
    echo "Error: Tree parsing failed"
    exit 1
fi
```

## 결과 반환

```
---tree-parse-result---
output_file: .claude/extract-tree.json
status: success
directory_count: {needs_claude_md 배열 길이}
---end-tree-parse-result---
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | 에러 메시지 출력, 실패 반환 |
| 루트 경로 없음 | 에러 메시지 출력, 실패 반환 |
| 파싱 실패 | CLI 에러 메시지 전달 |
