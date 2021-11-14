Feature: Fixtures can have scenario, feature and global scope
    Enforcement of these is done in the fixture itself, on teardown

    Scenario: Scenario scope 3
        Given a counter fixture with scenario scope, that should be 1 on teardown
        When I increment the scenario counter

    # Ensure that the counter gets reset between scenarios
    Scenario: Scenario scope 4
        Given a counter fixture with scenario scope, that should be 1 on teardown
        When I increment the scenario counter

    # Ensure that this gets reset between fixtures
    Scenario: Feature scope 3
        Given a counter fixture with feature scope, that should be 2 on teardown
        When I increment the feature counter

    Scenario: Feature scope 4
        Given a counter fixture with feature scope, that should be 2 on teardown
        When I increment the feature counter

    # The others are in a different file
    Scenario: Global scope 3
        Given a counter fixture with global scope, that should be 4 on teardown
        When I increment the global counter

    # The others are in a different file
    Scenario: Global scope 4
        Given a counter fixture with global scope, that should be 4 on teardown
        When I increment the global counter
