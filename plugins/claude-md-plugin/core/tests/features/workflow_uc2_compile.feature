Feature: UC-2 /compile Workflow Regression
  Ensures the /compile skill and compiler agent prompt files correctly describe
  the TDD compile workflow: RED → TEST REVIEW → GREEN+REFACTOR with incremental support.

  Scenario: compile skill has RED-TEST_REVIEW-GREEN phases
    Given the content of skill "compile" is loaded
    Then the content should contain all patterns:
      | pattern              |
      | phase=red            |
      | test-reviewer        |
      | phase=green-refactor |

  Scenario: compile skill describes TDD workflow
    Given the content of skill "compile" is loaded
    Then the content should mention "TDD"

  Scenario: compile skill workflow chain is ordered
    Given the content of skill "compile" is loaded
    Then the content should describe workflow chain:
      | step           | pattern              |
      | RED phase      | phase=red            |
      | TEST REVIEW    | test-reviewer        |
      | GREEN+REFACTOR | phase=green-refactor |

  Scenario: compile skill review loop max 3
    Given the content of skill "compile" is loaded
    Then the content should contain pattern "최대 3회"

  Scenario: compile skill test-reviewer approve threshold
    Given the content of skill "compile" is loaded
    Then the content should contain pattern "score.*==.*100"

  Scenario: compile skill incremental uses 4 skills
    Given the content of skill "compile" is loaded
    Then the content should contain all patterns:
      | pattern              |
      | git-status-analyzer  |
      | commit-comparator    |
      | interface-diff       |
      | dependency-tracker   |

  Scenario: compile skill incremental workflow ordered
    Given the content of skill "compile" is loaded
    Then the content should describe workflow chain:
      | step             | pattern             |
      | git-status       | git-status-analyzer |
      | commit-compare   | commit-comparator   |
      | interface-diff   | interface-diff      |
      | dependency-track | dependency-tracker  |

  Scenario: compiler agent supports phase parameter
    Given the content of agent "compiler" is loaded
    Then the content should contain all patterns:
      | pattern               |
      | phase.*red            |
      | phase.*green-refactor |

  Scenario: compiler agent uses claude-md-parse
    Given the content of agent "compiler" is loaded
    Then the content should mention "claude-md-parse"

  Scenario: compile skill IMPLEMENTS.md update responsibility
    Given the content of skill "compile" is loaded
    Then the content should contain pattern "IMPLEMENTS.*Implementation.*Section.*업데이트"
