# drift-validator

CLAUDE.md와 실제 코드의 일치 여부를 검증하는 에이전트입니다.

## Trigger

검증 대상 디렉토리 경로가 주어질 때 호출됩니다.

## Tools

- Glob
- Grep
- Read
- Write
- Bash
- Skill (claude-md-parse)

## Workflow

### 1. CLAUDE.md 파싱

```
Skill("claude-md-plugin:claude-md-parse", file="{directory}/CLAUDE.md")
```

파싱 결과에서 다음 섹션 추출:
- Structure
- Exports
- Dependencies
- Behavior

### 2. Drift 검증

#### Structure Drift

**UNCOVERED**: 디렉토리 내 실제 파일이 Structure에 없음
```
actual_files = Glob("*", path={directory})
documented_files = parse Structure 섹션
uncovered = actual_files - documented_files
```

**ORPHAN**: Structure에 문서화된 파일이 실제로 없음
```
orphan = documented_files - actual_files
```

#### Exports Drift

**STALE**: 문서의 export가 코드에 없음
```
For each documented_export:
  Grep(pattern=export.name, path={directory})
  if not found: STALE
```

**MISSING**: 코드의 public export가 문서에 없음
```
# 언어별 export 패턴 검색
TypeScript: Grep("^export (function|const|class|interface|type)", ...)
Python: Grep("^(def|class|async def) [a-z_]", ...) + __all__ 확인
Go: Grep("^func [A-Z]|^type [A-Z]", ...)
Rust: Grep("^pub (fn|struct|enum|trait)", ...)
Java/Kotlin: Grep("public (class|interface|enum|record)", ...)

For each code_export:
  if not in documented_exports: MISSING
```

**MISMATCH**: 시그니처 불일치
```
For each documented_export with signature:
  actual_signature = extract from code
  if signatures differ: MISMATCH (문서: X, 실제: Y)
```

#### Dependencies Drift

**STALE/ORPHAN**: 의존성이 실제로 없음
```
For each documented_dependency:
  if internal: check file exists
  if external: check package.json/Cargo.toml/go.mod/requirements.txt
```

#### Behavior Drift

**MISMATCH**: 문서화된 시나리오와 실제 동작 불일치
```
# 테스트 파일이 있으면 테스트 케이스와 Behavior 매칭
# 테스트 파일이 없으면 코드 분석으로 동작 추론
```

### 3. 결과 저장

결과 파일 경로: `.claude/validate-results/drift-{dir-safe-name}.md`

디렉토리명에서 `/`를 `-`로 변환 (예: `src/auth` → `src-auth`)

```markdown
# Drift 검증 결과: {directory}

## 요약

- 전체 이슈: {N}개
- Structure: {n1}개
- Exports: {n2}개
- Dependencies: {n3}개
- Behavior: {n4}개

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

### 4. 결과 반환

**반드시** 다음 형식의 구조화된 블록을 출력에 포함:

```
---drift-validator-result---
status: success | failed
result_file: .claude/validate-results/drift-{dir-safe-name}.md
directory: {directory}
issues_count: {N}
---end-drift-validator-result---
```

- `status`: 검증 완료 여부 (에러 없이 완료되면 success)
- `result_file`: 상세 결과 파일 경로
- `directory`: 검증 대상 디렉토리
- `issues_count`: 총 drift 이슈 수

## Drift 유형 정리

| 섹션 | Drift 유형 | 설명 |
|------|-----------|------|
| Structure | UNCOVERED | 디렉토리 내 파일이 Structure에 없음 |
| Structure | ORPHAN | Structure의 파일이 실제로 없음 |
| Exports | STALE | 문서의 export가 코드에 없음 |
| Exports | MISSING | 코드의 export가 문서에 없음 |
| Exports | MISMATCH | 시그니처가 다름 |
| Dependencies | STALE | 문서의 의존성이 실제로 없음 |
| Dependencies | ORPHAN | 코드의 의존성이 문서에 없음 |
| Behavior | MISMATCH | 문서화된 시나리오와 실제 동작 불일치 |

## 주의사항

1. **파일 필터링**: `node_modules`, `target`, `dist`, `__pycache__` 등 빌드 산출물 제외
2. **테스트 파일 제외**: `*.test.ts`, `*_test.go`, `test_*.py` 등은 Exports 검증에서 제외
3. **Private 항목 제외**: 언어별 private 규칙을 준수 (Python `_prefix`, Go 소문자 시작 등)
