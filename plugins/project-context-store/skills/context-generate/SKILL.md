---
name: context-generate
description: |
  소스 코드 진입점을 탐지하여 각 진입점마다 context-generator 에이전트를 실행합니다.
  test/, scripts/, docs/ 등은 자동으로 제외되고 실제 소스 디렉토리만 처리합니다.
trigger:
  - /context-generate
  - 컨텍스트 생성해줘
  - CLAUDE.md 생성
  - 프로젝트 문서화
tools:
  - Read
  - Glob
  - Task
  - AskUserQuestion
---

# Context Generate Skill

## 목적

프로젝트의 모든 소스 코드 디렉토리에 CLAUDE.md를 생성하여 코드 재현성을 보장합니다.

## 워크플로우

### 1. 소스 코드 진입점 탐지

프로젝트 루트에서 소스 코드 진입점을 탐지합니다.

**진입점 판정 기준** (우선순위 순):

1. **표준 소스 디렉토리**: `src/`, `lib/`, `app/`, `pkg/`, `internal/`, `cmd/`
2. **언어별 컨벤션**:
   - Rust: `src/` (Cargo.toml 존재 시)
   - Go: `cmd/`, `pkg/`, `internal/`
   - Python: 패키지 디렉토리 (`__init__.py` 존재)
   - Node.js: `src/`, `lib/` (package.json 존재 시)
   - JVM (Java/Kotlin/Scala):
     - Maven/Gradle: `src/main/java/`, `src/main/kotlin/`, `src/main/scala/`
     - 단순 구조: `src/`
     - 멀티모듈: 각 모듈의 `src/main/` 하위
3. **폴백**: 루트에 소스 파일이 직접 있으면 루트 자체

**탐지 절차:**
```
1. Glob으로 표준 디렉토리 존재 확인
2. 각 후보 디렉토리에 소스 파일 존재 확인
3. 탐지된 진입점 목록 생성
```

**자동 제외 (진입점으로 선택 안됨):**
- test/, tests/, spec/, __tests__/
- scripts/, tools/, bin/
- docs/, documentation/
- examples/, samples/
- fixtures/, mocks/

### 2. 각 진입점마다 Agent 실행

탐지된 각 진입점에 대해 병렬로 Task 생성:

```python
for entry_point in detected_entry_points:
    Task(
        subagent_type="project-context-store:context-generator",
        prompt=f"대상: {entry_point}",
        description=f"Generate context for {entry_point}"
    )
```

**최종 결과**: 소스 코드가 있는 **모든** 하위 디렉터리에 각각 CLAUDE.md가 생성됩니다.
예시:
- src/CLAUDE.md
- src/auth/CLAUDE.md
- src/auth/jwt/CLAUDE.md
- src/api/CLAUDE.md
- lib/CLAUDE.md

각 CLAUDE.md는 해당 디렉터리의 직접 파일만 담당합니다.

### 3. 결과 취합 및 보고

모든 Agent가 완료되면 결과를 요약합니다:

```
=== Context Generation Report ===

탐지된 진입점:
  - src/
  - lib/

생성된 CLAUDE.md 파일들:
  - src/CLAUDE.md
  - src/auth/CLAUDE.md
  - src/api/CLAUDE.md
  - lib/CLAUDE.md

사용자 질문: N개 응답됨
```

## 참고: Agent 동작

다음은 context-generator Agent가 담당합니다 (Skill이 하는 일 아님):
- 재귀적 하위 디렉토리 탐색 및 Task 생성
- 코드 분석 및 컨텍스트 추출
- 사용자 질문 (불명확한 부분)
- 기존 CLAUDE.md 처리 (병합/덮어쓰기)

상세 내용은 `agents/context-generator.md` 참조.
