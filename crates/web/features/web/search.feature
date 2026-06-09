Feature: Serving commit/author/committer search (search action)
  gitweb's git_search dispatches a commit/author/committer query to
  git_search_message: it opens the project, gates on the search feature (403 when
  off), requires a search term (400 "Text field is empty" when absent), roots at
  HEAD (or a named revision), and lists each matching commit as a row with its
  date, author, subject, the highlighted matching fragments of the message, and
  per-commit commit / commitdiff / tree links. The first page with no matches at
  all says "No match."; a request for a project that does not exist is a 404.

  The fixture repository "repo.git" has one commit on main: subject "init", by
  Ada Lovelace, dated 2023-11-14.

  Scenario: a message search lists the matching commits with highlighted snippets
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&s=init"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "Ada Lovelace"
    And the response body contains "2023-11-14"
    And the response body contains "<span class="match">init</span>"
    And the response body contains "commitdiff"

  Scenario: an author search matches the author identity
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=author&s=Ada"
    Then the response status is 200
    And the response body contains "Ada Lovelace"

  Scenario: a committer search matches the committing identity, case-insensitively
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=committer&s=ada"
    Then the response status is 200
    And the response body contains "Ada Lovelace"

  Scenario: a regexp search matches by pattern rather than literally
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&s=in.t&sr=1"
    Then the response status is 200
    And the response body contains "<span class="match">init</span>"

  Scenario: a search that matches nothing says so
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&s=zzzznotpresent"
    Then the response status is 200
    And the response body contains "No match."

  Scenario: search is forbidden when the search feature is disabled
    Given a project root containing repository "repo.git"
    And the search action is served with search disabled
    When I GET "/?p=repo.git&a=search&s=init"
    Then the response status is 403

  Scenario: a search with no term is a bad request
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search"
    Then the response status is 400

  Scenario: search for a project that does not exist is not found
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=ghost.git&a=search&s=init"
    Then the response status is 404
