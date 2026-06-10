Feature: Resolving the no-action default object view (gitweb's dispatch git_get_type)
  When a request names no action but carries a hash — or a base ref plus a file
  — gitweb's dispatch sub resolves the referenced object's kind and serves the
  matching view INLINE for the same request (no redirect). The use case does the
  repository half: it reads the kind over the port and returns the action whose
  view shows it.

  A hash resolves the revision and reads its kind. A base ref plus a file
  resolves the base, looks the path up under it, and reads the found object's
  kind. The misses are gitweb's dispatch strings — distinct from git_object's: a
  hash that names nothing is "Object does not exist"; a base+file that resolves
  to nothing — whether the base or the path is the miss — is the one error "File
  or directory does not exist".

  # --- by hash: the object's kind picks the inline view ---

  Scenario: A commit hash serves the commit view
    Given the repository has branch "main" committed at 1000
    When I resolve the dispatch action for hash "main"
    Then the dispatch action is "commit"

  Scenario: An annotated tag hash serves the tag view
    Given an annotated tag "v1.0" of a commit tagged at 1000 with subject "release"
    When I resolve the dispatch action for hash "v1.0"
    Then the dispatch action is "tag"

  # --- by base and path: the path's kind picks the inline view ---

  Scenario: A file under a base serves the blob view
    Given the tree base is commit "root"
    And the tree has file "README" of 5 bytes
    When I resolve the dispatch action for base "HEAD" and file "README"
    Then the dispatch action is "blob"

  Scenario: A directory under a base serves the tree view
    Given the tree base is commit "root"
    And the tree has directory "src"
    When I resolve the dispatch action for base "HEAD" and file "src"
    Then the dispatch action is "tree"

  # --- misses: gitweb's two dispatch 404 strings ---

  Scenario: A hash that resolves to nothing is the object-does-not-exist 404
    When I resolve the dispatch action for hash "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    Then resolving the dispatch action fails with "Object does not exist"

  Scenario: A base that resolves to nothing is the file-or-directory 404
    Given the tree base is commit "root"
    And the tree has file "README" of 5 bytes
    When I resolve the dispatch action for base "nope" and file "README"
    Then resolving the dispatch action fails with "File or directory does not exist"

  Scenario: A path absent under the base is the file-or-directory 404
    Given the tree base is commit "root"
    And the tree has file "README" of 5 bytes
    When I resolve the dispatch action for base "HEAD" and file "missing.txt"
    Then resolving the dispatch action fails with "File or directory does not exist"

  # A submodule is where dispatch and the object action deliberately DIVERGE.
  # The object action reads the kind from ls-tree's mode column, so a gitlink
  # redirects to the commit view (see usecase/object). dispatch instead runs
  # git_get_type = `cat-file -t hash_base:file_name`, which reads the recorded
  # object — and a gitlink's commit lives in the submodule, absent here, so the
  # read fails and gitweb 404s. We match that: a gitlink does not serve a view.
  Scenario: A submodule under a base is the file-or-directory 404, its commit absent
    Given the tree base is commit "root"
    And the tree has submodule "vendor"
    When I resolve the dispatch action for base "HEAD" and file "vendor"
    Then resolving the dispatch action fails with "File or directory does not exist"
