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

  Scenario Outline: A modification is a type change only when the file type changes
    A diff-tree raw line gives us two modes rather than a status letter, so we
    re-derive git's M-vs-T distinction: changing the executable bit keeps a
    regular file a regular file (M), but swapping between file, symlink, and
    gitlink changes the type (T).

    Given a modification from mode "<from>" to mode "<to>"
    When I classify the modification
    Then the change is "<kind>"
    And the similarity is 0

    Examples:
      | from   | to     | kind        |
      | 100644 | 100644 | Modified    |
      | 100644 | 100755 | Modified    |
      | 100755 | 100644 | Modified    |
      | 100644 | 120000 | TypeChanged |
      | 120000 | 100644 | TypeChanged |
      | 100644 | 160000 | TypeChanged |

  Scenario: A similarity score on a plain status is rejected
    Given a diff status token "M50"
    When I read the change status
    Then there is no change

  Scenario: An unknown status letter is rejected
    Given a diff status token "Z"
    When I read the change status
    Then there is no change
