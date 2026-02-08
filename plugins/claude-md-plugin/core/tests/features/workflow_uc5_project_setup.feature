Feature: UC-5 /project-setup Workflow Regression
  Ensures the /project-setup skill correctly describes
  the project setup workflow: config detection → command inference → confirmation → convention → save.

  Scenario: project-setup detects config files
    Given the content of skill "project-setup" is loaded
    Then the content should contain all patterns:
      | pattern        |
      | package\.json  |
      | Cargo\.toml    |
      | go\.mod        |

  Scenario: project-setup uses AskUserQuestion for confirmation
    Given the content of skill "project-setup" is loaded
    Then the content should mention "AskUserQuestion"

  Scenario: project-setup generates code-convention.md
    Given the content of skill "project-setup" is loaded
    Then the content should contain pattern "code-convention\.md"

  Scenario: project-setup Phase 1-6 ordered
    Given the content of skill "project-setup" is loaded
    Then the content should describe workflow chain:
      | step    | pattern  |
      | Phase 1 | Phase 1  |
      | Phase 2 | Phase 2  |
      | Phase 3 | Phase 3  |
      | Phase 4 | Phase 4  |
      | Phase 5 | Phase 5  |
      | Phase 6 | Phase 6  |

  Scenario: project-setup writes to CLAUDE.md
    Given the content of skill "project-setup" is loaded
    Then the content should contain pattern "CLAUDE\.md.*저장"
