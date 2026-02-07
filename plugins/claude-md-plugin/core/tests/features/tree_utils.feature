Feature: TreeUtils shared directory scanning
  Both TreeParser and Auditor should produce consistent results
  because they share the same DirScanner logic.

  Background:
    Given a clean test directory

  Scenario: Both detect same excluded directories
    Given directory "src" contains source files:
      | file     |
      | index.ts |
    And directory "node_modules" exists
    And directory "target" exists
    And directory ".git" exists
    When I parse the tree
    And I audit the directory tree
    Then TreeParser excluded list should match Auditor excluded list

  Scenario: Both detect same source file directories
    Given directory "src" contains source files:
      | file     |
      | index.ts |
    And directory "src/auth" contains source files:
      | file      |
      | login.ts  |
    And directory "src/utils" contains source files:
      | file       |
      | helpers.ts |
    And directory "docs" exists
    When I parse the tree
    And I audit the directory tree
    Then TreeParser detected directories should match Auditor detected directories
