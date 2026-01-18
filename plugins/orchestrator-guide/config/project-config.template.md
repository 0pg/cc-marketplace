# Project Configuration Template

> 프로젝트별 설정 템플릿. 이 파일을 프로젝트의 CLAUDE.md에 통합하거나 별도 참조합니다.

---

## Roles (역할 매핑)

프로젝트에서 사용하는 역할별 에이전트 이름을 정의합니다.
오케스트레이터는 역할을 참조하고, 실제 에이전트 선택은 이 매핑을 따릅니다.

```yaml
roles:
  # 구현 역할 - 코드를 작성하는 에이전트
  implementation:
    backend: "{agent_name}"      # 백엔드 구현 에이전트
    frontend: "{agent_name}"     # 프론트엔드 구현 에이전트
    general: "{agent_name}"      # 범용 구현 에이전트

  # 탐색 역할 - 코드베이스를 분석하는 에이전트
  exploration:
    codebase: "{agent_name}"     # 코드베이스 탐색 에이전트

  # 리뷰 역할 - 코드를 검토하는 에이전트
  review:
    code: "{agent_name}"         # 코드 리뷰 에이전트
```

---

## Modules (모듈 구조)

프로젝트의 모듈 구조와 spec 파일 위치를 정의합니다.

```yaml
modules:
  # 모듈 경로 패턴
  path_pattern: "{path_pattern}"  # 예: "crates/{module}/", "packages/{module}/"

  # spec 파일 위치
  spec_location: "{relative_path}"  # 예: "spec/task.md", "docs/spec.md"

  # 모듈 목록 (선택)
  list:
    - name: "{module_name}"
      path: "{module_path}"
      spec: "{spec_path}"
```

---

## Verification (검증 명령어)

프로젝트의 검증 명령어를 정의합니다.
모델이 이 명령어들을 참조하여 검증을 수행합니다.

```yaml
verification:
  # 정적 분석 (선택)
  lint:
    command: "{lint_command}"       # 예: "cargo clippy -p {module}", "npm run lint"
    flags: "{additional_flags}"     # 예: "-- -D warnings"

  # 테스트 (선택)
  test:
    command: "{test_command}"       # 예: "cargo test -p {module}", "npm test"
    flags: "{additional_flags}"

  # 빌드 (선택)
  build:
    command: "{build_command}"      # 예: "cargo build -p {module}", "npm run build"
    flags: "{additional_flags}"
```

---

## Patterns (프로젝트 패턴)

프로젝트에서 따르는 패턴과 컨벤션을 정의합니다.

```yaml
patterns:
  # 에러 처리 방식
  error_handling: "{description}"   # 프로젝트의 에러 처리 패턴

  # 비동기 처리 (해당 시)
  async_pattern: "{description}"    # 비동기 처리 방식

  # 코딩 스타일
  style: "{description}"            # 프로젝트 코딩 스타일

  # 기타 컨벤션
  conventions:
    - "{convention_1}"              # 프로젝트별 컨벤션
    - "{convention_2}"
```

> 예시는 프로젝트 언어/프레임워크에 맞게 작성합니다.

---

## Usage

이 템플릿을 프로젝트에 적용하는 방법:

### Option 1: CLAUDE.md에 직접 통합

```markdown
# Project Configuration

## Roles
- backend 구현: {backend_agent}
- frontend 구현: {frontend_agent}
- 코드 리뷰: {review_agent}
- 탐색: {exploration_agent}

## Modules
- 경로: {module_path_pattern}
- spec: {spec_path}

## Verification
- lint: {lint_command}
- test: {test_command}
```

### Option 2: 별도 파일로 참조

```markdown
# CLAUDE.md

Project config: .claude/project-config.md
```

---

## Notes

- 이 템플릿의 필드들은 **선택적**입니다
- 프로젝트에 맞게 필요한 필드만 정의하세요
- 오케스트레이터는 정의되지 않은 필드에 대해 기본 행동을 사용합니다
- 역할 매핑이 없으면 모델이 적절한 에이전트를 자율적으로 선택합니다
