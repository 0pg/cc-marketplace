---
name: draft-generate
description: (internal) 분석 데이터로부터 CLAUDE.md 초안 생성
allowed-tools: [Read, Write]
---

# Draft Generate Skill

## 목적

코드 분석 결과와 템플릿을 사용하여 CLAUDE.md 초안을 생성합니다.

## 입력

```
analysis_file: code-analyze 결과 파일 경로
child_claude_mds: 자식 디렉토리의 CLAUDE.md 경로 목록 (선택)
output_name: 출력 파일명 (디렉토리명 기반)
user_answers: 사용자 질문 응답 (선택)
```

## 출력

`.claude/extract-results/{output_name}-draft.md` 파일 생성

## 워크플로우

### 1. 분석 데이터 로드

```python
analysis = read_json(analysis_file)
template = read_file("plugins/claude-md-plugin/templates/claude-md-schema.md")
```

### 2. 자식 CLAUDE.md Purpose 추출

```python
child_purposes = {}
for child_path in child_claude_mds:
    if file_exists(child_path):
        content = read_file(child_path)
        purpose = extract_section(content, "Purpose")
        child_purposes[get_dirname(child_path)] = purpose
```

### 3. CLAUDE.md 생성

템플릿 스키마를 따라 생성:

```markdown
# {디렉토리명}

## Purpose
{analysis.purpose 또는 user_answers.purpose}

## Structure
{하위 디렉토리 및 파일 목록}
- subdir/: {child_purposes[subdir]} (상세는 subdir/CLAUDE.md 참조)
- file.ext: {역할}

## Exports

### Functions
{analysis.exports.functions를 시그니처 형태로}
- `{signature}`: {description}

### Types
{analysis.exports.types를 정의 형태로}
- `{definition}`: {description}

## Dependencies
- external: {외부 패키지}
- internal: {내부 모듈}

## Behavior

### 정상 케이스
{category가 success인 behaviors}
- {input} → {output}

### 에러 케이스
{category가 error인 behaviors}
- {input} → {output}

## Constraints
{user_answers.constraints 또는 코드에서 추론된 제약}
```

### 4. 파일 저장

```python
write_file(".claude/extract-results/{output_name}-draft.md", generated_content)
```

## 결과 반환

```
---draft-generate-result---
output_file: .claude/extract-results/{output_name}-draft.md
status: success
sections: [Purpose, Structure, Exports, Dependencies, Behavior]
child_references: {참조된 자식 CLAUDE.md 수}
---end-draft-generate-result---
```

## 생성 규칙

### Purpose 섹션
- 1-2문장으로 디렉토리의 핵심 책임 명시
- 분석 결과에서 추론하거나 사용자 응답 사용

### Structure 섹션
- 하위 디렉토리: `{name}/: {purpose} (상세는 {name}/CLAUDE.md 참조)`
- 직접 파일: `{name}: {역할}`

### Exports 섹션
- 함수: `{signature}`: {description}
- 타입: `{definition}`: {description}
- 언어별 관용 표현 유지

### Behavior 섹션
- 시나리오 형태: `{input} → {output}`
- 정상 케이스와 에러 케이스 구분

## 참조 규칙 준수

**허용**:
- 자식 참조: `auth/jwt/CLAUDE.md 참조`

**금지** (생성하지 않음):
- 부모 참조: `../`
- 형제 참조: `../sibling/`

## 오류 처리

| 상황 | 대응 |
|------|------|
| 분석 파일 없음 | 에러 반환 |
| 자식 CLAUDE.md 없음 | 해당 참조 스킵 |
| Purpose 추론 실패 | "[TODO: Purpose 작성 필요]" 표시 |
