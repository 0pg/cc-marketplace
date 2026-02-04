---
name: git-status-analyzer
version: 1.1.0
description: |
  Identifies uncommitted CLAUDE.md/IMPLEMENTS.md files for incremental compile filtering.
  Invoked by compile skill to detect staged, unstaged, or untracked spec files that need compilation.
allowed-tools: [Bash, Write]
---

# Git Status Analyzer Skill

## 목적

Uncommitted 변경이 있는 CLAUDE.md/IMPLEMENTS.md 파일을 찾습니다.
증분 compile의 첫 번째 필터링 단계입니다.

## 입력

```
target_path: 분석 대상 경로 (기본값: .)
output_name: 출력 파일명 (기본값: git-status)
```

## 출력

`.claude/incremental/{output_name}-uncommitted.json` 파일 생성

```json
{
  "uncommitted": [
    {"path": "src/auth/CLAUDE.md", "status": "M"},
    {"path": "src/auth/IMPLEMENTS.md", "status": "M"},
    {"path": "src/utils/CLAUDE.md", "status": "A"}
  ],
  "directories": ["src/auth", "src/utils"],
  "count": 3
}
```

## 상태 코드

| 코드 | 의미 |
|------|------|
| `M` | Modified (수정됨) |
| `A` | Added (새로 추가됨) |
| `??` | Untracked (추적되지 않음) |

## 워크플로우

### Step 1: 결과 디렉토리 생성

```bash
mkdir -p .claude/incremental
```

### Step 2: Git Status 실행

```bash
# Uncommitted 변경 (staged + unstaged + untracked)
git status --porcelain | grep -E "(CLAUDE|IMPLEMENTS)\.md$"
```

### Step 3: 결과 파싱 및 JSON 생성

출력 형식:
```
M  src/auth/CLAUDE.md
 M src/auth/IMPLEMENTS.md
?? src/utils/CLAUDE.md
```

파싱하여 JSON으로 변환:
- 첫 번째 문자: staged 상태
- 두 번째 문자: unstaged 상태
- 경로에서 디렉토리 추출 (CLAUDE.md/IMPLEMENTS.md 제거)

### Step 4: JSON 파일 저장

```bash
# JSON 결과를 파일로 저장
Write → .claude/incremental/{output_name}-uncommitted.json
```

## 결과 반환

```
---git-status-analyzer-result---
output_file: .claude/incremental/{output_name}-uncommitted.json
status: success
uncommitted_count: {uncommitted 파일 수}
directories: [{고유 디렉토리 목록}]
---end-git-status-analyzer-result---
```

## 예시

### 입력

```
target_path: src
output_name: incremental-check
```

### Git Status 출력

```
M  src/auth/CLAUDE.md
 M src/auth/IMPLEMENTS.md
?? src/utils/CLAUDE.md
```

### 생성된 JSON

```json
{
  "uncommitted": [
    {"path": "src/auth/CLAUDE.md", "status": "M"},
    {"path": "src/auth/IMPLEMENTS.md", "status": "M"},
    {"path": "src/utils/CLAUDE.md", "status": "??"}
  ],
  "directories": ["src/auth", "src/utils"],
  "count": 3
}
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| Git 저장소 아님 | 에러 메시지 반환 |
| 변경 없음 | 빈 배열 반환 (`uncommitted: [], directories: [], count: 0`) |
| 경로 접근 불가 | 에러 메시지 반환 |

## 참고

- 이 스킬은 `commit-comparator`와 함께 사용되어 전체 compile 대상을 결정합니다.
- uncommitted 변경 = 즉시 compile 필요
- committed 변경 = `commit-comparator`로 추가 분석 필요
