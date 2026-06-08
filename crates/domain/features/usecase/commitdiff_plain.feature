Feature: The commitdiff_plain use case (mailbox-format raw diff)
  gitweb's commitdiff_plain frames a commit's raw diff as a `git am`-able patch.
  This use case resolves the commit-ish (defaulting to HEAD), refuses a
  non-commit the way gitweb's git_commitdiff does (the 404 "Unknown commit
  object"), builds the mailbox header from the commit, and renders the patch
  body against the base the diff-base rule picks. git's `diff-tree` prints the
  bare commit hash ahead of the patch for the root (single-commit-ish) form but
  not for the two-tree (single-parent) form, so the commit-id line rides along
  exactly when the commit has no parent.

  Scenario: A root commit frames the header, comment and patch with its id line
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "Add the analytical engine"
    And the commit diff creates "engine.rs"
    When I assemble the commitdiff_plain for "HEAD"
    Then the commitdiff_plain body contains "From: Ada Lovelace <ada@example.com>"
    And the commitdiff_plain body contains "Subject: Add the analytical engine"
    And the commitdiff_plain body has a line "X-Git-Url: http://localhost?p=r.git;a=commitdiff_plain;h=HEAD"
    And the commitdiff_plain body has a line "Add the analytical engine"
    And the commitdiff_plain body has a line "---"
    And the commitdiff_plain body contains "diff --git a/engine.rs b/engine.rs"
    And the commitdiff_plain body carries the commit id on its own line

  Scenario: A commit with a parent has no bare commit-id line
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "Second commit"
    And the commit has parent "deadbeef"
    And the commit diff creates "engine.rs"
    When I assemble the commitdiff_plain for "HEAD"
    Then the commitdiff_plain body contains "diff --git a/engine.rs b/engine.rs"
    And the commitdiff_plain body has no bare commit-id line

  Scenario: A revision that is not a commit is unknown
    Given a commit "b10b" with author "Tester <t@example.com> 1 +0000"
    And the commit object kind is "blob"
    When I assemble the commitdiff_plain for "HEAD"
    Then assembling the commitdiff_plain fails with "Unknown commit object"
