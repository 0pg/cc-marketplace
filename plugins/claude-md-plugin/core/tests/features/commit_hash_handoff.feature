Feature: spec → dev Commit Hash Handoff
  spec records change context via commit message conventions,
  and dev automatically discovers them to perform incremental dev.

  Background:
    Given CLI is installed
    And a git repository is initialized

  # --- Commit Message Convention ---

  Scenario: spec commit message follows the convention
    When spec creates a commit for "src/auth"
    Then the commit message starts with "spec(src/auth):"
    And the commit message body contains a "Changes:" section

  Scenario: BREAKING tag is included when needed
    Given a spec change includes Requirements deletion
    When spec creates a commit
    Then the commit message includes "[BREAKING]"

  Scenario: Transition context is included in the commit message
    When spec creates a commit modifying an existing feature
    Then the commit message body first paragraph contains transition context
    And the transition context describes the direction of the change

  # --- dev's spec commit discovery ---

  Scenario: dev discovers spec commits since the last dev commit
    Given a "dev(src/auth): initial code generation" commit exists
    And a subsequent "spec(src/auth): add OAuth2" commit exists
    When dev runs for src/auth
    Then dev extracts the diff from the spec commit
    And the session file includes a "## Spec Changes" section

  Scenario: If no dev commit exists, search spec from full history
    Given no dev commit exists
    And a "spec(src/auth): initial requirements" commit exists
    When dev runs for src/auth
    Then dev searches for spec commits from the full history

  Scenario: If no spec commit exists, fallback to existing diff-compile-targets
    Given no dev commit exists
    And no spec commit exists
    When dev runs
    Then it falls back to existing diff-compile-targets behavior
    And the session file does not contain a "## Spec Changes" section

  Scenario: Manual edits are not picked up by spec discovery
    Given a "dev(src/auth): code generation" commit exists
    And a subsequent manual "fix: typo fix" commit exists
    And a subsequent "spec(src/auth): add feature" commit exists
    When dev runs for src/auth
    Then manual commits are ignored and only spec commits are processed

  Scenario: Diffs from multiple spec commits are aggregated
    Given a "dev(src/auth): code generation" commit exists
    And 3 subsequent spec(src/auth) commits exist
    When dev runs for src/auth
    Then diffs from all 3 spec commits are included in Spec Changes

  # --- Spec Changes Session File ---

  Scenario: Spec Changes includes Transition Context
    Given the spec commit message contains transition context
    When dev creates the session file
    Then the Spec Changes "### Transition Context" includes the transition context

  Scenario: Spec Changes categorizes Added/Modified/Removed
    Given the spec commit Changes include added, modified, and removed items
    When dev creates the session file
    Then Spec Changes contains "### Added", "### Modified", and "### Removed" sections

  Scenario: BREAKING spec commit includes breaking metadata
    Given the spec commit has a "[BREAKING]" tag
    When dev creates the session file
    Then Spec Changes includes "breaking: true"

  # --- dev SKILL Step 6: Implementation Tasks derivation ---

  Scenario: Spec Changes present triggers Implementation Tasks derivation
    Given the session file contains a "## Spec Changes" section
    When dev SKILL executes Step 6e
    Then it derives Implementation Tasks ([ADD]/[MODIFY]/[DELETE])
    And the session file includes a "## Implementation Tasks" section

  Scenario: No Spec Changes means Implementation Tasks are omitted
    Given the session file does not contain a "## Spec Changes" section
    When dev SKILL creates the session file
    Then it proceeds with the full TDD pipeline without a "## Implementation Tasks" section

  Scenario: No semantic changes results in early dev termination
    Given Spec Changes Added, Modified, and Removed are all empty
    When dev SKILL executes Step 6e
    Then it determines "nothing to do"
    And terminates early with status: skipped

  Scenario: BREAKING flag forces conflict overwrite mode
    Given Spec Changes contains "breaking: true"
    When dev SKILL executes Step 6e
    Then it forces conflict mode to overwrite

  Scenario: SKILL executes DELETE tasks directly
    Given Step 6e derived [DELETE] tasks
    When dev SKILL executes Step 6f
    Then the target code is deleted
    And import/call-site references are cleaned up
    And related tests are removed or updated
    And this completes before the TDD pipeline (Steps 7-9)

  # --- post-dev commit ---

  Scenario: dev completion commit message follows the convention
    When dev completes successfully
    Then the commit message starts with "dev(src/auth):"
