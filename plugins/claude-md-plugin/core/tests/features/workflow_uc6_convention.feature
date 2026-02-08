Feature: UC-6 /convention Workflow Regression
  Ensures the /convention skill correctly describes
  the convention workflow: view mode, analyze mode, and fallback to project-setup.

  Scenario: convention skill has view mode
    Given the content of skill "convention" is loaded
    Then the content should contain pattern "Mode 1.*조회"

  Scenario: convention skill has analyze mode
    Given the content of skill "convention" is loaded
    Then the content should mention "--analyze"

  Scenario: convention skill reads code-convention.md
    Given the content of skill "convention" is loaded
    Then the content should contain pattern "code-convention\.md"

  Scenario: convention skill suggests project-setup when missing
    Given the content of skill "convention" is loaded
    Then the content should mention "/project-setup"
