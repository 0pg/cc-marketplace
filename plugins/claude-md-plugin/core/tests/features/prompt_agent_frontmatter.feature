Feature: Agent Frontmatter Validation
  As a plugin developer
  I want to validate agent prompt file frontmatter
  So that all agents have consistent and correct metadata

  Background:
    Given a clean prompt test directory

  # === Happy Path ===

  Scenario: Valid agent with all fields
    Given an agent file "compiler.md":
      """
      ---
      name: compiler
      description: Compile source code from CLAUDE.md
      tools: [Read, Write, Task, Bash]
      ---
      System prompt body here
      """
    When I validate prompts
    Then prompt validation should pass
    And agents count should be 1

  Scenario: Valid agent with optional fields
    Given an agent file "my-agent.md":
      """
      ---
      name: my-agent
      description: An agent with optional fields
      tools: [Read, Write]
      model: sonnet
      color: blue
      ---
      Body content
      """
    When I validate prompts
    Then prompt validation should pass

  Scenario: Multiple valid agents
    Given an agent file "alpha.md":
      """
      ---
      name: alpha
      description: First agent
      tools: [Read]
      ---
      Body
      """
    And an agent file "beta.md":
      """
      ---
      name: beta
      description: Second agent
      tools: [Write]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass
    And agents count should be 2

  # === Missing Tools Warning ===

  Scenario: Agent without tools field gets warning
    Given an agent file "my-agent.md":
      """
      ---
      name: my-agent
      description: An agent without tools
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should pass with warnings
    And prompt warning should mention "tools"

  # === Missing Required Fields ===

  Scenario: Agent missing name field
    Given an agent file "my-agent.md":
      """
      ---
      description: An agent without name
      tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  Scenario: Agent missing description field
    Given an agent file "my-agent.md":
      """
      ---
      name: my-agent
      tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"

  # === Name Mismatch ===

  Scenario: Agent name does not match filename
    Given an agent file "compiler.md":
      """
      ---
      name: wrong-name
      description: Name does not match
      tools: [Read]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "does not match filename"

  # === Invalid Tools ===

  Scenario: Agent with invalid tool name
    Given an agent file "my-agent.md":
      """
      ---
      name: my-agent
      description: Agent with bad tool
      tools: [Read, NonexistentTool]
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "NonexistentTool"

  # === Structural Issues ===

  Scenario: Agent file with no frontmatter
    Given an agent file "my-agent.md":
      """
      No frontmatter delimiters here
      Just plain text content
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Missing YAML frontmatter"

  Scenario: Agent with malformed YAML
    Given an agent file "my-agent.md":
      """
      ---
      name: [broken yaml
      ---
      Body
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "Invalid YAML frontmatter"
