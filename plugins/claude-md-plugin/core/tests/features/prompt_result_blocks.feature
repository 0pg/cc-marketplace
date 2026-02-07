Feature: Result Block Validation
  As a plugin developer
  I want to validate result block format in agent prompts
  So that result blocks have matching delimiters and valid status values

  Background:
    Given a clean prompt test directory

  # === Happy Path ===

  Scenario: Valid result block with single status
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      ---compiler-result---
      status: approve
      ---end-compiler-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: Valid result block with multiple status values (template)
    Given an agent file "test-reviewer.md":
      """
      ---
      name: test-reviewer
      description: Review tests
      tools: [Read]
      ---

      ---test-reviewer-result---
      status: approve | feedback
      ---end-test-reviewer-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: Valid result block with brace-wrapped status
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      ---compiler-result---
      status: {approve|warning}
      ---end-compiler-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: Multiple result blocks in same file
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      Example 1:
      ---compiler-result---
      status: approve
      ---end-compiler-result---

      Example 2:
      ---compiler-result---
      status: warning
      ---end-compiler-result---
      """
    When I validate prompts
    Then prompt validation should pass

  # === Missing Delimiters ===

  Scenario: Result block missing end delimiter
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      ---compiler-result---
      status: approve
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "no matching end delimiter"

  Scenario: End delimiter without start
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      status: approve
      ---end-compiler-result---
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "no matching start delimiter"

  # === Missing Status ===

  Scenario: Result block without status field
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      ---compiler-result---
      output: some data
      ---end-compiler-result---
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "missing 'status' field"

  # === Invalid Status Values ===

  Scenario: Result block with invalid status value
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---

      ---compiler-result---
      status: invalid-status
      ---end-compiler-result---
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "invalid-status"
