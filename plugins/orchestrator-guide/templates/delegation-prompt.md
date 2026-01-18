# Delegation Prompt Template

> 5요소 위임 프로토콜 기반 역할별 위임 프롬프트 템플릿

**Note:** 위임 전 작업이 충분히 작은 단위인지 확인하세요. (단일 목표, 예측 가능한 파일 범위, 객관적 성공 기준)

---

## 기본 템플릿

```markdown
## GOAL
{goal_description}

- task.md 참조: {task_ref}
- 구체적 목표:
  - {specific_goal_1}
  - {specific_goal_2}

## CONTEXT
### 관련 파일
- {file_path_1}: {description_1}
- {file_path_2}: {description_2}

### 패턴 및 컨벤션
- {pattern_1}
- {pattern_2}

### notes.md 참고
- {note_1}
- {note_2}

## CONSTRAINTS
### 금지 사항
- {constraint_1}
- {constraint_2}

### 주의 사항
- {caution_1}
- {caution_2}

## SUCCESS
### 필수 검증
- {verification_command_1}
- {verification_command_2}

### 기능 검증
- {functional_check_1}
- {functional_check_2}

## HANDOFF
### 성공 시
- 다음 역할: {next_role}
- 전달 정보: {handoff_info}

### 실패 시
- 보고 대상: {escalation_target}
- 필요 정보: {required_info}
```

---

## 역할별 템플릿

### Implementation Role (구현 역할) 템플릿

```markdown
## GOAL
{target_module}에 {feature_name}을(를) 구현합니다.

- task.md 참조: {task_ref}
- 구현 항목:
  - {implementation_item_1}
  - {implementation_item_2}

## CONTEXT
### 관련 파일
- {module_path}/{entry_file}: 모듈 등록 패턴
- {module_path}/{related_file}: 기존 패턴 참고
- {module_path}/{error_file}: 에러 타입 정의 (있는 경우)

### 패턴 및 컨벤션
(project-config 또는 기존 코드에서 패턴 참조)
- 프로젝트 프레임워크 사용
- 프로젝트 에러 처리 방식 준수
- 코딩 컨벤션 준수

### notes.md 참고
{notes_content}

## CONSTRAINTS
### 금지 사항
- 프로젝트에서 금지하는 패턴 사용 금지 (project-config 참조)
- 기존 API 시그니처 변경 금지
- 범위 외 모듈 수정 금지
- {project_specific_constraint}

### 주의 사항
- 프로젝트 컨벤션 준수
- 프로젝트 에러 처리 패턴 준수
- 적절한 로깅 포함

## SUCCESS
### 필수 검증
(project-config의 검증 명령어 참조)
- lint 명령어 경고 없음
- test 명령어 모든 테스트 통과

### 기능 검증
- {api_endpoint_1} 동작 확인
- {api_endpoint_2} 동작 확인
- 에러 케이스 처리 확인

## HANDOFF
### 성공 시
- 다음 역할: Review (코드 리뷰)
- 전달 정보: 수정된 파일 목록, 주요 결정 사항

### 실패 시
- ARB status: failed
- issues에 실패 원인 상세 기록
- 오케스트레이터에 에스컬레이션
```

### Exploration Role (탐색 역할) 템플릿

```markdown
## GOAL
{exploration_target}을(를) 탐색하고 분석합니다.

- 탐색 목적: {purpose}
- 찾아야 할 정보:
  - {info_1}
  - {info_2}

## CONTEXT
### 탐색 범위
- {target_module}/
- 특히: {specific_path}

### 찾아야 할 패턴
- {pattern_to_find_1}
- {pattern_to_find_2}

## CONSTRAINTS
### 금지 사항
- 코드 수정 금지 (읽기 전용)
- 범위 외 탐색 금지

### 주의 사항
- 발견 사항 구조화하여 보고
- 관련 파일 경로 정확히 기록

## SUCCESS
### 완료 기준
- 필요한 정보 모두 수집
- 분석 결과 구조화
- 다음 단계 제안 포함

### 결과물
- 파일 목록 및 역할
- 패턴 분석 결과
- 권장 접근 방식

## HANDOFF
### 성공 시
- 탐색 결과를 오케스트레이터에 보고
- 다음 작업 제안

### 실패 시
- 찾지 못한 정보 명시
- 대안 탐색 방법 제안
```

### Review Role (리뷰 역할) 템플릿

```markdown
## GOAL
{target_module}의 최근 변경사항을 리뷰합니다.

- 리뷰 대상: {review_target}
- 리뷰 범위:
  - {file_1}
  - {file_2}

## CONTEXT
### 변경 내용
- {change_summary}

### 이전 ARB
- 에이전트: {previous_agent}
- 상태: {previous_status}

### 프로젝트 컨벤션
(project-config 또는 CLAUDE.md에서 참조)
- 프로젝트 언어/프레임워크 특성 준수
- 프로젝트 에러 처리 방식 확인
- 프로젝트 코딩 스타일 준수

## CONSTRAINTS
### 리뷰 범위
- 지정된 파일만 리뷰
- 구현 변경 금지 (리뷰만)

### 리뷰 기준
- 코드 품질
- 보안 취약점
- 성능 이슈
- DRY 원칙

## SUCCESS
### 리뷰 완료 기준
- 모든 파일 검토
- 이슈 분류 (critical/high/medium/low)
- 개선 제안 포함

### 통과 기준
- critical 이슈 0개
- high 이슈 0개 (또는 허용된 예외)

## HANDOFF
### 통과 시
- ARB status: success
- 다음 단계 진행 승인

### 이슈 발견 시
- ARB status: partial 또는 failed
- 이슈 목록과 수정 가이드
- 구현 역할 에이전트에 재작업 요청
```

---

## 사용 예시

```typescript
// 역할에 맞는 에이전트 선택 (project-config 참조)
Task({
  subagent_type: "{implementation_agent}",  // project-config에서 해당 역할의 에이전트명
  prompt: `
## GOAL
{goal_description}

- task.md 참조: {task_ref}
- 구현 항목:
  - {implementation_item_1}
  - {implementation_item_2}

## CONTEXT
### 관련 파일
- {module}/{path_1}: {description_1}
- {module}/{path_2}: {description_2}

### 패턴 및 컨벤션
- {pattern_1}
- {pattern_2}

### notes.md 참고
- {notes_content}

## CONSTRAINTS
### 금지 사항
- {constraint_1}
- {constraint_2}

## SUCCESS
### 필수 검증
(project-config의 검증 명령어 사용)
- {verification_1}
- {verification_2}

### 기능 검증
- {functional_check_1}
- {functional_check_2}

## HANDOFF
### 성공 시
- 다음 역할: {next_role}

### 실패 시
- ARB status: failed
- issues에 원인 기록
`
})
```

---

## 역할 → 에이전트 매핑

역할별로 어떤 에이전트를 사용할지는 프로젝트 설정(project-config 또는 CLAUDE.md)에서 정의합니다:

| 역할 | 설명 | 에이전트 예시 |
|------|------|-------------|
| **Implementation** | 코드 작성 | backend-impl, frontend-impl, general-rust-impl 등 |
| **Exploration** | 코드 분석 | explorer, code-explorer 등 |
| **Review** | 코드 검토 | rust-code-reviewer, code-reviewer 등 |

프로젝트 설정이 없으면 모델이 적절한 에이전트를 자율적으로 선택합니다.
