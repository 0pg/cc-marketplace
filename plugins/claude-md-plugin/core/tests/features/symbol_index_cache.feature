Feature: Symbol Index Caching with Incremental Rebuild
  As a developer
  I want symbol index to be cached and incrementally updated
  So that repeated lookups are instant even with hundreds of CLAUDE.md files

  Scenario: First build creates cache file
    Given a project with CLAUDE.md files and no cache
    When I build the symbol index with cache
    Then ".claude/.cache/symbol-index.json" should exist
    And the cache should contain all indexed symbols
    And the cache should contain file_hashes for each CLAUDE.md

  Scenario: Cache hit when files unchanged
    Given a project with a valid cache
    And no CLAUDE.md files have been modified
    When I build the symbol index with cache
    Then the result should be loaded from cache
    And no CLAUDE.md files should be parsed

  Scenario: Incremental rebuild on single file modification
    Given a project with 10 CLAUDE.md files and a valid cache
    When I modify "auth/CLAUDE.md" adding export "newFunction"
    And I build the symbol index with cache
    Then only "auth/CLAUDE.md" should be re-parsed
    And the other 9 files should NOT be re-parsed
    And the index should contain "newFunction"
    And references should be re-resolved

  Scenario: Incremental rebuild on file addition
    Given a project with a valid cache
    When I add a new "payments/CLAUDE.md" with export "processPayment"
    And I build the symbol index with cache
    Then only "payments/CLAUDE.md" should be parsed
    And existing symbols should be preserved
    And "processPayment" should appear in the index

  Scenario: Incremental rebuild on file removal
    Given a project with a valid cache containing "legacy/CLAUDE.md"
    When I delete "legacy/CLAUDE.md"
    And I build the symbol index with cache
    Then symbols from "legacy/CLAUDE.md" should be removed
    And references to those symbols should be marked unresolved

  Scenario: Force rebuild with --no-cache
    Given a project with a valid cache
    When I build the symbol index with "--no-cache"
    Then the cache should be rebuilt from scratch

  Scenario: Corrupted cache triggers full rebuild
    Given a project with a corrupted cache file
    When I build the symbol index with cache
    Then the index should be built successfully
    And the cache should be replaced

  Scenario: Sequential incremental rebuilds preserve symbol integrity
    Given a project with modules "auth", "payments", "api" each exporting one symbol
    And the symbol index is built with cache
    When I modify "auth/CLAUDE.md" changing its export
    And I rebuild the symbol index with cache
    Then the index should contain the new auth symbol
    And the index should still contain payments and api symbols unchanged
    When I modify "payments/CLAUDE.md" changing its export
    And I rebuild the symbol index with cache
    Then the index should contain the new payments symbol
    And the correct auth and api symbols should be preserved
