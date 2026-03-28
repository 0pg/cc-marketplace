<!--
  validator-templates.md
  Consolidated reference for the validator agent.
  Contains: Drift type definitions (v7 schema),
  Convention CODE_VIOLATION (architectural only),
  DEVELOPERS.md content drift (--strict only),
  result template format, and CLI output JSON structures.

  Loaded at runtime by the validator agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/validate/references/validator-templates.md"
-->

# Validator Templates & Reference

## Drift Type Definitions (v7 Schema)

v7 CLAUDE.md 스키마는 Purpose, Requirements, Domain Context 3개 필수 섹션과
조건부 섹션(Instructions, Conventions)으로 구성됩니다.

### 1. Requirements Drift

CLAUDE.md의 Requirements 섹션과 실제 코드 동작의 불일치.

| 유형 | 설명 | 검증 방법 | 신뢰도 |
|------|------|----------|--------|
| **VIOLATED** | 코드가 명시된 요구사항을 위반 | 코드에서 요구사항 위반 패턴 검색 (Grep) | MEDIUM (샘플 기반) |
| **STALE** | 요구사항이 코드에서 더 이상 적용되지 않음 | 요구사항과 관련된 코드 패턴이 존재하지 않음 | LOW (코드 리팩토링으로 패턴 변경 가능) |

**검증 방법:**
1. CLAUDE.md의 Requirements 섹션을 파싱하여 개별 요구사항 추출
2. 각 요구사항에서 키워드/수치를 추출 (e.g., "최대 7일" → `7`, `expiry`)
3. Grep으로 관련 코드 패턴 검색
4. 요구사항 위반 또는 미적용 여부 판정

**예시:**
```
Requirements: "동시 세션은 최대 5개"
코드 검색: MAX_SESSIONS, maxSessions, session.*limit
발견: const MAX_SESSIONS = 10  →  VIOLATED (5 vs 10)
```

### 2. Convention CODE_VIOLATION

코드가 CLAUDE.md Conventions 섹션의 **architectural 규칙**을 위반.
syntactic 규칙(네이밍 컨벤션, 코드 포맷팅 등)은 린터 영역이므로 검증하지 않습니다.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **CODE_VIOLATION** | 코드가 Convention의 architectural 규칙 위반 | 샘플 기반 Grep 검증 (신뢰도: MEDIUM) |

**검증 대상 예시:**
- 의존성 방향 규칙 (Module Boundaries)
- 패턴 준수 (Coding Rules 중 architectural 패턴)
- 레이어 분리 규칙

**Note:** Convention 구조 검증(MISSING_CONVENTION, MISSING_SUBSECTION)은 validate SKILL Phase 2b에서 CLI로 처리됩니다.

### 3. DEVELOPERS.md Content Drift (`--strict` only)

**이 섹션은 `--strict` 모드에서만 실행됩니다.** DEVELOPERS.md 존재 확인은 validate SKILL Phase 2d에서 deterministic으로 처리합니다.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **CONSTRAINTS_STALE** | DEVELOPERS.md Constraints와 코드 불일치 | 키워드 기반 매칭 |
| **TECHNICAL_CONTEXT_STALE** | Technical Context가 현재 코드와 맞지 않음 | 키워드 기반 코드 매칭 |

## Result Template

```markdown
# 검증 결과: {directory}

## 요약

- 전체 이슈: {N}개
- Requirements Drift: {n1}개
- Convention CODE_VIOLATION: {n2}개
- DEVELOPERS.md Content Drift: {n3}개 (strict only)

## 상세

### Requirements Drift

#### VIOLATED (요구사항 위반)
- "동시 세션은 최대 5개": 코드에서 MAX_SESSIONS = 10 (불일치)

#### STALE (미적용 요구사항)
- "Redis 캐시 TTL은 토큰 만료보다 짧아야 함": Redis 관련 코드 없음

### Convention CODE_VIOLATION
- Module Boundaries 위반: domain 레이어에서 infrastructure 직접 import (샘플: `auth/domain.ts:15`)

### DEVELOPERS.md Content Drift (strict only)

#### CONSTRAINTS_STALE
- DEVELOPERS.md Constraints의 "입력 최대 1MB" vs 코드 `MAX_INPUT_SIZE = 10MB` (불일치)

#### TECHNICAL_CONTEXT_STALE
- Technical Context에 "Redis 캐시 사용" 명시되나 코드에 Redis 의존성 없음
```

## CLI Output JSON Structures

### parse-claude-md 출력 (v7 schema)

```json
{
  "name": "auth",
  "purpose": "User authentication module",
  "requirements": ["토큰 만료 최대 7일", "동시 세션 최대 5개"],
  "domain_context": "JWT 토큰은 PCI-DSS 준수를 위해 7일 만료 정책 적용",
  "warnings": []
}
```
