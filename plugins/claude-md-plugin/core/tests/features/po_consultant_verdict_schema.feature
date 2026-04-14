Feature: po-consultant verdict carries self-described execution guidance

  Scenario: Verdict includes execution hint and reason
    Given a po-consultant result file
    When the result is parsed
    Then it MUST contain a "## Execution" section with value in: auto_executable | requires_human | halt
    And it MUST contain a "## Reason" section (non-empty iff Execution != auto_executable)
    And it MAY contain a "## Redirect To" section with a node path

  Scenario: auto_executable requires feasible verdict
    Given a result file with Verdict=feasible
    Then Execution MAY be auto_executable
    And Reason MAY be empty

  Scenario: halt requires non-empty Reason
    Given a result file with Execution=halt
    Then Reason MUST be non-empty

  Scenario: Redirect To is valid only when the verdict author proposes rerouting
    Given a result file with "## Redirect To" present
    Then Execution MUST NOT be auto_executable
    And Reason MUST describe why the redirect applies
