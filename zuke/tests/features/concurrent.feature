Feature: Scenarios execute concurrently

    Scenario: hello 1/4
        When I wait for 4 scenarios to say "hello"

    Scenario: hello 2/4
        When I wait for 4 scenarios to say "hello"

    Scenario: hello 3/4
        When I wait for 4 scenarios to say "hello"

    Scenario: hello 4/4
        When I wait for 4 scenarios to say "hello"

    Scenario Outline: Scenario outlines are concurrent
        When I wait for <count> scenarios to say "<word>"

        Examples:
            | count |    word |
            | 4     |   uncle |
            | 2     | goodbye |
            | 4     |   uncle |
            | 4     |   uncle |
            | 2     | goodbye |
            | 4     |   uncle |
