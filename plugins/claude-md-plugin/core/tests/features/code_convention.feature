Feature: Code Convention Guide
  As a developer using claude-md-plugin
  I want to generate and maintain a code-convention.md file
  So that compile and validate steps can enforce consistent coding style

  # /project-setup generates code-convention.md

  Scenario: Project setup generates code-convention.md from existing codebase
    Given a project with TypeScript source files using camelCase naming
    And package.json with build/test/lint scripts
    When I run /project-setup
    Then code-convention.md should be created at project root
    And it should contain a Naming section
    And it should contain a Formatting section
    And CLAUDE.md should contain Build and Test Commands

  Scenario: Project setup generates code-convention.md for Python project
    Given a project with Python source files using snake_case naming
    And pyproject.toml with pytest and ruff configuration
    When I run /project-setup
    Then code-convention.md should be created at project root
    And Naming Variables pattern should be "snake_case"
    And Formatting indentation should be "4 spaces"

  Scenario: Project setup with no existing source code
    Given a project with no source files
    When I run /project-setup
    Then code-convention.md should not be created
    And user should be notified that convention analysis was skipped

  # /convention updates code-convention.md

  Scenario: Convention command with --analyze re-analyzes codebase
    Given a project with existing code-convention.md
    And source files have been modified since last analysis
    When I run /convention --analyze
    Then code-convention.md should be updated with new patterns

  Scenario: Convention command without options shows current conventions
    Given a project with existing code-convention.md
    When I run /convention
    Then current code-convention.md contents should be displayed

  # /validate includes convention review

  Scenario: Validate includes convention compliance in report
    Given a project with code-convention.md specifying camelCase for variables
    And source code containing snake_case variables
    When I run /validate
    Then validation report should include Convention column
    And convention violations should list the snake_case variables

  Scenario: Validate without code-convention.md skips convention review
    Given a project without code-convention.md
    When I run /validate
    Then validation report should not include Convention column
    And a note should suggest running /project-setup

  # /compile references code-convention.md

  Scenario: Compile loads code-convention.md during code generation
    Given a project with code-convention.md specifying camelCase naming
    And CLAUDE.md with exports defined
    When I run /compile
    Then generated code should follow camelCase naming convention
    And REFACTOR phase should reference code-convention.md

  Scenario: Compile without code-convention.md shows warning
    Given a project without code-convention.md
    And CLAUDE.md with exports defined
    When I run /compile
    Then a warning should be shown about missing code-convention.md
    And compilation should proceed with language defaults
