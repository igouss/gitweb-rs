Feature: Serving a directory listing (tree action)
  gitweb's git_tree opens the project, peels the base revision (HEAD by default)
  to its tree, optionally descends into a path, and serves one row per entry:
  its mode, name, and per-kind links. A file links to its blob and offers
  blob | history | raw (a symlink shows where it points); a directory links to
  its own tree; a gitlink/submodule is named plainly. Browsing a subdirectory
  offers a ".." parent row. A request for a project that does not exist is
  die_error(404); a path that is not a directory in the tree is die_error(404).

  Scenario: the tree page lists every entry kind of the root
    Given a repository "t.git" with a tree
    And the tree action is served
    When I GET "/?p=t.git&a=tree"
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response body contains "README"
    And the response body contains "build.sh"
    And the response body contains "latest"
    And the response body contains "src"
    And the response body contains "vendor"
    And the response body contains "src/main.rs"
    And the response body contains "-rwxr-xr-x"
    And the response body contains "m---------"

  Scenario: browsing a subdirectory lists its entries and offers a parent row
    Given a repository "t.git" with a tree
    And the tree action is served
    When I GET "/?p=t.git&a=tree&f=src"
    Then the response status is 200
    And the response body contains "main.rs"
    And the response body contains ">..<"

  Scenario: the tree for a project that does not exist is not found
    Given a repository "t.git" with a tree
    And the tree action is served
    When I GET "/?p=ghost.git&a=tree"
    Then the response status is 404

  Scenario: a path that is a file, not a directory, is not found
    Given a repository "t.git" with a tree
    And the tree action is served
    When I GET "/?p=t.git&a=tree&f=README"
    Then the response status is 404

  Scenario: a path absent from the tree is not found
    Given a repository "t.git" with a tree
    And the tree action is served
    When I GET "/?p=t.git&a=tree&f=ghost"
    Then the response status is 404
