Feature: Resolving the object action over the repository (git_object)
  gitweb's git_object learns the kind of the object a request names and redirects
  to the view for that kind. The use case does the repository half: it classifies
  the request, looks the object up, and returns the redirect target — the
  destination action and the parameters that name the object under it.

  A direct id (a hash, or a base ref standing in for one) resolves the revision
  and reads its kind. A base ref plus a file resolves the base, looks the path up
  under it, and reads the found object's kind — the redirect then carries the
  resolved object id as its hash, with the base and file unchanged. Misses are
  gitweb's three distinct 404s; naming neither is the 400.

  # --- by id: a hash resolves to its kind's view ---

  Scenario: A commit id redirects to the commit view, hash carried through
    Given the repository has branch "main" committed at 1000
    When I assemble the object redirect for hash "main"
    Then the redirect action is "commit"
    And the redirect hash is "main"
    And the redirect has no base
    And the redirect has no file

  Scenario: An annotated tag id redirects to the tag view
    Given an annotated tag "v1.0" of a commit tagged at 1000 with subject "release"
    When I assemble the object redirect for hash "v1.0"
    Then the redirect action is "tag"
    And the redirect hash is "v1.0"

  # --- by base and path: the path's object kind picks the view ---

  Scenario: A file under a base redirects to the blob view with the resolved id
    Given the tree base is commit "root"
    And the tree has file "README" of 5 bytes
    When I assemble the object redirect for base "HEAD" and file "README"
    Then the redirect action is "blob"
    And the redirect base is "HEAD"
    And the redirect file is "README"
    And the redirect has a hash

  Scenario: A directory under a base redirects to the tree view
    Given the tree base is commit "root"
    And the tree has directory "src"
    When I assemble the object redirect for base "HEAD" and file "src"
    Then the redirect action is "tree"
    And the redirect base is "HEAD"
    And the redirect file is "src"
    And the redirect has a hash

  Scenario: A trailing slash on the file is stripped in the redirect
    Given the tree base is commit "root"
    And the tree has directory "src"
    When I assemble the object redirect for base "HEAD" and file "src/"
    Then the redirect action is "tree"
    And the redirect file is "src"

  # A submodule entry (mode 160000) is classified by its mode — gitweb's ls-tree
  # type column reads "commit" — so it redirects to the commit view even though
  # the recorded commit lives in the submodule and is absent from this repository.
  Scenario: A submodule under a base redirects to the commit view, its commit absent
    Given the tree base is commit "root"
    And the tree has submodule "vendor"
    When I assemble the object redirect for base "HEAD" and file "vendor"
    Then the redirect action is "commit"
    And the redirect base is "HEAD"
    And the redirect file is "vendor"
    And the redirect has a hash

  # --- misses: gitweb's three distinct 404s and the 400 ---

  Scenario: Naming neither a hash nor a base is not enough information
    When I assemble the object redirect with neither a hash nor a base
    Then assembling the object fails with "Not enough information to find object"

  Scenario: An id that resolves to nothing is the object-does-not-exist 404
    When I assemble the object redirect for hash "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    Then assembling the object fails with "Object does not exist"

  Scenario: A base that resolves to nothing is the base-does-not-exist 404
    Given the tree base is commit "root"
    And the tree has file "README" of 5 bytes
    When I assemble the object redirect for base "nope" and file "README"
    Then assembling the object fails with "Base object does not exist"

  Scenario: A path absent under the base is the file-not-found 404
    Given the tree base is commit "root"
    And the tree has file "README" of 5 bytes
    When I assemble the object redirect for base "HEAD" and file "missing.txt"
    Then assembling the object fails with "File or directory for given base does not exist"
