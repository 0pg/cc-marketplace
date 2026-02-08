Feature: Uncovered Skills Prompt Validation
  As a plugin developer
  I want to validate the 3 uncovered skill prompts (commit-comparator, interface-diff, dependency-tracker)
  So that all skills have consistent frontmatter, valid result blocks, and correct cross-references

  Background:
    Given a clean prompt test directory

  # === commit-comparator ===

  Scenario: commit-comparator valid frontmatter
    Given a skill directory "commit-comparator" with SKILL.md:
      """
      ---
      name: commit-comparator
      version: 1.1.0
      description: Compares spec vs source commit timestamps
      allowed-tools: [Bash, Read, Write, Glob]
      ---
      Body content
      """
    When I validate prompts
    Then prompt validation should pass
    And skills count should be 1

  Scenario: commit-comparator result block valid
    Given a skill directory "commit-comparator" with SKILL.md:
      """
      ---
      name: commit-comparator
      description: Compares commit timestamps
      allowed-tools: [Bash, Read, Write, Glob]
      ---

      ---commit-comparator-result---
      status: approve
      ---end-commit-comparator-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: commit-comparator invalid tool fails
    Given a skill directory "commit-comparator" with SKILL.md:
      """
      ---
      name: commit-comparator
      description: Compares commit timestamps
      allowed-tools: [Bash, Read, GitTool]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "GitTool"

  Scenario: commit-comparator name mismatch fails
    Given a skill directory "commit-comparator" with SKILL.md:
      """
      ---
      name: wrong-comparator
      description: Compares commit timestamps
      allowed-tools: [Bash, Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "does not match directory"

  # === interface-diff ===

  Scenario: interface-diff valid frontmatter
    Given a skill directory "interface-diff" with SKILL.md:
      """
      ---
      name: interface-diff
      version: 1.1.0
      description: Detects interface changes by comparing exports
      allowed-tools: [Read, Write, Grep, Glob]
      ---
      Body content
      """
    When I validate prompts
    Then prompt validation should pass
    And skills count should be 1

  Scenario: interface-diff result block valid
    Given a skill directory "interface-diff" with SKILL.md:
      """
      ---
      name: interface-diff
      description: Detects interface changes
      allowed-tools: [Read, Write, Grep, Glob]
      ---

      ---interface-diff-result---
      status: approve
      ---end-interface-diff-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: interface-diff missing description fails
    Given a skill directory "interface-diff" with SKILL.md:
      """
      ---
      name: interface-diff
      allowed-tools: [Read, Write]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  # === dependency-tracker ===

  Scenario: dependency-tracker valid frontmatter
    Given a skill directory "dependency-tracker" with SKILL.md:
      """
      ---
      name: dependency-tracker
      version: 1.1.0
      description: Tracks module dependencies and analyzes impact
      allowed-tools: [Read, Write, Glob, Grep]
      ---
      Body content
      """
    When I validate prompts
    Then prompt validation should pass
    And skills count should be 1

  Scenario: dependency-tracker result block valid
    Given a skill directory "dependency-tracker" with SKILL.md:
      """
      ---
      name: dependency-tracker
      description: Tracks module dependencies
      allowed-tools: [Read, Write, Glob, Grep]
      ---

      ---dependency-tracker-result---
      status: approve
      ---end-dependency-tracker-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: dependency-tracker invalid status fails
    Given a skill directory "dependency-tracker" with SKILL.md:
      """
      ---
      name: dependency-tracker
      description: Tracks module dependencies
      allowed-tools: [Read, Write, Glob, Grep]
      ---

      ---dependency-tracker-result---
      status: completed
      ---end-dependency-tracker-result---
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "completed"

  # === Cross-Reference ===

  Scenario: compile references 3 incremental skills resolved
    Given a skill directory "commit-comparator" with SKILL.md:
      """
      ---
      name: commit-comparator
      description: Compares timestamps
      allowed-tools: [Bash, Read, Write, Glob]
      ---
      Body
      """
    And a skill directory "interface-diff" with SKILL.md:
      """
      ---
      name: interface-diff
      description: Detects interface changes
      allowed-tools: [Read, Write, Grep, Glob]
      ---
      Body
      """
    And a skill directory "dependency-tracker" with SKILL.md:
      """
      ---
      name: dependency-tracker
      description: Tracks dependencies
      allowed-tools: [Read, Write, Glob, Grep]
      ---
      Body
      """
    And an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write, Task, Bash]
      ---
      Body
      """
    And an agent file "test-reviewer.md":
      """
      ---
      name: test-reviewer
      description: Review tests
      tools: [Read]
      ---
      Body
      """
    And a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md to source code
      allowed-tools: [Task, Skill]
      ---
      Task(compiler, phase=red)
      Task(test-reviewer)
      Task(compiler, phase=green-refactor)
      git status --porcelain for uncommitted analysis
      Skill("commit-comparator")
      Skill("interface-diff")
      Skill("dependency-tracker")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 3 task references
    And cross-reference summary should show 0 unresolved task references
    And cross-reference summary should show 3 skill references
    And cross-reference summary should show 0 unresolved skill references

  Scenario: compile with missing incremental skills has unresolved refs
    Given a skill directory "commit-comparator" with SKILL.md:
      """
      ---
      name: commit-comparator
      description: Compares timestamps
      allowed-tools: [Bash, Read, Write, Glob]
      ---
      Body
      """
    And an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Write]
      ---
      Body
      """
    And a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md to source code
      allowed-tools: [Task, Skill]
      ---
      Task(compiler)
      Skill("commit-comparator")
      Skill("interface-diff")
      Skill("dependency-tracker")
      """
    When I validate prompts
    Then prompt validation should fail
    And cross-reference summary should show 3 skill references
    And cross-reference summary should show 2 unresolved skill references
