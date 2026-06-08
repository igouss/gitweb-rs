Feature: Limiting a summary section to its cap
  gitweb's summary page shows at most a fixed number of items in each of its
  shortlog, tags, heads and forks sections, and offers a "..." link to the full
  action only when the list overran that cap. That single rule — keep the first
  `cap` items, remember whether more were dropped — is captured here once. The
  cap is a parameter, so "more than the cap" is exercised with a small list.

  Background:
    Given a section cap of 2

  Scenario: an empty section shows nothing and is not truncated
    Given the section items ""
    When I limit the section
    Then the shown section items are ""
    And the section is not truncated

  Scenario: a section under the cap shows everything and is not truncated
    Given the section items "a"
    When I limit the section
    Then the shown section items are "a"
    And the section is not truncated

  Scenario: a section exactly at the cap shows everything and is not truncated
    Given the section items "a, b"
    When I limit the section
    Then the shown section items are "a, b"
    And the section is not truncated

  Scenario: a section over the cap is cut to the cap and flagged truncated
    Given the section items "a, b, c"
    When I limit the section
    Then the shown section items are "a, b"
    And the section is truncated
