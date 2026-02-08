Feature: UC-1 /spec Workflow Regression
  Ensures the /spec skill and spec-agent prompt files correctly describe
  the specification workflow: spec skill → spec-agent → spec-reviewer → validate-schema CLI.

  Scenario: spec skill delegates to spec-agent
    Given the content of skill "spec" is loaded
    Then the content should contain pattern "spec-agent"

  Scenario: spec skill dual document output
    Given the content of skill "spec" is loaded
    Then the content should contain all patterns:
      | pattern                          |
      | CLAUDE\.md.*WHAT                 |
      | IMPLEMENTS\.md.*Planning         |

  Scenario: spec skill mentions architecture analysis
    Given the content of skill "spec" is loaded
    Then the content should mention "Phase 2.5"

  Scenario: spec-agent uses parse-tree and dependency-graph CLI
    Given the content of agent "spec-agent" is loaded
    Then the content should contain all patterns:
      | pattern            |
      | parse-tree         |
      | dependency-graph   |

  Scenario: spec-agent invokes spec-reviewer
    Given the content of agent "spec-agent" is loaded
    Then the content should mention "spec-reviewer"

  Scenario: spec-agent max 3 iteration review cycle
    Given the content of agent "spec-agent" is loaded
    Then the content should contain pattern "max.*3"
    And the content should mention "iteration"

  Scenario: spec-agent uses AskUserQuestion
    Given the content of agent "spec-agent" is loaded
    Then the content should mention "AskUserQuestion"

  Scenario: spec-agent runs validate-schema CLI
    Given the content of agent "spec-agent" is loaded
    Then the content should mention "validate-schema"
