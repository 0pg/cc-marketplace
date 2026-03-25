---
name: status
version: 2.0.0
aliases: [health, dashboard, overview]
description: |
  This skill should be used when the user asks to "show project status", "project health",
  "CLAUDE.md overview", "documentation coverage", or uses "/status".
  Shows a project-wide health dashboard: schema validity, compile freshness, convention health, and DEVELOPERS.md pairing.
  Trigger keywords: 프로젝트 상태, 건강도, 대시보드, 문서 커버리지, 전체 현황
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write]
---

# /status

프로젝트 전체의 CLAUDE.md 건강도를 대시보드 형태로 표시합니다.

빠른 진단 도구로, `/validate`보다 가볍고 빠르게 전체 현황을 파악합니다.

## Triggers

- `/status`
- `프로젝트 상태`
- `건강도`

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

### 1. CLAUDE.md 수집

```
Glob("{path}/**/CLAUDE.md")
```

수집된 파일이 없으면: "대상 경로에 CLAUDE.md가 없습니다." → 종료.

### 2. 스키마 검증

각 CLAUDE.md에 대해:

```bash
$CLI_PATH validate-schema --file "$claude_md" --output "${TMP_DIR}status-schema-${dir_safe}.json"
```

PASS/FAIL 집계.

### 3. Compile 신선도 확인

```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

각 모듈 상태 분류:
- **FRESH**: 코드가 문서와 동기화
- **STALE**: 문서가 코드보다 최신 (재컴파일 필요)
- **UNCOMPILED**: 소스코드 없음 (첫 컴파일 필요)

### 4. Convention 건강도 확인

```bash
$CLI_PATH validate-convention --project-root {project_root}
```

결과에서 필수 서브섹션 존재 여부와 위반 수를 확인합니다.

### 5. DEVELOPERS.md 존재 확인 (INV-3)

각 CLAUDE.md 디렉토리에 DEVELOPERS.md가 존재하는지 확인합니다.

### 6. 대시보드 출력

```markdown
# 프로젝트 건강도

## 요약

| 지표 | 값 | 상태 |
|------|-----|------|
| CLAUDE.md 수 | 5 | - |
| 스키마 유효 | 4/5 (80%) | WARNING |
| Compile 신선도 | 3/5 FRESH | WARNING |
| Convention | PASS (6/6 서브섹션) | OK |
| DEVELOPERS.md 쌍 | 4/5 (80%) | WARNING |

## 모듈별 상태

| 모듈 | 스키마 | Compile | DEVELOPERS.md |
|------|--------|---------|---------------|
| src/auth | PASS | FRESH | EXISTS |
| src/api | PASS | STALE | EXISTS |
| src/utils | PASS | FRESH | EXISTS |
| src/legacy | FAIL (1) | STALE | MISSING |
| src/new | PASS | UNCOMPILED | EXISTS |

## 추천 액션

1. `src/legacy`: 스키마 오류 수정 → `/validate src/legacy`
2. `src/api`, `src/legacy`: 재컴파일 필요 → `/compile`
3. `src/new`: 첫 컴파일 필요 → `/compile --path src/new`
4. `src/legacy`: DEVELOPERS.md 생성 필요 → `/decompile src/legacy`
```

## 전체 건강도 판정

| 등급 | 조건 |
|------|------|
| HEALTHY | 스키마 100% PASS, Compile 100% FRESH, DEVELOPERS.md 100% |
| GOOD | 스키마 ≥80%, Compile ≥60% FRESH |
| WARNING | 스키마 ≥50% 또는 Compile ≥30% FRESH |
| CRITICAL | 스키마 <50% 또는 Compile <30% FRESH |

## DO / DON'T

**DO:**
- 빠르게 전체 현황 파악 (CLI 기반, LLM 분석 최소화)
- 추천 액션 제시
- Convention 건강도 포함

**DON'T:**
- 파일 수정 (읽기 전용)
- 상세 drift 분석 (/validate가 담당)
- 느린 코드 분석 (analyze-code 미사용)

## 오류 처리

| 상황 | 대응 |
|------|------|
| CLI 빌드 실패 | install-cli.sh가 자동 빌드 |
| CLAUDE.md 없음 | 안내 메시지, 종료 |
| diff-compile-targets 실패 | Compile 컬럼을 "N/A"로 표시 |
| validate-convention 실패 | Convention 컬럼을 "N/A"로 표시 |

## Examples

<example>
<user_request>/status</user_request>
<assistant_response>
프로젝트 건강도: GOOD

| 지표 | 값 | 상태 |
|------|-----|------|
| CLAUDE.md 수 | 3 | - |
| 스키마 유효 | 3/3 (100%) | OK |
| Compile 신선도 | 2/3 FRESH | WARNING |
| Convention | PASS | OK |
| DEVELOPERS.md 쌍 | 3/3 (100%) | OK |

추천: `/compile` 으로 stale 모듈 재컴파일
</assistant_response>
</example>
