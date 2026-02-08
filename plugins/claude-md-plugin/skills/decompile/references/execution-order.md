# Decompile Execution Order Details

## Architecture Overview

```
/decompile [path] [options]
    │
    ▼
Task(recursive-decompiler, root_path)
    │
    ├─ Phase 1: boundary-resolve → subdirs
    │
    ├─ Phase 2: Filtered subdirs (ignore node_modules, etc.)
    │
    ├─ Phase 3: For each subdir (recursive):
    │      Task(recursive-decompiler, subdir)
    │          └─ ... (same phases)
    │
    ├─ Phase 4: Incremental judgment (git-based)
    │      └─ needs_decompile? → true/false
    │
    ├─ Phase 5: Self processing (if needs_decompile)
    │      Task(decompiler, self) → CLAUDE.md + IMPLEMENTS.md
    │
    └─ Phase 6: Result aggregation
           └─ processed, skipped, child_claude_mds
```

## 재귀적 Leaf-First 순서

기존 tree-parse 기반 접근 대신 재귀적 구조를 사용합니다:

**기존 (v1.x):**
```
tree-parse → 전체 스캔 → depth 정렬 → 순차 실행
```

**현재 (v2.x):**
```
recursive-decompiler → 자식 재귀 → 자식 완료 후 자신 처리
```

**장점:**
1. tree-parse 전체 스캔 불필요
2. 개별 node별 독립된 context
3. node 누락 가능성 제거
4. 자연스러운 leaf-first 순서 보장

**실행 순서 예시:**
```
recursive-decompiler(src)
    ├─ recursive-decompiler(src/auth)
    │      ├─ recursive-decompiler(src/auth/jwt)
    │      │      └─ decompiler(src/auth/jwt) → #1
    │      └─ decompiler(src/auth) → #2
    ├─ recursive-decompiler(src/utils)
    │      └─ decompiler(src/utils) → #3
    └─ decompiler(src) → #4
```

## Incremental Mode (Git 기반)

### 판단 기준

**방향: 소스코드 → CLAUDE.md (compile의 역방향)**

| 조건 | 판단 | 동작 |
|------|------|------|
| CLAUDE.md 없음 | 신규 | decompile 필요 |
| 소스 uncommitted 변경 | 미확정 | decompile 필요 |
| 소스 commit > CLAUDE.md commit | outdated | decompile 필요 |
| CLAUDE.md commit >= 소스 commit | up-to-date | **skip** |
| 소스 파일 없음 + 자식 없음 | empty | **skip** |

### Git 명령어

```bash
# CLAUDE.md + IMPLEMENTS.md 중 최신 커밋 시점
spec_time=$(git log -1 --format=%ct -- "{path}/CLAUDE.md" "{path}/IMPLEMENTS.md" 2>/dev/null | head -1 || echo "0")

# 소스 파일의 최신 커밋 시점
source_time=$(git log -1 --format=%ct -- "{path}/*.ts" "{path}/*.tsx" "{path}/*.js" "{path}/*.jsx" "{path}/*.py" "{path}/*.go" "{path}/*.rs" "{path}/*.java" "{path}/*.kt" 2>/dev/null | head -1 || echo "0")

# Uncommitted 소스 파일 확인
git status --porcelain "{path}" | grep -v -E "(CLAUDE|IMPLEMENTS)\.md$" | grep -E "\.(ts|tsx|js|jsx|py|go|rs|java|kt)$"
```

### 예시 시나리오

```
src/
├── auth/
│   ├── CLAUDE.md      (commit: 1월 15일)  ← 스펙
│   ├── IMPLEMENTS.md  (commit: 1월 15일)
│   └── index.ts       (commit: 1월 20일)  ← 소스 (스펙보다 최신!)
├── utils/
│   ├── CLAUDE.md      (commit: 1월 25일)  ← 스펙 (소스보다 최신)
│   └── helpers.ts     (commit: 1월 10일)
└── api/
    └── index.ts       (uncommitted)       ← 미커밋 소스
```

**결과:**
- `src/auth`: decompile 필요 (source_newer)
- `src/utils`: skip (up_to_date)
- `src/api`: decompile 필요 (uncommitted_sources + no_claude_md)

## Ignored Directories

다음 디렉토리는 Phase 2에서 필터링됩니다:

**IGNORED_DIRS:**
- `node_modules` - npm/yarn 의존성
- `target` - Rust/Maven 빌드 결과
- `dist`, `build` - 빌드 출력
- `__pycache__` - Python 캐시
- `.git` - Git 메타데이터
- `vendor` - Go/PHP 의존성
- `.next`, `.nuxt` - JS 프레임워크 빌드
- `coverage` - 테스트 커버리지
- `.venv`, `venv`, `env` - Python 가상환경

**숨김 디렉토리 (`.` 시작)도 제외됩니다.**

## 순환 감지 (Defense in Depth)

순환 참조(symlink)로 인한 무한 재귀를 방지하기 위해 두 단계의 방어선을 구현합니다.

### 1차 방어: visited_paths 기반 순환 감지

각 재귀 호출에서 현재 디렉토리의 실제 경로(realpath)를 추적합니다.

```bash
# 실제 경로 확인 (symlink 해석)
realpath "{target_path}"
```

**동작:**
1. Phase 0에서 target_path의 realpath를 확인
2. visited_paths에 이미 존재하면 순환으로 판단
3. 즉시 반환: `needs_decompile=false`, `reason="cycle_detected"`
4. Phase 3에서 자식 호출 시 `visited_paths + [real_path]`를 전달

### 2차 방어: max_depth 안전장치

- **기본값:** max_depth = 100
- **역할:** visited_paths 로직에 버그가 있을 경우를 대비한 최후의 안전장치
- **초과 시:** 경고 로그 출력, 재귀 중단

### 예시 시나리오: 형제 간 symlink 순환

```
src/
├── moduleA/
│   ├── index.ts
│   └── link_to_B -> ../moduleB   # symlink
└── moduleB/
    ├── index.ts
    └── link_to_A -> ../moduleA   # symlink
```

**실행 흐름:**
```
recursive-decompiler(src, visited=[])
    │
    ├─ recursive-decompiler(src/moduleA, visited=["/abs/src"])
    │      ├─ 실제 경로: /abs/src/moduleA
    │      ├─ visited에 추가: ["/abs/src", "/abs/src/moduleA"]
    │      │
    │      └─ recursive-decompiler(src/moduleA/link_to_B, visited=["/abs/src", "/abs/src/moduleA"])
    │             ├─ realpath: /abs/src/moduleB
    │             └─ visited에 없음 → 정상 진행
    │
    └─ recursive-decompiler(src/moduleB, visited=["/abs/src"])
           ├─ 실제 경로: /abs/src/moduleB
           ├─ visited에 추가: ["/abs/src", "/abs/src/moduleB"]
           │
           └─ recursive-decompiler(src/moduleB/link_to_A, visited=["/abs/src", "/abs/src/moduleB"])
                  ├─ realpath: /abs/src/moduleA
                  ├─ visited에 없음 (이 경로에서는 처음)
                  └─ 정상 진행 → 다시 decompiler 호출되지만
                     moduleA는 이미 처리됨 (incremental skip)
```

**핵심:** 같은 물리적 경로를 다른 논리적 경로에서 접근하는 경우도 감지합니다.

## decompiler Agent 아키텍처

recursive-decompiler가 호출하는 decompiler agent:

```
┌─────────────────────────────────────────────┐
│ decompiler AGENT (디렉토리별)               │
│                                             │
│ ┌─ CLI: resolve-boundary ────────────────┐  │
│ │ 바운더리 분석                           │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ Skill("code-analyze") ────────────────┐  │
│ │ 코드 분석 (exports, deps, behaviors)    │  │
│ │ + 알고리즘, 상수, 에러처리, 상태 분석   │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ AskUserQuestion ──────────────────────┐  │
│ │ 불명확한 부분 질문                      │  │
│ │ (Domain Context, Implementation 배경)   │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ CLAUDE.md + IMPLEMENTS.md 생성 ───────┐  │
│ │ CLAUDE.md: WHAT 초안 생성               │  │
│ │ IMPLEMENTS.md: HOW 전체 섹션 생성       │  │
│ └──────────────────┬─────────────────────┘  │
│                    ▼                        │
│ ┌─ CLI: validate-schema ─────────────────┐  │
│ │ 스키마 검증 (실패시 경고)               │  │
│ └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## 결과 배치

각 decompiler 완료 즉시:
1. `.claude/tmp/{session-id}-decompile-*` 결과 파일 확인 (CLAUDE.md + IMPLEMENTS.md)
2. 검증 통과 시 실제 위치로 복사
3. **중요:** 복사 후 부모 Agent가 자식 CLAUDE.md Purpose를 읽을 수 있음

```bash
# 검증 성공 시 즉시 배치
cp .claude/tmp/{session-id}-decompile-src-auth-claude.md src/auth/CLAUDE.md
cp .claude/tmp/{session-id}-decompile-src-auth-implements.md src/auth/IMPLEMENTS.md
```

## Edge Cases

| 케이스 | 처리 |
|--------|------|
| 무한 재귀 | 1차: visited_paths 기반 순환 감지, 2차: max_depth=100 안전장치 |
| 빈 디렉토리 | 소스 파일/자식 없으면 skip |
| Ignored 디렉토리 | Phase 2에서 필터링 |
| 순환 참조 (symlink) | realpath로 실제 경로 확인, visited_paths에서 중복 체크 |
| Git 저장소 아님 | incremental 비활성화, 전체 처리 |
| 새 디렉토리 (CLAUDE.md 없음) | 항상 decompile |

## 통계 집계

recursive-decompiler가 자식 결과를 집계합니다:

1. `child_stats`와 `child_claude_mds`를 초기화합니다.
2. 각 하위 디렉토리에 대해 `recursive-decompiler`를 재귀 호출합니다.
3. 자식 결과에서 `processed`, `skipped`, `child_claude_mds`를 수집하여 누적합니다.
4. 자신이 decompile이 필요하면 `decompiler` agent를 호출하고 `processed`를 증가시킵니다.
5. 자신이 스킵되면 `skipped`를 증가시킵니다.
6. 최종 결과를 부모에게 반환합니다.
