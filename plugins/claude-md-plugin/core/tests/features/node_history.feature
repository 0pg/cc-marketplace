Feature: Node History - Section-level diffs across recent commits

  As a SKILL creating session files,
  I want section-level change context from recent commits touching a node's CLAUDE.md/DEVELOPERS.md,
  So that agents understand how the node has evolved before performing their work.

  Background:
    Given a clean git test repository for node history

  Scenario: Single commit touching CLAUDE.md Requirements
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: User login"
    And a new commit changing Requirements in "src/auth/CLAUDE.md" to "- REQ-1: User login\n- REQ-2: Token refresh"
    When I run diff-node-history for "src/auth" with limit 5
    Then the result has 2 commit entries
    And commit 0 has a "CLAUDE.md" file diff
    And the "CLAUDE.md" diff in commit 0 has section "Requirements" with 1 "added" change

  Scenario: Multiple commits with limit
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And 4 additional commits changing "src/auth/CLAUDE.md" Requirements
    When I run diff-node-history for "src/auth" with limit 3
    Then the result has 3 commit entries
    And total_commits_found is 5

  Scenario: CLAUDE.md and DEVELOPERS.md changed in same commit
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And a committed DEVELOPERS.md at "src/auth" with Constraints "- CONST-1: JWT"
    And a new commit changing both "src/auth/CLAUDE.md" and "src/auth/DEVELOPERS.md"
    When I run diff-node-history for "src/auth" with limit 5
    Then commit 0 has a "CLAUDE.md" file diff
    And commit 0 has a "DEVELOPERS.md" file diff

  Scenario: Multiple H2 sections changed in one commit
    Given a committed CLAUDE.md at "src/auth" with Purpose "Auth module" and Requirements "- REQ-1: Login"
    And a new commit changing both Purpose and Requirements in "src/auth/CLAUDE.md"
    When I run diff-node-history for "src/auth" with limit 1
    Then the "CLAUDE.md" diff in commit 0 has section "Purpose"
    And the "CLAUDE.md" diff in commit 0 has section "Requirements"

  Scenario: grep filter for spec commits only
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And a commit with message "spec(src/auth): add OAuth" changing "src/auth/CLAUDE.md"
    And a commit with message "fix: typo in auth" changing "src/auth/CLAUDE.md"
    When I run diff-node-history for "src/auth" with limit 10 and grep "^spec(src/auth):"
    Then the result has 1 commit entry
    And commit 0 subject contains "spec(src/auth): add OAuth"

  Scenario: since-commit filter excludes older commits
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And a commit "A" changing "src/auth/CLAUDE.md" Requirements
    And a commit "B" changing "src/auth/CLAUDE.md" Requirements
    When I run diff-node-history for "src/auth" with limit 10 and since-commit "A"
    Then the result has 1 commit entry
    And commit 0 subject matches commit "B"

  Scenario: Non-git directory returns empty result
    Given a non-git test directory for node history
    When I run diff-node-history for "src/auth" with limit 5 in the non-git directory
    Then is_git_repo is false
    And has_history is false
    And the result has 0 commit entries

  Scenario: No commits touching the node
    Given an empty git repository for node history
    When I run diff-node-history for "src/auth" with limit 5
    Then has_history is false
    And the result has 0 commit entries

  Scenario: Root commit handling (no parent)
    Given a single root commit creating "src/auth/CLAUDE.md" with Requirements "- REQ-1: Login"
    When I run diff-node-history for "src/auth" with limit 5
    Then the result has 1 commit entry
    And the "CLAUDE.md" diff in commit 0 has section "Requirements" with 1 "added" change

  Scenario: BREAKING flag detection in commit message
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And a commit with message "spec(src/auth): remove login [BREAKING]" changing "src/auth/CLAUDE.md"
    When I run diff-node-history for "src/auth" with limit 5
    Then commit 0 has breaking flag true

  Scenario: Source changed files since oldest commit
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And a new commit changing Requirements in "src/auth/CLAUDE.md" to "- REQ-1: Login\n- REQ-2: OAuth"
    And a committed source file "src/auth/handler.ts" after the spec commit
    When I run diff-node-history for "src/auth" with limit 5
    Then source_changed is true
    And source_changed_files includes "src/auth/handler.ts"

  Scenario: Commit body preserved for transition context
    Given a committed CLAUDE.md at "src/auth" with Requirements "- REQ-1: Login"
    And a commit with subject "spec(src/auth): add OAuth" and body "Transition: basic auth to OAuth2" changing "src/auth/CLAUDE.md"
    When I run diff-node-history for "src/auth" with limit 5
    Then commit 0 body contains "Transition: basic auth to OAuth2"
