Feature: Assembling grep-search results
  gitweb's git_search dispatches a grep search to git_search_files: it greps the
  base revision's tree and lists, grouped per file in tree order, each matching
  line with its matched span highlighted (untabified, the whole pattern matched
  case-insensitively for display), each file linking to its blob and each line to
  blob#lN; a binary match shows "Binary file"; the listing is capped, with a
  "listing trimmed" notice when it overflows. This use case orchestrates that over
  the Repository port: it gates on `search` then `grep` (403 each when off),
  validates the pattern (400 on a bad regexp), resolves the base (HEAD by default,
  404 "Unknown commit object" when it names no commit), greps it, and groups the
  matches. The matching, rooting, and 1000-match cap are the adapter's conformance;
  the fake returns the configured matches so the assembly's own behaviour is what
  is driven here. In a line's text the escape "\t" is a tab.

  Background:
    Given a commit "base" at epoch 1000 by "Tester" titled "grep base"
    And the repository HEAD is at commit "base"

  # --- the two feature gates, in gitweb's order ---

  Scenario: a grep is forbidden when the search feature is disabled
    When I assemble a search-disabled grep for "needle"
    Then the grep is forbidden as "Search is disabled"

  Scenario: a grep is forbidden when the grep feature is disabled
    When I assemble a grep-disabled grep for "needle"
    Then the grep is forbidden as "Grep search is disabled"

  # --- pattern validation and base resolution ---

  Scenario: a malformed regular expression is rejected
    When I assemble a regexp grep for "a(b"
    Then the grep is invalid

  Scenario: a base revision that names no commit is an unknown commit object
    When I assemble a grep for "needle" rooted at "nope"
    Then the grep reports an unknown commit object

  # --- the page header is the base commit's subject ---

  Scenario: the header shows the base commit subject and roots blob links at it
    When I assemble a grep for "needle"
    Then the grep header title is "grep base"
    And the grep roots blob links at "base"

  # --- zero / one / many files ---

  Scenario: a grep with no matches lists no files and is not trimmed
    When I assemble a grep for "needle"
    Then the grep lists 0 files
    And the grep result is not trimmed

  Scenario: a single matching line in one file is grouped under that file
    Given a grep line match in "src/a.txt" at line 7 with text "the needle here"
    When I assemble a grep for "needle"
    Then the grep lists 1 file
    And grep file 0 is "src/a.txt" with 1 row
    And grep file 0 row 0 is line 7 highlighting lead "the " match "needle" trail " here"

  Scenario: several lines in one file are grouped, and several files are listed in order
    Given a grep line match in "a.txt" at line 1 with text "needle one"
    And a grep line match in "a.txt" at line 4 with text "needle two"
    And a grep line match in "b.txt" at line 2 with text "a needle"
    When I assemble a grep for "needle"
    Then the grep lists 2 files
    And grep file 0 is "a.txt" with 2 rows
    And grep file 0 row 0 is line 1 highlighting lead "" match "needle" trail " one"
    And grep file 0 row 1 is line 4 highlighting lead "" match "needle" trail " two"
    And grep file 1 is "b.txt" with 1 row
    And grep file 1 row 0 is line 2 highlighting lead "a " match "needle" trail ""

  # --- binary matches ---

  Scenario: a binary file is listed once as a binary row
    Given a grep binary match in "data.bin"
    When I assemble a grep for "needle"
    Then the grep lists 1 file
    And grep file 0 is "data.bin" with 1 row
    And grep file 0 row 0 is a binary file

  # --- the highlight is case-insensitive in fixed mode ---

  Scenario: a fixed-mode match is highlighted case-insensitively
    Given a grep line match in "a.txt" at line 1 with text "x NEEDLE y"
    When I assemble a grep for "needle"
    Then grep file 0 row 0 is line 1 highlighting lead "x " match "NEEDLE" trail " y"

  # --- regexp mode highlights the matched span, not the literal ---

  Scenario: a regexp-mode match highlights the regexp's span
    Given a grep line match in "a.txt" at line 1 with text "xx abc yy"
    When I assemble a regexp grep for "a.c"
    Then grep file 0 row 0 is line 1 highlighting lead "xx " match "abc" trail " yy"

  # --- tabs are expanded before the line is shown ---

  Scenario: a matching line is untabified before highlighting
    Given a grep line match in "a.txt" at line 1 with text "\tneedle"
    When I assemble a grep for "needle"
    Then grep file 0 row 0 is line 1 highlighting lead "        " match "needle" trail ""

  # --- a listed line the display regexp cannot mark is shown plain (gitweb's else) ---

  Scenario: a line the highlighter does not match is shown whole and plain
    Given a grep line match in "a.txt" at line 1 with text "\tnothing to mark"
    When I assemble a grep for "needle"
    Then grep file 0 row 0 is line 1 plain "        nothing to mark"

  # --- the cap trim notice ---

  Scenario: a trimmed listing reports it for the notice
    Given a grep line match in "a.txt" at line 1 with text "needle"
    And the grep listing is trimmed
    When I assemble a grep for "needle"
    Then the grep result is trimmed
