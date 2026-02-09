Feature: Prompt Tool Consistency Validation
  As a plugin developer
  I want tools declared in frontmatter to match actual usage
  So that agent permissions are accurate

  # 4a. Tool declaration consistency
  Scenario: Agent declares Skill but never uses Skill()
    Given a clean prompt test directory
    And an agent file "test-agent.md":
      """
      ---
      name: test-agent
      description: Test
      tools: [Bash, Read, Skill]
      ---
      Run claude-md-core parse-tree
      Read the output file
      """
    When I validate prompts
    Then tool consistency should warn "test-agent" has unused tool "Skill"

  Scenario: Agent uses Task() but does not declare it
    Given a clean prompt test directory
    And an agent file "test-agent.md":
      """
      ---
      name: test-agent
      description: Test
      tools: [Read]
      ---
      Task(some-agent) to process
      Read the result
      """
    When I validate prompts
    Then tool consistency should warn "test-agent" has undeclared tool "Task"

  Scenario: Skill with all tools matching body usage
    Given a clean prompt test directory
    And a skill file "my-skill":
      """
      ---
      name: my-skill
      description: Test
      allowed-tools: [Bash, Read, Task]
      ---
      Run claude-md-core parse-tree via Bash
      Read the output file
      Task(some-agent) to process
      """
    And an agent file "some-agent.md":
      """
      ---
      name: some-agent
      description: Test
      tools: [Read]
      ---
      Read files
      """
    When I validate prompts
    Then tool consistency issues count should be 0

  # 4b. CLI reference validity
  Scenario: Agent references invalid CLI command
    Given a clean prompt test directory
    And an agent file "bad-agent.md":
      """
      ---
      name: bad-agent
      description: Test
      tools: [Bash]
      ---
      Run claude-md-core nonexistent-command
      """
    When I validate prompts
    Then prompt validation should fail
    And issue should mention "nonexistent-command"

  Scenario: Agent references valid CLI commands
    Given a clean prompt test directory
    And an agent file "good-agent.md":
      """
      ---
      name: good-agent
      description: Test
      tools: [Bash]
      ---
      Run claude-md-core validate-schema to check
      Then claude-md-core parse-tree for scanning
      """
    When I validate prompts
    Then invalid CLI references count should be 0

  Scenario: Multiple invalid CLI references
    Given a clean prompt test directory
    And an agent file "multi-bad.md":
      """
      ---
      name: multi-bad
      description: Test
      tools: [Bash]
      ---
      Run claude-md-core fake-cmd1
      Then claude-md-core fake-cmd2
      """
    When I validate prompts
    Then prompt validation should fail
    And invalid CLI references count should be 2

  # 4c. Real project validation
  Scenario: All project agents have consistent tool declarations
    Given a clean prompt test directory
    And the actual project skills directory is loaded
    And the actual project agents directory is loaded
    When I validate prompts
    Then tool consistency issues count should be 0

  Scenario: All project CLI references are valid
    Given a clean prompt test directory
    And the actual project skills directory is loaded
    And the actual project agents directory is loaded
    When I validate prompts
    Then invalid CLI references count should be 0
