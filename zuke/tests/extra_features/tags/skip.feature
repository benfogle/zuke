Feature: We can skip rules and scenarios by tags

    Scenario: This scenario runs
        Given a step that returns nothing

    @skip
    Scenario: This scenario doesn't run
        Then I shouldn't get here

    @skip
    Rule: This rule doesn't run
        
        Scenario: This scenario in a rule doesn't run
            Then I shouldn't get here
