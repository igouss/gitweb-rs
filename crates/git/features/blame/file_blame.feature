Feature: Line-by-line blame from a repository
  The gix adapter attributes each line of a file, as of a commit, to the commit
  that last changed it — the data gitweb renders from `git blame -p`. Each blamed
  line carries the introducing commit, its line number in that commit (the
  original line number) and in the final file, and its text, in final-file order.

  Like gitweb's plain `git blame`, attribution follows a file across a whole-file
  rename: a line untouched since before the rename is still attributed to the
  commit that first wrote it, not to the rename.

  Scenario: A file written by a single commit attributes every line to it
    Given a file first written whole by one commit
    When I blame "poem.txt" at "c1"
    Then 3 lines are blamed
    And line 1 is attributed to "c1"
    And line 2 is attributed to "c1"
    And line 3 is attributed to "c1"
    And line 1 reads "alpha"
    And line 3 reads "gamma"

  Scenario: A file edited by several commits attributes each line to its commit
    Given a file edited over three commits
    When I blame "f.txt" at "c3"
    Then 4 lines are blamed
    And line 1 is attributed to "c1"
    And line 2 is attributed to "c3"
    And line 3 is attributed to "c1"
    And line 4 is attributed to "c2"
    And line 2 reads "BETA"
    And line 4 reads "delta"

  Scenario: Blame follows a file across a rename
    Given a file renamed then appended to
    When I blame "new.txt" at "c3"
    Then 4 lines are blamed
    And line 1 is attributed to "c1"
    And line 3 is attributed to "c1"
    And line 4 is attributed to "c3"
    And line 1 reads "one"

  Scenario: An empty file has no blamed lines
    Given a commit holding an empty file
    When I blame "empty.txt" at "c1"
    Then 0 lines are blamed

  Scenario: A line keeps the original line number it had in its source commit
    Given a file edited over three commits
    When I blame "f.txt" at "c3"
    Then line 4 has original line number 4
    And line 4 has final line number 4

  Scenario: Blaming a missing path fails to find it
    Given a file first written whole by one commit
    When I blame "nope.txt" at "c1"
    Then the blame fails to find it

  Scenario: Blaming at a missing commit fails to find it
    Given a file first written whole by one commit
    When I blame "poem.txt" at a missing commit
    Then the blame fails to find it
