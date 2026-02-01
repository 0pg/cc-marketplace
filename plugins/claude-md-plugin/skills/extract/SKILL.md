---
name: extract
aliases: [ext, claude-md]
description: |
  소스 코드를 분석하여 각 디렉토리에 CLAUDE.md를 생성합니다.
  Rust CLI로 트리를 파싱하고, 각 디렉토리에 extractor 에이전트를 실행합니다.
  트리거: "/extract", "/ext", "CLAUDE.md 추출", "소스코드 문서화", "claude-md 생성"
allowed-tools: [Bash, Read, Glob, Task, AskUserQuestion]
---

# Extract Skill

## 목적

기존 소스 코드를 분석하여 CLAUDE.md 초안을 생성합니다.
CLAUDE.md는 해당 디렉토리의 Source of Truth가 되어 코드 재현의 기반이 됩니다.

## 워크플로우

### 1. Core Engine 빌드 확인

```bash
# CLI가 빌드되어 있는지 확인
if [ ! -f "plugins/claude-md-plugin/core/target/release/claude-md-core" ]; then
    echo "Building claude-md-core..."
    cd plugins/claude-md-plugin/core && cargo build --release
fi
```

빌드된 CLI가 없으면 먼저 빌드합니다.

### 2. 트리 파싱

```bash
claude-md-core parse-tree --root . --output .claude/extract-tree.json
```

프로젝트 루트에서 트리를 파싱하여 CLAUDE.md가 필요한 디렉토리 목록을 생성합니다.

**출력 예시** (tree.json):
```json
{
  "root": "/path/to/project",
  "needs_claude_md": [
    {"path": "src", "source_file_count": 2, "subdir_count": 3, "reason": "2 source files and 3 subdirectories", "depth": 1},
    {"path": "src/auth", "source_file_count": 4, "subdir_count": 1, "reason": "4 source files", "depth": 2},
    {"path": "src/api", "source_file_count": 5, "subdir_count": 0, "reason": "5 source files", "depth": 2}
  ],
  "excluded": ["node_modules", "target", "dist"]
}
```

### 3. 대상 디렉토리 확인

tree.json을 읽고 CLAUDE.md가 필요한 디렉토리 목록을 **depth 내림차순** (leaf-first)으로 정렬하여 사용자에게 보여줍니다.

```
=== CLAUDE.md 생성 대상 ===

다음 디렉토리에 CLAUDE.md를 생성합니다 (leaf-first 순서):
  1. [depth=2] src/auth/ (4 source files)
  2. [depth=2] src/api/ (5 source files)
  3. [depth=1] src/ (2 source files, 3 subdirectories)

제외된 디렉토리: node_modules, target, dist

계속하시겠습니까?
```

**핵심:** 자식 디렉토리 CLAUDE.md가 먼저 생성되어야 부모가 자식의 Purpose를 읽을 수 있습니다.

### 4. Leaf-first 순차 실행

**병렬이 아닌 순차 실행**으로 depth가 깊은 디렉토리(leaf)부터 처리합니다.

```python
# depth 내림차순 정렬 (leaf-first)
sorted_dirs = sorted(needs_claude_md, key=lambda d: -d.depth)

for dir_info in sorted_dirs:
    # 하위 CLAUDE.md 경로 목록 (이미 생성된 자식들)
    child_claude_mds = [
        f"{dir_info.path}/{subdir}/CLAUDE.md"
        for subdir in dir_info.subdirs
        if Path(f".claude/extract-results/{dir_info.path}-{subdir}.md").exists()
    ]

    Task(
        subagent_type="claude-md-plugin:extractor",
        prompt=f"""
대상 디렉토리: {dir_info.path}
직접 파일 수: {dir_info.source_file_count}
하위 디렉토리 수: {dir_info.subdir_count}
자식 CLAUDE.md: {child_claude_mds}

이 디렉토리의 CLAUDE.md를 생성해주세요.
결과 파일: .claude/extract-results/{dir_info.path.replace('/', '-')}.md
""",
        description=f"Extract CLAUDE.md for {dir_info.path}"
    )
    # 다음 디렉토리 처리 전 완료 대기
```

**실행 순서 예시:**
```
1. src/auth/jwt  (depth=3) → CLAUDE.md 생성
2. src/auth      (depth=2) → jwt/CLAUDE.md Purpose 읽기 가능
3. src/api       (depth=2) → CLAUDE.md 생성
4. src           (depth=1) → auth/CLAUDE.md, api/CLAUDE.md Purpose 읽기 가능
```

### 5. 결과 수집 및 검증

각 Agent 실행 완료 즉시 (순차 실행이므로):

1. `.claude/extract-results/{dir-name}.md` 결과 파일 확인
2. `claude-md-core validate-schema` 실행
3. 검증 실패 시 Agent에게 수정 요청 (최대 3회)
4. 검증 통과 시 실제 CLAUDE.md 위치로 복사
5. **중요:** 복사 후 다음 depth의 Agent가 읽을 수 있도록 즉시 배치

```bash
# 검증 성공 시 즉시 배치 (다음 Agent가 읽을 수 있도록)
cp .claude/extract-results/src-auth.md src/auth/CLAUDE.md
```

### 6. 최종 보고

```
=== CLAUDE.md 추출 완료 ===

생성된 파일:
  ✓ src/CLAUDE.md
  ✓ src/auth/CLAUDE.md
  ✓ src/api/CLAUDE.md

검증 결과:
  - 스키마 검증: 3/3 통과
  - 참조 규칙: 3/3 통과

사용자 질문: 5개 응답됨

다음 단계:
  - /context-validate로 재현성 검증 가능
```

## 참고: Agent 동작

다음은 extractor Agent가 담당합니다 (Skill이 하는 일 아님):

- 소스 코드 분석 (시그니처, 의존성, 동작)
- 불명확한 부분 사용자 질문
- CLAUDE.md 초안 생성 (templates/claude-md-schema.md 따름)
- 스키마 검증 및 수정

상세 내용은 `agents/extractor.md` 참조.

## 파일 기반 결과 전달

Agent는 결과를 파일로 저장하고 경로만 반환합니다:

| Agent | 결과 파일 경로 |
|-------|--------------|
| extractor | `.claude/extract-results/{dir-name}.md` |

이로써 Skill context 폭발을 방지합니다.
