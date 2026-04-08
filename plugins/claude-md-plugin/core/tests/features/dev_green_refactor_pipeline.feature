Feature: Dev Green-Coder and Refactorer Pipeline
  As a developer using /dev,
  I want implementation and refactoring to be separate phases with strict test protection,
  So that approved tests remain frozen throughout the pipeline.

  Background:
    Given approved tests exist in target "src/auth/__tests__/"
    And mapping.json links all Constraints and Requirements to tests
    And dev session file exists for "src/auth"

  # green-coder scenarios

  Scenario: green-coder implements code that passes all approved tests
    When green-coder runs with approved tests
    Then all approved tests pass
    And green-result status is "success"
    And implemented_files list is not empty

  Scenario: green-coder uses stall-based termination on test failure
    Given green-coder first attempt fails 2 tests
    When green-coder retries and each attempt reduces failures
    Then green-coder continues until all tests pass or stall detected
    And stall is detected after 2 consecutive attempts with no improvement
    And final status reflects pass or partial

  Scenario: green-coder does not modify test assertions
    When green-coder runs
    Then no test file assertion logic is changed
    And no test case is deleted or disabled (skip, xfail)
    And no expected value is changed

  Scenario: green-coder may fix test import paths
    Given approved tests have import paths to not-yet-existing modules
    When green-coder creates production modules
    Then green-coder may fix test import/path errors only
    And assertion logic remains unchanged

  Scenario: green-coder returns partial on stall detection
    Given approved tests where failures stall at 2 remaining
    When green-coder detects 2 consecutive attempts with no improvement
    Then green-result status is "partial"
    And tests_failed count is greater than 0

  # refactorer scenarios

  Scenario: refactorer applies conventions without breaking tests
    Given green-coder completed successfully
    And Conventions specify naming rules
    When refactorer runs
    Then all approved tests still pass
    And refactored_files list is not empty

  Scenario: refactorer rolls back on regression
    Given green-coder completed successfully
    When refactorer applies conventions
    And a test fails after refactoring
    Then refactorer rolls back changes
    And refactor-result status is "rolled_back"
    And all approved tests pass after rollback

  Scenario: refactorer does not modify test assertions
    When refactorer runs
    Then no test file assertion logic is changed
    And no test case is deleted or disabled
    And no expected value is changed

  Scenario: refactorer does not change public API
    Given green-coder produced public functions
    When refactorer runs
    Then public function signatures are unchanged
    And only internal structure is modified

  # DELETE scenarios (SKILL-level)

  Scenario: SKILL handles DELETE tasks before TDD pipeline
    Given Spec Changes include [DELETE] for "refresh_token"
    When SKILL processes DELETE tasks in Step 6e
    Then "refresh_token" function is removed
    And imports referencing "refresh_token" are cleaned
    And related test files are removed
    And regression tests pass after deletion
