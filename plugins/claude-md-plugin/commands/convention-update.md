---
name: convention-update
description: |
  CLAUDE.md 내 Convention 섹션(Project Convention, Code Convention)을 업데이트합니다.
  인자가 있으면 직접 반영하고, 없으면 대화형으로 수정 사항을 수집합니다.
argument-hint: "[업데이트 내용]"
allowed-tools: [Bash, Read, Write, Glob, AskUserQuestion]
---

# /convention-update

CLAUDE.md 내 Convention 섹션을 업데이트합니다.

## Triggers

- `/convention-update`
- `컨벤션 업데이트`
- `컨벤션 수정`

## Arguments

| 이름 | 필수 | 기본값 | 설명 |
|------|------|--------|------|
| `업데이트 내용` | 아니오 | - | 반영할 변경 지시사항 (없으면 대화형) |

## Workflow

### 1. Convention 섹션 존재 확인

project_root CLAUDE.md와 module_root CLAUDE.md에서 Convention 섹션을 찾습니다:

프로젝트 루트를 탐지한 후 `{project_root}/CLAUDE.md`를 Read합니다. `## Project Convention` 섹션과 `## Code Convention` 섹션의 존재 여부를 확인합니다.

**섹션이 없는 경우:**

```
Convention 섹션이 CLAUDE.md에 존재하지 않습니다.
먼저 `/project-setup`을 실행하여 Convention 섹션을 추가해주세요.
```

메시지를 출력하고 종료합니다.

### 2. 대상 섹션 결정

#### 2-A. 인자가 있는 경우: 내용 분석으로 자동 판별

인자의 내용을 분석하여 대상 섹션을 자동으로 결정합니다:

| 키워드/내용 | 대상 섹션 |
|------------|-----------|
| 디렉토리 구조, 모듈 경계, 레이어링, 의존성 방향 | `## Project Convention` |
| 코드 스타일, 네이밍 규칙, 들여쓰기, 포맷팅, import 순서 | `## Code Convention` |
| 둘 다 해당하는 경우 | 두 섹션 모두 업데이트 |

#### 2-B. 인자가 없는 경우: 대화형 선택

AskUserQuestion으로 수정 대상을 질문합니다:
- **Project Convention**: 프로젝트 구조, 모듈 경계, 네이밍 규칙
- **Code Convention**: 코드 스타일, 네이밍 규칙, 타입 시스템
- **둘 다**: 두 섹션 모두 수정

### 3. 변경 사항 수집

#### 3-A. 인자가 있는 경우: 직접 적용

1. 대상 CLAUDE.md를 Read로 읽기
2. 해당 Convention 섹션 추출
3. 인자 내용을 변경 지시사항으로 해석하여 섹션 내용 수정
4. 수정된 섹션을 CLAUDE.md에 다시 Write

대상 CLAUDE.md를 Read하고, 인자의 변경 지시사항에 따라 해당 Convention 섹션을 수정한 후, 전체 파일을 Write합니다. 예: "들여쓰기를 4 spaces로 변경" → Code Convention의 Code Style 서브섹션에서 들여쓰기 규칙을 업데이트합니다.

#### 3-B. 인자가 없는 경우: 대화형 수집

1. 현재 섹션 내용을 사용자에게 표시
2. 수정할 서브섹션 선택

AskUserQuestion으로 수정할 서브섹션을 질문합니다. 옵션은 대상 섹션의 현재 서브섹션 목록으로 동적 생성합니다 (각 옵션에 현재 내용 요약 포함). "새 서브섹션 추가" 옵션도 포함합니다.

3. 선택된 서브섹션에 대해 수정 내용을 수집하고 적용

### 4. CLAUDE.md 섹션 업데이트

1. 기존 CLAUDE.md를 Read합니다.
2. 대상 섹션을 추출하고 수정 내용을 적용합니다.
3. 수정된 섹션으로 교체한 후 전체 CLAUDE.md를 Write합니다.

### 4.5 DRY 준수 확인 (module_root 업데이트 시)

module_root Convention을 업데이트한 경우:
1. 업데이트 후 내용을 project_root 동일 섹션과 비교
2. 동일해진 경우 → AskUserQuestion: "project_root와 동일해졌습니다. 제거하고 상속으로 전환할까요?"
   - **제거**: module_root에서 해당 섹션 삭제 (project_root에서 상속)
   - **유지**: 명시적으로 유지
3. 여전히 다른 경우 → 그대로 유지

### 5. 검증

`claude-md-core validate-convention` CLI를 실행하여 수정된 섹션을 검증합니다:

```bash
CORE_DIR="${CLAUDE_PLUGIN_ROOT}/core"
CLI_PATH="$CORE_DIR/target/release/claude-md-core"
$CLI_PATH validate-convention --project-root {project_root}
```

검증 실패 시 에러 내용을 표시하고, 수동 수정을 안내합니다.

### 6. 결과 보고

변경 완료 후 다음 내용을 사용자에게 안내합니다:

```
Convention 섹션이 업데이트되었습니다.

변경된 파일: {변경된 CLAUDE.md 경로}
변경된 섹션:
  - ## {섹션명} > ### {서브섹션명}: {변경 요약}

다음 `/compile` 실행 시 REFACTOR 단계에서 업데이트된 규칙이 반영됩니다.
```

## 오류 처리

| 상황 | 대응 |
|------|------|
| Convention 섹션 없음 | `/project-setup` 실행 안내 |
| 프로젝트 루트 탐지 실패 | 사용자에게 경로 입력 요청 |
| 파일 쓰기 실패 | 에러 메시지 출력 |
| validate-convention 실패 | 에러 내용 표시, 수동 수정 안내 |
