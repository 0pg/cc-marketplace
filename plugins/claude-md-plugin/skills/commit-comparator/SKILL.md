---
name: commit-comparator
version: 1.1.0
description: |
  Compares spec vs source commit timestamps to find outdated directories requiring compilation.
  Invoked by compile skill with uncommitted_dirs from git-status-analyzer to identify stale or new modules.
allowed-tools: [Bash, Read, Write, Glob]
---

# Commit Comparator Skill

## 목적

CLAUDE.md/IMPLEMENTS.md(스펙)와 소스 파일의 마지막 커밋 시점을 비교합니다.
스펙이 소스보다 최신인 경우 = compile 필요.

## 입력

```
target_path: 분석 대상 경로 (기본값: .)
output_name: 출력 파일명 (기본값: commit-compare)
exclude_uncommitted: uncommitted 디렉토리 제외 여부 (기본값: true)
uncommitted_dirs: 제외할 디렉토리 목록 (git-status-analyzer 결과)
```

## 출력

`.claude/incremental/{output_name}-outdated.json` 파일 생성

```json
{
  "outdated": [
    {
      "path": "src/utils",
      "spec_commit": "abc123",
      "spec_time": 1706000000,
      "source_commit": "def456",
      "source_time": 1705900000,
      "reason": "spec_newer"
    }
  ],
  "up_to_date": [
    {
      "path": "src/api",
      "spec_time": 1705800000,
      "source_time": 1705900000,
      "reason": "source_newer"
    }
  ],
  "no_source": [
    {
      "path": "src/new-module",
      "spec_time": 1706000000,
      "reason": "no_source_files"
    }
  ],
  "summary": {
    "total_checked": 3,
    "outdated_count": 1,
    "up_to_date_count": 1,
    "no_source_count": 1
  }
}
```

## 워크플로우

### Step 1: CLAUDE.md 파일 검색

```bash
# target_path 하위의 모든 CLAUDE.md 찾기 (root CLAUDE.md 제외)
find {target_path} -name "CLAUDE.md" -type f | grep -v "^./CLAUDE.md$"
```

### Step 2: 각 디렉토리 분석

각 CLAUDE.md가 있는 디렉토리에 대해:

```bash
# 스펙 파일의 마지막 커밋 타임스탬프
spec_claude_time=$(git log -1 --format=%ct -- {dir}/CLAUDE.md 2>/dev/null || echo "0")
spec_impl_time=$(git log -1 --format=%ct -- {dir}/IMPLEMENTS.md 2>/dev/null || echo "0")
spec_time=$((spec_claude_time > spec_impl_time ? spec_claude_time : spec_impl_time))

# 소스 파일들의 마지막 커밋 타임스탬프 (테스트 파일 제외)
# 확장자: .ts, .tsx, .js, .jsx, .py, .go, .rs, .java, .kt
source_time=$(git log -1 --format=%ct -- "{dir}/*.ts" "{dir}/*.tsx" "{dir}/*.js" "{dir}/*.jsx" "{dir}/*.py" "{dir}/*.go" "{dir}/*.rs" "{dir}/*.java" "{dir}/*.kt" 2>/dev/null | head -1 || echo "0")
```

### Step 3: 비교 및 분류

```
if spec_time == 0:
    → 에러 (CLAUDE.md가 커밋된 적 없음, uncommitted로 처리됨)
elif source_time == 0:
    → no_source (아직 소스 파일 없음, compile 필요)
elif spec_time > source_time:
    → outdated (스펙이 최신, compile 필요)
else:
    → up_to_date (소스가 최신 또는 동일, compile 불필요)
```

### Step 4: JSON 결과 생성 및 저장

```
Write → .claude/incremental/{output_name}-outdated.json
```

## 결과 반환

```
---commit-comparator-result---
output_file: .claude/incremental/{output_name}-outdated.json
status: approve
outdated_count: {outdated 디렉토리 수}
no_source_count: {소스 없는 디렉토리 수}
up_to_date_count: {최신 상태 디렉토리 수}
compile_needed: [{outdated + no_source 디렉토리 목록}]
---end-commit-comparator-result---
```

## 예시

### 시나리오

```
src/
├── auth/
│   ├── CLAUDE.md      (commit: 1월 15일)
│   ├── IMPLEMENTS.md  (commit: 1월 16일)  ← 스펙 기준
│   └── index.ts       (commit: 1월 10일)  ← 소스 기준
├── utils/
│   ├── CLAUDE.md      (commit: 1월 5일)   ← 스펙 기준
│   └── index.ts       (commit: 1월 10일)  ← 소스 기준
└── new-module/
    └── CLAUDE.md      (commit: 1월 17일)  ← 스펙만 존재
```

### 결과

```json
{
  "outdated": [
    {
      "path": "src/auth",
      "spec_time": 1705363200,
      "source_time": 1704844800,
      "reason": "spec_newer"
    }
  ],
  "up_to_date": [
    {
      "path": "src/utils",
      "spec_time": 1704412800,
      "source_time": 1704844800,
      "reason": "source_newer"
    }
  ],
  "no_source": [
    {
      "path": "src/new-module",
      "spec_time": 1705449600,
      "reason": "no_source_files"
    }
  ]
}
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| Git 저장소 아님 | 에러 메시지 반환 |
| CLAUDE.md 없음 | 빈 결과 반환 |
| CLAUDE.md가 커밋된 적 없음 | uncommitted로 처리 (이 스킬에서 제외) |
| 권한 오류 | 해당 디렉토리 스킵, 경고 로그 |

## 참고

- `git-status-analyzer`와 함께 사용됩니다.
- uncommitted 파일은 `git-status-analyzer`에서 처리되므로 이 스킬에서는 제외합니다.
- 소스 파일 확장자는 프로젝트에 따라 확장 가능합니다.
