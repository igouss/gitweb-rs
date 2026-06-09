Feature: Serving grep search (search action, searchtype grep)
  gitweb's git_search dispatches a grep query to git_search_files: it opens the
  project, gates on the search feature (403 when off) and then on grep (403
  "Grep search is disabled"), requires a search term (400 when absent), greps the
  base revision's tree (HEAD by default), and lists the matches grouped per file —
  each file linking to its blob and each matching line to blob#lN, the matched
  span highlighted. An empty result says "No matches found"; a request for a
  project that does not exist is a 404.

  The fixture repository "repo.git" has one commit on main whose only file
  "file.txt" contains the line "hello".

  Scenario: a grep lists the matching file and highlights the matched line
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=grep&s=hello"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "file.txt"
    And the response body contains "<span class="match">hello</span>"
    And the response body contains "#l1"

  Scenario: a regexp grep matches by pattern and case-insensitively
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=grep&s=H.LLO&sr=1"
    Then the response status is 200
    And the response body contains "<span class="match">hello</span>"

  Scenario: a grep that matches nothing says so
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=grep&s=zucchini"
    Then the response status is 200
    And the response body contains "No matches found"

  Scenario: grep is forbidden when the grep feature is disabled
    Given a project root containing repository "repo.git"
    And the search action is served with grep disabled
    When I GET "/?p=repo.git&a=search&st=grep&s=hello"
    Then the response status is 403

  Scenario: grep is forbidden when the search feature is disabled
    Given a project root containing repository "repo.git"
    And the search action is served with search disabled
    When I GET "/?p=repo.git&a=search&st=grep&s=hello"
    Then the response status is 403

  Scenario: a grep with no term is a bad request
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=grep"
    Then the response status is 400

  Scenario: grep for a project that does not exist is not found
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=ghost.git&a=search&st=grep&s=hello"
    Then the response status is 404
