Feature: The patch use case (single commit as a format-patch mail)
  gitweb's `git_patch` streams `git format-patch --stdout -1` for one commit.
  This use case resolves the commit-ish (defaulting to HEAD), gates on the
  `patches` feature the way gitweb does — `die_error(403, "Patch view not
  allowed") unless $patch_max` — refuses a non-commit with the 404 "Unknown
  commit object", and frames the mail: a `From <id> Mon Sep 17 00:00:00 2001`
  separator with git's magic date, the From / Date / Subject headers, the body,
  a `---`, the diffstat, the diff, and a `-- ` / git-version signature.

  Scenario: A commit assembles a [PATCH] mail with header, diffstat and diff
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And the commit message is "Add the analytical engine"
    And the commit diff creates "engine.txt"
    When I assemble the patch for "HEAD" with limit 16 and version "2.54.0"
    Then the patch stream contains " Mon Sep 17 00:00:00 2001"
    And the patch stream has a line "From: Ada Lovelace <ada@example.com>"
    And the patch stream has a line "Date: Tue, 14 Nov 2023 22:13:20 +0000"
    And the patch stream has a line "Subject: [PATCH] Add the analytical engine"
    And the patch stream contains "diff --git a/engine.txt b/engine.txt"
    And the patch stream has a line " create mode 100644 engine.txt"
    And the patch stream has a line "-- "
    And the patch stream has a line "2.54.0"

  Scenario: A body paragraph rides between the subject and the --- separator
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And the commit has subject "Add the engine" and body "The first program."
    And the commit diff creates "engine.txt"
    When I assemble the patch for "HEAD" with limit 16 and version "2.54.0"
    Then the patch stream has a line "Subject: [PATCH] Add the engine"
    And the patch stream has a line "The first program."

  Scenario: A binary file shows a Bin diffstat and the --no-binary notice
    # gitweb streams `git format-patch`, which embeds a binary file as a base85
    # `GIT binary patch` of git's own zlib output — unreproducible by the gix-only,
    # no-unsafe port. The port emits git's `--no-binary` form instead: the
    # `Binary files … differ` notice, while the `Bin <old> -> <new> bytes` diffstat
    # row (read from the blob sizes over the port) is still byte-exact.
    Given a commit "b1a0c0" with author "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And the commit message is "Add a binary asset"
    And the commit diff modifies binary "logo.bin" from 5 to 7 bytes
    When I assemble the patch for "HEAD" with limit 16 and version "2.54.0"
    Then the patch stream has a line " logo.bin | Bin 5 -> 7 bytes"
    And the patch stream has a line "Binary files a/logo.bin and b/logo.bin differ"
    And the patch stream does not contain "GIT binary patch"

  Scenario: The patches feature being off forbids the view
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1700000000 +0000"
    And the commit diff creates "engine.txt"
    When I assemble the patch for "HEAD" with limit 0 and version "2.54.0"
    Then assembling the patch fails with "Patch view not allowed"

  Scenario: A revision that is not a commit is unknown
    Given a commit "b10b" with author "Tester <t@example.com> 1 +0000"
    And the commit object kind is "blob"
    When I assemble the patch for "HEAD" with limit 16 and version "2.54.0"
    Then assembling the patch fails with "Unknown commit object"
