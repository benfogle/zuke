Feature: Steps can capture arguments

    Scenario: Steps can capture context by keyword
        Given a step that captures context
        And a step that captures _context

    Scenario: Regex steps can capture by named group
        Given a regex step that expects "foo"

    Scenario: Regex steps can capture by named group and context
        Given a regex step that captures context and expects "foo"

    Scenario: Regex steps can capture an integer
        Given a regex step that expects number 100

    Scenario: Regex steps can capture a &str
        Given a regex step that expects str "foo"

    Scenario: Regex steps can capture a custom type
        Given a regex step that expects the color red

    @expect-fail
    Scenario: Regex steps will fail on conversion errors
        Given a regex step that expects the color zlurple

    Scenario: The name 'context' is not reserved
        Given a regex step that captures "foo" using the name context

    Scenario: The name '_context' is not reserved
        Given a regex step that captures "foo" using the name _context

    Scenario: Basic steps can capture by named group
        Given a step that expects "foo"

    Scenario: Basic steps can capture by named group and context
        Given a step that captures context and expects "foo"

    Scenario: Basic steps can capture an integer
        Given a step that expects number 100

    Scenario: Basic steps can capture a &str
        Given a step that expects str "foo"

    Scenario: Basic steps can capture a custom type
        Given a step that expects the color red

    @expect-fail
    Scenario: Basic steps will fail on conversion errors
        Given a step that expects the color zlurple

    Scenario: The name 'context' is not reserved in basic steps
        Given a step that captures "foo" using the name context

    Scenario: The name '_context' is not reserved in basic steps
        Given a step that captures "foo" using the name _context
