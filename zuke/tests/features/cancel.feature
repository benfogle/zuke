Feature: Steps can be canceled

    Scenario: Scenario can be canceled
        Given a zuke sub-instance
        When I add the feature source
            """
            Feature: An inline feature
                Scenario: Never finishes
                    When I pause forever
            """
        And I run the tests
        And I cancel the tests
        Then the tests were canceled
