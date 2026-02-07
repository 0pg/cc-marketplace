Feature: Skill Frontmatter Validation
  As a plugin developer
  I want to validate skill prompt file frontmatter
  So that all skills have consistent and correct metadata

  Background:
    Given a clean prompt test directory

  # === Happy Path ===

  Scenario: Valid skill with all required fields
    Given a skill directory "compile" with SKILL.md:
      """
      ---
      name: compile
      description: Compile CLAUDE.md to source code
      allowed-tools: [Bash, Read, Write, Task, Skill]
      ---
      Body content here
      """
    When I validate prompts
    Then prompt validation should pass
    And skills count should be 1

  Scenario: Valid skill with optional fields
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      name: my-skill
      version: 1.0.0
      aliases: [gen, build]
      trigger:
        - /my-skill
        - generate code
      description: A skill with optional fields
      allowed-tools: [Read, Glob, Grep]
      ---
      Body content here
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: Multiple valid skills
    Given a skill directory "alpha" with SKILL.md:
      """
      ---
      name: alpha
      description: First skill
      allowed-tools: [Read]
      ---
      Body
      """
    And a skill directory "beta" with SKILL.md:
      """
      ---
      name: beta
      description: Second skill
      allowed-tools: [Write]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass
    And skills count should be 2

  # === Missing Required Fields ===

  Scenario: Skill missing name field
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      description: A skill
      allowed-tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  Scenario: Skill missing description field
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      name: my-skill
      allowed-tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  Scenario: Skill missing allowed-tools field
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      name: my-skill
      description: A skill
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  # === Name Mismatch ===

  Scenario: Skill name does not match directory name
    Given a skill directory "compile" with SKILL.md:
      """
      ---
      name: wrong-name
      description: Name does not match directory
      allowed-tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "does not match directory"

  # === Invalid Tools ===

  Scenario: Skill with invalid tool name
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      name: my-skill
      description: A skill with bad tool
      allowed-tools: [Read, FakeTool]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "FakeTool"

  Scenario: Skill with multiple invalid tools
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      name: my-skill
      description: A skill with bad tools
      allowed-tools: [Read, BadTool, WorseTool]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "BadTool"
    And issue should mention "WorseTool"

  # === Structural Issues ===

  Scenario: Skill file with no frontmatter
    Given a skill directory "my-skill" with SKILL.md:
      """
      No frontmatter delimiters here
      Just plain text content
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Missing YAML frontmatter"

  Scenario: Skill with malformed YAML
    Given a skill directory "my-skill" with SKILL.md:
      """
      ---
      name: [invalid yaml
      description: broken
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  Scenario: All valid tools are accepted
    Given a skill directory "all-tools" with SKILL.md:
      """
      ---
      name: all-tools
      description: Uses every valid tool
      allowed-tools: [Bash, Read, Write, Glob, Grep, Task, Skill, AskUserQuestion, Edit, WebFetch, WebSearch, NotebookRead, TodoWrite]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass
