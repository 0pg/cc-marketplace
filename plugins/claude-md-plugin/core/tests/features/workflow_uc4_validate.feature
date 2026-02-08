Feature: UC-4 /validate Workflow Regression
  Ensures the /validate skill and validator agents correctly describe
  the validation workflow: audit → parallel validators → integrated report.

  Scenario: validate skill invokes drift-validator
    Given the content of skill "validate" is loaded
    Then the content should contain pattern "drift-validator"

  Scenario: validate skill invokes export-validator
    Given the content of skill "validate" is loaded
    Then the content should contain pattern "export-validator"

  Scenario: validate skill runs validators in parallel
    Given the content of skill "validate" is loaded
    Then the content should mention "병렬"

  Scenario: validate skill conditional code-reviewer
    Given the content of skill "validate" is loaded
    Then the content should contain all patterns:
      | pattern              |
      | code-reviewer        |
      | code-convention\.md  |

  Scenario: validate skill uses schema-validate CLI
    Given the content of skill "validate" is loaded
    Then the content should mention "claude-md-core validate-schema"

  Scenario: validate skill runs audit completeness check
    Given the content of skill "validate" is loaded
    Then the content should mention "claude-md-core audit"

  Scenario: validate skill workflow chain ordered
    Given the content of skill "validate" is loaded
    Then the content should describe workflow chain:
      | step            | pattern            |
      | audit           | claude-md-core audit |
      | drift-validator | drift-validator    |
      | report          | 통합 보고서        |

  Scenario: drift-validator uses claude-md-parse
    Given the content of agent "drift-validator" is loaded
    Then the content should mention "claude-md-parse"
