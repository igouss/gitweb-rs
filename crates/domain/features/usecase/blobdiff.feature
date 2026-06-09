Feature: The blobdiff use case (resolving the single file an html diff shows)
  gitweb's git_blobdiff html view shows the diff of one file between two base
  commits. Before rendering, it resolves WHICH file from the raw diff-tree —
  directly by the file's new-side path, or the legacy by-hash URI (the blob's
  new-side id) — and reads the base commit's subject for the header. This use
  case is that resolution over the Repository port: it reads the two-tree patch
  between the parent base and the base, selects the one file, and reports its
  new-side path, a rename's old path, and the commit subject. The diff bytes are
  fetched separately by the client viewer, so this use case never renders them.

  When the base is not a commit (a tree reached by a hand-crafted URI), gitweb's
  git_blobdiff falls back to a degenerate "<new-id> vs <old-id>" title built from
  the resolved blob ids instead of a commit subject; the resolution still works
  because a tree is tree-ish and the single-file diff is the same.

  The fake resolves both bases to the one fixture commit and ignores the
  from-side (it serves the fixture's whole patch), so these scenarios exercise
  the resolution and its error mapping, not the diff algorithm.

  Scenario: Resolving a modified file reports its path and the commit subject
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "Rework the engine"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    When I assemble the blobdiff of "engine.rs" with base "HEAD" and parent base "base"
    Then the blobdiff file name is "engine.rs"
    And the blobdiff has no file parent
    And the blobdiff title is "Rework the engine"

  Scenario: Resolving a renamed file reports the old path as the file parent
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "Move it"
    And the repository has branch "base" committed at 900
    And the diff renames "old.rs" to "new.rs"
    When I assemble the blobdiff of "new.rs" with base "HEAD" and parent base "base"
    Then the blobdiff file name is "new.rs"
    And the blobdiff file parent is "old.rs"

  Scenario: A base that is not a commit gets the degenerate "new-id vs old-id" title
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "Rework the engine"
    And the commit object kind is "tree"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    When I assemble the blobdiff of "engine.rs" with base "HEAD" and parent base "base"
    Then the blobdiff file name is "engine.rs"
    And the blobdiff title pairs the new-side and old-side ids of "engine.rs"

  Scenario: A path the diff does not touch is not found
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "x"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    When I assemble the blobdiff of "absent.rs" with base "HEAD" and parent base "base"
    Then assembling the blobdiff fails with "Blob diff not found"

  Scenario: Resolving by a blob's new-side id finds the file
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "x"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    When I assemble the blobdiff by the new-side id of "engine.rs" with base "HEAD" and parent base "base"
    Then the blobdiff file name is "engine.rs"

  Scenario: Neither a file nor a hash is a missing-parameter error
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "x"
    And the repository has branch "base" committed at 900
    And the diff modifies "engine.rs"
    When I assemble the blobdiff with neither file nor hash, base "HEAD" and parent base "base"
    Then assembling the blobdiff fails with "Missing one of the blob diff parameters"

  Scenario: A new-side id two added files share is ambiguous
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit message is "x"
    And the repository has branch "base" committed at 900
    And the diff adds "twin-a.rs" and "twin-b.rs" sharing new-side content
    When I assemble the blobdiff by the shared new-side id with base "HEAD" and parent base "base"
    Then assembling the blobdiff fails with "Ambiguous blob diff specification"
