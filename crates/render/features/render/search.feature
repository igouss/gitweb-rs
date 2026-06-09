Feature: Rendering the search-results table
  The search-results page (modernized git_search_message / git_search_grep_body)
  shows one row per matching commit: a date cell, the author, the subject link,
  the highlighted matching fragments of the message — each a lead, a match in a
  <span class="match">, and a trail — and the per-commit commit / commitdiff /
  tree links. The first page with no matches at all shows "No match." The page
  navigation carries a first · prev · next paging affordance, each a link only
  when that move is possible (otherwise disabled text). URLs are built by the web
  boundary, so this layer takes finished hrefs and only decides layout and
  escaping.

  Scenario: a result row shows its date, author, subject link, and per-commit links
    Given a search result by "Ada" titled "Add the thing" dated "3 days ago" at "/r/commit/abc"
    When I render the search page
    Then the result contains "3 days ago"
    And the result contains "Add the thing"
    And the result contains "/r/commit/abc"
    And the result contains "commitdiff"
    And the result contains "tree"

  Scenario: a matching message line is highlighted around the match
    Given a search result highlighting lead "fix the " match "banana" trail " bug"
    When I render the search page
    Then the result contains "fix the "
    And the result contains "<span class="match">banana</span>"
    And the result contains " bug"

  Scenario: the first page with no matches says so
    Given the search has no matches
    When I render the search page
    Then the result contains "No match."

  Scenario: a next-page affordance links forward when a further page exists
    Given a search result by "Ada" titled "x" dated "now" at "/c"
    And the search offers a next page at "/next-page"
    When I render the search page
    Then the result contains "/next-page"

  Scenario: a later page links back to the first and previous pages
    Given a search result by "Ada" titled "x" dated "now" at "/c"
    And the search offers first and previous pages at "/first-page" and "/prev-page"
    When I render the search page
    Then the result contains "/first-page"
    And the result contains "/prev-page"

  Scenario: a single page disables the paging moves
    Given a search result by "Ada" titled "x" dated "now" at "/c"
    When I render the search page
    Then the result contains "nav-disabled"
