Feature: Diff change status
  A diff-tree status token names what happened to a path, matching the
  (.)([0-9]{0,3}) status field gitweb reads in parse_difftree_raw_line and the
  six letters git_difftree_body renders.

  Rename and copy tokens carry a similarity score; the other statuses do not.
  A letter gitweb never displays parses to nothing.

  Scenario Outline: Plain status letters name the change
    Given a diff status token "<token>"
    When I read the change status
    Then the change is "<kind>"
    And the similarity is 0

    Examples:
      | token | kind        |
      | A     | Added       |
      | D     | Deleted     |
      | M     | Modified    |
      | T     | TypeChanged |

  Scenario Outline: Rename and copy tokens carry a similarity score
    Given a diff status token "<token>"
    When I read the change status
    Then the change is "<kind>"
    And the similarity is <score>

    Examples:
      | token | kind    | score |
      | R100  | Renamed | 100   |
      | C75   | Copied  | 75    |
      | R     | Renamed | 0     |

  Scenario: A similarity score on a plain status is rejected
    Given a diff status token "M50"
    When I read the change status
    Then there is no change

  Scenario: An unknown status letter is rejected
    Given a diff status token "Z"
    When I read the change status
    Then there is no change
