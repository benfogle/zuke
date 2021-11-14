Feature: Step implementations can match in a variety of ways

    Scenario: Basic expressions are case insensitive by default
        Given A sTeP tHaT rEtuRnS nOtHiNg

    Scenario: Regex expressions are case insensitive by default
        Given A rEgEx StEp ThAt ReTUrNs NoThInG

    @expect-fail
    Scenario: Regex expressions may be case sensitive
        Given A cAsE-sEnSiTiVe ReGeX sTeP tHaT rEtuRnS nOtHiNg

    Scenario: Basic expressions escape regex characters (1)
        Given a step with special characters...

    @expect-fail
    Scenario: Basic expressions escape regex characters (2)
        Given a step with special characters123

    Scenario: Regex expressions can match special characters
        Given a word with a double vowel "book"
        Given a word with a double vowel "aardvark"

    @expect-fail
    Scenario: Regex expressions are anchored to the start
        Given blah a word with a double vowel "book"

    @expect-fail
    Scenario: Regex expressions are anchored to the end
        Given a word with a double vowel "book" blah
