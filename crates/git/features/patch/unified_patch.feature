Feature: Unified diff (patch) text from a repository
  The gix adapter renders git's patch text for a commit against its parent, the
  way `git diff-tree -r -M -p` does: a `diff --git` header per changed file, the
  extended headers that announce creation / deletion / mode change / rename, the
  `--- `/`+++ ` lines, and the `@@` hunks. The structured changes come from the
  same tree-to-tree diff the `diff` operation uses; this adds the blob reads and
  the hunk computation, and hands the result to the domain's patch formatter.

  Combined multi-parent merge diffs (--cc) and byte-exact copy similarity are
  separate capabilities and are not exercised here.

  Scenario: A modified file produces a hunk with the old and new lines
    Given a commit that modifies one file
    When I take the patch
    Then the patch contains "diff --git a/a.txt b/a.txt"
    And the patch contains "@@"
    And the patch contains "-one"
    And the patch contains "+two"

  Scenario: A created file reads from /dev/null
    Given a commit that adds a file
    When I take the patch
    Then the patch contains "new file mode 100644"
    And the patch contains "--- /dev/null"
    And the patch contains "+++ b/added.txt"
    And the patch contains "+a brand new file here"

  Scenario: A deleted file writes to /dev/null
    Given a commit that deletes a file
    When I take the patch
    Then the patch contains "deleted file mode 100644"
    And the patch contains "+++ /dev/null"
    And the patch contains "-this file will be removed"

  Scenario: A mode change announces the old and new modes
    Given a commit that makes a file executable
    When I take the patch
    Then the patch contains "old mode 100644"
    And the patch contains "new mode 100755"

  Scenario: A binary file is a notice, not a hunk
    Given a commit that changes a binary file
    When I take the patch
    Then the patch contains "Binary files a/logo.bin and b/logo.bin differ"
    And the patch does not contain "@@"

  Scenario: An exact rename shows rename headers and no hunk
    Given a commit that renames a file
    When I take the patch
    Then the patch contains "rename from old.txt"
    And the patch contains "rename to new.txt"
    And the patch contains "similarity index 100%"
    And the patch does not contain "@@"

  Scenario: A symlink records its target and the missing-newline marker
    Given a commit that adds a symlink
    When I take the patch
    Then the patch contains "new file mode 120000"
    And the patch contains "+a.txt"
    And the patch contains "\ No newline at end of file"

  Scenario: A submodule is shown as a Subproject commit line
    Given a commit that adds a submodule
    When I take the patch
    Then the patch contains "new file mode 160000"
    And the patch contains "+Subproject commit "

  Scenario: A commit that changes nothing has an empty patch
    Given a commit that changes nothing
    When I take the patch
    Then the patch is empty

  Scenario: Non-UTF-8 content still diffs as text
    Given a commit that modifies a latin-1 file
    When I take the patch
    Then the patch contains "@@"
    And the patch does not contain "Binary files"

  Scenario: Diffing against a missing object fails to find it
    Given a commit and a missing object id
    When I take the patch
    Then the patch fails to find an object

  # The plain endpoints abbreviate their `index` ids to git's default
  # `core.abbrev`, which auto-scales by object count but floors at 7. A small
  # fixture repository sits at that floor, so the adapter reports 7.
  Scenario: A small repository abbreviates ids to git's floor of 7
    Given a commit that modifies one file
    When I read the default abbreviation length
    Then the default abbreviation length is 7

  # gix reports a 1-based hunk start and keeps each line's trailing newline; git
  # writes `-0,0` for the empty before-side of a creation, and one newline per
  # line. The adapter reconciles both, so a multi-line creation comes out exactly
  # as `git diff-tree -p` would write it — no `@@ -1,0` and no doubled newlines.
  Scenario: A multi-line creation numbers from zero with one newline per line
    Given a commit that creates a two-line file
    When I take the patch
    Then the hunk text is:
      """
      @@ -0,0 +1,2 @@
      +alpha
      +beta
      """
