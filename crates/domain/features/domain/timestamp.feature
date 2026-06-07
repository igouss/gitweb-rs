Feature: Absolute timestamp formatting
  Commit, tag and tagger dates are shown as absolute civil timestamps, matching
  gitweb's parse_date + format_timestamp_html. The UTC forms (RFC-2822 and
  ISO-8601) feed the machine-readable endpoints — the RSS/Atom feeds and the
  Last-Modified header — while the local form renders the date in the commit's
  own timezone. The "at night" rule (a local hour before 06:00) is gitweb's, and
  drives the render layer's highlighting.

  All civil-date arithmetic is done in-domain with no calendar library: negative
  epochs and timezone offsets that cross day, month and year boundaries all fall
  out of the same days-from-epoch computation.

  Scenario Outline: The RFC-2822 form is UTC, regardless of timezone
    Given a timestamp at epoch <epoch> with timezone "<tz>"
    When I render its RFC-2822 form
    Then the timestamp reads "<text>"

    Examples:
      | epoch      | tz    | text                            |
      | 0          | +0000 | Thu, 1 Jan 1970 00:00:00 +0000  |
      | 1780842645 | +0200 | Sun, 7 Jun 2026 14:30:45 +0000  |
      | 1719790200 | +0200 | Sun, 30 Jun 2024 23:30:00 +0000 |
      | 1704074400 | -0500 | Mon, 1 Jan 2024 02:00:00 +0000  |
      | 1709208000 | +0000 | Thu, 29 Feb 2024 12:00:00 +0000 |
      | 1704067199 | +0000 | Sun, 31 Dec 2023 23:59:59 +0000 |

  Scenario Outline: The ISO-8601 form is UTC, with a trailing Z
    Given a timestamp at epoch <epoch> with timezone "<tz>"
    When I render its ISO-8601 form
    Then the timestamp reads "<text>"

    Examples:
      | epoch      | tz    | text                 |
      | 0          | +0000 | 1970-01-01T00:00:00Z |
      | 1780842645 | +0200 | 2026-06-07T14:30:45Z |
      | 1709208000 | +0000 | 2024-02-29T12:00:00Z |
      | 1704067199 | +0000 | 2023-12-31T23:59:59Z |

  Scenario Outline: The local form shifts the civil date into the commit timezone
    Given a timestamp at epoch <epoch> with timezone "<tz>"
    When I render its local form
    Then the timestamp reads "<text>"

    Examples:
      | epoch      | tz    | text                      |
      | 0          | +0000 | 1970-01-01 00:00:00 +0000 |
      | 1780842645 | +0200 | 2026-06-07 16:30:45 +0200 |
      | 1719790200 | +0200 | 2024-07-01 01:30:00 +0200 |
      | 1704074400 | -0500 | 2023-12-31 21:00:00 -0500 |

  Scenario: An empty timezone defaults to -0000, like gitweb's parse_date
    Given a timestamp at epoch 0 with timezone ""
    When I render its local form
    Then the timestamp reads "1970-01-01 00:00:00 -0000"

  Scenario Outline: A local hour before 06:00 counts as "at night"
    Given a timestamp at epoch <epoch> with timezone "<tz>"
    When I check whether it is at night
    Then at-night is <flag>

    Examples:
      | epoch      | tz    | flag  | local |
      | 0          | +0000 | true  | 00:00 |
      | 1719790200 | +0200 | true  | 01:30 |
      | 1780842645 | +0200 | false | 16:30 |
      | 1704074400 | -0500 | false | 21:00 |

  Scenario: The local hint exposes the hour, minute and timezone the html shows
    Given a timestamp at epoch 1780842645 with timezone "+0200"
    Then the local hour is 16
    And the local minute is 30
    And the displayed timezone is "+0200"

  Scenario: The local hint reflects a negative timezone offset
    Given a timestamp at epoch 1704074400 with timezone "-0500"
    Then the local hour is 21
    And the local minute is 0
    And the displayed timezone is "-0500"

  Scenario: An absent timestamp renders nothing, like git_summary's defined-guard
    Given there is no timestamp
    When I render its RFC-2822 form
    Then nothing is rendered
