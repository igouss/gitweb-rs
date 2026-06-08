Feature: The blobdiff_plain use case (single-file raw diff)
  gitweb's blobdiff_plain streams the raw unified diff of one file between two
  revisions. This use case reads the two-tree patch between the parent base and
  the base over the Repository port, then narrows it to the single file selected
  by its new-side path, rendered with the repository's default index
  abbreviation. A path the diff does not touch is gitweb's 404 "Blob diff not
  found".

  The fake resolves both bases to the one fixture commit and ignores the
  from-side (it serves the fixture's whole patch), so these scenarios exercise
  the selection and the 404, not the diff algorithm.

  Scenario: Selecting one file from a multi-file diff renders just that file
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    And the diff modifies "loom.rs"
    When I assemble the blobdiff_plain of "engine.rs" with base "HEAD" and parent base "base"
    Then the blobdiff_plain body contains "diff --git a/engine.rs b/engine.rs"
    And the blobdiff_plain body does not contain "loom.rs"

  Scenario: A path the diff does not touch is not found
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    When I assemble the blobdiff_plain of "absent.rs" with base "HEAD" and parent base "base"
    Then assembling the blobdiff_plain fails with "Blob diff not found"
