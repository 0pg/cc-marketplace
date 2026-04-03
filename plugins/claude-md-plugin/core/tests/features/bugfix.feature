Feature: /bugfix — 3-Layer Bug Root Cause Analysis and Fix
  As a developer using claude-md-plugin,
  I want /bugfix to trace bugs through CLAUDE.md, DEVELOPERS.md, and source code,
  So that fixes are applied at the highest affected layer and CLAUDE.md remains the SSOT.

  Background:
    Given a project with CLAUDE.md and DEVELOPERS.md in "src/auth"
    And CLAUDE.md has Requirements:
      | id    | text                                        |
      | REQ-1 | authenticate(token) returns User on success  |
      | REQ-2 | expired tokens are rejected with AuthError   |
    And DEVELOPERS.md has Constraints:
      | id      | text                                              |
      | CONST-1 | authenticate(token: string) → User \| AuthError   |
      | CONST-2 | token expiry: max 7 days, reject at day 8+        |

  # ─────────────────────────────────────────────
  # Judgment Algorithm: 자명한 케이스
  # ─────────────────────────────────────────────

  Scenario: E == A — not_a_bug (bug does not exist or already fixed)
    Given user reports expected "AuthError on expired token", actual "AuthError on expired token"
    When bugfixer agent applies judgment algorithm
    Then result status is "not_a_bug"
    And judgment is "unambiguous"
    And no fix is applied

  Scenario: E == S AND A != S — Layer 3 autonomous fix
    Given CLAUDE.md REQ-2 says "expired tokens are rejected with AuthError"
    And user reports expected "AuthError on expired token", actual "User returned on expired token"
    And diff-spec-range shows source_changed=false and changed_requirements is empty
    When bugfixer agent applies judgment algorithm
    Then root_cause_layer is "3"
    And judgment is "unambiguous"
    And agent writes a failing test reproducing the bug
    And agent fixes source code to make the test pass
    And test_result is "passed"

  Scenario: spec stale — changed_requirements not empty, source_changed=false
    Given diff-spec-range shows changed_requirements=["REQ-2 modified"] and source_changed=false
    And user reports expected "AuthError on expired token", actual "User returned on expired token"
    When bugfixer agent applies judgment algorithm
    Then judgment is "unambiguous"
    And fix_type is "none" (no manual code fix — /dev rerun handles it)
    And SKILL runs /dev to regenerate code rather than a manual code fix
    And no separate bugfix commit is created

  Scenario: source diverged — source_changed=true, changed_requirements empty, A != S
    Given diff-spec-range shows source_changed=true and changed_requirements is empty
    And user reports expected "AuthError on expired token", actual "null returned"
    When bugfixer agent applies judgment algorithm
    Then root_cause_layer is "3"
    And judgment is "unambiguous"

  # ─────────────────────────────────────────────
  # Judgment Algorithm: 모호한 케이스 → escalation
  # ─────────────────────────────────────────────

  Scenario: S == null — CLAUDE.md requirement missing → ambiguous escalation
    Given CLAUDE.md has no Requirement matching the reported behavior
    And user reports expected "token refresh on near-expiry", actual "no refresh happens"
    When bugfixer agent applies judgment algorithm
    Then judgment is "ambiguous"
    And result status is "escalated"
    And escalation reason mentions "S == null"
    And escalation choices include "A", "B", "C"

  Scenario: E != S AND A == S — code matches spec but user expectation differs → ambiguous
    Given CLAUDE.md REQ-2 says "tokens expire after 7 days"
    And user reports expected "tokens expire after 30 days", actual "tokens expire after 7 days"
    When bugfixer agent applies judgment algorithm
    Then judgment is "ambiguous"
    And result status is "escalated"
    And escalation choices include "A", "B", "C"
    And escalation reason mentions "code matches spec but user expectation differs"
    And escalation cites "REQ-2" directly

  Scenario: E != S AND S is explicit — ambiguous escalation
    Given CLAUDE.md REQ-1 says "returns User on success"
    And user reports expected "returns UserDTO on success", actual "returns User on success"
    When bugfixer agent applies judgment algorithm
    Then judgment is "ambiguous"
    And result status is "escalated"
    And escalation choices include "A", "B", "C"
    And escalation cites the relevant Requirement text

  Scenario: all_requirements=true — no git context → ambiguous
    Given diff-spec-range shows all_requirements=true (not a git repo or first commit)
    And user reports a bug with unclear git history
    When bugfixer agent applies judgment algorithm
    Then judgment is "ambiguous"
    And result status is "escalated"
    And escalation choices include "A", "B", "C"
    And escalation reason mentions "no git context"

  Scenario: E itself is unclear — SKILL asks for clarification before dispatch
    Given user reports a bug with vague description "login is broken"
    And expected behavior is not specified
    When SKILL receives the bug report
    Then SKILL calls AskUserQuestion to clarify expected behavior
    And SKILL does not dispatch bugfixer agent until E is clarified

  Scenario: Multiple conflicting requirements — ambiguous escalation
    Given CLAUDE.md has REQ-1 saying "returns User on success"
    And CLAUDE.md has REQ-3 saying "returns null when user has no active subscription"
    And user reports expected "User returned", actual "null returned for active user"
    When bugfixer agent applies judgment algorithm
    Then judgment is "ambiguous"
    And result status is "escalated"
    And escalation reason mentions "conflicting requirements"
    And escalation choices include "A", "B", "C"

  # ─────────────────────────────────────────────
  # Fix paths — SKILL behavior
  # ─────────────────────────────────────────────

  Scenario: Layer 1 fix — SKILL requires user approval before CLAUDE.md modification
    Given bugfixer returns root_cause_layer="1" with judgment="unambiguous"
    When SKILL processes the result
    Then SKILL calls AskUserQuestion with proposed CLAUDE.md change before modifying
    And user approval is required even though judgment is "unambiguous" (INV-bugfix-2)
    And SKILL modifies CLAUDE.md after user approval
    And SKILL creates a spec commit
    And SKILL runs /dev to regenerate code
    And no separate bugfix commit is created

  Scenario: Layer 2 fix — SKILL requires user approval before DEVELOPERS.md modification
    Given bugfixer returns root_cause_layer="2" with judgment="unambiguous"
    When SKILL processes the result
    Then SKILL calls AskUserQuestion with proposed DEVELOPERS.md change before modifying
    And user approval is required even though judgment is "unambiguous" (INV-bugfix-2)
    And SKILL modifies DEVELOPERS.md after user approval
    And SKILL runs /dev to regenerate code
    And no separate bugfix commit is created

  Scenario: Layer 3 fix — agent completes autonomously, SKILL commits
    Given bugfixer returns root_cause_layer="3", judgment="unambiguous", test_result="passed"
    When SKILL processes the result
    Then SKILL reads test_result from result block (agent already completed fix)
    And SKILL creates a bugfix commit

  Scenario: ambiguous — SKILL presents escalation format to user
    Given bugfixer returns judgment="ambiguous" with escalation context
    When SKILL processes the result
    Then SKILL presents AskUserQuestion with three choices A, B, C
    And choice A triggers Fix-Highest-Layer-First path (CLAUDE.md edit first, then /dev)
    And choice B triggers spec addition path (new Requirement → spec commit → /dev)
    And choice C exits with "not a bug" message

  Scenario: multi-layer — SKILL processes L1 first, then L3 if needed
    Given bugfixer returns root_cause_layer="multi"
    When SKILL processes the result
    Then SKILL processes Layer 1 fix first (user approval → CLAUDE.md → spec commit → /dev)
    And SKILL checks if Layer 3 issue remains after /dev
    And SKILL applies Layer 3 fix only if residual code issue exists

  Scenario: not_a_bug — SKILL notifies user and exits
    Given bugfixer returns status="not_a_bug"
    When SKILL processes the result
    Then SKILL displays message that behavior matches spec
    And SKILL exits without making any changes
