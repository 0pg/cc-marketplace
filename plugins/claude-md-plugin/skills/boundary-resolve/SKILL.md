---
name: boundary-resolve
version: 1.0.0
description: (internal) 단일 디렉토리의 바운더리(직접 파일, 하위 디렉토리) 결정
allowed-tools: [Bash, Read]
---

# Boundary Resolve Skill

## 목적

단일 디렉토리의 바운더리를 분석합니다:
- 직접 소스 파일 목록
- 하위 디렉토리 목록

Rust CLI `claude-md-core resolve-boundary`를 래핑합니다.

## 입력

```
target_path: 분석 대상 디렉토리 경로
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
  "subdir_count": 2
}
```

## 워크플로우

### 1. CLI 실행

```bash
mkdir -p .claude/extract-results
claude-md-core resolve-boundary \
  --path {target_path} \
  --output .claude/extract-results/{output_name}-boundary.json
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

## 오류 처리

| 상황 | 대응 |
|------|------|
| 경로 없음 | 에러 메시지 출력, 실패 반환 |
| 권한 없음 | 에러 메시지 출력, 실패 반환 |
| CLI 실패 | CLI 에러 메시지 전달 |
