Feature: Test Reviewer Agent
  As a compile skill orchestrator
  I want to verify generated tests cover CLAUDE.md specifications before implementation
  So that GREEN phase produces correct code based on comprehensive tests

  # test-reviewer validates test quality against CLAUDE.md spec

  Scenario: All checks pass with perfect score
    Given a CLAUDE.md spec with 3 behaviors, 2 exports, and 1 contract
    And generated test files covering all behaviors, exports, and contracts
    And all assertions verify input/output state meaningfully
    And edge-case behaviors have dedicated tests
    When test-reviewer evaluates the tests against the spec
    Then the status should be "approve"
    And the score should be 100
    And all 5 checks should have full marks

  Scenario: Missing behavior coverage results in feedback
    Given a CLAUDE.md spec with 3 behaviors including "expired token returns error"
    And generated test files covering only 2 of 3 behaviors
    When test-reviewer evaluates the tests against the spec
    Then the status should be "feedback"
    And the score should be less than 100
    And BEHAVIOR-COVERAGE check should report the missing behavior
    And feedback should suggest adding a test for "expired token returns error"

  Scenario: Missing export coverage results in feedback
    Given a CLAUDE.md spec with exports "validateToken" and "refreshToken"
    And generated test files only testing "validateToken"
    When test-reviewer evaluates the tests against the spec
    Then the status should be "feedback"
    And EXPORT-COVERAGE check should report "refreshToken" as untested
    And feedback should suggest adding tests for "refreshToken"

  Scenario: Weak assertions result in feedback
    Given a CLAUDE.md spec with behavior "valid token returns Claims object"
    And generated tests that only check return value is not null
    And tests do not verify specific Claims fields
    When test-reviewer evaluates the tests against the spec
    Then the status should be "feedback"
    And TEST-QUALITY check should report weak assertions
    And feedback should suggest verifying specific Claims properties

  Scenario: Missing edge case tests result in feedback
    Given a CLAUDE.md spec with error-category behavior "malformed token throws ParseError"
    And generated tests with no dedicated test for malformed token
    When test-reviewer evaluates the tests against the spec
    Then the status should be "feedback"
    And EDGE-CASE check should report the missing edge case test
    And feedback should include the specific error-category behavior

  Scenario: Missing contract tests result in feedback
    Given a CLAUDE.md spec with contract precondition "token must be non-empty string"
    And generated tests that never test empty string input
    When test-reviewer evaluates the tests against the spec
    Then the status should be "feedback"
    And CONTRACT-COVERAGE check should report the untested precondition
    And feedback should suggest adding a test for empty string input

  Scenario: Maximum review iterations reached proceeds with warning
    Given a test-reviewer evaluation that returns feedback 3 times
    And score does not reach 100 after 3 iterations
    When the compile skill reaches max review iterations
    Then the test review status should be "warning"
    And GREEN phase should proceed with the current tests
    And the final iteration count should be 3

  Scenario: No progress in score triggers early termination
    Given a test-reviewer first evaluation with score 65
    And a second evaluation with score 68
    And the score delta is less than 5
    When the compile skill detects no progress
    Then the test review status should be "warning"
    And GREEN phase should proceed with the current tests
    And the final iteration count should be 2
