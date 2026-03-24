<!--
  validator-templates.md
  Consolidated reference for the validator agent.
  Contains: Drift type definitions (v6 schema),
  Convention drift, DEVELOPERS.md drift, Boundary violations,
  result template format, and CLI output JSON structures.

  Loaded at runtime by the validator agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/validate/references/validator-templates.md"
-->

# Validator Templates & Reference

## Drift Type Definitions (v6 Schema)

v6 CLAUDE.md 스키마는 Purpose, Constraints, Domain Context 3개 필수 섹션과
조건부 섹션(Instructions, Conventions)으로 구성됩니다.

### 1. Constraints Drift

CLAUDE.md의 Constraints 섹션과 실제 코드 동작의 불일치.

| 유형 | 설명 | 검증 방법 | 신뢰도 |
|------|------|----------|--------|
| **VIOLATED** | 코드가 명시된 제약을 위반 | 코드에서 제약 위반 패턴 검색 (Grep) | MEDIUM (샘플 기반) |
| **STALE** | 제약이 코드에서 더 이상 적용되지 않음 | 제약과 관련된 코드 패턴이 존재하지 않음 | LOW (코드 리팩토링으로 패턴 변경 가능) |

**검증 방법:**
1. CLAUDE.md의 Constraints 섹션을 파싱하여 개별 제약 추출
2. 각 제약에서 키워드/수치를 추출 (e.g., "최대 7일" → `7`, `expiry`)
3. Grep으로 관련 코드 패턴 검색
4. 제약 위반 또는 미적용 여부 판정

**예시:**
```
Constraints: "동시 세션은 최대 5개"
코드 검색: MAX_SESSIONS, maxSessions, session.*limit
발견: const MAX_SESSIONS = 10  →  VIOLATED (5 vs 10)
```

### 2. Domain Context Drift

CLAUDE.md의 Domain Context와 코드/환경의 불일치.

| 유형 | 설명 | 검증 방법 | 신뢰도 |
|------|------|----------|--------|
| **STALE** | Domain Context가 현재 코드와 맞지 않음 | 키워드 기반 코드 매칭 | LOW (맥락 변경 감지 어려움) |

**검증 방법:**
1. Domain Context에서 기술적 키워드 추출 (e.g., "Redis 캐시", "HMAC-SHA256")
2. Grep으로 관련 코드/설정 존재 확인
3. 언급된 기술/패턴이 코드에서 사용되지 않으면 STALE

### 3. Convention Drift

CLAUDE.md의 Conventions 섹션(프로젝트/코드 수준 규칙)과 실제 코드 스타일의 불일치.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **MISSING_CONVENTION** | project_root에 필수 Convention 섹션 없음 | CLI validate-convention 또는 수동 확인 |
| **MISSING_SUBSECTION** | Convention에 필수 서브섹션 없음 | 섹션 구조 확인 |
| **CODE_VIOLATION** | 코드가 Convention 규칙 위반 | 샘플 기반 Grep 검증 (신뢰도: MEDIUM) |

### 4. DEVELOPERS.md Drift

DEVELOPERS.md의 존재 여부와 내용 일치성 검증.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **MISSING_DEVELOPERS_MD** | CLAUDE.md가 있는데 DEVELOPERS.md 없음 | INV-3 검증 |
| **FILE_MAP_ORPHAN** | File Map에 있지만 실제로 없는 파일 | 파일 존재 확인 |
| **FILE_MAP_UNCOVERED** | 실제에만 있는 소스 파일 | File Map과 비교 |
| **INVARIANT_STALE** | Invariants가 코드와 맞지 않음 | 키워드 기반 코드 매칭 (LOW) |

### 5. Boundary Violations (INV-1)

CLAUDE.md 또는 코드 내 참조가 트리 구조 의존성을 위반.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **PARENT_REFERENCE** | `../` 참조 (부모 참조 금지) | resolve-boundary CLI |
| **SIBLING_REFERENCE** | 형제 디렉토리 참조 | resolve-boundary CLI |

## Result Template

```markdown
# 검증 결과: {directory}

## 요약

- 전체 이슈: {N}개
- Constraints Drift: {n1}개
- Domain Context Drift: {n2}개
- Convention Drift: {n3}개
- DEVELOPERS.md Drift: {n4}개
- Boundary Violations: {n5}개

## 상세

### Constraints Drift

#### VIOLATED (제약 위반)
- "동시 세션은 최대 5개": 코드에서 MAX_SESSIONS = 10 (불일치)

#### STALE (미적용 제약)
- "Redis 캐시 TTL은 토큰 만료보다 짧아야 함": Redis 관련 코드 없음

### Domain Context Drift

#### STALE (맥락 불일치)
- "Redis 캐시를 사용하여 인증 지연을 최소화": Redis 의존성/코드 없음

### Convention Drift

#### MISSING_CONVENTION
- project_root에 `## Conventions` 섹션 없음

#### CODE_VIOLATION
- Naming Rules 위반: `myFunc` → Convention에서 `snake_case` 요구 (샘플: `utils.py:15`)

### DEVELOPERS.md Drift

#### MISSING_DEVELOPERS_MD
- src/auth/CLAUDE.md 존재하나 DEVELOPERS.md 없음 (INV-3 위반)

#### FILE_MAP_ORPHAN
- `legacy.ts`: File Map에 있으나 실제로 존재하지 않음

### Boundary Violations

#### PARENT_REFERENCE
- `../utils` 참조 발견 (line 15)
```

## CLI Output JSON Structures

### parse-claude-md 출력 (v6 schema)

```json
{
  "name": "auth",
  "purpose": "User authentication module",
  "constraints": ["토큰 만료 최대 7일", "동시 세션 최대 5개"],
  "domain_context": "JWT 토큰은 PCI-DSS 준수를 위해 7일 만료 정책 적용",
  "warnings": []
}
```

### resolve-boundary 출력

```json
{
  "path": "src/auth",
  "direct_files": [{"name": "index.ts", "type": "typescript"}, {"name": "types.ts", "type": "typescript"}],
  "subdirs": [{"name": "jwt", "has_claude_md": true}],
  "source_file_count": 2,
  "subdir_count": 1,
  "violations": [{"violation_type": "Parent", "reference": "../utils", "line_number": 15}]
}
```
