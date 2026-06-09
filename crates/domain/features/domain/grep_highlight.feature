Feature: Full-line grep match highlighting
  On a grep search, gitweb highlights the matched span of every listed line
  (`git_search_files`): it splits the line at the first match into the part
  before it, the match, and the part after, wrapping the match in a
  `<span class="match">`. Unlike the commit-search snippet, the grep highlight
  does NOT trim the surrounding text — the whole line is shown, only the span
  marked. The split is always case-insensitive (gitweb's `m/^(.*)(…)(.*)$/i`), so
  it reuses the case-insensitive `SearchPattern`; a line that does not match
  yields no split (gitweb's else branch shows the line plain).

  Scenario: a line that does not match yields no highlight
    Given the fixed search pattern "banana"
    Then grep-highlighting "an apple a day" yields no snippet

  Scenario: a match in the middle splits the line into lead, match and trail
    Given the fixed search pattern "bar"
    When I grep-highlight "foo bar baz"
    Then the snippet lead is "foo "
    And the snippet match is "bar"
    And the snippet trail is " baz"

  Scenario: a match at the very start has an empty lead
    Given the fixed search pattern "foo"
    When I grep-highlight "foobar"
    Then the snippet lead is ""
    And the snippet match is "foo"
    And the snippet trail is "bar"

  Scenario: a match at the very end has an empty trail
    Given the fixed search pattern "bar"
    When I grep-highlight "foobar"
    Then the snippet lead is "foo"
    And the snippet match is "bar"
    And the snippet trail is ""

  Scenario: the highlight is case-insensitive even in fixed mode
    Given the fixed search pattern "foo"
    When I grep-highlight "x FOO y"
    Then the snippet lead is "x "
    And the snippet match is "FOO"
    And the snippet trail is " y"

  Scenario: a regexp pattern highlights its matched span
    Given the regexp search pattern "a.c"
    When I grep-highlight "xxabcyy"
    Then the snippet lead is "xx"
    And the snippet match is "abc"
    And the snippet trail is "yy"

  Scenario: a long lead and trail are kept whole, not trimmed
    Given the fixed search pattern "MATCH"
    When I grep-highlight "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLMATCHRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR"
    Then the snippet lead is "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL"
    And the snippet match is "MATCH"
    And the snippet trail is "RRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR"
