Feature: Compile Target Resolution (Incremental Diff)

  As a developer using /compile,
  I want only changed CLAUDE.md files to be recompiled,
  So that I save time and cost on large projects.

  Background:
    Given a clean git test repository

  Scenario: Staged CLAUDE.md is a compile target
    Given a spec file "src/auth/CLAUDE.md" with basic content
    And I stage "src/auth/CLAUDE.md"
    When I resolve compile targets
    Then "src/auth" should be a compile target with reason "staged"

  Scenario: Staged IMPLEMENTS.md alone does not trigger compile
    Given a committed spec file "src/auth/CLAUDE.md"
    And a committed source file "src/auth/handler.ts" after the spec
    And a spec file "src/auth/IMPLEMENTS.md" with basic content
    And I stage "src/auth/IMPLEMENTS.md"
    When I resolve compile targets
    Then "src/auth" should be skipped with reason "up-to-date"

  Scenario: Untracked CLAUDE.md is a compile target
    Given an untracked spec file "src/utils/CLAUDE.md" with basic content
    When I resolve compile targets
    Then "src/utils" should be a compile target with reason "untracked"

  Scenario: Spec commit newer than source — target (spec-newer)
    Given a committed spec file "src/api/CLAUDE.md"
    And a committed source file "src/api/handler.ts" before the spec
    When I resolve compile targets
    Then "src/api" should be a compile target with reason "spec-newer"

  Scenario: Source commit newer than spec — skipped (up-to-date)
    Given a committed spec file "src/core/CLAUDE.md"
    And a committed source file "src/core/index.ts" after the spec
    When I resolve compile targets
    Then "src/core" should be skipped with reason "up-to-date"

  Scenario: CLAUDE.md with no source files — target (no-source-code)
    Given a committed spec file "src/new/CLAUDE.md"
    And no source files in "src/new"
    When I resolve compile targets
    Then "src/new" should be a compile target with reason "no-source-code"

  Scenario: Non-git directory — warning and empty targets
    Given a non-git test directory
    When I resolve compile targets in the non-git directory
    Then I should get a warning of type "no-git-repo"
    And the targets should be empty

  Scenario: Mixed — staged + spec-newer + up-to-date
    Given a committed spec file "src/mod-a/CLAUDE.md"
    And a committed source file "src/mod-a/index.ts" after the spec
    And a committed spec file "src/mod-b/CLAUDE.md"
    And a committed source file "src/mod-b/service.ts" before the spec
    And a spec file "src/mod-c/CLAUDE.md" with basic content
    And I stage "src/mod-c/CLAUDE.md"
    When I resolve compile targets
    Then "src/mod-a" should be skipped with reason "up-to-date"
    And "src/mod-b" should be a compile target with reason "spec-newer"
    And "src/mod-c" should be a compile target with reason "staged"

  Scenario: Modified (unstaged) CLAUDE.md is a compile target
    Given a committed spec file "src/auth/CLAUDE.md"
    And a committed source file "src/auth/handler.ts" after the spec
    And I modify "src/auth/CLAUDE.md" without staging
    When I resolve compile targets
    Then "src/auth" should be a compile target with reason "modified"

  Scenario: Root-level CLAUDE.md is excluded
    Given a committed spec file "src/auth/CLAUDE.md"
    And a committed source file "src/auth/token.ts" before the spec
    And a committed root-level CLAUDE.md
    When I resolve compile targets
    Then root CLAUDE.md should not be a compile target
    And "src/auth" should be a compile target with reason "spec-newer"

  Scenario: Dependency warning for changed module
    Given a committed spec file "core/domain/CLAUDE.md"
    And a committed source file "core/domain/model.ts" before the spec
    And a committed spec file "src/auth/CLAUDE.md" depending on "core/domain/CLAUDE.md"
    And a committed source file "src/auth/token.ts" after the spec
    When I resolve compile targets
    Then "core/domain" should be a compile target with reason "spec-newer"
    And I should get a dependency warning for "core/domain" affecting "src/auth"
