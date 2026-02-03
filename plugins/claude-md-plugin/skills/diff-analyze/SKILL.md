---
name: diff-analyze
description: |
  (internal) Git diff 기반 변경된 CLAUDE.md 파일 감지.
  브랜치 base commit 또는 지정된 ref 기준으로 변경 분석.
allowed-tools: [Bash, Read, Write]
---

# Diff Analyze Skill

## 목적

Git diff를 기반으로 변경된 CLAUDE.md 파일을 감지합니다.
브랜치의 base commit 기준으로 변경 사항을 분석하여 incremental 처리를 지원합니다.

## 내부 전용

이 Skill은 `incremental-generate` Skill에서 내부적으로 호출됩니다.
사용자가 직접 호출하지 않습니다.

## 입력 파라미터

| 파라미터 | 기본값 | 설명 |
|----------|--------|------|
| `path` | `.` | 분석 대상 경로 |
| `base` | `auto` | 비교 기준 (`auto`=merge-base, 또는 특정 commit/branch) |
| `include-untracked` | `true` | 새로 추가된 (untracked) CLAUDE.md 포함 여부 |

## 핵심 로직

### 1. Base Commit 결정

```bash
# base="auto" 인 경우 (기본값)
# main 또는 master 브랜치와의 merge-base 찾기
BASE=$(git merge-base HEAD main 2>/dev/null || git merge-base HEAD master 2>/dev/null)

# merge-base 실패 시 (main도 master도 없는 경우)
if [ -z "$BASE" ]; then
    # 현재 브랜치의 첫 커밋 이전
    BASE=$(git rev-list --max-parents=0 HEAD)
fi

# base가 명시적으로 지정된 경우
# 예: base="abc1234" 또는 base="develop"
BASE="$specified_base"
```

### 2. 변경된 파일 감지

```bash
# Git tracked 파일 중 변경된 CLAUDE.md
git diff --name-only --diff-filter=ACMR "$BASE"...HEAD -- '**/CLAUDE.md' 2>/dev/null

# diff-filter 설명:
# A = Added (새로 추가)
# C = Copied (복사됨)
# M = Modified (수정됨)
# R = Renamed (이름 변경)
```

### 3. Untracked 파일 감지 (옵션)

```bash
# include-untracked=true 인 경우
# 아직 커밋되지 않은 새 CLAUDE.md 파일도 포함
git status --porcelain -- '**/CLAUDE.md' | grep '^??' | cut -c4-
```

### 4. 상태 판별

```bash
# 각 파일의 상태 판별
for file in $changed_files; do
    if git diff --name-only --diff-filter=A "$BASE"...HEAD -- "$file" | grep -q .; then
        status="added"
    elif git diff --name-only --diff-filter=M "$BASE"...HEAD -- "$file" | grep -q .; then
        status="modified"
    elif git diff --name-only --diff-filter=R "$BASE"...HEAD -- "$file" | grep -q .; then
        status="renamed"
    fi
done

# Untracked 파일
for file in $untracked_files; do
    status="untracked"
done
```

## 출력

결과를 `.claude/diff-analyze-result.json`에 저장합니다.

```json
{
  "base_ref": "abc1234def5678",
  "base_description": "merge-base with main",
  "head_ref": "HEAD",
  "analysis_path": ".",
  "changed_files": [
    {
      "path": "src/auth/CLAUDE.md",
      "status": "modified"
    },
    {
      "path": "src/new-feature/CLAUDE.md",
      "status": "added"
    },
    {
      "path": "src/experimental/CLAUDE.md",
      "status": "untracked"
    }
  ],
  "unchanged_count": 5,
  "total_claude_md_count": 8
}
```

## 실행 절차

```python
# 1. 결과 디렉토리 준비
mkdir -p .claude

# 2. Base commit 결정
if base == "auto":
    base_ref = run("git merge-base HEAD main") or run("git merge-base HEAD master")
    if not base_ref:
        base_ref = run("git rev-list --max-parents=0 HEAD")
    base_description = f"merge-base with {detected_branch}"
else:
    base_ref = base
    base_description = f"specified ref: {base}"

# 3. 전체 CLAUDE.md 파일 수집
all_claude_md = glob(f"{path}/**/CLAUDE.md")

# 4. 변경된 파일 감지
changed_tracked = run(f"git diff --name-only --diff-filter=ACMR {base_ref}...HEAD -- '**/CLAUDE.md'")

# 5. Untracked 파일 감지 (옵션)
if include_untracked:
    untracked = run("git status --porcelain -- '**/CLAUDE.md' | grep '^??' | cut -c4-")

# 6. 상태별 분류
changed_files = []
for file in changed_tracked:
    status = determine_status(file, base_ref)
    changed_files.append({"path": file, "status": status})

for file in untracked:
    changed_files.append({"path": file, "status": "untracked"})

# 7. path 필터 적용
if path != ".":
    changed_files = [f for f in changed_files if f["path"].startswith(path)]

# 8. 결과 저장
result = {
    "base_ref": base_ref,
    "base_description": base_description,
    "head_ref": "HEAD",
    "analysis_path": path,
    "changed_files": changed_files,
    "unchanged_count": len(all_claude_md) - len(changed_files),
    "total_claude_md_count": len(all_claude_md)
}
write_json(".claude/diff-analyze-result.json", result)

# 9. 요약 출력
print(f"Base: {base_ref[:8]} ({base_description})")
print(f"변경된 CLAUDE.md: {len(changed_files)}개")
print(f"변경 없음: {result['unchanged_count']}개")
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| Git 저장소 아님 | "Git 저장소가 아닙니다" 오류 메시지, 빈 결과 반환 |
| main/master 없음 | 첫 커밋 기준으로 fallback |
| 지정된 base 없음 | "지정된 ref를 찾을 수 없습니다: {base}" 오류 |
| path 존재하지 않음 | "경로를 찾을 수 없습니다: {path}" 오류 |

## 사용 예시

```bash
# incremental-generate에서 호출
Skill("claude-md-plugin:diff-analyze")

# 특정 경로만 분석
Skill("claude-md-plugin:diff-analyze", path="src/auth")

# 특정 commit 기준
Skill("claude-md-plugin:diff-analyze", base="abc1234")

# develop 브랜치 기준
Skill("claude-md-plugin:diff-analyze", base="develop")

# untracked 파일 제외
Skill("claude-md-plugin:diff-analyze", include_untracked=false)
```
