Feature: The git format-patch diffstat
  Between a patch's `---` line and its diff, git format-patch prints a diffstat:
  a `--stat` block (one `name | total <graph>` line per file, then a
  `N files changed, …` summary) and a `--summary` block (create / delete /
  rename / copy / mode-change lines). gitweb streams format-patch verbatim, so
  these bytes are git's `show_stats` + `diff_summary` and must match exactly.

  The `+`/`-` graph is scaled to format-patch's fixed 72-column wrap
  (`MAIL_DEFAULT_WRAP`), not the terminal width, with at least one bar kept per
  changed file. The name column shows a rename or copy as the compact
  `old => new` form, collapsing any shared directory prefix or suffix into
  braces, exactly as git's `pprint_rename` does.

  Scenario: A single created text file
    Given a diffstat
    And a created file "f.txt" mode "100644" with 1 added 0 deleted
    When I render the diffstat
    Then the diffstat is:
      """
       f.txt | 1 +
       1 file changed, 1 insertion(+)
       create mode 100644 f.txt
      """

  Scenario: A modified text file scales its graph and counts both sides
    Given a diffstat
    And a modified file "f.txt" mode "100644" with 5 added 1 deleted
    When I render the diffstat
    Then the diffstat is:
      """
       f.txt | 6 +++++-
       1 file changed, 5 insertions(+), 1 deletion(-)
      """

  Scenario: A deleted text file
    Given a diffstat
    And a deleted file "f.txt" mode "100644" with 0 added 5 deleted
    When I render the diffstat
    Then the diffstat is:
      """
       f.txt | 5 -----
       1 file changed, 5 deletions(-)
       delete mode 100644 f.txt
      """

  Scenario: An exact rename shows a zero-change stat and a rename summary
    Given a diffstat
    And a renamed file from "old.txt" to "new.txt" similarity 100 mode "100644"
    When I render the diffstat
    Then the diffstat is:
      """
       old.txt => new.txt | 0
       1 file changed, 0 insertions(+), 0 deletions(-)
       rename old.txt => new.txt (100%)
      """

  Scenario: A rename within a directory collapses the shared prefix into braces
    Given a diffstat
    And a renamed file from "src/old.rs" to "src/foo.rs" similarity 100 mode "100644"
    When I render the diffstat
    Then the diffstat is:
      """
       src/{old.rs => foo.rs} | 0
       1 file changed, 0 insertions(+), 0 deletions(-)
       rename src/{old.rs => foo.rs} (100%)
      """

  Scenario: A pure mode change shows a zero-change stat and a mode-change summary
    Given a diffstat
    And a mode change on "script.sh" from "100644" to "100755"
    When I render the diffstat
    Then the diffstat is:
      """
       script.sh | 0
       1 file changed, 0 insertions(+), 0 deletions(-)
       mode change 100644 => 100755 script.sh
      """

  Scenario: A binary file shows its byte sizes instead of a graph
    Given a diffstat
    And a binary file "b.dat" mode "100644" sized 4 to 6
    When I render the diffstat
    Then the diffstat is:
      """
       b.dat | Bin 4 -> 6 bytes
       1 file changed, 0 insertions(+), 0 deletions(-)
      """

  Scenario: Many files align the name and number columns
    Given a diffstat
    And a created file "b.c" mode "100644" with 1 added 0 deleted
    And a created file "deep/nested/reallylongpathname.md" mode "100644" with 10 added 0 deleted
    And a created file "longername.txt" mode "100644" with 5 added 0 deleted
    When I render the diffstat
    Then the diffstat is:
      """
       b.c                               |  1 +
       deep/nested/reallylongpathname.md | 10 ++++++++++
       longername.txt                    |  5 +++++
       3 files changed, 16 insertions(+)
       create mode 100644 b.c
       create mode 100644 deep/nested/reallylongpathname.md
       create mode 100644 longername.txt
      """

  Scenario: A change wider than the wrap scales the graph to 56 bars
    Given a diffstat
    And a created file "big.txt" mode "100644" with 200 added 0 deleted
    When I render the diffstat
    Then the diffstat is:
      """
       big.txt | 200 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++
       1 file changed, 200 insertions(+)
       create mode 100644 big.txt
      """
