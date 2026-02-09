Feature: UC-1 /impl Workflow Regression
  Ensures the /impl skill and impl-agent prompt files correctly describe
  the specification workflow: impl skill → impl-agent → impl-reviewer → validate-schema CLI.

  Scenario: impl skill delegates to impl-agent
    Given the content of skill "impl" is loaded
    Then the content should contain pattern "impl-agent"

  Scenario: impl skill dual document output
    Given the content of skill "impl" is loaded
    Then the content should contain all patterns:
      | pattern                          |
      | CLAUDE\.md.*WHAT                 |
      | IMPLEMENTS\.md.*Planning         |

  Scenario: impl skill mentions architecture analysis
    Given the content of skill "impl" is loaded
    Then the content should mention "Phase 2.5"

  Scenario: impl-agent uses parse-tree and dependency-graph CLI
    Given the content of agent "impl-agent" is loaded
    Then the content should contain all patterns:
      | pattern            |
      | parse-tree         |
      | dependency-graph   |

  Scenario: impl-agent invokes impl-reviewer
    Given the content of agent "impl-agent" is loaded
    Then the content should mention "impl-reviewer"

  Scenario: impl-agent max 3 iteration review cycle
    Given the content of agent "impl-agent" is loaded
    Then the content should contain pattern "max.*3"
    And the content should mention "iteration"

  Scenario: impl-agent uses AskUserQuestion
    Given the content of agent "impl-agent" is loaded
    Then the content should mention "AskUserQuestion"

  Scenario: impl-agent runs validate-schema CLI
    Given the content of agent "impl-agent" is loaded
    Then the content should mention "validate-schema"
