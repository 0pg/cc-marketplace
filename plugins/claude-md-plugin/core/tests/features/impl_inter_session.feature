Feature: Inter-session spec → dev pipeline
  As a developer or CI system
  I want spec workflow state to persist between sessions
  So that Socratic loop and dev can run in separate processes or machines

  Background:
    Given a project with an spec workflow for target path "src/auth"
    And the dir_safe value is "src-auth"

  Scenario: Workflow state file is written after plan step
    Given spec SKILL executes mode=plan for target "src/auth"
    When the plan step completes successfully
    Then ".claude/workflows/src-auth/state.json" should exist
    And state.json should contain:
      | field       | value              |
      | status      | awaiting-review    |
      | target_path | src/auth           |
      | dir_safe    | src-auth           |
      | round       | 1                  |
    And ".claude/workflows/src-auth/spec-plan.md" should be a copy of the TMP plan artifact

  Scenario: spec-step resumes Socratic loop in a new session
    Given ".claude/workflows/src-auth/state.json" exists with status "awaiting-revise" and round 2
    And ".claude/workflows/src-auth/spec-plan.md" contains the round-1 plan
    And ".claude/workflows/src-auth/reviewer-v1.md" contains the round-1 rejection
    When "/spec-step --target src/auth" is invoked in a new session
    Then spec-step reads state.json and identifies status as "awaiting-revise"
    And spec-step dispatches Task(impl, mode=revise) with session file referencing:
      | field                | value                                           |
      | feedback_file        | .claude/workflows/src-auth/reviewer-v1.md       |
      | existing_plan_file   | .claude/workflows/src-auth/spec-plan.md         |
    And after revise completes, state.json is updated to status "awaiting-review" with round 3
    And ".claude/workflows/src-auth/spec-plan.md" is updated with the revised plan content

  Scenario: dev runs in a separate session after spec completes
    Given spec workflow for "src/auth" completed with status "executed"
    And "src/auth/CLAUDE.md" and "src/auth/DEVELOPERS.md" exist and are committed
    When "/dev --path src/auth" is invoked in a new session
    Then dev reads "src/auth/CLAUDE.md" and "src/auth/DEVELOPERS.md" directly from the filesystem
    And dev does not require any TMP files from the previous spec session
    And source code is generated under "src/auth/"

  Scenario: Auto-commit after execute step includes requirement context
    Given spec SKILL completes mode=execute for target "src/auth" with action "create"
    And the user requirement was "사용자는 JWT 토큰으로 인증할 수 있다"
    When the execute step finishes generating CLAUDE.md and DEVELOPERS.md
    Then spec SKILL creates a git commit with message matching:
      """
      feat(src/auth): create CLAUDE.md + DEVELOPERS.md

      요구사항: 사용자는 JWT 토큰으로 인증할 수 있다
      """
    And the commit includes "src/auth/CLAUDE.md" and "src/auth/DEVELOPERS.md"
    And the commit does NOT include TMP files or workflow state files
