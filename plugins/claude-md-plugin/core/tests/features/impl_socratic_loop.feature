Feature: Socratic Feedback Loop for spec Skill
  As a developer using /spec
  I want the impl agent's execution plan to be critically reviewed before document generation
  So that CLAUDE.md and DEVELOPERS.md are complete, unambiguous, and test-convertible

  Background:
    Given spec SKILL is invoked with a requirement
    And decompose agent returns scope: single

  Scenario: Reviewer approves plan on first round
    Given impl agent produces plan.md with complete Requirements and typed Constraints
    When impl-reviewer agent reviews the plan
    Then verdict should be "approved"
    And spec SKILL proceeds to mode=execute
    And CLAUDE.md and DEVELOPERS.md are generated from the approved plan

  Scenario: Reviewer rejects plan and impl agent revises
    Given impl agent produces plan.md with vague Requirements ("handle appropriately")
    When impl-reviewer agent reviews the plan
    Then verdict should be "rejected"
    And Critical Questions should reference the specific vague items
    When impl agent revises plan in mode=revise
    Then revised plan.md should address the Critical Questions
    When impl-reviewer reviews the revised plan
    Then verdict should be "approved"

  Scenario: Loop terminates at max_safety without approval
    Given impl agent produces plan.md that cannot pass review
    When Socratic loop runs for max_safety(5) rounds without approval
    Then SKILL emits a warning message
    And SKILL proceeds to mode=execute with the best available plan

  Scenario: SKILL context does not explode across rounds
    Given Socratic loop runs for 3 rounds
    Then SKILL session files contain only file paths, not plan content
    And each round adds only result block (verdict line) to SKILL context

  Scenario: Parallel mode disables AskUserQuestion in plan mode
    Given spec SKILL is invoked with scope: multi
    When impl agent runs in mode=plan with parallel: true
    Then impl agent does not call AskUserQuestion
    And ambiguous items are recorded as warnings in plan.md
