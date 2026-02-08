Feature: Uncovered Agents Prompt Validation
  As a plugin developer
  I want to validate the 3 uncovered agent prompts (recursive-decompiler, drift-validator, export-validator)
  So that all agents have consistent frontmatter, valid result blocks, and correct cross-references

  Background:
    Given a clean prompt test directory

  # === recursive-decompiler ===

  Scenario: recursive-decompiler valid frontmatter
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: recursive-decompiler
      description: Recursive orchestrator agent for decompile workflow
      tools: [Bash, Read, Glob, Grep, Write, Task, Skill]
      ---
      System prompt body
      """
    When I validate prompts
    Then prompt validation should pass
    And agents count should be 1

  Scenario: recursive-decompiler result block valid
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: recursive-decompiler
      description: Recursive orchestrator for decompile
      tools: [Read, Write, Task, Skill]
      ---

      ---recursive-decompiler-result---
      status: approve
      ---end-recursive-decompiler-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: recursive-decompiler self and decompiler Task refs resolved
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: recursive-decompiler
      description: Recursive orchestrator for decompile
      tools: [Read, Write, Task, Skill]
      ---
      Task(recursive-decompiler, subdir)
      Task(decompiler, target)
      """
    And an agent file "decompiler.md":
      """
      ---
      name: decompiler
      description: Decompile single directory
      tools: [Read, Write, Skill]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 2 task references
    And cross-reference summary should show 0 unresolved task references

  Scenario: recursive-decompiler boundary-resolve Skill ref resolved
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: recursive-decompiler
      description: Recursive orchestrator for decompile
      tools: [Read, Write, Task, Skill]
      ---
      Skill("boundary-resolve")
      """
    And a skill directory "boundary-resolve" with SKILL.md:
      """
      ---
      name: boundary-resolve
      description: Resolve boundary
      allowed-tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 skill reference
    And cross-reference summary should show 0 unresolved skill references

  Scenario: recursive-decompiler missing Task target fails
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: recursive-decompiler
      description: Recursive orchestrator for decompile
      tools: [Read, Write, Task]
      ---
      Task(decompiler)
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "decompiler"
    And cross-reference summary should show 1 task reference
    And cross-reference summary should show 1 unresolved task reference

  Scenario: recursive-decompiler name mismatch fails
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: wrong-decompiler
      description: Recursive orchestrator for decompile
      tools: [Read, Write]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "does not match filename"

  # === drift-validator ===

  Scenario: drift-validator valid frontmatter
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates consistency between CLAUDE.md and actual code
      tools: [Glob, Grep, Read, Write, Bash, Skill]
      color: yellow
      ---
      System prompt body
      """
    When I validate prompts
    Then prompt validation should pass
    And agents count should be 1

  Scenario: drift-validator result block with pipe status
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates CLAUDE.md-code consistency
      tools: [Read, Write, Skill]
      ---

      ---drift-validator-result---
      status: approve | error
      ---end-drift-validator-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: drift-validator Skill ref claude-md-parse resolved
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates CLAUDE.md-code consistency
      tools: [Read, Write, Skill]
      ---
      Skill("claude-md-parse")
      """
    And a skill directory "claude-md-parse" with SKILL.md:
      """
      ---
      name: claude-md-parse
      description: Parse CLAUDE.md
      allowed-tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 skill reference
    And cross-reference summary should show 0 unresolved skill references

  Scenario: drift-validator unmatched end delimiter fails
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates CLAUDE.md-code consistency
      tools: [Read, Write]
      ---

      status: approve
      ---end-drift-validator-result---
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "no matching start delimiter"

  Scenario: drift-validator without tools warning
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates CLAUDE.md-code consistency
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass with warnings
    And prompt warning should mention "tools"

  # === export-validator ===

  Scenario: export-validator valid frontmatter
    Given an agent file "export-validator.md":
      """
      ---
      name: export-validator
      description: Validates if CLAUDE.md exports exist in codebase
      tools: [Read, Write, Glob, Grep]
      color: cyan
      ---
      System prompt body
      """
    When I validate prompts
    Then prompt validation should pass
    And agents count should be 1

  Scenario: export-validator result block valid
    Given an agent file "export-validator.md":
      """
      ---
      name: export-validator
      description: Validates exports existence
      tools: [Read, Write, Glob, Grep]
      ---

      ---export-validator-result---
      status: approve | error
      ---end-export-validator-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: export-validator invalid tool fails
    Given an agent file "export-validator.md":
      """
      ---
      name: export-validator
      description: Validates exports existence
      tools: [Read, Write, FileSystem]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "FileSystem"

  # === Workflow Cross-References ===

  Scenario: validate skill references drift-validator and export-validator and code-reviewer
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates CLAUDE.md-code consistency
      tools: [Read, Write]
      ---
      Body
      """
    And an agent file "export-validator.md":
      """
      ---
      name: export-validator
      description: Validates exports existence
      tools: [Read, Write]
      ---
      Body
      """
    And an agent file "code-reviewer.md":
      """
      ---
      name: code-reviewer
      description: Reviews code quality
      tools: [Read, Write]
      ---
      Body
      """
    And a skill directory "validate" with SKILL.md:
      """
      ---
      name: validate
      description: Validate document-code consistency
      allowed-tools: [Task]
      ---
      Task(drift-validator, prompt="validate")
      Task(export-validator, prompt="validate")
      Task(code-reviewer, prompt="review")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 3 task references
    And cross-reference summary should show 0 unresolved task references

  Scenario: validate skill missing drift-validator fails
    Given an agent file "export-validator.md":
      """
      ---
      name: export-validator
      description: Validates exports existence
      tools: [Read, Write]
      ---
      Body
      """
    And a skill directory "validate" with SKILL.md:
      """
      ---
      name: validate
      description: Validate document-code consistency
      allowed-tools: [Task]
      ---
      Task(drift-validator, prompt="validate")
      Task(export-validator, prompt="validate")
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "drift-validator"
    And cross-reference summary should show 2 task references
    And cross-reference summary should show 1 unresolved task reference

  Scenario: decompile skill references recursive-decompiler resolved
    Given an agent file "recursive-decompiler.md":
      """
      ---
      name: recursive-decompiler
      description: Recursive orchestrator for decompile
      tools: [Read, Write, Task]
      ---
      Body
      """
    And a skill directory "decompile" with SKILL.md:
      """
      ---
      name: decompile
      description: Decompile source code to CLAUDE.md
      allowed-tools: [Task]
      ---
      Task(recursive-decompiler, root_path)
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 task reference
    And cross-reference summary should show 0 unresolved task references

  Scenario: decompile skill missing recursive-decompiler fails
    Given a skill directory "decompile" with SKILL.md:
      """
      ---
      name: decompile
      description: Decompile source code to CLAUDE.md
      allowed-tools: [Task]
      ---
      Task(recursive-decompiler)
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "recursive-decompiler"
    And cross-reference summary should show 1 task reference
    And cross-reference summary should show 1 unresolved task reference
