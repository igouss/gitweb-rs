Feature: Dispatching the object action (git_object)
  gitweb's object action is the generic dispatcher: given a hash, a base ref, or
  a base ref plus a file, it resolves the object's kind and 302-redirects to the
  view for that kind, building the target with href(-full => 1, …). The redirect
  carries the request's hash / base / file through, except the base+file form
  replaces the hash with the object id the path resolved to. The three lookup
  misses are gitweb's three distinct 404s; naming neither a hash nor a base is
  the 400 "Not enough information to find object".

  # --- by hash: a commit id redirects to the commit view ---

  Scenario: A hash redirects to the commit view, the hash carried through
    Given a project root containing a commit repository "c.git"
    And the object action is served
    When I GET "/?p=c.git&a=object&h=main"
    Then the response status is 302
    And the response redirects to "http://localhost/?p=c.git;a=commit;h=main"

  # --- by base and path: the path's kind picks the view ---

  Scenario: A file under a base redirects to the blob view with the resolved id
    Given a repository "t.git" with a tree
    And the object action is served
    When I GET "/?p=t.git&a=object&hb=main&f=README"
    Then the response status is 302
    And the response location contains "a=blob"
    And the response location contains "hb=main"
    And the response location contains "f=README"

  Scenario: A directory under a base redirects to the tree view
    Given a repository "t.git" with a tree
    And the object action is served
    When I GET "/?p=t.git&a=object&hb=main&f=src"
    Then the response status is 302
    And the response location contains "a=tree"
    And the response location contains "hb=main"
    And the response location contains "f=src"

  # A submodule entry's type comes from ls-tree's mode column ("commit"), so the
  # object action redirects to the commit view even though the recorded commit
  # lives in the submodule and is absent from this repository.
  Scenario: A submodule under a base redirects to the commit view
    Given a repository "t.git" with a tree
    And the object action is served
    When I GET "/?p=t.git&a=object&hb=main&f=vendor"
    Then the response status is 302
    And the response location contains "a=commit"
    And the response location contains "hb=main"
    And the response location contains "f=vendor"

  # --- the 400: not enough information ---

  Scenario: Naming neither a hash nor a base is a bad request
    Given a project root containing a commit repository "c.git"
    And the object action is served
    When I GET "/?p=c.git&a=object"
    Then the response status is 400
    And the response body contains "Not enough information to find object"

  # --- the three 404 misses ---

  Scenario: A hash that resolves to nothing is the object-not-found error
    Given a project root containing a commit repository "c.git"
    And the object action is served
    When I GET "/?p=c.git&a=object&h=0000000000000000000000000000000000000000"
    Then the response status is 404
    And the response body contains "Object does not exist"

  Scenario: A base that resolves to nothing is the base-not-found error
    Given a repository "t.git" with a tree
    And the object action is served
    When I GET "/?p=t.git&a=object&hb=0000000000000000000000000000000000000000&f=README"
    Then the response status is 404
    And the response body contains "Base object does not exist"

  Scenario: A path absent under the base is the file-not-found error
    Given a repository "t.git" with a tree
    And the object action is served
    When I GET "/?p=t.git&a=object&hb=main&f=does-not-exist.txt"
    Then the response status is 404
    And the response body contains "File or directory for given base does not exist"
