Feature: Compile Skill Test Review Integration
  As a user running /compile
  I want the compile skill to automatically verify test quality before implementation
  So that GREEN phase receives well-tested specifications

  # compile skill orchestrates compiler (phase=red) -> test-reviewer -> compiler (phase=green-refactor)

  Scenario: Phase red produces only test files without implementation
    Given a CLAUDE.md with 2 exports and 3 behaviors
    And compiler is invoked with phase "red"
    When compiler completes the RED phase
    Then test files should be generated
    And no implementation files should be created
    And the result should include test_files and spec_json_path
    And the result phase should be "red"

  Scenario: First attempt approved proceeds to green-refactor
    Given a CLAUDE.md with well-defined behaviors and exports
    And compiler phase=red generates comprehensive tests
    When test-reviewer evaluates and returns score 100 with status "approve"
    Then the compile skill should invoke compiler with phase "green-refactor"
    And review_iterations in the result should be 1
    And test_review_status should be "approve"

  Scenario: Feedback on first attempt triggers re-generation then approved
    Given a CLAUDE.md with 3 behaviors
    And compiler phase=red generates tests missing 1 behavior
    And test-reviewer returns feedback with score 75
    When compiler phase=red regenerates tests with feedback
    And test-reviewer returns score 100 with status "approve" on second attempt
    Then review_iterations in the result should be 2
    And test_review_status should be "approve"
    And the compile skill should invoke compiler with phase "green-refactor"

  Scenario: Three failed reviews proceeds with warning
    Given a CLAUDE.md with complex behaviors
    And test-reviewer returns feedback on all 3 attempts
    And scores are 60, 70, and 78
    When the compile skill exhausts max review iterations
    Then test_review_status should be "warning"
    And review_iterations should be 3
    And the compile skill should still invoke compiler with phase "green-refactor"
    And the final result should include test_review with status "warning"
