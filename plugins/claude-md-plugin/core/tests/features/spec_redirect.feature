Feature: redirect honored until authority converges
  Scenario: Single-hop redirect reroutes target
    Given target "core/src/tree_parser" verdict has Redirect To=core/src/symbol_index
    Then Step 2 MUST re-run with target_path=core/src/symbol_index
    And the new target MUST receive its own po-consultant verdict

  Scenario: Cycle detection halts
    Given tree_parser redirects to symbol_index, then symbol_index redirects back to tree_parser
    Then spec MUST halt with reason "redirect cycle: tree_parser → symbol_index → tree_parser"
    And no plan MUST be generated
