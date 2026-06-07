Feature: Combined merge diff entry
  A combined diff compares a merge against all its parents at once, the way
  gitweb renders `git diff-tree -c`/`--cc`. For one changed path it keeps, per
  parent, the status of the path relative to that parent, plus a single result
  (to) side. These are the pure rules gitweb derives from that shape:
  is_deleted (the result id is the null id), has_history (some parent's change
  is not an add, so there is a prior version to link), and not_deleted (some
  parent's change is not a delete, so the path survives somewhere).

  Scenario: A path modified against both parents
    Given a combined entry modified against two parents
    When I read its combined-diff flags
    Then the combined entry has 2 parents
    And the combined entry is not a deletion
    And the combined entry has history
    And the combined entry survives against some parent

  Scenario: A path added against every parent has no prior version
    Given a combined entry added against two parents
    When I read its combined-diff flags
    Then the combined entry is not a deletion
    And the combined entry has no history
    And the combined entry survives against some parent

  Scenario: A path deleted against every parent is gone
    Given a combined entry deleted against two parents
    When I read its combined-diff flags
    Then the combined entry is a deletion
    And the combined entry has history
    And the combined entry is gone from every parent

  Scenario: An octopus merge keeps one from-side per parent
    Given a combined entry modified against three parents
    When I read its combined-diff flags
    Then the combined entry has 3 parents
