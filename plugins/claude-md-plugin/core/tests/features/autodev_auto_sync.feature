Feature: autodev --auto-sync executes consumer verdicts verbatim
  Scenario: consumer auto_executable → /sync runs
    Given consumer C's po-consultant emits Execution=auto_executable
    Then /sync MUST be invoked on C

  Scenario: consumer halt → chain halts with reason preserved
    Given consumer C's po-consultant emits Execution=halt with reason "breaking change"
    Then /sync MUST NOT run on C or any subsequent consumer
    And the result block MUST record C's halt reason verbatim
    And the result block MUST suggest `git revert HEAD`

  Scenario: consumer requires_human → chain halts with context
    Given consumer C's po-consultant emits Execution=requires_human
    Then /sync MUST NOT run on C or any subsequent consumer
    And the result block MUST record C's reason verbatim
