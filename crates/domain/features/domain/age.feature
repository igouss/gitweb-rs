Feature: Relative age formatting
  Commit timestamps are shown as human-readable relative ages, matching
  gitweb's age_string, and bucketed into CSS classes via age_class.

  Because gitweb uses factor-of-two thresholds, the smallest count rendered
  for any unit is always two — it never prints "1 min ago".

  Scenario: Something that just happened reads as "right now"
    Given an age of 0 seconds
    When I humanize it
    Then the age reads "right now"

  Scenario: The two-second boundary is still "right now"
    Given an age of 2 seconds
    When I humanize it
    Then the age reads "right now"

  Scenario Outline: Ages render with gitweb's factor-of-two thresholds
    Given an age of <seconds> seconds
    When I humanize it
    Then the age reads "<text>"

    Examples:
      | seconds  | text         |
      | 3        | 3 sec ago    |
      | 119      | 119 sec ago  |
      | 121      | 2 min ago    |
      | 7201     | 2 hours ago  |
      | 172801   | 2 days ago   |
      | 1209601  | 2 weeks ago  |
      | 5256001  | 2 months ago |
      | 63072001 | 2 years ago  |

  Scenario: An unknown age has no class
    Given an unknown age
    When I classify it
    Then the class is "Unknown"

  Scenario Outline: Ages bucket into CSS classes
    Given an age of <seconds> seconds
    When I classify it
    Then the class is "<class>"

    Examples:
      | seconds | class  |
      | 0       | Fresh  |
      | 7199    | Fresh  |
      | 7200    | Recent |
      | 172799  | Recent |
      | 172800  | Old    |
