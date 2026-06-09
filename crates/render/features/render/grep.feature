Feature: Rendering the grep-results table
  The grep page (modernized git_search_files) groups the matches per file: a file
  row links to the blob, then one row per matching line — the 1-based line number
  linking to blob#lN and the untabified line text with its matched span in a
  <span class="match"> — or a single "Binary file" row for a binary match. An
  empty result shows "No matches found"; an overflowing one appends "Too many
  matches, listing trimmed". URLs are built by the web boundary, so this layer
  takes finished hrefs and only decides layout and escaping.

  Scenario: a file links to its blob and a matching line links to its blob anchor
    Given a grep file "src/a.txt" at "/r/blob/a"
    And a grep line 7 at "/r/blob/a#l7" highlighting lead "the " match "needle" trail " here"
    When I render the grep page
    Then the result contains "/r/blob/a"
    And the result contains "src/a.txt"
    And the result contains "/r/blob/a#l7"
    And the result contains "the "
    And the result contains "<span class="match">needle</span>"
    And the result contains " here"

  Scenario: a binary match shows a binary-file row
    Given a grep file "data.bin" at "/r/blob/bin"
    And a grep binary row
    When I render the grep page
    Then the result contains "data.bin"
    And the result contains "Binary file"

  Scenario: a line the highlighter could not mark is shown whole and plain
    Given a grep file "src/a.txt" at "/r/blob/a"
    And a grep line 2 at "/r/blob/a#l2" plain "nothing to mark"
    When I render the grep page
    Then the result contains "nothing to mark"
    And the result does not contain "<span class="match">"

  Scenario: an empty result says there were no matches
    Given the grep has no matches
    When I render the grep page
    Then the result contains "No matches found"

  Scenario: an overflowing result appends the trim notice
    Given a grep file "src/a.txt" at "/r/blob/a"
    And a grep line 1 at "/r/blob/a#l1" highlighting lead "" match "x" trail ""
    And the grep listing is trimmed
    When I render the grep page
    Then the result contains "Too many matches, listing trimmed"
