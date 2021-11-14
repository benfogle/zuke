Feature: Steps may be defined in a variety of ways
    Tests the ways that steps may be defined, and the values that they may
    return. This is not concerned with capturing parameters or regex matching.

    This feature probably makes no sense without viewing the step
    implementations.

    # These are all do-nothing, return nothing steps.
    Scenario: Can implement steps
        Given a lever long enough
        And a place to stand
        Then I will move the world

    # These are all do-nothing, return nothing steps, but async!
    Scenario: Can implement async steps
        Given an async lever long enough
        And an async place to stand
        Then I will move the world without blocking

    Scenario: Steps can return nothing to indicate success
        Given a step that returns nothing

    Scenario: Steps can return Ok(_) from any result
        Given a step that returns Ok from anyhow::Result
        And a step that returns Ok from std::io::Result
        And a step that returns Ok(42) from std::io::Result

    Scenario: Async steps can return nothing to indicate success
        Given an async step that returns nothing

    Scenario: Async steps can return Ok(_) from any result
        Given an async step that returns Ok from anyhow::Result
        And an async step that returns Ok from std::io::Result
        And an async step that returns Ok(42) from std::io::Result

    @expect-fail
    Scenario: Steps can panic
        Given a step that panics

    @expect-fail
    Scenario: Steps can return Err(_) from any result
        Given a step that returns Err from anyhow::Result
        And a step that returns Err from std::io::Result

    @expect-fail
    Scenario: Unimplemented steps cause errors
        Given a step that isn't actually implemented anywhere

    @expect-fail
    Scenario: Multiply-implemented steps cause errors
        Given a step that is implemented twice
