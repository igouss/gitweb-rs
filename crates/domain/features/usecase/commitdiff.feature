Feature: The commitdiff use case (clean unified diff)
  The diff viewer fetches a commit's diff as git's clean unified patch text —
  the bytes starting at "diff --git", with no mail headers (those are
  commitdiff_plain's, and they break the viewer's parser). This use case
  resolves the commit-ish (defaulting to HEAD), refuses a non-commit the way
  gitweb's git_commitdiff does (the 404 "Unknown commit object"), picks the base
  to diff against, and renders the patch over the Repository port.

  Scenario: A commit's clean diff is git's unified patch text
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "Add the analytical engine"
    And the commit diff creates "engine.rs"
    When I assemble the commitdiff text for "HEAD"
    Then the commitdiff text contains "diff --git a/engine.rs b/engine.rs"

  Scenario: A revision that is not a commit is unknown
    Given a commit "b10b" with author "Tester <t@example.com> 1 +0000"
    And the commit object kind is "blob"
    When I assemble the commitdiff text for "HEAD"
    Then assembling the commitdiff text fails with "Unknown commit object"
