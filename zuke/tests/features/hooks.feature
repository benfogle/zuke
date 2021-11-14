@inherited-tag @non-inherited-tag
Feature: Before/after hooks can be implemented as functions
    This test relies on hooks and tag exprs to be defined

    @use-a-fixture
    Scenario: A before hook can trigger on a tag
        Then the TaggedFixture fixture is present

    @use-a-fixture-left
    Scenario: Boolean exprs (1)
        Then the AndFixture fixture is not present
        And the OrFixture fixture is present

    @use-a-fixture-right
    Scenario: Boolean exprs (2)
        Then the AndFixture fixture is not present
        And the OrFixture fixture is present

    @use-a-fixture-left @use-a-fixture-right
    Scenario: Boolean exprs (3)
        Then the AndFixture fixture is present
        And the OrFixture fixture is present

    Scenario: Trigger on an inherited tag
        Then the InheritedFixture fixture is present

    Scenario: Exclude inherited tags
        Then the NonInheritedFixture fixture is not present
