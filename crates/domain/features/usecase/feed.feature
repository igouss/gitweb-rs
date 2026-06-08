Feature: Assembling a syndication feed (gitweb git_feed)

  The feed use case reads the recent log of a branch (HEAD by default), windows
  it, and diffs each commit against its parent to list the files it changed —
  narrowing to one file's history when a path is given. It drives the Repository
  port; here it runs against the in-memory fake.

  Scenario: an unborn repository yields an empty feed
    When I assemble the feed of the default branch
    Then the feed has 0 entries
    And the feed has no latest timestamp

  Scenario: a single commit becomes one entry with the files it changed
    Given the repository HEAD is at commit "c1"
    And a commit "c1" at epoch 1700000000 by "Ada" titled "first commit"
    And commit "c1" changes file "README" to blob "r1"
    When I assemble the feed of the default branch
    Then the feed has 1 entries
    And the feed has a latest timestamp
    And the feed entry "c1" is titled "first commit"
    And the feed entry "c1" lists files "README"

  Scenario: every commit appears in a whole-branch feed
    Given the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1700000300 by "Ada" titled "third"
    And commit "c3" changes file "c.txt" to blob "c3b"
    And a commit "c2" at epoch 1700000200 by "Ada" titled "second"
    And commit "c2" changes file "b.txt" to blob "c2b"
    And a commit "c1" at epoch 1700000100 by "Ada" titled "first"
    And commit "c1" changes file "a.txt" to blob "c1b"
    When I assemble the feed of the default branch
    Then the feed has 3 entries

  Scenario: narrowing to a file keeps only its commits and only its changes
    Given the repository HEAD is at commit "c3"
    And a commit "c3" at epoch 1700000300 by "Ada" titled "touch both"
    And commit "c3" changes file "src/foo.c" to blob "f3"
    And commit "c3" changes file "docs/bar.md" to blob "d3"
    And a commit "c2" at epoch 1700000200 by "Ada" titled "docs only"
    And commit "c2" changes file "docs/bar.md" to blob "d2"
    And a commit "c1" at epoch 1700000100 by "Ada" titled "foo only"
    And commit "c1" changes file "src/foo.c" to blob "f1"
    When I assemble the feed of the default branch narrowed to "src/foo.c"
    Then the feed has 2 entries
    And the feed entry "c3" lists files "src/foo.c"
    And the feed entry "c1" lists files "src/foo.c"
