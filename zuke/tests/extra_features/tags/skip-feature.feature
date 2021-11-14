@skip
Feature: We can skip features by tag

    Scenario: This scenario doesn't run
        Then I shouldn't get here

    Rule: This rule doesn't run
        
        Scenario: This scenario in a rule doesn't run
            Then I shouldn't get here
