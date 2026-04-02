Feature: Dev Test Writing Loop
  As a developer using /dev,
  I want tests to be reviewed against the spec before code generation,
  So that every Constraint and Requirement is covered by tests before implementation begins.

  Background:
    Given a project with CLAUDE.md and DEVELOPERS.md in "src/auth"
    And CLAUDE.md has Requirements:
      | id    | text                                    |
      | REQ-1 | 유효한 토큰으로 사용자 인증 가능          |
      | REQ-2 | 만료된 토큰은 거부                       |
    And DEVELOPERS.md has Constraints:
      | id      | text                                                    |
      | CONST-1 | authenticate(token: string) → User \| AuthError         |
      | CONST-2 | token expiry: max 7 days, reject at day 8               |

  Scenario: test-writer generates tests with complete mapping
    When test-writer runs with mode "write"
    Then test files exist in TMP directory
    And mapping.json has all Constraints mapped to tests
    And mapping.json has all Requirements mapped to acceptance tests
    And unmapped_constraints is empty
    And unmapped_requirements is empty

  Scenario: test-reviewer approves complete tests on first round
    Given test-writer has produced tests with complete mapping
    When test-reviewer reviews round 1
    Then verdict is "approved"
    And Critical Questions count is 0

  Scenario: test-reviewer rejects tests missing boundary values
    Given test-writer has produced tests without boundary tests for CONST-2
    When test-reviewer reviews round 1
    Then verdict is "rejected"
    And Critical Questions reference "CONST-2"
    And Critical Questions mention "경계값"

  Scenario: test-writer revises tests based on reviewer feedback
    Given test-reviewer rejected with feedback about CONST-2 boundary tests
    When test-writer runs with mode "revise" and round 2
    Then test files include boundary tests for day 7 and day 8
    And mapping.json CONST-2 tests include boundary cases

  Scenario: review loop approves after revision
    Given test-writer revised tests addressing all Critical Questions
    When test-reviewer reviews round 2
    Then verdict is "approved"

  Scenario: review loop terminates at max_safety without approval
    Given test-reviewer rejects for 5 consecutive rounds
    When round exceeds max_safety of 5
    Then SKILL proceeds with best-effort tests
    And a warning message is emitted

  Scenario: TMP tests are copied to target after approval
    Given test-reviewer approved the tests
    When SKILL copies TMP to target
    Then test files exist in target directory
    And TMP test files match target test files

  Scenario: Verify RED — tests fail before implementation
    Given approved tests are copied to target
    And no production code exists yet
    When SKILL runs Verify RED
    Then all tests fail or compilation fails
    And SKILL proceeds to green-coder

  Scenario: Incremental mode — existing tests are accessible
    Given existing tests in "src/auth/__tests__/"
    And Spec Changes with [MODIFY] CONST-1
    When test-writer runs with mode "write"
    Then test-writer can read existing tests via existing_test_dir
    And modified tests are written to TMP

  Scenario: Approved tests are frozen — assertion contract
    Given test-reviewer approved the tests
    When green-coder receives approved tests
    Then assertion logic in test files must not be modified
    And test cases must not be deleted or disabled
    And expected values must not be changed
