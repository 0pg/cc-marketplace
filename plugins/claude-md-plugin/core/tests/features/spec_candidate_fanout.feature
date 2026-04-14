Feature: spec SKILL fans out po-consultant across all candidate nodes in parallel
  Scenario: Three candidates all receive independent po-consultants
    Given explorer emitted ".", "core/src/a", "core/src/b" as candidates
    When Step 2.1d fans out across the candidate set
    Then 3 consult-result files MUST exist (one per candidate)
    And they MUST be produced in parallel (no serial dependency)
