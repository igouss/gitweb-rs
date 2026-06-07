Feature: Tree-to-tree diff
  The gix adapter computes the set of changed paths between two commits, the way
  gitweb's `git diff-tree -r -M` does: recursing into subtrees so only leaf
  files are reported, with rename detection at git's default 50% similarity.
  Diffing against a missing left side (a root commit) compares against the empty
  tree, so every path reads as an addition.

  Copy detection (gitweb's `@diff_opts` `-C` / `-C --find-copies-harder`) is an
  opt-in level on top of the default renames-only diff, exercised in the copy
  scenarios at the end. Combined multi-parent merge diffs are out of scope here;
  they are a separate capability.

  Scenario: A commit diffed against itself has no changes
    Given a commit that changes nothing
    When I diff them
    Then 0 paths changed

  Scenario: A single modified file
    Given a commit that modifies one file
    When I diff them
    Then 1 path changed
    And the change to "a.txt" is "Modified"
    And the from-mode of "a.txt" is "100644"
    And the to-mode of "a.txt" is "100644"

  Scenario: Additions, deletions, and modifications together
    Given a commit that adds, deletes, and modifies files
    When I diff them
    Then 3 paths changed
    And the change to "added.txt" is "Added"
    And the change to "gone.txt" is "Deleted"
    And the change to "kept.txt" is "Modified"

  Scenario: An added file has the empty from-side
    Given a commit that adds, deletes, and modifies files
    When I diff them
    Then the from-mode of "added.txt" is "000000"
    And the from-oid of "added.txt" is all zeros

  Scenario: A deleted file has the empty to-side
    Given a commit that adds, deletes, and modifies files
    When I diff them
    Then the to-mode of "gone.txt" is "000000"
    And the to-oid of "gone.txt" is all zeros

  Scenario: Flipping the executable bit is a modification, not a type change
    Given a commit that makes a file executable
    When I diff them
    Then the change to "a.txt" is "Modified"
    And the from-mode of "a.txt" is "100644"
    And the to-mode of "a.txt" is "100755"

  Scenario: Turning a file into a symlink is a type change
    Given a commit that turns a file into a symlink
    When I diff them
    Then the change to "a.txt" is "TypeChanged"
    And the to-mode of "a.txt" is "120000"

  Scenario: Adding a symlink
    Given a commit that adds a symlink
    When I diff them
    Then the change to "link" is "Added"
    And the to-mode of "link" is "120000"

  Scenario: Adding a submodule
    Given a commit that adds a submodule
    When I diff them
    Then the change to "sub" is "Added"
    And the to-mode of "sub" is "160000"

  Scenario: A changed binary file is reported as differing, not as hunks
    Given a commit that changes a binary file
    When I diff them
    Then 1 path changed
    And the change to "logo.bin" is "Modified"

  Scenario: A root commit diffs against the empty tree
    Given a root commit with two files
    When I diff the root commit
    Then 2 paths changed
    And the change to "a.txt" is "Added"
    And the change to "b.txt" is "Added"

  Scenario: A renamed file is detected with full similarity
    Given a commit that renames a file
    When I diff them
    Then 1 path changed
    And the change to "new.txt" is "Renamed"
    And the change to "new.txt" comes from "old.txt"
    And the change to "new.txt" has similarity 100

  Scenario: A change inside a subdirectory reports the leaf path only
    Given a commit that changes a file inside a subdirectory
    When I diff them
    Then 1 path changed
    And the change to "dir/a.txt" is "Modified"
    And no path "dir" changed

  Scenario: Diffing against a missing object fails to find it
    Given a commit and a missing object id
    When I diff them
    Then the diff fails to find an object

  # --- Copy detection (gitweb @diff_opts: -C / -C --find-copies-harder) -------
  # gitweb's default level is renames-only (-M); copy detection is opt-in. The
  # levels differ only in which sources a copy may come from. Renames are still
  # detected at every level. (gix only scans for copies when a commit changes
  # more than one path, so these fixtures carry an unrelated change alongside the
  # copy — a lone-copy-only commit is a documented gix limitation.)

  Scenario: The renames-only level reports a copied file as a plain addition
    Given a commit that copies an unchanged file
    When I diff them
    Then the change to "copy.txt" is "Added"

  Scenario: Find-copies-harder detects an exact copy of an unchanged file
    Given a commit that copies an unchanged file
    When I diff them detecting copies harder
    Then the change to "copy.txt" is "Copied"
    And the change to "copy.txt" comes from "orig.txt"
    And the change to "copy.txt" has similarity 100
    And no path "orig.txt" changed

  Scenario: Copy detection from the modified set ignores an unchanged source
    Given a commit that copies an unchanged file
    When I diff them detecting copies
    Then the change to "copy.txt" is "Added"

  Scenario: Copy detection from the modified set finds a copy of a modified file
    Given a commit that modifies a file and copies its original content
    When I diff them detecting copies
    Then the change to "copy.txt" is "Copied"
    And the change to "copy.txt" comes from "orig.txt"
    And the change to "copy.txt" has similarity 100

  Scenario: A copy of a modified source keeps the source's own modification
    Given a commit that modifies a file and copies its original content
    When I diff them detecting copies
    Then 2 paths changed
    And the change to "orig.txt" is "Modified"

  Scenario: A near-copy above the similarity threshold is a copy
    Given a commit that copies a file with small edits
    When I diff them detecting copies harder
    Then the change to "near.txt" is "Copied"
    And the change to "near.txt" comes from "orig.txt"

  Scenario: A dissimilar new file below the threshold is not a copy
    Given a commit that adds a file barely resembling another
    When I diff them detecting copies harder
    Then the change to "different.txt" is "Added"

  Scenario: A rename and a copy in the same diff are both detected
    Given a commit that renames one file and copies another
    When I diff them detecting copies harder
    Then the change to "renamed.txt" is "Renamed"
    And the change to "renamed.txt" comes from "moved.txt"
    And the change to "copy.txt" is "Copied"
    And the change to "copy.txt" comes from "kept.txt"
