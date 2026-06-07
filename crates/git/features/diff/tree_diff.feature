Feature: Tree-to-tree diff
  The gix adapter computes the set of changed paths between two commits, the way
  gitweb's `git diff-tree -r -M` does: recursing into subtrees so only leaf
  files are reported, with rename detection at git's default 50% similarity.
  Diffing against a missing left side (a root commit) compares against the empty
  tree, so every path reads as an addition.

  Combined multi-parent merge diffs and copy detection are out of scope here;
  they are tracked as separate capabilities.

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
