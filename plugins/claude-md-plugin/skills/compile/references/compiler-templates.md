# Compiler Templates

## Compile Session File Format

```markdown
# Compile Task: {path}
type: compile | target: {path} | language: {lang} | conflict: {mode}

## Origin
claude_md: {path}/CLAUDE.md
developers_md: {path}/DEVELOPERS.md
project_conventions: {project_root}/CLAUDE.md#Conventions

## Requirements (from CLAUDE.md)
{Requirements 섹션 전체}

## Constraints (from DEVELOPERS.md)
{Constraints 섹션 전체 — 테스트 생성 원천}

## Data Schemas (from DEVELOPERS.md, reference only)
{Data Schemas 섹션 — 타입 참조용, 테스트 생성 원천 아님}

## Technical Context
{Technical Context 섹션 전체}

## Conventions (resolved)
{계층 해소된 Conventions}

## Dependencies
{compile-context 또는 탐색 결과}

## Verification Contract
- All Constraints → corresponding tests exist
- All tests pass
- /validate --strict {path}
```

## Inline TDD Workflow

### RED Phase: Constraints → Tests

| Constraints 유형 | 테스트 패턴 |
|-----------------|------------|
| 수치 제한 (`최대 N`) | 경계값: N OK, N+1 실패 |
| 형식 제약 (`UTF-8만`) | 유효 입력 통과, 무효 입력 거부 |
| 보안 제약 (`secure storage`) | 보안 속성 검증 |
| 비즈니스 규칙 | 규칙 준수/위반 시나리오 |
| I/O 계약 (`f(a) → b`) | 입�� a에 대해 출력 b 검증 |

### GREEN Phase: Requirements → Implementation

| 세션 파일 요소 | 생성 대상 |
|---------------|----------|
| Requirements | 고수준 기능 구현 |
| Constraints (수치) | 상수 + 검증 로직 |
| Constraints (형식) | guard clause |
| Constraints (보안) | 보안 로직 |
| Technical Context | 구현 방식 (라이브러��, 패턴) |
| Domain Context | 상수 값, 도메인 규칙 |

### REFACTOR Phase: Conventions 적용

- Naming Rules → 변수/함수/클래스명 조정
- Coding Rules → 패턴 적용
- Project Structure → 파일 위치 조정
- 회귀 테스트 실행 → 실패 시 REFACTOR 롤백

## Result Format

```json
{
  "status": "success",
  "compiled_files": ["src/auth/index.ts", "src/auth/types.ts"],
  "test_files": ["src/auth/__tests__/auth.test.ts"],
  "skipped_files": [],
  "tests_passed": 8,
  "tests_failed": 0
}
```

## Error Handling

| 상황 | 대응 |
|------|------|
| 세션 파일 파싱 실패 | Agent 실패 반환 |
| RED 검증 실패 (일부 통과) | 기존 구현 커버리지로 기록 |
| GREEN 3회 실패 | partial 상태 반환 |
| REFACTOR 회귀 실패 | REFACTOR 롤백 |
| 파일 쓰기 실패 | 해당 파일 건너뛰기 |
