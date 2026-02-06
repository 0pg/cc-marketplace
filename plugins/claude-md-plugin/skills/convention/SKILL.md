---
name: convention
version: 1.0.0
aliases: [style, codestyle, code-convention]
trigger:
  - /convention
  - 컨벤션 업데이트
  - 코드 스타일
  - update convention
description: |
  This skill should be used when the user asks to "update code convention", "change coding style",
  "re-analyze code style", or uses "/convention".
  Views or updates the project's code-convention.md file.
allowed-tools: [Read, Glob, Grep, Write, AskUserQuestion]
---

# /convention

<example>
<context>
사용자가 현재 프로젝트의 컨벤션을 확인하려고 합니다.
</context>
<user>/convention</user>
<assistant_response>
## Current Code Convention

파일: /path/to/project/code-convention.md

### Naming
- Variables: camelCase
- Functions: camelCase (verb prefix)
- Types: PascalCase
- Constants: UPPER_SNAKE_CASE
- Files: kebab-case

### Formatting
- Indentation: 2 spaces
- Quotes: single
- Semicolons: false

수정이 필요하면 알려주세요.
</assistant_response>
</example>

<example>
<context>
사용자가 컨벤션을 재분석하려고 합니다.
</context>
<user>/convention --analyze</user>
<assistant_response>
소스 파일 분석 중...

38개 파일 분석 완료.

## 변경 감지

| 항목 | 기존 | 분석 결과 |
|------|------|----------|
| Quotes | single | double (78%) |
| Semicolons | false | true (85%) |

code-convention.md를 업데이트할까요?
</assistant_response>
</example>

<example>
<context>
code-convention.md가 없는 프로젝트.
</context>
<user>/convention</user>
<assistant_response>
code-convention.md 파일이 없습니다.

`/project-setup`을 실행하면 기존 코드를 분석하여 자동으로 생성합니다.
또는 직접 생성할 내용을 알려주세요.
</assistant_response>
</example>

## 목적

프로젝트의 `code-convention.md` 파일을 확인하거나 업데이트합니다.

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `--analyze` | 아니오 | - | 기존 코드를 재분석하여 컨벤션 업데이트 |

## 워크플로우

### Mode 1: 조회 (옵션 없음)

```
/convention
    │
    ├─ Read(code-convention.md)
    │   ├─ 있음 → 내용 표시
    │   └─ 없음 → /project-setup 안내
    │
    └─ 수정 요청 대기
```

1. 프로젝트 루트에서 `code-convention.md`를 읽습니다
2. 파일이 있으면 현재 내용을 보기 좋게 표시합니다
3. 파일이 없으면 `/project-setup` 실행을 안내합니다
4. 사용자가 수정을 요청하면 해당 섹션만 업데이트합니다

### Mode 2: 재분석 (--analyze)

```
/convention --analyze
    │
    ├─ 기존 code-convention.md 읽기 (있으면)
    │
    ├─ 소스 파일 수집 및 패턴 분석
    │   └─ /project-setup Phase 4와 동일한 분석 로직
    │
    ├─ 변경 사항 비교 (기존 vs 분석 결과)
    │   └─ 달라진 항목만 하이라이트
    │
    ├─ AskUserQuestion으로 확인
    │
    └─ code-convention.md 업데이트
```

1. 기존 `code-convention.md`가 있으면 읽어서 현재 상태 파악
2. `/project-setup`의 Phase 4(컨벤션 분석)와 동일한 로직으로 소스 코드 재분석
3. 기존 값과 분석 결과를 비교하여 달라진 항목만 표시
4. 사용자 확인 후 `code-convention.md` 업데이트

### 수동 수정 지원

사용자가 직접 특정 규칙을 변경하고 싶을 때:

```
사용자: "들여쓰기를 4 spaces로 변경해줘"
    │
    ├─ Read(code-convention.md)
    ├─ Formatting.indentation = "4 spaces"로 변경
    └─ Write(code-convention.md)
```

## 출력 형식

### 조회 시

```
## Current Code Convention

파일: {path}/code-convention.md

### Naming
- Variables: {pattern}
- Functions: {pattern}
- Types: {pattern}
- Constants: {pattern}
- Files: {pattern}

### Formatting
- Indentation: {value}
- Line length: {value}
- Quotes: {value}
- Semicolons: {value}

수정이 필요하면 알려주세요.
```

### 재분석 후 변경 감지 시

```
## Convention Analysis

{N}개 파일 분석 완료.

### 변경 감지

| 항목 | 기존 | 분석 결과 |
|------|------|----------|
| {changed_item} | {old} | {new} ({percentage}%) |

code-convention.md를 업데이트할까요?
```

### 변경 없을 때

```
{N}개 파일 분석 완료.

코드 스타일이 code-convention.md와 일치합니다. 변경 사항이 없습니다.
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| code-convention.md 없음 (조회) | `/project-setup` 실행 안내 |
| 소스 파일 없음 (재분석) | 분석 불가 안내, 수동 편집 제안 |
| 쓰기 실패 | 에러 메시지 출력, 수동 저장 안내 |

## Related Skills

- `/project-setup`: 빌드/테스트 커맨드 감지와 함께 code-convention.md 최초 생성 시 사용
