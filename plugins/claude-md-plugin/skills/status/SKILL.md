---
name: status
version: 1.0.0
aliases: [health, dashboard, overview]
description: |
  This skill should be used when the user asks to "show project status", "project health",
  "CLAUDE.md overview", "documentation coverage", or uses "/status".
  Shows a project-wide health dashboard: schema validity, export coverage, drift count, and compile freshness.
  Trigger keywords: 프로젝트 상태, 건강도, 대시보드, 문서 커버리지, 전체 현황
user_invocable: true
allowed-tools: [Bash, Read, Glob, Grep, Write]
---

> **DEPRECATED (v6.0.0)**: This skill depends on CLAUDE.md sections (Exports, Behavior, Contract, Protocol) that were removed in v6.0.0. Will be redesigned in a future version.

# /status

프로젝트 전체의 계약(CLAUDE.md) 건강도를 대시보드 형태로 표시합니다.

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

### 1. CLAUDE.md 수집

```bash
CLI_PATH=$("${CLAUDE_PLUGIN_ROOT}/scripts/install-cli.sh")
TMP_DIR=".claude/tmp/${CLAUDE_SESSION_ID:+${CLAUDE_SESSION_ID}/}"
mkdir -p "$TMP_DIR"
```

Glob으로 대상 경로의 모든 CLAUDE.md를 수집합니다.

### 2. 빠른 스키마 검증

각 CLAUDE.md에 대해 스키마 검증을 실행합니다:

```bash
$CLI_PATH validate-schema --file "$claude_md" --output "${TMP_DIR}status-schema-${dir_safe}.json"
```

결과를 수집하여 PASS/FAIL 집계합니다.

### 3. Compile 신선도 확인

각 CLAUDE.md 디렉토리의 compile 상태를 확인합니다:

```bash
$CLI_PATH diff-compile-targets --root {project_root}
```

결과에서 각 모듈의 상태를 분류합니다:
- **FRESH**: 코드가 계약과 동기화됨 (up-to-date)
- **STALE**: 계약이 코드보다 최신 (재컴파일 필요)
- **UNCOMPILED**: 소스코드 없음 (첫 컴파일 필요)

### 4. DEVELOPERS.md 존재 확인 (INV-3)

각 CLAUDE.md 디렉토리에 DEVELOPERS.md가 존재하는지 확인합니다.

### 5. 대시보드 출력

```markdown
# 프로젝트 계약 건강도

## 요약

| 지표 | 값 | 상태 |
|------|-----|------|
| CLAUDE.md 수 | 5 | - |
| 스키마 유효 | 4/5 (80%) | WARNING |
| Compile 신선도 | 3/5 FRESH | WARNING |
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
4. `src/legacy`: DEVELOPERS.md 생성 필요
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

**DON'T:**
- 파일 수정 (읽기 전용)
- 상세 drift 분석 (/validate가 담당)
- 느린 코드 분석 (analyze-code 미사용)

## Examples

<example>
<user_request>/status</user_request>
<assistant_response>
프로젝트 계약 건강도: GOOD

| 지표 | 값 | 상태 |
|------|-----|------|
| CLAUDE.md 수 | 3 | - |
| 스키마 유효 | 3/3 (100%) | OK |
| Compile 신선도 | 2/3 FRESH | WARNING |
| DEVELOPERS.md 쌍 | 3/3 (100%) | OK |

추천: `/compile` 으로 stale 모듈 재컴파일
</assistant_response>
</example>
