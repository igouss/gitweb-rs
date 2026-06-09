Feature: Serving pickaxe search (search action, searchtype pickaxe)
  gitweb's git_search dispatches a pickaxe query to git_search_changes: it opens
  the project, gates on the search feature (403 when off) and then on pickaxe (403
  "Pickaxe search is disabled"), requires a search term (400 when absent), runs
  `git log -S<text>` from the base revision (HEAD by default), and lists the
  commits that changed the pattern's occurrence count — under each, the
  count-changing files linking to their blobs. An empty result is an empty table
  (gitweb prints no note); a request for a project that does not exist is a 404.

  The fixture repository "repo.git" has one commit on main whose only file
  "file.txt" contains the line "hello" — so the root commit raises the "hello"
  count from 0 to 1 and is a pickaxe hit, listing file.txt.

  Scenario: a pickaxe lists the matching commit and links its changed file
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=pickaxe&s=hello"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "file.txt"
    And the response body contains "<span class="match">file.txt</span>"
    And the response body contains "a=blob"

  Scenario: a regexp pickaxe matches by pattern
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=pickaxe&s=h.llo&sr=1"
    Then the response status is 200
    And the response body contains "<span class="match">file.txt</span>"

  Scenario: a pickaxe that matches nothing is an empty table with no note
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=pickaxe&s=zucchini"
    Then the response status is 200
    And the response body does not contain "No matches found"
    And the response body does not contain "<span class="match">"

  Scenario: pickaxe is forbidden when the pickaxe feature is disabled
    Given a project root containing repository "repo.git"
    And the search action is served with pickaxe disabled
    When I GET "/?p=repo.git&a=search&st=pickaxe&s=hello"
    Then the response status is 403

  Scenario: pickaxe is forbidden when the search feature is disabled
    Given a project root containing repository "repo.git"
    And the search action is served with search disabled
    When I GET "/?p=repo.git&a=search&st=pickaxe&s=hello"
    Then the response status is 403

  Scenario: a pickaxe with no term is a bad request
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=repo.git&a=search&st=pickaxe"
    Then the response status is 400

  Scenario: pickaxe for a project that does not exist is not found
    Given a project root containing repository "repo.git"
    And the search action is served
    When I GET "/?p=ghost.git&a=search&st=pickaxe&s=hello"
    Then the response status is 404
