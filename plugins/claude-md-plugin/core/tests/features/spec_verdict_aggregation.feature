Feature: spec SKILL aggregates verdict fields into a single JSONL file
  Scenario: Each target's verdict produces one line with all fields
    Given consult result files for targets "." and "core/src/foo"
    And both files contain Verdict, Execution, Reason, RoadmapFit
    When Step 2.1d runs
    Then ${TMP_DIR}verdict-aggregate.jsonl MUST contain one line per target
    And each line MUST have keys: target, verdict, execution, reason, roadmap_fit
    And if Redirect To was present, the line MUST include redirect_to
