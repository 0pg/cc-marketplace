Feature: Cross-Agent Workflow Chain Regression
  Ensures the delegation chains across skills and agents are intact,
  verifying that each layer properly references the next in the workflow.

  Scenario: spec chain - skill delegates to agent delegates to reviewer
    Given the content of skill "spec" is loaded
    Then the content should mention "spec-agent"
    Given the content of agent "spec-agent" is loaded
    Then the content should mention "spec-reviewer"

  Scenario: compile chain - skill orchestrates compiler and test-reviewer
    Given the content of skill "compile" is loaded
    Then the content should describe workflow chain:
      | step           | pattern              |
      | compiler-red   | phase=red            |
      | test-reviewer  | test-reviewer        |
      | compiler-green | phase=green-refactor |

  Scenario: decompile chain - skill delegates to recursive-decompiler delegates to decompiler
    Given the content of skill "decompile" is loaded
    Then the content should mention "recursive-decompiler"
    Given the content of agent "recursive-decompiler" is loaded
    Then the content should contain pattern "Task\(decompiler"

  Scenario: validate chain - skill delegates to drift export and code-reviewer
    Given the content of skill "validate" is loaded
    Then the content should contain all patterns:
      | pattern          |
      | drift-validator  |
      | export-validator |
      | code-reviewer    |

  Scenario: spec-agent pre-validation before reviewer
    Given the content of agent "spec-agent" is loaded
    Then the content should describe workflow chain:
      | step            | pattern          |
      | pre-validation  | Pre-validation   |
      | spec-reviewer   | spec-reviewer    |
