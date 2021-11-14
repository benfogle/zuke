Feature: Zuke can parse files
    Scenario: Zuke can parse a single file
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null.feature"
        And I run the tests
        Then the tests complete successfully
        And there are 1/1 passing features
        And there are 0/0 passing scenarios

    Scenario: Zuke can parse a multiple files
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null.feature"
        When I add the path "tests/extra_features/null/null_items.feature"
        And I run the tests
        Then the tests complete successfully
        And there are 2/2 passing features
        And there are 2/2 passing rules
        And there are 6/6 passing scenarios

    Scenario: Zuke can parse a directory
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null"
        And I run the tests
        Then the tests complete successfully
        And there are 3/3 passing features
        And there are 2/2 passing rules
        And there are 8/8 passing scenarios

    Scenario: Zuke can parse a source string
        Given a zuke sub-instance
        When I add the feature source
            """
            Feature: An inline feature
                Scenario: Scenario 1
                Scenario: Scenario 2
            """
        And I run the tests
        Then the tests complete successfully
        And there are 1/1 passing features
        And there are 2/2 passing scenarios

    Scenario: Zuke can parse a mix of sources
        Given a zuke sub-instance
        When I add the feature source
            """
            Feature: An inline feature
                Scenario: Scenario 1
                Scenario: Scenario 2
            """
        And I add the path "tests/extra_features/null"
        And I run the tests
        Then the tests complete successfully
        And there are 4/4 passing features
        And there are 2/2 passing rules
        And there are 10/10 passing scenarios
