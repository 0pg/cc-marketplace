Feature: Unified generate-diagram command
  The three separate diagram commands (generate-usecase, generate-state,
  generate-component) are unified into a single generate-diagram command
  with a --type parameter.

  Background:
    Given a clean test directory

  Scenario: Generate usecase diagram via unified command
    Given CLAUDE.md with content:
      """
      ## Purpose
      Test usecase diagram generation.

      ## Summary
      A module for user authentication.

      ## Exports
      - `login(user: string, pass: string): Token`

      ## Behavior

      ### Actors
      - User
      - Admin

      ### Use Cases
      - UC-1: User logs in with valid credentials

      ## Contract
      None

      ## Protocol
      None

      ## Domain Context
      None
      """
    When I generate a usecase diagram
    Then the diagram output should contain "flowchart"

  Scenario: Generate state diagram via unified command
    Given a CLAUDE.md spec with protocol for diagram test
    When I generate a state diagram from spec
    Then the diagram output should contain "stateDiagram"

  Scenario: Generate component diagram via unified command
    Given a project with CLAUDE.md files:
      | directory | content                                                                                                                                                                                                                                       |
      | src/auth  | ## Purpose\nAuth module.\n\n## Summary\nHandles auth.\n\n## Exports\n- `login(): void`\n\n## Behavior\nNone\n\n## Contract\nNone\n\n## Protocol\nNone\n\n## Dependencies\n\n### Internal\n- `src/utils`\n\n## Domain Context\nNone |
      | src/utils | ## Purpose\nUtils module.\n\n## Summary\nShared utils.\n\n## Exports\n- `hash(): string`\n\n## Behavior\nNone\n\n## Contract\nNone\n\n## Protocol\nNone\n\n## Domain Context\nNone                                             |
    When I generate a component diagram from the project root
    Then the diagram output should contain "flowchart TB"
