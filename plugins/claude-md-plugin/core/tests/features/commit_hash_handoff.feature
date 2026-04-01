Feature: impl → compile Commit Hash Handoff
  impl이 커밋 메시지 컨벤션으로 변경 맥락을 기록하고,
  compile이 이를 자동 탐색하여 incremental compile을 수행한다.

  Background:
    Given CLI가 설치되어 있다
    And git 저장소가 초기화되어 있다

  # --- 커밋 메시지 컨벤션 ---

  Scenario: impl 커밋 메시지가 컨벤션을 따른다
    When impl이 "src/auth"에 대해 커밋을 생성한다
    Then 커밋 메시지가 "impl(src/auth):" 로 시작한다
    And 커밋 메시지 body에 "Changes:" 섹션이 있다

  Scenario: BREAKING 태그가 필요할 때 포함된다
    Given Requirements 삭제가 포함된 impl 변경이 있다
    When impl이 커밋을 생성한다
    Then 커밋 메시지에 "[BREAKING]"이 포함된다

  Scenario: 전환 맥락이 커밋 메시지에 포함된다
    When impl이 기존 기능을 수정하는 커밋을 생성한다
    Then 커밋 메시지 body 첫 단락에 전환 맥락이 있다
    And 전환 맥락이 변경의 방향을 기술한다

  # --- compile의 impl 커밋 탐색 ---

  Scenario: compile이 마지막 compile 이후 impl 커밋을 탐색한다
    Given "compile(src/auth): 초기 코드 생성" 커밋이 있다
    And 그 이후 "impl(src/auth): OAuth2 추가" 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then compile은 impl 커밋의 diff를 추출한다
    And 세션 파일에 "## Spec Changes" 섹션이 포함된다

  Scenario: compile 커밋이 없으면 전체 히스토리에서 impl 탐색
    Given compile 커밋이 없다
    And "impl(src/auth): 초기 요구사항" 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then compile은 전체 히스토리에서 impl 커밋을 탐색한다

  Scenario: impl 커밋이 없으면 기존 diff-compile-targets fallback
    Given compile 커밋이 없다
    And impl 커밋도 없다
    When compile이 실행된다
    Then 기존 diff-compile-targets 동작으로 fallback한다
    And 세션 파일에 "## Spec Changes" 섹션이 없다

  Scenario: 수동 수정은 impl 탐색에 잡히지 않는다
    Given "compile(src/auth): 코드 생성" 커밋이 있다
    And 그 이후 수동 "fix: 타이포 수정" 커밋이 있다
    And 그 이후 "impl(src/auth): 기능 추가" 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then 수동 커밋은 무시되고 impl 커밋만 처리된다

  Scenario: 여러 impl 커밋의 diff가 합산된다
    Given "compile(src/auth): 코드 생성" 커밋이 있다
    And 그 이후 3개의 impl(src/auth) 커밋이 있다
    When compile이 src/auth에 대해 실행된다
    Then 3개 impl 커밋의 diff가 모두 Spec Changes에 포함된다

  # --- Spec Changes 세션 파일 ---

  Scenario: Spec Changes에 Transition Context가 포함된다
    Given impl 커밋 메시지에 전환 맥락이 있다
    When compile이 세션 파일을 생성한다
    Then Spec Changes의 "### Transition Context"에 전환 맥락이 포함된다

  Scenario: Spec Changes에 Added/Modified/Removed가 분류된다
    Given impl 커밋 Changes에 added, modified, removed 항목이 있다
    When compile이 세션 파일을 생성한다
    Then Spec Changes에 "### Added", "### Modified", "### Removed" 섹션이 있다

  Scenario: BREAKING impl 커밋이면 breaking 메타데이터가 포함된다
    Given impl 커밋에 "[BREAKING]" 태그가 있다
    When compile이 세션 파일을 생성한다
    Then Spec Changes에 "breaking: true"가 포함된다

  # --- compiler agent Phase 0 ---

  Scenario: Spec Changes가 있으면 Phase 0 Task Definition 실행
    Given 세션 파일에 "## Spec Changes" 섹션이 있다
    When compiler agent가 실행된다
    Then Phase 0에서 Implementation Tasks를 도출한다
    And Phase 1에서 Task 단위로 TDD를 실행한다

  Scenario: Spec Changes가 없으면 Phase 0 건너뛰기
    Given 세션 파일에 "## Spec Changes" 섹션이 없다
    When compiler agent가 실행된다
    Then Phase 0을 건너뛰고 기존 TDD로 직행한다

  Scenario: 의미적 변경 없음 판단 시 compile 조기 종료
    Given Spec Changes의 Added, Modified, Removed가 모두 비어있다
    When compiler agent Phase 0이 실행된다
    Then "할 일 없음"으로 판단한다
    And status: skipped로 조기 종료한다

  Scenario: BREAKING 플래그 시 conflict overwrite 강제
    Given Spec Changes에 "breaking: true"가 있다
    When compiler agent Phase 0이 실행된다
    Then conflict 모드를 overwrite로 강제한다

  Scenario: DELETE 태스크가 코드 제거 + 참조 정리를 수행한다
    Given Phase 0에서 [DELETE] 태스크가 도출되었다
    When Phase 1에서 DELETE 태스크를 실행한다
    Then 대상 코드를 삭제한다
    And import/호출부 참조를 정리한다
    And 관련 테스트를 제거 또는 수정한다

  # --- post-compile 커밋 ---

  Scenario: compile 완료 후 커밋 메시지가 컨벤션을 따른다
    When compile이 성공적으로 완료된다
    Then 커밋 메시지가 "compile(src/auth):" 로 시작한다
