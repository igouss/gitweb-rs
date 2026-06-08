Feature: Commit date display (the two-week age/date swap)
  gitweb's commit-list bodies (shortlog, log, history, commit, commitdiff) show a
  commit's date in one of two ways depending on how old it is. parse_commit_text
  computes both a relative age ("3 days ago") and the absolute UTC calendar date
  ("2026-06-04"), then decides which one is shown in the cell and which becomes
  the cell's hover tooltip: a commit older than two weeks shows the absolute date
  (age as tooltip); a newer one shows the relative age (date as tooltip). The
  threshold is strict — exactly two weeks still counts as recent.

  The relative age reuses the Age primitive and the absolute date reuses the UTC
  form of the Timestamp primitive; this rule only owns the swap.

  Scenario Outline: A recent commit shows the relative age, with the date as tooltip
    Given a commit dated at epoch <epoch> viewed at <now>
    When I form its date cell
    Then the date cell shows "<shown>"
    And the date cell tooltip is "<tooltip>"

    Examples:
      | epoch      | now        | shown        | tooltip    |
      | 1780583445 | 1780842645 | 3 days ago   | 2026-06-04 |
      | 1779633045 | 1780842645 | 14 days ago  | 2026-05-24 |

  Scenario Outline: An old commit shows the absolute date, with the age as tooltip
    Given a commit dated at epoch <epoch> viewed at <now>
    When I form its date cell
    Then the date cell shows "<shown>"
    And the date cell tooltip is "<tooltip>"

    Examples:
      | epoch      | now        | shown      | tooltip      |
      | 1779633044 | 1780842645 | 2026-05-24 | 2 weeks ago  |
      | 1779028245 | 1780842645 | 2026-05-17 | 3 weeks ago  |
