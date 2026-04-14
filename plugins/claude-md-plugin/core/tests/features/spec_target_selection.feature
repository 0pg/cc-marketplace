Feature: target selection honors verdict self-described execution
  Scenario: Single auto_executable candidate is selected
    Given verdicts: A.execution=auto_executable, B.execution=halt, C.execution=requires_human
    Then target_path MUST equal A

  Scenario: Multiple auto_executable candidates - halt and surface
    Given verdicts: A.execution=auto_executable, B.execution=auto_executable
    Then spec MUST halt with a surface-state reason including A and B and their reasons

  Scenario: No auto_executable candidate in auto mode - halt
    Given all candidates have execution in halt or requires_human
    And --no-ask is set
    Then spec MUST halt with each candidate's reason preserved verbatim

  Scenario: No auto_executable candidate in interactive mode - AskUserQuestion
    Given all candidates have execution in halt or requires_human
    And --no-ask is NOT set
    Then spec MUST AskUserQuestion with each candidate's reason preserved
