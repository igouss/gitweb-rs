Feature: Search result snippet highlighting
  On a commit/author/committer search, gitweb highlights the matching fragment of
  each commit-message line (git_search_grep_body). For a line that matches the
  search pattern it splits the line into the part before the match, the match
  itself, and the part after, then trims each so the row stays compact: the match
  is centre-chopped to 70 characters, and the surrounding context is chopped to
  half of the remaining 80-character budget (capped at 30) — the lead from its
  left, the trail from its right. A line that does not match yields no snippet.

  Scenario: a line that does not match yields no snippet
    Given the fixed search pattern "banana"
    Then highlighting "an apple a day" yields no snippet

  Scenario: a short match splits the line into lead, match and trail
    Given the fixed search pattern "banana"
    When I highlight "fix the banana bug"
    Then the snippet lead is "fix the "
    And the snippet match is "banana"
    And the snippet trail is " bug"

  Scenario: an over-long match is centre-chopped
    Given the regexp search pattern ".+"
    When I highlight "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau"
    Then the snippet match contains " ... "

  Scenario: a long lead and trail are trimmed toward the match
    Given the fixed search pattern "MATCH"
    When I highlight "LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLMATCHTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTTT"
    Then the snippet lead starts with " ..."
    And the snippet trail ends with "... "
    And the snippet match is "MATCH"
