Feature: v1 to v2 Migration
  As a project maintainer
  I want to migrate CLAUDE.md files from v1 to v2 format
  So that I can use v2 features like symbol indexing and diagram generation

  # =============================================================================
  # Version Marker
  # =============================================================================

  Scenario: Add schema version marker to v1 file
    Given a v1 CLAUDE.md file without version marker
    When I run migration
    Then the file should start with "<!-- schema: 2.0 -->"
    And the migration result should include change "AddVersionMarker"

  # =============================================================================
  # Export Format Conversion
  # =============================================================================

  Scenario: Convert bullet exports to heading format
    Given a v1 CLAUDE.md file with exports:
      """
      ### Functions
      - `validateToken(token: string): Claims`
      - `issueToken(userId: string): string`
      """
    When I run migration
    Then the exports should have heading "#### validateToken"
    And the exports should have heading "#### issueToken"
    And the migration result should include change "ConvertExportToHeading"

  # =============================================================================
  # Dry Run
  # =============================================================================

  Scenario: Dry run does not modify files
    Given a v1 CLAUDE.md file
    When I run migration with --dry-run
    Then the original file should be unchanged
    And the migration result should list proposed changes

  # =============================================================================
  # Already Migrated
  # =============================================================================

  Scenario: Skip already v2 files
    Given a CLAUDE.md file with "<!-- schema: 2.0 -->" marker
    When I run migration
    Then the file should not be modified
    And the migration result should say "Already at schema version 2.0"

  # =============================================================================
  # Suggestions
  # =============================================================================

  Scenario: Suggest Actor/UC structure for Behavior section
    Given a v1 CLAUDE.md file with Behavior section but no Actors
    When I run migration
    Then the migration result should suggest adding Actors and UC sections
