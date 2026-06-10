Feature: Conditional-GET freshness (If-Modified-Since)

  gitweb's exit_if_unmodified_since compares a resource's Last-Modified epoch
  with the client's If-Modified-Since: a resource no newer than the cached copy
  is "not modified" (a bare 304); a newer resource, a missing header, or an
  unparseable one is served. The epoch under test is 1700086400, which is
  "Wed, 15 Nov 2023 22:13:20 +0000".

  Scenario: a request with no If-Modified-Since serves the resource
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness with no cached copy
    Then the resource is modified

  Scenario: a cached copy older than the resource serves it
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "Wed, 15 Nov 2023 21:13:20 +0000"
    Then the resource is modified

  Scenario: a cached copy equal to the resource is not modified
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "Wed, 15 Nov 2023 22:13:20 +0000"
    Then the resource is not modified

  Scenario: a cached copy newer than the resource is not modified
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "Thu, 16 Nov 2023 00:00:00 +0000"
    Then the resource is not modified

  Scenario: a GMT-zoned HTTP date is honoured
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "Wed, 15 Nov 2023 22:13:20 GMT"
    Then the resource is not modified

  Scenario: a date with no leading weekday is honoured
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "15 Nov 2023 22:13:20 GMT"
    Then the resource is not modified

  Scenario: a non-zero numeric offset is normalised to UTC
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "Wed, 15 Nov 2023 23:13:20 +0100"
    Then the resource is not modified

  Scenario: an unparseable date serves the resource
    Given a resource last modified at epoch 1700086400
    When I evaluate freshness against "yesterday afternoon"
    Then the resource is modified

  # The parser that backs freshness normalises every zone it accepts to a single
  # UTC epoch. GMT/UTC are zero; a numeric +HHMM/-HHMM is its signed offset, east
  # positive; an unrecognised or malformed zone is read as UTC. Each label below
  # is the SAME instant as the reference "Wed, 15 Nov 2023 22:13:20 +0000" — epoch
  # 1700086400 — so a correct parse lands them all on that number.

  Scenario: a western numeric offset normalises to UTC
    When I parse the HTTP date "Wed, 15 Nov 2023 17:13:20 -0500"
    Then the parsed epoch is 1700086400

  Scenario: a non-zero-minute eastern offset normalises to UTC
    When I parse the HTTP date "Thu, 16 Nov 2023 10:58:20 +1245"
    Then the parsed epoch is 1700086400

  Scenario: a zone token of the wrong length is read as UTC
    When I parse the HTTP date "Wed, 15 Nov 2023 22:13:20 +05"
    Then the parsed epoch is 1700086400

  Scenario: a zone token with non-digits is read as UTC
    When I parse the HTTP date "Wed, 15 Nov 2023 22:13:20 +12ab"
    Then the parsed epoch is 1700086400

  Scenario: an out-of-range day of month is unparseable
    When I parse the HTTP date "Wed, 32 Nov 2023 22:13:20 GMT"
    Then the date is unparseable
