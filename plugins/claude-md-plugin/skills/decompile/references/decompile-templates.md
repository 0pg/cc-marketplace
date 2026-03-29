# Decompile Templates

## Decompile Session File Format

```markdown
# Decompile Task: {path}
type: decompile | target: {path}

## Tree Info
source_file_count: {n}
subdir_count: {n}
depth: {n}

## Children CLAUDE.md
{이미 생성된 자식 CLAUDE.md 경로 목록, 없으면 "None"}

## Project Conventions
{project root Conventions 또는 "None"}
```

## CLI Workflow

### Step 1: Boundary Resolution
```bash
$CLI_PATH resolve-boundary --dir {target_dir} --output ${TMP_DIR}decompile-boundary-{dir-safe}.json
```
결과: direct_files, subdirectories, reference_violations

### Step 2: Code Analysis
```bash
$CLI_PATH analyze-code --path {target_dir} --output ${TMP_DIR}decompile-analyze-{dir-safe}.json
```
결과: exports, dependencies, behaviors, contracts, protocol

### Step 3: Analysis Formatting
```bash
$CLI_PATH format-analysis --input ${TMP_DIR}decompile-analyze-{dir-safe}.json --output ${TMP_DIR}decompile-summary-{dir-safe}.md
```
결과: LLM-ready 요약 마크다운

### Step 4: Schema Validation
```bash
$CLI_PATH validate-schema --file {claude_md_path} --dir {target_dir}
```
실패 시:
```bash
$CLI_PATH fix-schema --file {claude_md_path}
```

## Document Generation Rules

### CLAUDE.md (Primary SSOT)

- **Purpose**: 코드의 존재 이유를 비즈니스 가치 관점에서 서술. "None" 불가.
- **Requirements**: 코드가 충족하는 요구사항을 사용자 관점으로 역추출. 정말 없으면 "None".
- **Domain Context**: 코드에서 유추되는 비즈니스 제약. 유추 불가하면 "None" 또는 AskUserQuestion.

### DEVELOPERS.md (Derived Spec)

- **Constraints**: 코드의 입출력 계약을 정밀하게 기술 (테스트 변환 가능하도록)
- **Technical Context**: 사용된 기술과 그 이유
- **Decision Log**: 코드에서 유추되는 설계 결정 (선택적)
- **Operations**: 배포/모니터링 관련 (선택적)

### Smart Merge (기존 CLAUDE.md가 있을 때)

1. Purpose: 기존 유지 (사람이 작성한 것이 더 정확)
2. Requirements: 기존 보존 + 코드에서 발견된 미문서화 항목 추가
3. Domain Context: 기존 보존 + 보충

### INV-1 준수

```
node.dependencies ⊆ node.children
```
자식 CLAUDE.md 목록을 참조하여 의존성이 children 범위 내인지 확인.

## Result Format

```
---decompiler-result---
status: success | failed_with_warnings
target_dir: {path}
validation: passed | failed_with_warnings
developers_md: generated | skipped
---end-decompiler-result---
```
