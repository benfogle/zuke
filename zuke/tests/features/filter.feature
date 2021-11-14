Feature: We can include and exclude items from the command line
    Scenario: Zuke can include select scenarios
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null_items.feature"
        And I add "--name outline" to the command line
        And I run the tests
        Then the tests complete successfully
        And there are 1/1 passing features
        And there are 1/2 passing rules
        And there are 4/6 passing scenarios

    Scenario: Zuke can include select rules
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null_items.feature"
        And I add "--name 'rule with empty scenarios'" to the command line
        And I run the tests
        Then the tests complete successfully
        And there are 1/1 passing features
        And there are 1/2 passing rules
        And there are 3/6 passing scenarios

    Scenario: Zuke can include select features
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null_items.feature"
        When I add the path "tests/extra_features/null/null.feature"
        And I add "--name 'a lot of empty'" to the command line
        And I run the tests
        Then the tests complete successfully
        And there are 1/2 passing features
        And there are 2/2 passing rules
        And there are 6/6 passing scenarios

    Scenario: Zuke can exclude select scenarios
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null_items.feature"
        And I add "--exclude outline" to the command line
        And I run the tests
        Then the tests complete successfully
        And there are 1/1 passing features
        And there are 2/2 passing rules
        And there are 2/6 passing scenarios

    Scenario: Zuke can exclude select rules
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null_items.feature"
        And I add "--exclude 'rule with empty scenarios'" to the command line
        And I run the tests
        Then the tests complete successfully
        And there are 1/1 passing features
        And there are 1/2 passing rules
        And there are 3/6 passing scenarios

    Scenario: Zuke can exclude select features
        Given a zuke sub-instance
        When I add the path "tests/extra_features/null/null_items.feature"
        When I add the path "tests/extra_features/null/null.feature"
        And I add "--exclude 'a lot of empty'" to the command line
        And I run the tests
        Then the tests complete successfully
        And there are 1/2 passing features
        And there are 0/2 passing rules
        And there are 0/6 passing scenarios
