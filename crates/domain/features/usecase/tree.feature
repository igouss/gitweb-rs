Feature: Listing a directory (tree)
  gitweb's git_tree lists one directory of a revision: one row per entry,
  carrying its mode (the permission string and git object type), name, and —
  when show-sizes is on — its ls-tree size. Each entry renders by its kind: a
  file links to its blob, a symlink shows where it points, a directory links to
  its own tree, and a gitlink/submodule is just named. Browsing inside a
  directory offers a ".." parent affordance.

  Background:
    Given the tree base is commit "Initial import"

  Scenario: an empty tree lists no entries
    When I assemble the tree
    Then no tree entries are listed
    And the tree offers no parent row

  Scenario: a tree lists a single file
    Given the tree has file "README" of 12 bytes
    When I assemble the tree
    Then the listed tree entries are "README"
    And the tree entry "README" is a "blob"
    And the tree entry "README" has size 12

  Scenario: a tree lists every entry kind distinctly
    Given the tree has file "README" of 12 bytes
    And the tree has executable "build.sh" of 40 bytes
    And the tree has symlink "latest" pointing to "releases/v2"
    And the tree has directory "src"
    And the tree has submodule "vendor"
    When I assemble the tree
    Then the listed tree entries are "README, build.sh, latest, src, vendor"
    And the tree entry "build.sh" permission string is "-rwxr-xr-x"
    And the tree entry "build.sh" is a "blob"
    And the tree entry "latest" is a "blob"
    And the tree entry "latest" points to "releases/v2"
    And the tree entry "latest" has size 11
    And the tree entry "src" is a "tree"
    And the tree entry "src" has no size
    And the tree entry "vendor" is a "commit"
    And the tree entry "vendor" has no size
    And the tree shows the commit subject "Initial import"

  Scenario: the size column is hidden when show-sizes is off
    Given the tree has file "README" of 12 bytes
    And the show-sizes feature is off
    When I assemble the tree
    Then the tree size column is hidden
    And the tree entry "README" has no size

  Scenario: browsing a subdirectory offers a parent row to the root
    Given the directory "src" lists file "main.rs" of 80 bytes
    When I assemble the tree of "src"
    Then the listed tree entries are "main.rs"
    And the tree offers a parent row to the root

  Scenario: browsing a nested subdirectory links the parent to its parent
    Given the directory "src/inner" lists file "deep.rs" of 5 bytes
    When I assemble the tree of "src/inner"
    Then the listed tree entries are "deep.rs"
    And the tree offers a parent row to "src"

  Scenario: a path that is a file, not a directory, is not found
    Given the tree has file "README" of 12 bytes
    When I assemble the tree of "README"
    Then assembling the tree fails as not found

  Scenario: a path absent from the tree is not found
    When I assemble the tree of "ghost"
    Then assembling the tree fails as not found

  Scenario: a path that is a gitlink, not a directory, is not found
    Given the tree has submodule "vendor"
    When I assemble the tree of "vendor"
    Then assembling the tree fails as "No such tree"
