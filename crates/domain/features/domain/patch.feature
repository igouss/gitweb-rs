Feature: Unified diff (patch) text formatting
  The textual unified diff is git's own patch format: a `diff --git` header,
  the extended headers that announce creation / deletion / mode change /
  rename / copy, an `index` line, the `--- ` / `+++ ` from/to lines, and the
  `@@` hunks themselves. gitweb does not invent this format — it streams what
  `git diff-tree -p` emits — so the domain owns the format as a pure rule and
  the gix adapter feeds it the structured changes.

  The rule is exercised here over hand-built file patches; the gix adapter's
  conformance specs drive the same formatter from real repositories.

  Scenario: A modified text file shows index, from/to and a hunk
    Given a modified file patch for "a.txt"
    When I render the patch
    Then the patch contains "diff --git a/a.txt b/a.txt"
    And the patch contains "index "
    And the patch contains "--- a/a.txt"
    And the patch contains "+++ b/a.txt"
    And the patch contains "@@ -1 +1 @@"
    And the patch contains "-old"
    And the patch contains "+new"

  Scenario: An unchanged mode rides on the index line
    Given a modified file patch for "a.txt"
    When I render the patch
    Then the patch contains "index 1111111111111111111111111111111111111111..2222222222222222222222222222222222222222 100644"

  Scenario: A created file reads from /dev/null with a zero index source
    Given a created file patch for "new.txt" with two lines
    When I render the patch
    Then the patch contains "new file mode 100644"
    And the patch has a line "index 0000000000000000000000000000000000000000..2222222222222222222222222222222222222222"
    And the patch contains "--- /dev/null"
    And the patch contains "+++ b/new.txt"
    And the patch contains "@@ -0,0 +1,2 @@"

  Scenario: A deleted file writes to /dev/null with a zero index target
    Given a deleted file patch for "gone.txt"
    When I render the patch
    Then the patch contains "deleted file mode 100644"
    And the patch contains "..0000000000000000000000000000000000000000"
    And the patch contains "--- a/gone.txt"
    And the patch contains "+++ /dev/null"
    And the patch contains "-was here"

  Scenario: A pure mode change carries no index and no hunk
    Given an executable-bit file patch for "run.sh"
    When I render the patch
    Then the patch is:
      """
      diff --git a/run.sh b/run.sh
      old mode 100644
      new mode 100755
      """

  Scenario: A mode change with content keeps the modes off the index line
    Given an executable-bit-and-content file patch for "run.sh"
    When I render the patch
    Then the patch contains "old mode 100644"
    And the patch contains "new mode 100755"
    And the patch contains "--- a/run.sh"
    And the patch has a line "index 1111111111111111111111111111111111111111..2222222222222222222222222222222222222222"

  Scenario: An exact rename is similarity 100% with no hunk
    Given an exact rename file patch from "old.txt" to "new.txt"
    When I render the patch
    Then the patch is:
      """
      diff --git a/old.txt b/new.txt
      similarity index 100%
      rename from old.txt
      rename to new.txt
      """

  Scenario: An inexact rename shows its similarity and a hunk
    Given an inexact rename file patch from "old.txt" to "new.txt" at 80%
    When I render the patch
    Then the patch contains "similarity index 80%"
    And the patch contains "rename from old.txt"
    And the patch contains "rename to new.txt"
    And the patch contains "--- a/old.txt"
    And the patch contains "+++ b/new.txt"
    And the patch contains "@@"

  Scenario: A copy shows copy headers
    Given an exact copy file patch from "src.txt" to "dst.txt"
    When I render the patch
    Then the patch contains "copy from src.txt"
    And the patch contains "copy to dst.txt"

  Scenario: A binary modification is a notice, not a hunk
    Given a binary modification file patch for "logo.bin"
    When I render the patch
    Then the patch contains "index "
    And the patch contains "Binary files a/logo.bin and b/logo.bin differ"
    And the patch does not contain "@@"
    And the patch does not contain "--- a/logo.bin"

  Scenario: A binary creation names /dev/null on the left
    Given a binary creation file patch for "logo.bin"
    When I render the patch
    Then the patch contains "Binary files /dev/null and b/logo.bin differ"

  Scenario: A missing trailing newline is marked
    Given a created file patch for "link" whose single line has no trailing newline
    When I render the patch
    Then the patch contains "+target/path"
    And the patch contains "\ No newline at end of file"

  Scenario: A single-line hunk omits the run lengths
    Given a modified file patch for "a.txt"
    When I render the patch
    Then the patch contains "@@ -1 +1 @@"

  Scenario: Adding to an empty file counts from zero
    Given a created file patch for "new.txt" with two lines
    When I render the patch
    Then the patch contains "@@ -0,0 +1,2 @@"

  Scenario: A patch over no files is empty
    Given a patch over no files
    When I render the patch
    Then the patch is empty

  Scenario: A patch concatenates several file patches
    Given a patch over a created file and a deleted file
    When I render the patch
    Then the patch contains "new file mode 100644"
    And the patch contains "deleted file mode 100644"
    And the patch contains "diff --git a/added.txt b/added.txt"
    And the patch contains "diff --git a/gone.txt b/gone.txt"
