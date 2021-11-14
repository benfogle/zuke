Feature: Default tags work as expected
    Scenario: We can skip rules and scenarios with @skip
        Given a zuke sub-instance
        When I add the path "tests/extra_features/tags/skip.feature"
        And I run the tests
        Then there are 1/1 passing features
        And there are 1/1 skipped rules
        And there are 2/3 skipped scenarios

    Scenario: We can skip features with @skip
        Given a zuke sub-instance
        When I add the path "tests/extra_features/tags/skip-feature.feature"
        And I run the tests
        Then there are 1/1 skipped features
        And there are 1/1 skipped rules
        And there are 2/2 skipped scenarios

    Scenario: We can conditionally skip features with @skip-if
        Given a zuke sub-instance
        When I add the path "tests/extra_features/tags/skip-if.feature"
        And I run the tests
        Then there are 0/1 skipped features
        And there are 2/3 skipped scenarios
