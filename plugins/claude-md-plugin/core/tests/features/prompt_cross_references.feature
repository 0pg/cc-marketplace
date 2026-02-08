Feature: Cross-Reference Validation
  As a plugin developer
  I want to validate cross-references between skills and agents
  So that Task() and Skill() references point to existing components

  Background:
    Given a clean prompt test directory

  # === Task() References ===

  Scenario: Resolved Task reference to existing agent
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile source code
      tools: [Read, Write]
      ---
      Body
      """
    And a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md
      allowed-tools: [Task]
      ---
      For each directory:
        Task(compiler, prompt="compile this")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 task references
    And cross-reference summary should show 0 unresolved task references

  Scenario: Unresolved Task reference to missing agent
    Given a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md
      allowed-tools: [Task]
      ---
      Task(nonexistent-agent)
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "nonexistent-agent"
    And cross-reference summary should show 1 task references
    And cross-reference summary should show 1 unresolved task references

  Scenario: Multiple Task references with some unresolved
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile source code
      tools: [Read]
      ---
      Body
      """
    And a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md
      allowed-tools: [Task]
      ---
      Task(compiler, phase=red)
      Task(test-reviewer)
      Task(compiler, phase=green-refactor)
      """
    When I validate prompts
    Then prompt validation should fail
    And cross-reference summary should show 3 task references
    And cross-reference summary should show 1 unresolved task references

  # === Skill() References ===

  Scenario: Resolved Skill reference to existing skill
    Given a skill directory "code-analyze" with SKILL.md:
      """
      ---
      name: code-analyze
      description: Analyze code
      allowed-tools: [Read, Glob]
      ---
      Body
      """
    And an agent file "decompiler.md":
      """
      ---
      name: decompiler
      description: Decompile code
      tools: [Read, Skill]
      ---
      Skill("claude-md-plugin:code-analyze")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 skill references
    And cross-reference summary should show 0 unresolved skill references

  Scenario: Skill reference without plugin prefix
    Given a skill directory "code-analyze" with SKILL.md:
      """
      ---
      name: code-analyze
      description: Analyze code
      allowed-tools: [Read, Glob]
      ---
      Body
      """
    And an agent file "decompiler.md":
      """
      ---
      name: decompiler
      description: Decompile code
      tools: [Read, Skill]
      ---
      Skill("code-analyze")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 skill references
    And cross-reference summary should show 0 unresolved skill references

  Scenario: Unresolved Skill reference to missing skill
    Given an agent file "spec-agent.md":
      """
      ---
      name: spec-agent
      description: Generate spec
      tools: [Skill]
      ---
      Skill("nonexistent-skill")
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "nonexistent-skill"
    And cross-reference summary should show 1 skill references
    And cross-reference summary should show 1 unresolved skill references

  # === Mixed References ===

  Scenario: Both Task and Skill references resolved
    Given a skill directory "code-analyze" with SKILL.md:
      """
      ---
      name: code-analyze
      description: Analyze code
      allowed-tools: [Read, Glob]
      ---
      Body
      """
    And an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile code
      tools: [Read, Skill]
      ---
      Skill("code-analyze")
      """
    And a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md
      allowed-tools: [Task, Skill]
      ---
      Task(compiler)
      Skill("code-analyze")
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 1 task references
    And cross-reference summary should show 2 skill references

  Scenario: No cross-references is valid
    Given a skill directory "simple" with SKILL.md:
      """
      ---
      name: simple
      description: A simple skill
      allowed-tools: [Read]
      ---
      No references here
      """
    When I validate prompts
    Then prompt validation should pass
    And cross-reference summary should show 0 task references
    And cross-reference summary should show 0 skill references
