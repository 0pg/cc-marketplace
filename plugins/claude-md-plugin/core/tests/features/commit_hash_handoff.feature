Feature: spec → dev Commit Hash Handoff
  spec이 커밋 메시지 컨벤션으로 변경 맥락을 기록하고,
  dev가 이를 자동 탐색하여 incremental dev를 수행한다.

  Background:
    Given CLI가 설치되어 있다
    And git 저장소가 초기화되어 있다

  # --- 커밋 메시지 컨벤션 ---

  Scenario: spec 커밋 메시지가 컨벤션을 따른다
    When spec이 "src/auth"에 대해 커밋을 생성한다
    Then 커밋 메시지가 "spec(src/auth):" 로 시작한다
    And 커밋 메시지 body에 "Changes:" 섹션이 있다

  Scenario: BREAKING 태그가 필요할 때 포함된다
    Given Requirements 삭제가 포함된 spec 변경이 있다
    When spec이 커밋을 생성한다
    Then 커밋 메시지에 "[BREAKING]"이 포함된다

  Scenario: 전환 맥락이 커밋 메시지에 포함된다
    When spec이 기존 기능을 수정하는 커밋을 생성한다
    Then 커밋 메시지 body 첫 단락에 전환 맥락이 있다
    And 전환 맥락이 변경의 방향을 기술한다

  # --- compile의 spec 커밋 탐색 ---

  Scenario: dev가 마지막 dev 이후 spec 커밋을 탐색한다
    Given "dev(src/auth): 초기 코드 생성" 커밋이 있다
    And 그 이후 "spec(src/auth): OAuth2 추가" 커밋이 있다
    When dev가 src/auth에 대해 실행된다
    Then dev는 spec 커밋의 diff를 추출한다
    And 세션 파일에 "## Spec Changes" 섹션이 포함된다

  Scenario: dev 커밋이 없으면 전체 히스토리에서 spec 탐색
    Given dev 커밋이 없다
    And "spec(src/auth): 초기 요구사항" 커밋이 있다
    When dev가 src/auth에 대해 실행된다
    Then dev는 전체 히스토리에서 spec 커밋을 탐색한다

  Scenario: spec 커밋이 없으면 기존 diff-compile-targets fallback
    Given dev 커밋이 없다
    And spec 커밋도 없다
    When dev가 실행된다
    Then 기존 diff-compile-targets 동작으로 fallback한다
    And 세션 파일에 "## Spec Changes" 섹션이 없다

  Scenario: 수동 수정은 spec 탐색에 잡히지 않는다
    Given "dev(src/auth): 코드 생성" 커밋이 있다
    And 그 이후 수동 "fix: 타이포 수정" 커밋이 있다
    And 그 이후 "spec(src/auth): 기능 추가" 커밋이 있다
    When dev가 src/auth에 대해 실행된다
    Then 수동 커밋은 무시되고 spec 커밋만 처리된다

  Scenario: 여러 spec 커밋의 diff가 합산된다
    Given "dev(src/auth): 코드 생성" 커밋이 있다
    And 그 이후 3개의 spec(src/auth) 커밋이 있다
    When dev가 src/auth에 대해 실행된다
    Then 3개 spec 커밋의 diff가 모두 Spec Changes에 포함된다

  # --- Spec Changes 세션 파일 ---

  Scenario: Spec Changes에 Transition Context가 포함된다
    Given spec 커밋 메시지에 전환 맥락이 있다
    When dev가 세션 파일을 생성한다
    Then Spec Changes의 "### Transition Context"에 전환 맥락이 포함된다

  Scenario: Spec Changes에 Added/Modified/Removed가 분류된다
    Given spec 커밋 Changes에 added, modified, removed 항목이 있다
    When dev가 세션 파일을 생성한다
    Then Spec Changes에 "### Added", "### Modified", "### Removed" 섹션이 있다

  Scenario: BREAKING spec 커밋이면 breaking 메타데이터가 포함된다
    Given spec 커밋에 "[BREAKING]" 태그가 있다
    When dev가 세션 파일을 생성한다
    Then Spec Changes에 "breaking: true"가 포함된다

  # --- dev SKILL Step 6: Implementation Tasks 도출 ---

  Scenario: Spec Changes가 있으면 Implementation Tasks 도출
    Given 세션 파일에 "## Spec Changes" 섹션이 있다
    When dev SKILL이 Step 6e를 실행한다
    Then Implementation Tasks ([ADD]/[MODIFY]/[DELETE])를 도출한다
    And 세션 파일에 "## Implementation Tasks" 섹션이 포함된다

  Scenario: Spec Changes가 없으면 Implementation Tasks 생략
    Given 세션 파일에 "## Spec Changes" 섹션이 없다
    When dev SKILL이 세션 파일을 생성한다
    Then "## Implementation Tasks" 섹션 없이 전체 TDD 파이프라인으로 진행한다

  Scenario: 의미적 변경 없음 판단 시 dev 조기 종료
    Given Spec Changes의 Added, Modified, Removed가 모두 비어있다
    When dev SKILL이 Step 6e를 실행한다
    Then "할 일 없음"으로 판단한다
    And status: skipped로 조기 종료한다

  Scenario: BREAKING 플래그 시 conflict overwrite 강제
    Given Spec Changes에 "breaking: true"가 있다
    When dev SKILL이 Step 6e를 실행한다
    Then conflict 모드를 overwrite로 강제한다

  Scenario: DELETE 태스크를 SKILL이 직접 실행한다
    Given Step 6e에서 [DELETE] 태스크가 도출되었다
    When dev SKILL이 Step 6f를 실행한다
    Then 대상 코드를 삭제한다
    And import/호출부 참조를 정리한다
    And 관련 테스트를 제거 또는 수정한다
    And TDD 파이프라인 (Step 7-9) 전에 완료된다

  # --- post-dev 커밋 ---

  Scenario: dev 완료 후 커밋 메시지가 컨벤션을 따른다
    When dev가 성공적으로 완료된다
    Then 커밋 메시지가 "dev(src/auth):" 로 시작한다
