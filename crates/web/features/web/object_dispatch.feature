Feature: The no-action default object view (gitweb's dispatch git_get_type)
  When a request names no action but carries a hash — or a base ref plus a file —
  gitweb's dispatch sub does NOT redirect like the object action: it resolves the
  referenced object's kind and serves the matching view INLINE for the same
  request, with the request's parameters unchanged. A hash that resolves to
  nothing is the 404 "Object does not exist"; a base+file that resolves to
  nothing is the 404 "File or directory does not exist" — both distinct from the
  object action's strings.

  # --- a bare hash serves the object's view inline ---

  Scenario: A bare hash serves the commit view inline
    Given a project root containing a commit repository "c.git"
    And a stub "commit" page handler
    And the no-action object dispatch is enabled
    When I GET "/?p=c.git&h=main"
    Then the response status is 200
    And the response body contains "STUB:commit"

  # --- a base ref plus a file serves the path's view inline ---

  Scenario: A base ref plus a file serves the blob view inline
    Given a repository "t.git" with a tree
    And a stub "blob" page handler
    And the no-action object dispatch is enabled
    When I GET "/?p=t.git&hb=main&f=README"
    Then the response status is 200
    And the response body contains "STUB:blob"

  Scenario: A base ref plus a directory serves the tree view inline
    Given a repository "t.git" with a tree
    And a stub "tree" page handler
    And the no-action object dispatch is enabled
    When I GET "/?p=t.git&hb=main&f=src"
    Then the response status is 200
    And the response body contains "STUB:tree"

  # --- the two dispatch 404 misses ---

  Scenario: A bare hash that resolves to nothing is the object-not-found error
    Given a project root containing a commit repository "c.git"
    And the no-action object dispatch is enabled
    When I GET "/?p=c.git&h=0000000000000000000000000000000000000000"
    Then the response status is 404
    And the response body contains "Object does not exist"

  Scenario: A base and file that resolve to nothing is the file-not-found error
    Given a repository "t.git" with a tree
    And the no-action object dispatch is enabled
    When I GET "/?p=t.git&hb=main&f=does-not-exist.txt"
    Then the response status is 404
    And the response body contains "File or directory does not exist"

  # Unlike the object action, dispatch resolves the kind with git_get_type
  # (cat-file -t), which reads the recorded object. A gitlink's commit is absent
  # here, so it 404s rather than serving the commit view — the deliberate
  # asymmetry between dispatch and the object action.
  Scenario: A submodule under a base is the file-not-found error, its commit absent
    Given a repository "t.git" with a tree
    And the no-action object dispatch is enabled
    When I GET "/?p=t.git&hb=main&f=vendor"
    Then the response status is 404
    And the response body contains "File or directory does not exist"
