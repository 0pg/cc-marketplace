Feature: diff-preservation validates declared preserved sections are byte-identical

  Scenario: all declared sections preserved verbatim
    Given a prior DEVELOPERS.md with sections "Technical Context" and "Decision Log"
    And a new DEVELOPERS.md where those sections are byte-identical
    When diff-preservation is run with sections "Technical Context,Decision Log"
    Then the drifted list MUST be empty
    And the preserved list MUST contain both sections

  Scenario: declared section body was paraphrased
    Given a prior section "Technical Context" body "Uses X library"
    And a new section "Technical Context" body "Uses library X"
    When diff-preservation is run with sections "Technical Context"
    Then the drifted list MUST contain "Technical Context"
    And its reason MUST be "body_changed"

  Scenario: declared section was removed
    Given a prior DEVELOPERS.md with a "Roadmap" section
    And a new DEVELOPERS.md without a "Roadmap" section
    When diff-preservation is run with sections "Roadmap"
    Then the drifted list MUST contain "Roadmap"
    And its reason MUST be "removed"

  Scenario: sections not declared are ignored
    Given a prior and new DEVELOPERS.md differing only in "Constraints"
    When diff-preservation is run with sections "Technical Context"
    Then the drifted list MUST be empty

  Scenario: declared section was absent in prior
    Given a prior DEVELOPERS.md without a "Roadmap" section
    And a new DEVELOPERS.md with a "Roadmap" section
    When diff-preservation is run with sections "Roadmap"
    Then the drifted list MUST contain "Roadmap"
    And its reason MUST be "absent_in_prior"

  Scenario: H2 heading inside a fenced code block is not treated as a section boundary
    Given a prior "Decision Log" section containing a fenced code block with a literal "## Example" line followed by "Prior conclusion."
    And a new "Decision Log" section containing the same fenced block followed by "New conclusion."
    When diff-preservation is run with sections "Decision Log"
    Then the drifted list MUST contain "Decision Log"
    And its reason MUST be "body_changed"
