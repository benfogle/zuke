Feature: We can conditionally skip

    This feature will fail if we run it on a 16-bit machine, or on some
    mythical >64-bit machine. Fix that if it ever becomes a problem.

    @skip-if-target_pointer_width-16
    Scenario: Skip on 16 bit
        Given a step that returns nothing

    @skip-if-not-target_pointer_width-16
    Scenario: Skip except on 16 bit
        Then we shouldn't get here

    @skip-if-target_pointer_width-32 @skip-if-target_pointer_width-64
    Scenario: Skip on 32 or 64 bit
        Then we shouldn't get here
