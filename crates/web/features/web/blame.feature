Feature: Serving blame (blame, blame_incremental, blame_data actions)
  gitweb's git_blame_common serves a file's line-by-line attribution three ways,
  all gated on the blame feature (die_error(403, "Blame view not allowed") when
  off). The blame action serves the server-rendered table; blame_incremental
  serves the empty-row shell that boots the client module over the blame_data URL;
  blame_data streams the `git blame --incremental` text the module parses. A
  request that names no file is die_error(400); a project that does not exist is
  die_error(404). The base revision defaults to HEAD.

  Scenario: the blame action serves the server-rendered table
    Given a project root with a file history "h.git"
    And the blame action is served
    When I GET "/?p=h.git&a=blame&hb=HEAD&f=file.txt"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "id="blame_table""
    And the response body contains "class="sha1""
    And the response body contains "a=commit"
    And the response body contains "v2"
    And the response body contains "Ada Lovelace"

  Scenario: the blame_incremental action serves the shell and boots the client
    Given a project root with a file history "h.git"
    And the blame_incremental action is served
    When I GET "/?p=h.git&a=blame_incremental&hb=HEAD&f=file.txt"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "id="blame_table""
    And the response body contains "id="progress_bar""
    And the response body contains "id="progress_info""
    And the response body contains "startBlame"
    And the response body contains "a=blame_data"
    And the response body contains "/static/blame-incremental.js"

  Scenario: the blame_data action streams the git blame --incremental text
    Given a project root with a file history "h.git"
    And the blame_data action is served
    When I GET "/?p=h.git&a=blame_data&hb=HEAD&f=file.txt"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response body contains "author Ada Lovelace"
    And the response body contains "author-time 1700086400"
    And the response body contains "author-tz +0000"
    And the response body contains "filename file.txt"
    And the response body contains "END"

  Scenario: blame is forbidden when the feature is off
    Given a project root with a file history "h.git"
    And the blame action is served with blame disabled
    When I GET "/?p=h.git&a=blame&hb=HEAD&f=file.txt"
    Then the response status is 403

  Scenario: a blame request with no file name is a bad request
    Given a project root with a file history "h.git"
    And the blame action is served
    When I GET "/?p=h.git&a=blame&hb=HEAD"
    Then the response status is 400

  Scenario: blame of a missing project is not found
    Given a project root with a file history "h.git"
    And the blame action is served
    When I GET "/?p=ghost.git&a=blame&hb=HEAD&f=file.txt"
    Then the response status is 404
