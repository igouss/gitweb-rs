Feature: Assembling commit-search results
  gitweb's git_search dispatches a commit/author/committer search to
  git_search_message: it lists the commits reachable from the search base whose
  message (or matched identity) contains the pattern, newest first, a page at a
  time, and decorates each row with the date, the author, the subject, and the
  highlighted matching fragments of the message. This use case orchestrates that
  over the Repository port: it gates on the `search` feature (403 when off),
  validates the pattern (400 on a bad regexp), resolves the base revision (HEAD by
  default, 404 "Unknown commit object" when it names no commit), asks the port for
  one page plus one (so a further page is detectable), and maps each commit into a
  row. The fixed-vs-regexp matching and the base rooting are the adapter's
  conformance; here we drive the assembly's own behaviour, so the fake simply
  returns the configured hit set windowed by the requested page.

  Background:
    Given a commit "base" at epoch 1000 by "Tester" titled "base commit"
    And the repository HEAD is at commit "base"

  Scenario: a search is forbidden when the search feature is disabled
    When I assemble a disabled message search for "anything"
    Then the search is forbidden

  Scenario: a malformed regular expression is rejected
    When I assemble a regexp message search for "a(b"
    Then the search is invalid

  Scenario: a base revision that names no commit is an unknown commit object
    When I assemble a message search for "anything" rooted at "nope"
    Then the search reports an unknown commit object

  Scenario: a search with no matches lists nothing and offers no further page
    When I assemble a message search for "ghost"
    Then no commits are listed in the search
    And the search has no further page

  Scenario: a search lists the matching commits with highlighted snippets
    Given a search hit "h1" at epoch 1100 by "Ada Lovelace" titled "fix the banana bug"
    And a search hit "h2" at epoch 1200 by "Grace Hopper" titled "ripen a banana"
    When I assemble a message search for "banana"
    Then the search lists 2 commits
    And the search has no further page
    And search row "h1" shows the subject "fix the banana bug"
    And search row "h1" highlights "banana"

  Scenario: a full page leaves a further page when more matches remain
    Given a search hit "h1" at epoch 1100 by "Ada" titled "banana one"
    And a search hit "h2" at epoch 1200 by "Ada" titled "banana two"
    And a search hit "h3" at epoch 1300 by "Ada" titled "banana three"
    When I assemble a message search for "banana" with page size 2
    Then the search lists 2 commits
    And the search has a further page

  Scenario: a long author name is chopped to fifteen characters for the row
    Given a search hit "h1" at epoch 1100 by "Maximilian Alexander Throckmorton" titled "edit banana"
    When I assemble a message search for "banana"
    Then search row "h1" author shortens to "Maximilian Alexander... "
