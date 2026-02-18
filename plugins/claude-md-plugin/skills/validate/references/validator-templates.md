<!--
  validator-templates.md
  Consolidated reference for the validator agent.
  Contains: Drift type definitions, export coverage formula,
  language-specific export patterns, result template format,
  and CLI output JSON structures.

  Loaded at runtime by the validator agent via:
    cat "${CLAUDE_PLUGIN_ROOT}/skills/validate/references/validator-templates.md"
-->

# Validator Templates & Reference

## Drift Type Definitions

### 1. Structure Drift

CLAUDE.md의 Structure 섹션과 실제 디렉토리 내 파일/디렉토리의 불일치.

| 유형 | 설명 | 원인 |
|------|------|------|
| **UNCOVERED** | 실제 파일이 Structure에 없음 | 새 파일 추가 후 CLAUDE.md 미갱신 |
| **ORPHAN** | Structure에 있으나 실제로 없음 | 파일 삭제 후 CLAUDE.md 미갱신 |

### 2. Exports Drift

CLAUDE.md의 Exports 섹션과 코드의 export 후보(candidates) 불일치.

| 유형 | 설명 | 신뢰도 | 원인 |
|------|------|--------|------|
| **STALE** | 문서의 export가 candidates에 없음 | 높음 | 함수 삭제/이름 변경 후 CLAUDE.md 미갱신 |
| **MISSING** | 코드의 export 후보가 문서에 없음 | 중간 (의도적 제외 가능) | 새 함수 추가 후 CLAUDE.md 미갱신 또는 LLM이 의도적으로 제외 |
| **MISMATCH** | 시그니처 불일치 | 높음 | 파라미터/반환 타입 변경 후 CLAUDE.md 미갱신 |

### 3. Dependencies Drift

CLAUDE.md의 Dependencies 섹션과 실제 의존성의 불일치.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **STALE** | 문서의 의존성이 실제로 없음 | internal → 파일 존재 확인, external → 패키지 매니저 확인 |
| **ORPHAN** | 코드에서 사용하지만 문서에 없음 | import문 분석 vs 문서 비교 |

### 4. Behavior Drift

CLAUDE.md의 Behavior 섹션과 실제 동작의 불일치.

| 유형 | 설명 | 검증 방법 |
|------|------|----------|
| **MISMATCH** | 문서화된 시나리오와 실제 동작 불일치 | 테스트 케이스 매칭, 코드 분기문 분석 |

## Export Coverage Formula

커버리지 = (문서화된 전체 export 수 - STALE 수) ÷ (문서화된 전체 export 수 + MISSING 수) × 100

- `total_exports`: 문서화된 전체 export 수
- `stale_count`: STALE로 판정된 수
- `missing_count`: MISSING으로 판정된 수
- `total_exports + missing_count`가 0이면 coverage = 100

## Export Candidates (Primary Method)

`format-exports` CLI를 사용하여 코드에서 export 후보(candidates) 목록을 생성합니다:

```bash
# 1. 코드 분석
claude-md-core analyze-code --path {directory} --output ${TMP_DIR}validate-{name}-analysis.json

# 2. Export candidates 마크다운 생성
claude-md-core format-exports --input ${TMP_DIR}validate-{name}-analysis.json --output ${TMP_DIR}validate-{name}-candidates.md
```

생성된 candidates 파일과 CLAUDE.md의 Exports 섹션을 비교하여 STALE/MISSING/MISMATCH를 판정합니다.

**비교 방법:**
- candidates의 각 항목(backtick 내 이름)과 CLAUDE.md Exports의 각 항목을 이름 기준으로 매칭
- 이름이 candidates에만 있으면 → MISSING (중간 신뢰도: LLM이 의도적으로 제외했을 수 있음)
- 이름이 CLAUDE.md에만 있으면 → STALE (높은 신뢰도: permissive analyzer도 못 찾으면 삭제된 것)
- 양쪽에 있으나 시그니처가 다르면 → MISMATCH (description 차이는 무시)

## Language-Specific Export Patterns (Fallback)

`format-exports` CLI 실행이 실패할 때 사용하는 Grep 기반 fallback 방법입니다.

디렉토리 내 파일 확장자로 언어를 감지합니다 (.ts/.tsx → TypeScript, .py → Python, .go → Go, .rs → Rust, .java → Java, .kt → Kotlin).

- export 키워드 기반 언어 (TS/JS): `^export (function|const|class)`
- public 키워드 기반 언어 (Java): `^public (class|interface)`
- public이 기본인 언어 (Kotlin): `^(fun|class|interface|object) [A-Z]`
- 대문자 시작이 public인 언어 (Go): `^func [A-Z]|^type [A-Z]`
- pub 키워드 언어 (Rust): `^pub (fn|struct|enum)`
- 명시적 export 없는 언어 (Python): `__all__` 리스트 확인 또는 `_` 접두사 없는 top-level `^(def|class) [a-zA-Z]`

## Result Template

```markdown
# 검증 결과: {directory}

## 요약

- 전체 이슈: {N}개
- Structure: {n1}개
- Exports: {n2}개
- Dependencies: {n3}개
- Behavior: {n4}개

## IMPLEMENTS.md Presence

- 상태: {EXISTS | MISSING}

## Export 커버리지

- 커버리지: {coverage}%
- 전체: {total_exports}개, 발견: {found_count}개, 누락(STALE): {stale_count}개

| 점수 범위 | 해석 |
|----------|------|
| 90-100% | 우수 - CLAUDE.md exports가 코드와 일치 |
| 70-89% | 양호 - 일부 export 보완 필요 |
| 50-69% | 보통 - 주요 export 누락 |
| 0-49% | 미흡 - CLAUDE.md 재작성 권장 |

## 상세

### Structure Drift

#### UNCOVERED (문서에 없는 파일)
- `helper.ts`: 디렉토리에 존재하나 Structure에 없음

#### ORPHAN (실제 없는 파일)
- `legacy.ts`: Structure에 있으나 실제로 존재하지 않음

### Exports Drift

#### STALE (코드에 없는 export)
- `formatDate(date: Date): string`: 문서에 있으나 코드에 없음

#### MISSING (문서에 없는 export)
- `parseNumber`: 코드에 있으나 문서에 없음

#### MISMATCH (시그니처 불일치)
- `validateToken`:
  - 문서: `validateToken(token: string): boolean`
  - 실제: `validateToken(token: string, options?: ValidateOptions): Promise<boolean>`

### Dependencies Drift

#### STALE (없는 의존성)
- `lodash`: package.json에 없음

### Behavior Drift

#### MISMATCH (동작 불일치)
- "빈 입력 시 빈 배열 반환": 실제로는 null 반환
```

## CLI Output JSON Structures

### parse-claude-md 출력

```json
{
  "name": "auth",
  "purpose": "User authentication module",
  "exports": {
    "functions": [{"name": "validateToken", "signature": "validateToken(token: string): Promise<Claims>", "is_async": true}],
    "types": [{"name": "Claims", "definition": "Claims { userId: string, role: Role }", "kind": "interface"}],
    "classes": [{"name": "TokenManager", "constructor_signature": "TokenManager(secret: string)"}],
    "enums": [],
    "variables": []
  },
  "dependencies": {
    "external": ["jsonwebtoken@9.0.0"],
    "internal": ["./types"]
  },
  "behaviors": [
    {"input": "valid JWT token", "output": "Claims object", "category": "success"},
    {"input": "expired token", "output": "TokenExpiredError", "category": "error"}
  ],
  "contracts": [{"function_name": "validateToken", "preconditions": [...], "postconditions": [...], "throws": [...], "invariants": []}],
  "protocol": {"states": [...], "transitions": [...], "lifecycle": [...]},
  "structure": {"subdirs": [{"name": "jwt", "description": "..."}], "files": [{"name": "types.ts", "description": "..."}]},
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
