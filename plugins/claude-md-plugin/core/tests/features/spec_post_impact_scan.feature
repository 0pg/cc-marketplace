Feature: spec surfaces affected consumers after schema change
  Scenario: schema change triggers consumer listing
    Given /spec modified target's ## Data Schemas
    When Step 4.5 executes
    Then the result block MUST contain a "## Affected Consumers" section
    And each referencing consumer MUST appear as a list item

  Scenario: no schema change suppresses the section
    Given /spec modified only ## Constraints
    When Step 4.5 executes
    Then the result block MUST NOT contain "## Affected Consumers"
