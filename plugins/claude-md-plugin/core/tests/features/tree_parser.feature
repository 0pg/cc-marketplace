Feature: CLAUDE.md Tree Parser
  As a developer using claude-md-plugin
  I want to identify directories that need CLAUDE.md
  So that I can ensure all source code is properly documented

  Background:
    Given a clean test directory

  Scenario: Directory with source files needs CLAUDE.md
    Given directory "src/auth" contains source files:
      | file        |
      | token.rs    |
      | session.rs  |
    When I parse the tree
    Then "src/auth" should need CLAUDE.md
    And the reason should mention "2 source files"

  Scenario: Directory with single source file needs CLAUDE.md
    Given directory "src/utils" contains source files:
      | file      |
      | helper.ts |
    When I parse the tree
    Then "src/utils" should need CLAUDE.md
    And the reason should mention "1 source files"

  Scenario: Directory with 2+ subdirectories needs CLAUDE.md
    Given directory "src" has subdirectories:
      | subdir |
      | auth   |
      | api    |
    When I parse the tree
    Then "src" should need CLAUDE.md
    And the reason should mention "2 subdirectories"

  Scenario: Directory with both source files and subdirs needs CLAUDE.md
    Given directory "src" has subdirectories:
      | subdir |
      | auth   |
      | api    |
    And directory "src" contains source files:
      | file    |
      | main.go |
    When I parse the tree
    Then "src" should need CLAUDE.md
    And the reason should mention "source files"
    And the reason should mention "subdirectories"

  Scenario: Empty directory does not need CLAUDE.md
    Given directory "empty" exists
    When I parse the tree
    Then "empty" should not need CLAUDE.md

  Scenario: Directory with only non-source files does not need CLAUDE.md
    Given directory "docs" contains files:
      | file      |
      | README.md |
      | guide.txt |
    When I parse the tree
    Then "docs" should not need CLAUDE.md

  Scenario: Build directories are excluded
    Given directory "target" contains source files:
      | file    |
      | main.rs |
    When I parse the tree
    Then "target" should be excluded
    And "target" should not need CLAUDE.md

  Scenario: node_modules is excluded
    Given directory "node_modules/express" contains source files:
      | file     |
      | index.js |
    When I parse the tree
    Then "node_modules" should be excluded

  Scenario: Multiple language source files are detected
    Given directory "polyglot" contains source files:
      | file          |
      | main.ts       |
      | helper.py     |
      | utils.go      |
      | lib.rs        |
      | Service.java  |
      | Model.kt      |
    When I parse the tree
    Then "polyglot" should need CLAUDE.md
    And the source file count should be 6

  Scenario: Directory depth is calculated correctly
    Given directory "src" has subdirectories:
      | subdir |
      | auth   |
      | api    |
    And directory "src/auth" contains source files:
      | file      |
      | token.rs  |
    And directory "src/api" contains source files:
      | file       |
      | routes.ts  |
    When I parse the tree
    Then "src" should have depth 1
    And "src/auth" should have depth 2
    And "src/api" should have depth 2

  Scenario: Results are sorted by depth descending for leaf-first processing
    Given directory "src" has subdirectories:
      | subdir |
      | auth   |
      | api    |
    And directory "src/auth" has subdirectories:
      | subdir |
      | jwt    |
      | saml   |
    And directory "src/auth/jwt" contains source files:
      | file       |
      | verify.rs  |
    And directory "src/api" contains source files:
      | file       |
      | routes.ts  |
    When I parse the tree
    Then the results sorted by depth descending should be:
      | path         | depth |
      | src/auth/jwt | 3     |
      | src/auth     | 2     |
      | src/api      | 2     |
      | src          | 1     |
