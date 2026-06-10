Feature: Rendering the tree (directory listing)
  The tree table (modernized git_print_tree_entry) shows one row per entry:
  its permission string, an optional size, the name, and a links cell. A file
  links its name to the blob view and offers blob | history | raw; a symlink
  also shows where it points; a directory links its name to its own tree; a
  gitlink/submodule is named in plain text. A ".." row links up when browsing
  inside a directory. The size column appears only when show-sizes is on. URLs
  are built by the web boundary, so this layer takes finished hrefs.

  Scenario: a file row links its name and offers blob, history, and raw
    Given a tree file row "README" sized "12" at "/r/HEAD/README"
    When I render the tree table
    Then the result contains ">README</a>"
    And the result contains "/r/HEAD/README/blob"
    And the result contains "/r/HEAD/README/history"
    And the result contains "/r/HEAD/README/raw"
    And the result contains "-rw-r--r--"
    And the result contains "12"

  Scenario: a symlink row shows where it points
    Given a tree symlink row "latest" pointing to "releases/v2" at "/r/HEAD/latest"
    When I render the tree table
    Then the result contains "releases/v2"
    And the result contains "lrwxrwxrwx"

  Scenario: a directory row links its name to its own tree
    Given a tree directory row "src" at "/r/HEAD/src"
    When I render the tree table
    Then the result contains "/r/HEAD/src/tree"
    And the result contains ">src</a>"
    And the result contains "drwxr-xr-x"

  Scenario: a submodule row names the entry in plain text, with no name link
    Given a tree submodule row "vendor"
    When I render the tree table
    Then the result contains "vendor"
    And the result does not contain ">vendor</a>"
    And the result contains "m---------"

  Scenario: the size column is shown when sizes are on
    Given a tree file row "README" sized "12" at "/r/HEAD/README"
    When I render the tree table
    Then the result contains "<td class="size""

  Scenario: the size column is omitted when sizes are off
    Given a tree file row "README" sized "12" at "/r/HEAD/README"
    And the tree size column is off
    When I render the tree table
    Then the result does not contain "<td class="size""

  Scenario: a parent row links back up the tree
    Given a tree file row "main.rs" sized "80" at "/r/HEAD/src/main.rs"
    And the tree offers a parent row at "/r/HEAD/src-up"
    When I render the tree table
    Then the result contains "/r/HEAD/src-up"
    And the result contains ">..<"

  Scenario: the tree page body wraps the table in the page chrome
    Given a tree file row "README" sized "12" at "/r/HEAD/README"
    When I render the tree page
    Then the result contains "class="page-header""
    And the result contains "main tree"
    And the result contains ">README</a>"
    And the result contains "class="page-footer""
