Feature: explorer emits judged candidate node set when target is unspecified
  Scenario: No --path given; explorer judges multiple nodes potentially relevant
    Given a requirement text with no --path
    And project index lists nodes A, B, C, D
    When requirement-explorer runs Phase 1 (pre-judgment pass)
    Then explorer MUST output a "## Candidate Nodes" section in its result
    And the list MUST contain at least one node
    And the list MUST include "." (project root) as baseline

  Scenario: --path given; candidate set is exactly that path + root
    Given --path core/src/foo
    Then "## Candidate Nodes" MUST equal [".", "core/src/foo"]
