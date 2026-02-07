Feature: Git blob hash based cache invalidation
  As a developer
  I want cache invalidation based on file content hashes instead of mtimes
  So that cache is accurate regardless of file timestamp changes (e.g. git checkout, touch)

  Scenario: Content change invalidates cache
    Given a project with cached symbol index
    When I modify "auth/CLAUDE.md" content
    And I build the symbol index with cache
    Then the modified file should be re-indexed

  Scenario: Cache hit when content unchanged (touch only)
    Given a project with cached symbol index
    When I touch "auth/CLAUDE.md" without changing content
    And I build the symbol index with cache
    Then the cache should be hit

  Scenario: Cache version mismatch triggers full rebuild
    Given a cache file with version 2 (mtime-based)
    When I build the symbol index with cache
    Then a full rebuild should occur
    And the new cache should have version 3

  Scenario: Graceful fallback when git is unavailable
    Given a project with CLAUDE.md files
    And git is not available in PATH
    When I build the symbol index with cache
    Then a full rebuild should occur without errors
