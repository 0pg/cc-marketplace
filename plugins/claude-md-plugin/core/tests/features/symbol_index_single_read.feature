Feature: Symbol index single file read (P0.4)

  Scenario: build() reads each CLAUDE.md file exactly once
    Given a project with 3 CLAUDE.md files
    When I build the symbol index
    Then each file should be read exactly once
    And all symbols should be extracted correctly
    And all cross-references should be resolved

  Scenario: incremental_rebuild reads changed files once
    Given a cached symbol index
    And one CLAUDE.md file has been modified
    When I perform incremental rebuild
    Then the modified file should be read exactly once
    And unchanged files should be read at most once for reference extraction
