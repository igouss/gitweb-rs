Feature: Blame (line-by-line attribution)
  gitweb's blame (git_blame_common over `git blame -p`) attributes every line of a
  file, as of a base commit, to the commit that last changed it. The use case
  resolves the base revision (HEAD by default), reads the base commit (so the page
  can show its subject, and a missing one is an error), runs the repository's
  blame over the path, then folds the per-line attribution into groups of
  consecutive lines sharing one commit — the unit gitweb renders as a single
  rowspan and emits as one `git blame --incremental` record.

  Each group carries its commit (full id and the 8-char short id gitweb shows),
  the author and the author's local civil date (gitweb's iso-tz), the original and
  final line numbers of the group's first line, and the number of lines in the
  group. Every line keeps its own original and final line numbers and its text, so
  the same view serves the server-rendered table, the incremental shell, and the
  blame_data stream.

  The author and the date come from the introducing commit, read once per distinct
  commit (gitweb's %metainfo cache), not once per line.

  Scenario: a file written whole by one commit is one group of every line
    Given the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1136160000 by "Alice" titled "root"
    And blaming "poem.txt" attributes line 1 to "c1" reading "alpha"
    And blaming "poem.txt" attributes line 2 to "c1" reading "beta"
    And blaming "poem.txt" attributes line 3 to "c1" reading "gamma"
    When I assemble the blame of "poem.txt" from the default branch
    Then the blame commit title is "root"
    And the blame file name is "poem.txt"
    And the blame has 1 group
    And blame group 1 is attributed to "c1"
    And blame group 1 short id is "63310000"
    And blame group 1 is by "Alice"
    And blame group 1 spans 3 lines
    And blame group 1 starts at final line 1
    And the blame has 3 lines
    And blame line 1 reads "alpha"
    And blame line 3 reads "gamma"

  Scenario: consecutive lines from different commits are separate groups
    Given the repository HEAD is at commit "c3"
    And a commit "c1" at epoch 1136160000 by "Alice" titled "first"
    And a commit "c2" at epoch 1136246400 by "Bob" titled "second"
    And a commit "c3" at epoch 1136332800 by "Carol" titled "third"
    And blaming "f.txt" attributes line 1 to "c1" reading "one"
    And blaming "f.txt" attributes line 2 to "c3" reading "TWO"
    And blaming "f.txt" attributes line 3 to "c1" reading "three"
    And blaming "f.txt" attributes line 4 to "c2" reading "four"
    When I assemble the blame of "f.txt" from the default branch
    Then the blame has 4 groups
    And blame group 1 is attributed to "c1"
    And blame group 1 spans 1 line
    And blame group 2 is attributed to "c3"
    And blame group 2 is by "Carol"
    And blame group 2 starts at final line 2
    And blame group 3 is attributed to "c1"
    And blame group 4 is attributed to "c2"
    And blame group 4 is by "Bob"

  Scenario: adjacent lines from one commit fold into a single multi-line group
    Given the repository HEAD is at commit "c2"
    And a commit "c1" at epoch 1136160000 by "Alice" titled "first"
    And a commit "c2" at epoch 1136246400 by "Bob" titled "second"
    And blaming "f.txt" attributes line 1 to "c1" reading "a"
    And blaming "f.txt" attributes line 2 to "c2" reading "b"
    And blaming "f.txt" attributes line 3 to "c2" reading "c"
    And blaming "f.txt" attributes line 4 to "c1" reading "d"
    When I assemble the blame of "f.txt" from the default branch
    Then the blame has 3 groups
    And blame group 2 is attributed to "c2"
    And blame group 2 spans 2 lines
    And blame group 2 starts at final line 2

  Scenario: the author date is the introducing commit's local civil time
    Given the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1136214245 in zone "+0900" by "Junio" titled "stamp"
    And blaming "g.txt" attributes line 1 to "c1" reading "x"
    When I assemble the blame of "g.txt" from the default branch
    Then blame group 1 date is "2006-01-03 00:04:05 +0900"
    And blame group 1 author time is 1136214245
    And blame group 1 author zone is "+0900"

  Scenario: an empty file has no groups and no lines
    Given the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1136160000 by "Alice" titled "empty"
    And blaming "empty.txt" yields no lines
    When I assemble the blame of "empty.txt" from the default branch
    Then the blame has 0 groups
    And the blame has 0 lines

  Scenario: a missing base commit is an error
    Given a commit "c1" at epoch 1136160000 by "Alice" titled "orphan"
    And blaming "poem.txt" attributes line 1 to "c1" reading "alpha"
    When I assemble the blame of "poem.txt" from "ghost"
    Then assembling the blame fails

  Scenario: a missing path is an error
    Given the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1136160000 by "Alice" titled "root"
    And blaming "nope.txt" fails to find it
    When I assemble the blame of "nope.txt" from the default branch
    Then assembling the blame fails
