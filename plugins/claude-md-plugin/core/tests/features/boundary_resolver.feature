Feature: Boundary Resolution
  As a developer maintaining CLAUDE.md files
  I want to validate that references follow the tree structure rules
  So that each CLAUDE.md remains self-contained

  Background:
    Given a clean test directory

  Scenario: Child reference is allowed
    Given directory structure:
      | path     |
      | src      |
      | src/auth |
      | src/api  |
    And CLAUDE.md at "src" with content:
      """
      ## Structure
      - auth/: Authentication module (see auth/CLAUDE.md for details)
      - api/: API endpoints
      """
    When I validate references for "src"
    Then no violation should be reported

  Scenario: Parent reference is forbidden
    Given directory structure:
      | path     |
      | src      |
      | src/auth |
    And CLAUDE.md at "src/auth" with content:
      """
      ## Dependencies
      - See ../utils for shared utilities
      """
    When I validate references for "src/auth"
    Then violation "Parent" should be reported
    And the violation reference should contain ".."

  Scenario: Sibling reference is forbidden
    Given directory structure:
      | path      |
      | src       |
      | src/auth  |
      | src/api   |
      | src/utils |
    And CLAUDE.md at "src/auth" with content:
      """
      ## Dependencies
      - Uses src/api for API calls
      """
    When I validate references for "src/auth"
    Then violation "Sibling" should be reported

  Scenario: Multiple violations in single file
    Given directory structure:
      | path      |
      | src       |
      | src/auth  |
    And CLAUDE.md at "src/auth" with content:
      """
      ## Dependencies
      - See ../utils for shared code
      - References src/api endpoints
      """
    When I validate references for "src/auth"
    Then multiple violations should be reported

  Scenario: URL references are not considered violations
    Given directory structure:
      | path     |
      | src      |
      | src/auth |
    And CLAUDE.md at "src/auth" with content:
      """
      ## References
      - API docs: https://api.example.com/docs
      - GitHub: https://github.com/user/repo
      """
    When I validate references for "src/auth"
    Then no violation should be reported

  Scenario: Direct files are correctly identified
    Given directory "src/auth" with files:
      | file       |
      | token.rs   |
      | session.rs |
      | CLAUDE.md  |
    When I resolve boundary for "src/auth"
    Then direct files should include:
      | file       |
      | token.rs   |
      | session.rs |
      | CLAUDE.md  |

  Scenario: Subdirectories are correctly identified
    Given directory structure:
      | path          |
      | src/auth      |
      | src/auth/jwt  |
      | src/auth/saml |
    When I resolve boundary for "src/auth"
    Then subdirs should include:
      | subdir |
      | jwt    |
      | saml   |
