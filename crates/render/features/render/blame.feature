Feature: Rendering blame (server-rendered, incremental shell, and data stream)
  gitweb's blame has three renderings. The server-rendered table (git_blame_common
  in porcelain mode) shows one row per final line: a `sha1` cell spanning the
  group's lines with the short id linking to the commit, a `linenr` cell linking
  to the line's source, and the line text. The incremental shell serves the same
  rows empty, with a progress bar and a bootstrap that hands the blame_data URL
  and the project URL to the shipped client module. The blame_data stream is the
  plain `git blame --incremental` text the client parses. URLs are built by the
  web boundary; this layer places finished strings and escapes every value.

  Scenario: the server-rendered table spans a group and links its commit and lines
    Given a blame group of 2 lines by "Alice" dated "2006-01-03 00:04:05 +0000"
    And the group commit is "abcd1234" linking to "/p/commit/abcd1234ef"
    And the group line 1 is final 5 reading "alpha" linking to "/p/blame/parent#l5"
    And the group line 2 is final 6 reading "beta" linking to "/p/blame/parent#l6"
    When I render the blame table
    Then the result contains "id="blame_table""
    And the result contains "id="l5""
    And the result contains "id="l6""
    And the result contains "class="sha1""
    And the result contains "rowspan="2""
    And the result contains "/p/commit/abcd1234ef"
    And the result contains "abcd1234"
    And the result contains "/p/blame/parent#l5"
    And the result contains "alpha"
    And the result contains "beta"

  Scenario: the server-rendered group cell carries the author and date tooltip
    Given a blame group of 1 line by "Junio C Hamano" dated "2006-01-03 00:04:05 +0900"
    And the group commit is "deadbeef" linking to "/p/commit/deadbeef00"
    And the group line 1 is final 1 reading "x" linking to "/p/blame/p#l1"
    When I render the blame table
    Then the result contains "Junio C Hamano, 2006-01-03 00:04:05 +0900"

  Scenario: the incremental shell serves empty rows with progress and a bootstrap
    Given an incremental blame line 1 reading "first"
    And an incremental blame line 2 reading "second"
    And the blame data url is "/p/blame_data/HEAD/f.txt"
    And the blame project url is "/p/"
    When I render the incremental blame shell
    Then the result contains "id="blame_table""
    And the result contains "id="progress_bar""
    And the result contains "id="progress_info""
    And the result contains "id="l1""
    And the result contains "id="l2""
    And the result contains "first"
    And the result contains "class="sha1""
    And the result contains "type="module""
    And the result contains "startBlame"
    And the result contains "/p/blame_data/HEAD/f.txt"

  Scenario: the bootstrap escapes control characters but leaves the space above them
    Given an incremental blame line 1 reading "x"
    And the blame data url has a tab then a space
    And the blame project url is "/p/"
    When I render the incremental blame shell
    Then the result contains "/p/\u0009"
    And the result does not contain "\u0020"

  Scenario: the data stream is the git blame --incremental format
    Given a blame data group "abcd1234ef0000000000000000000000000000aa" orig 5 final 5 span 2
    And the data group author is "Alice"
    And the data group author time is 1136160000 zone "+0000"
    And the data group filename is "f.txt"
    When I render the blame data
    Then the result contains "abcd1234ef0000000000000000000000000000aa 5 5 2"
    And the result contains "author Alice"
    And the result contains "author-time 1136160000"
    And the result contains "author-tz +0000"
    And the result contains "filename f.txt"

  Scenario: every data group ends with a filename line so the client fills it
    Given a blame data group "11111111111111111111111111111111111111aa" orig 1 final 1 span 1
    And the data group author is "Bob"
    And the data group author time is 1136160000 zone "+0000"
    And the data group filename is "g.txt"
    When I render the blame data
    Then the result ends after a filename line
