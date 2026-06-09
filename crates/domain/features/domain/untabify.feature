Feature: Tab expansion for display (untabify)
  gitweb's `untabify` expands every tab to the spaces that reach the next
  eight-column tab stop, so a grep line (or any pre-formatted text) lines up the
  way it would in an editor with `tabstop=8`. The column counts characters from
  the start of the line, so a tab's width is `8 - (column % 8)`: a tab at the
  very start fills a whole eight columns, a tab one column in fills seven, and a
  tab sitting exactly on a stop fills another full eight. The expansion re-counts
  from the running column, so several tabs each reach the next stop.

  In these scenarios the visible width of every expected line is pinned by its
  literal spaces.

  # --- zero tabs: the line is unchanged ---

  Scenario: a line with no tab is returned unchanged
    When I untabify "plain text"
    Then the untabified line is "plain text"

  Scenario: an empty line stays empty
    When I untabify ""
    Then the untabified line is ""

  # --- one tab: width depends on the column ---

  Scenario: a tab at the start fills a whole eight-column stop
    When I untabify "\tx"
    Then the untabified line is "        x"

  Scenario: a tab one column in fills seven columns to the next stop
    When I untabify "a\tb"
    Then the untabified line is "a       b"

  Scenario: a tab sitting exactly on a stop fills another full eight columns
    When I untabify "12345678\tx"
    Then the untabified line is "12345678        x"

  # --- many tabs: each reaches the next stop from the running column ---

  Scenario: two leading tabs each fill a full stop
    When I untabify "\t\t"
    Then the untabified line is "                "

  Scenario: tabs interleaved with text each reach the next stop
    When I untabify "a\tbc\td"
    Then the untabified line is "a       bc      d"
