---
name: dev
description: |
  사용자의 자연어 요청을 분석하여 /impl 또는 /bugfix로 자동 라우팅합니다.
  기능 요청이면 /impl, 버그 수정이면 /bugfix로 전달합니다.
argument-hint: "<자연어 요청>"
allowed-tools: [Skill, AskUserQuestion]
---

# /dev

사용자의 자연어 요청을 분석하여 적절한 스킬(`/impl` 또는 `/bugfix`)로 자동 라우팅합니다.

## Arguments

| 이름 | 필수 | 설명 |
|------|------|------|
| `request` | 예 | 자연어 요청 텍스트 |

## 라우팅 로직

사용자의 요청 텍스트를 분석하여 아래 3개 Route 중 하나로 분류합니다.

### Route A → /impl (다음 신호 중 하나라도 해당)

- 새 기능 요청: "~기능이 필요합니다", "~모듈 추가", "~을 만들어주세요"
- 요구사항/스펙 정의: "요구사항 정의", "스펙 작성", "인터페이스 설계"
- 행동 정의: "~해야 합니다", "~를 지원해야"
- 모듈/컴포넌트 생성: "새 모듈", "컴포넌트 추가"

### Route B → /bugfix (다음 신호 중 하나라도 해당)

- 에러/예외 메시지: "TypeError:", "Error:", stack trace 포함
- 버그 키워드: "에러", "버그", "오류", "error", "bug", "fix"
- 테스트 실패: "테스트 실패", "test fail"
- 잘못된 동작: "~가 안 됩니다", "~이 깨졌습니다"

### Route C → AskUserQuestion (모호한 경우)

- 두 Route 신호가 혼재하거나 판별 불가한 경우
- 옵션: "새 기능/스펙 정의 (/impl)" vs "버그 수정/에러 진단 (/bugfix)"

## Workflow

### 1. 요청 분석

`$ARGUMENTS`에서 요청 텍스트를 추출합니다. 요청이 비어있으면 AskUserQuestion으로 요청 내용을 질문합니다.

### 2. Route 판별

요청 텍스트에서 Route A, B 신호를 탐지합니다. 한쪽 신호만 존재하면 해당 Route로 진행합니다. 양쪽 모두 존재하거나 어느 쪽도 해당하지 않으면 Route C로 진행합니다.

### 3. 스킬 호출

#### Route A → /impl

원본 요청 텍스트를 그대로 전달합니다:

```
Skill("impl", args: "{request}")
```

#### Route B → /bugfix

요청에서 인자를 추출하여 구성합니다:

- 에러 메시지 포함 시: `--error "{extracted_error}"`
- 테스트 이름 포함 시: `--test "{extracted_test}"`
- 경로 명시 시: path 인자로 전달
- 추출 불가 시: `--error "{전체 요청}"` 로 전달

```
Skill("bugfix", args: "{constructed_args}")
```

#### Route C → AskUserQuestion

```
AskUserQuestion:
  question: "요청의 의도를 확인합니다. 어떤 작업을 원하시나요?"
  options:
    - label: "새 기능/스펙 정의 (/impl)"
      description: "새로운 기능 요구사항을 정의하고 CLAUDE.md를 생성합니다"
    - label: "버그 수정/에러 진단 (/bugfix)"
      description: "런타임 에러나 버그를 3계층 추적으로 진단하고 수정합니다"
```

사용자 선택에 따라 Route A 또는 Route B로 진행합니다.
