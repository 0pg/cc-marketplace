---
name: dev
description: |
  자연어 요청을 분류하여 적절한 claude-md-plugin 스킬로 라우팅합니다.
  /dev "로그인 기능 추가" → /impl, /dev "토큰 에러" → /bugfix 등.
argument-hint: "<request> [--path <target_path>]"
allowed-tools: [Read, Glob, Skill, AskUserQuestion]
---

# /dev — 자연어 → 스킬 라우팅

사용자의 자연어 요청을 분석하여 적절한 claude-md-plugin 스킬로 라우팅합니다.

## Step 1: 인자 파싱

- **request**: 자연어 요청 (필수)
- **--path**: 대상 경로 (기본: `.`)

인자가 없으면 AskUserQuestion으로 요청 내용을 수집합니다.

## Step 2: 의도 분류

request의 키워드를 분석하여 다음 카테고리로 분류합니다:

| 카테고리 | 키워드 (EN) | 키워드 (KR) | 대상 스킬 | 상태 |
|----------|-------------|-------------|-----------|------|
| FEATURE | add, create, implement, new, change, update, feature, requirement | 추가, 생성, 구현, 새, 변경, 수정, 기능, 요구사항 | `/impl` | active |
| BUGFIX | fix, bug, error, fail, broken, crash, debug, trace | 수정, 버그, 에러, 실패, 오류, 크래시, 진단, 추적 | `/bugfix` | active |
| COMPILE | compile, generate, build, code generation | 컴파일, 코드 생성, 빌드 | `/compile` | active |
| VALIDATE | validate, check, verify, drift, lint | 검증, 확인, 드리프트, 린트 | `/validate` | active |
| SYNC | sync, index, reindex, refresh index | 동기화, 인덱스, 인덱스 생성, 인덱스 갱신 | `/sync` | active |
| IMPACT | impact, affected, breaking, depends, dependency, influence | 영향, 영향 분석, 의존, 의존 모듈, breaking change | `/impact` | active |
| DIFF | diff, compare, changes, what changed, spec diff | 비교, 변경사항, 뭐가 바뀌었, 스펙 변경, 계약 비교 | `/diff-spec` | active |
| STATUS | status, health, dashboard, overview, coverage | 상태, 건강도, 대시보드, 현황, 커버리지 | `/status` | active |
| RESOLVE | resolve, handle violation, resolve drift | drift 해소, 위반 해소, 결과 처리 | `/resolve` | active |
| REFACTOR | refactor, split, merge, restructure, reorganize | 리팩토링, 분할, 병합, 재구성, 구조 변경 | `/refactor` | active |

**분류 규칙:**
1. 키워드 매칭은 대소문자 무시
2. 여러 카테고리에 해당하면 **첫 번째 매칭** 우선 (FEATURE > BUGFIX > COMPILE > VALIDATE > SYNC > RESOLVE > IMPACT > DIFF > STATUS > REFACTOR)
3. 어느 카테고리에도 해당하지 않으면 **AMBIGUOUS** → AskUserQuestion으로 명확화:
   - "요청을 다음 중 어느 작업으로 분류해야 할까요?"
   - 선택지: 기능 추가(/impl), 버그 수정(/bugfix), 코드 생성(/compile), 검증(/validate), 영향 분석(/impact), 기타(구체적 설명 요청)

## Step 3: CLAUDE.md 존재 확인

**FEATURE(→ /impl)와 SYNC(→ /sync) 카테고리는 이 단계를 건너뜁니다** — /impl은 새 CLAUDE.md를 생성, /sync는 소스코드에서 직접 생성하므로.

나머지 카테고리(BUGFIX, COMPILE, VALIDATE, IMPACT, DIFF, STATUS, RESOLVE, REFACTOR):
1. `--path` 경로에서 CLAUDE.md를 Glob으로 검색
2. CLAUDE.md가 없으면:
   - 메시지 출력: "대상 경로에 CLAUDE.md가 없습니다. `/decompile`로 기존 코드에서 CLAUDE.md를 생성하거나, 소스코드를 직접 작업하세요."
   - **라우팅하지 않고 종료**

## Step 4: 스킬 라우팅

분류 결과에 따라 해당 스킬을 호출합니다:

| 카테고리 | 호출 |
|----------|------|
| FEATURE | `Skill("claude-md-plugin:impl", args: "{request} [--path]")` |
| BUGFIX | `Skill("claude-md-plugin:bugfix", args: "--error \"{request}\" [--path]")` |
| COMPILE | `Skill("claude-md-plugin:compile", args: "[--path]")` |
| VALIDATE | `Skill("claude-md-plugin:validate", args: "[path]")` |
| SYNC | `Skill("claude-md-plugin:sync", args: "[--path] [--all]")` |
| RESOLVE | `Skill("claude-md-plugin:resolve", args: "[path]")` |
| IMPACT | `Skill("claude-md-plugin:impact", args: "[path]")` |
| DIFF | `Skill("claude-md-plugin:diff-spec", args: "{path}")` |
| STATUS | `Skill("claude-md-plugin:status", args: "[path]")` |
| REFACTOR | `Skill("claude-md-plugin:refactor", args: "{path}")` |

## DO / DON'T

**DO:**
- CLAUDE.md 확인 후 라우팅 (FEATURE 제외)
- 한국어/영어 키워드 모두 지원
- 분류가 모호하면 반드시 AskUserQuestion

**DON'T:**
- 직접 코드 수정하지 않음
- 소스코드를 탐색하지 않음
- 사용자 호출 없이 자동 트리거하지 않음
- 분류가 불확실할 때 임의로 결정하지 않음
