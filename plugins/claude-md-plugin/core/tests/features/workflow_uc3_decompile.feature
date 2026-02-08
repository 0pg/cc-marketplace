Feature: UC-3 /decompile Workflow Regression
  Ensures the /decompile skill and decompiler agents correctly describe
  the decompile workflow: decompile skill → recursive-decompiler → decompiler.

  Scenario: decompile skill delegates to recursive-decompiler
    Given the content of skill "decompile" is loaded
    Then the content should contain pattern "recursive-decompiler"

  Scenario: decompile skill describes leaf-first processing
    Given the content of skill "decompile" is loaded
    Then the content should mention "leaf-first"

  Scenario: decompile skill supports incremental mode
    Given the content of skill "decompile" is loaded
    Then the content should contain all patterns:
      | pattern      |
      | incremental  |
      | --full       |

  Scenario: decompile skill dual document output
    Given the content of skill "decompile" is loaded
    Then the content should contain all patterns:
      | pattern                    |
      | CLAUDE\.md.*WHAT           |
      | IMPLEMENTS\.md.*HOW        |

  Scenario: recursive-decompiler uses resolve-boundary CLI
    Given the content of agent "recursive-decompiler" is loaded
    Then the content should mention "resolve-boundary"

  Scenario: recursive-decompiler is self-recursive
    Given the content of agent "recursive-decompiler" is loaded
    Then the content should contain pattern "Task\(recursive-decompiler"

  Scenario: recursive-decompiler delegates to decompiler
    Given the content of agent "recursive-decompiler" is loaded
    Then the content should contain pattern "Task\(decompiler"

  Scenario: decompiler uses code-analyze skill and CLI tools
    Given the content of agent "decompiler" is loaded
    Then the content should contain all patterns:
      | pattern           |
      | resolve-boundary  |
      | code-analyze      |
      | validate-schema   |
