Feature: validate-schema --index-file option
  Allow pre-built symbol index to be loaded from a JSON file
  to avoid rebuilding the index for each CLAUDE.md file.

  Background:
    Given a clean test directory

  Scenario: Load pre-built index and validate cross-references
    Given CLAUDE.md with content:
      """
      ## Purpose
      Test cross-reference resolution with pre-built index.

      ## Summary
      Module for testing index file loading.

      ## Exports
      - `myFunction(input: string): void`

      ## Behavior
      None

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    And a pre-built symbol index file with symbols:
      | anchor        | module      |
      | myFunction    | src/utils   |
    When I validate schema with the pre-built index file
    Then validation should pass

  Scenario: Detect unresolved cross-references with index file
    Given CLAUDE.md with content:
      """
      <!-- schema: 2.0 -->

      ## Purpose
      Test unresolved cross-reference detection.

      ## Summary
      Module that references a non-existent symbol.

      ## Exports
      None

      ## Behavior
      None

      ## Contract
      None

      ## Protocol
      None

      ## Dependencies

      ### Internal
      - `other/CLAUDE.md#nonExistentSymbol` — does not exist

      ## Domain Context
      None
      """
    And a pre-built symbol index file with symbols:
      | anchor        | module      |
      | realSymbol    | src/other   |
    When I validate schema with the pre-built index file
    Then validation should fail
    And error should mention "Cross-reference"

  Scenario: Index file conflicts with --with-index
    Given CLAUDE.md with content:
      """
      ## Purpose
      Test conflict detection.

      ## Summary
      Conflict test module.

      ## Exports
      None

      ## Behavior
      None

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    Then index-file and with-index options should conflict

  Scenario: Non-existent index file produces error
    Given CLAUDE.md with content:
      """
      ## Purpose
      Test missing file error.

      ## Summary
      Missing file test module.

      ## Exports
      None

      ## Behavior
      None

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    When I validate schema with non-existent index file
    Then an index file error should occur

  Scenario: Corrupted JSON index file produces error
    Given CLAUDE.md with content:
      """
      ## Purpose
      Test corrupted file error.

      ## Summary
      Corrupted file test module.

      ## Exports
      None

      ## Behavior
      None

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    And a corrupted index file
    When I validate schema with the corrupted index file
    Then an index file error should occur
