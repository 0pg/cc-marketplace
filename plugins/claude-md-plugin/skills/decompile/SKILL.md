---
name: decompile
version: 2.0.0
aliases: [decom]
description: |
  This skill should be used when the user asks to "decompile code to CLAUDE.md", "extract CLAUDE.md from code",
  "document existing codebase", "reverse engineer spec", or uses "/decompile" or "/decom".
  Analyzes existing source code and creates CLAUDE.md + DEVELOPERS.md documentation for each directory.
  Trigger keywords: 디컴파일, 코드에서 문서 추출, 기존 코드 문서화
user_invocable: true
allowed-tools: [Bash, Read, Write, Glob, Task, Skill]
---

# /decompile

소스코드를 분석하여 CLAUDE.md + DEVELOPERS.md를 추출합니다.

## Triggers

- `/decompile`
- `코드에서 문서 추출`
- `기존 코드 문서화`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `path` | 아니오 | `.` | 대상 경로 |

## Workflow

### 0. 초기화

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

### 1. 디렉토리 트리 파싱

`Skill("claude-md-plugin:tree-parse")`로 대상 디렉토리 구조를 분석합니다.

결과: `.claude/extract-tree.json`

### 2. 실행 순서 결정

tree.json에서 `needs_claude_md` 배열을 depth DESC로 정렬 (leaf-first):

```bash
jq '[.needs_claude_md | sort_by(-.depth)]' .claude/extract-tree.json
```

### 3. 디렉토리별 decompiler 실행

정렬된 순서(leaf-first)로 각 디렉토리에 대해 `Task(decompiler)`를 실행합니다.

같은 depth의 독립 디렉토리는 병렬 실행 가능 (최대 3개).

각 디렉토리에 대해:

1. 자식 CLAUDE.md 목록 생성 (하위 디렉토리 중 이미 CLAUDE.md가 생성된 것)
2. `Task(decompiler)` 호출:
   ```
   대상: {path}  tree: .claude/extract-tree.json
   자식 CLAUDE.md: [{children_list}]
   ```
3. 결과 블록에서 `status` 확인

### 4. 변경사항 표시

```bash
git diff --stat
```

### 5. 최종 보고

```
=== Decompile 완료 ===
총 디렉토리: {total}개
성공: {success}개
실패: {failed}개

생성된 문서:
  - {path}/CLAUDE.md
  - {path}/DEVELOPERS.md
  ...
```

## DO / DON'T

**DO:**
- leaf-first 순서 준수 (자식 → 부모)
- 각 디렉토리에서 CLAUDE.md + DEVELOPERS.md 쌍 생성 (INV-3)
- 기존 CLAUDE.md가 있으면 decompiler agent가 병합 처리

**DON'T:**
- 소스코드 수정
- 부모→자식 순서로 실행 (자식 CLAUDE.md가 없으면 부모가 참조 불가)
- decompiler agent의 내부 진행 상황을 사용자에게 중계

## 오류 처리

| 상황 | 대응 |
|------|------|
| tree-parse 실패 | 에러 메시지 출력, 종료 |
| decompiler 실패 (단일 디렉토리) | 경고 출력, 나머지 계속 |
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |

## Examples

<example>
<user_request>/decompile src</user_request>
<assistant_response>
트리 파싱 중... 4개 디렉토리 감지

Decompile 진행:
  • src/auth/jwt (depth=3) — 완료
  • src/auth (depth=2) — 완료
  • src/utils (depth=2) — 완료
  • src (depth=1) — 완료

=== Decompile 완료 ===
총 디렉토리: 4개
성공: 4개

생성된 문서:
  - src/auth/jwt/CLAUDE.md + DEVELOPERS.md
  - src/auth/CLAUDE.md + DEVELOPERS.md
  - src/utils/CLAUDE.md + DEVELOPERS.md
  - src/CLAUDE.md + DEVELOPERS.md
</assistant_response>
</example>
