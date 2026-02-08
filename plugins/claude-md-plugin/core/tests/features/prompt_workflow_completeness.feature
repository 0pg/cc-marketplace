Feature: Workflow Completeness Validation
  As a plugin developer
  I want to validate actual project prompt files and workflow cross-reference completeness
  So that all skills and agents pass validation and all references are resolved

  Background:
    Given a clean prompt test directory

  # === Actual Project File Validation ===

  Scenario: All 16 project skills pass validation with agents loaded
    Given the actual project skills directory is loaded
    And the actual project agents directory is loaded
    When I validate prompts
    Then prompt validation should pass
    And skills count should be 16

  Scenario: All 9 project agents pass validation with skills loaded
    Given the actual project skills directory is loaded
    And the actual project agents directory is loaded
    When I validate prompts
    Then prompt validation should pass
    And agents count should be 9

  Scenario: All project cross-references resolved
    Given the actual project skills directory is loaded
    And the actual project agents directory is loaded
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 0 unresolved task references
    And cross-reference summary should show 0 unresolved skill references

  # === Uncovered Skill Workflow Sections (Scenario Outline) ===

  Scenario Outline: Skill <skill_name> has valid result block
    Given a skill directory "<skill_name>" with SKILL.md:
      """
      ---
      name: <skill_name>
      description: Skill description
      allowed-tools: [Read, Write]
      ---

      ---<skill_name>-result---
      status: approve
      ---end-<skill_name>-result---
      """
    When I validate prompts
    Then prompt validation should pass

    Examples:
      | skill_name            |
      | git-status-analyzer   |
      | commit-comparator     |
      | interface-diff        |
      | dependency-tracker    |

  # === Uncovered Agent Result Blocks (Scenario Outline) ===

  Scenario Outline: Agent <agent_name> has valid result block
    Given an agent file "<agent_name>.md":
      """
      ---
      name: <agent_name>
      description: Agent description
      tools: [Read, Write]
      ---

      ---<agent_name>-result---
      status: approve
      ---end-<agent_name>-result---
      """
    When I validate prompts
    Then prompt validation should pass

    Examples:
      | agent_name            |
      | recursive-decompiler  |
      | drift-validator       |
      | export-validator      |

  # === Workflow Reference Chains ===

  Scenario: spec skill references spec-agent and schema-validate
    Given an agent file "spec-agent.md":
      """
      ---
      name: spec-agent
      description: Generate spec from requirements
      tools: [Read, Write, Task, Skill]
      ---
      Body
      """
    And a skill directory "schema-validate" with SKILL.md:
      """
      ---
      name: schema-validate
      description: Validate schema
      allowed-tools: [Read]
      ---
      Body
      """
    And a skill directory "spec" with SKILL.md:
      """
      ---
      name: spec
      description: Generate CLAUDE.md from requirements
      allowed-tools: [Task, Skill]
      ---
      Task(spec-agent) invocation
      Skill("schema-validate") for final check
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 task reference
    And cross-reference summary should show 0 unresolved task references
    And cross-reference summary should show 1 skill reference
    And cross-reference summary should show 0 unresolved skill references

  Scenario: spec-agent references spec-reviewer and 3 skills
    Given an agent file "spec-reviewer.md":
      """
      ---
      name: spec-reviewer
      description: Review spec documents
      tools: [Read]
      ---
      Body
      """
    And a skill directory "tree-parse" with SKILL.md:
      """
      ---
      name: tree-parse
      description: Parse directory tree
      allowed-tools: [Read, Glob]
      ---
      Body
      """
    And a skill directory "dependency-graph" with SKILL.md:
      """
      ---
      name: dependency-graph
      description: Build dependency graph
      allowed-tools: [Read, Glob]
      ---
      Body
      """
    And a skill directory "schema-validate" with SKILL.md:
      """
      ---
      name: schema-validate
      description: Validate schema
      allowed-tools: [Read]
      ---
      Body
      """
    And an agent file "spec-agent.md":
      """
      ---
      name: spec-agent
      description: Generate spec from requirements
      tools: [Read, Write, Task, Skill]
      ---
      Task(spec-reviewer) for review
      Skill("tree-parse") for structure
      Skill("dependency-graph") for deps
      Skill("schema-validate") for validation
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 task reference
    And cross-reference summary should show 0 unresolved task references
    And cross-reference summary should show 3 skill references
    And cross-reference summary should show 0 unresolved skill references

  Scenario: decompiler references boundary-resolve, code-analyze, and schema-validate
    Given a skill directory "boundary-resolve" with SKILL.md:
      """
      ---
      name: boundary-resolve
      description: Resolve boundary
      allowed-tools: [Read]
      ---
      Body
      """
    And a skill directory "code-analyze" with SKILL.md:
      """
      ---
      name: code-analyze
      description: Analyze code
      allowed-tools: [Read, Glob]
      ---
      Body
      """
    And a skill directory "schema-validate" with SKILL.md:
      """
      ---
      name: schema-validate
      description: Validate schema
      allowed-tools: [Read]
      ---
      Body
      """
    And an agent file "decompiler.md":
      """
      ---
      name: decompiler
      description: Decompile single directory
      tools: [Read, Write, Skill]
      ---
      Skill("boundary-resolve") for boundary
      Skill("code-analyze") for analysis
      Skill("schema-validate") for validation
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 3 skill references
    And cross-reference summary should show 0 unresolved skill references

  Scenario: compile references compiler, test-reviewer, and 4 incremental skills
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code from spec
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
    And a skill directory "git-status-analyzer" with SKILL.md:
      """
      ---
      name: git-status-analyzer
      description: Identifies uncommitted files
      allowed-tools: [Bash, Write]
      ---
      Body
      """
    And a skill directory "commit-comparator" with SKILL.md:
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
      Skill("git-status-analyzer")
      Skill("commit-comparator")
      Skill("interface-diff")
      Skill("dependency-tracker")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 3 task references
    And cross-reference summary should show 0 unresolved task references
    And cross-reference summary should show 4 skill references
    And cross-reference summary should show 0 unresolved skill references

  # === Special Status Values ===

  Scenario: code-reviewer result block triple status valid
    Given an agent file "code-reviewer.md":
      """
      ---
      name: code-reviewer
      description: Reviews code quality
      tools: [Read, Write]
      ---

      ---code-reviewer-result---
      status: approve | feedback | warning
      ---end-code-reviewer-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: drift-validator error status valid
    Given an agent file "drift-validator.md":
      """
      ---
      name: drift-validator
      description: Validates CLAUDE.md-code consistency
      tools: [Read, Write]
      ---

      ---drift-validator-result---
      status: error
      ---end-drift-validator-result---
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: export-validator approve status valid
    Given an agent file "export-validator.md":
      """
      ---
      name: export-validator
      description: Validates exports existence
      tools: [Read, Write]
      ---

      ---export-validator-result---
      status: approve
      ---end-export-validator-result---
      """
    When I validate prompts
    Then prompt validation should pass
