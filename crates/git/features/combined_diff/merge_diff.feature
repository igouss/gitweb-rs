Feature: Combined multi-parent merge diff
  The gix adapter computes the combined diff of a merge against all its parents
  at once, the way gitweb's `git diff-tree -c`/`--cc` does. gix has no native
  combined diff, so the adapter composes one from per-parent tree comparisons,
  applying git's rule: a path appears only if it differs from EVERY parent (a
  path that matches even one parent was taken verbatim from that parent and is
  uninteresting). Each shown path keeps one from-side per parent — with the
  status of the path relative to that parent — and a single merge-result side.

  Comparison is path-keyed: rename and copy detection across the combined merge
  is out of scope here, as for gitweb's raw combined output.

  Scenario: A path that differs from both parents is the only one shown
    Given a two-parent merge where one file differs from both parents
    When I compute the combined diff
    Then 1 combined path changed
    And the merged path "only.txt" has 2 parents
    And the change to "only.txt" against parent 1 is "Modified"
    And the change to "only.txt" against parent 2 is "Modified"

  Scenario: A path identical to one parent is taken verbatim and omitted
    Given a two-parent merge where one file matches a parent
    When I compute the combined diff
    Then no combined path "same.txt" changed
    And no combined path "matches.txt" changed
    And the merged path "only.txt" has 2 parents

  Scenario: A file new to every parent is a combined addition
    Given a two-parent merge that adds a file new to both parents
    When I compute the combined diff
    Then 1 combined path changed
    And the change to "added.txt" against parent 1 is "Added"
    And the change to "added.txt" against parent 2 is "Added"
    And the from-oid of "added.txt" against parent 1 is all zeros

  Scenario: A file dropped from every parent is a combined deletion
    Given a two-parent merge that deletes a file present in both parents
    When I compute the combined diff
    Then 1 combined path changed
    And the change to "gone.txt" against parent 1 is "Deleted"
    And the change to "gone.txt" against parent 2 is "Deleted"
    And the merged path "gone.txt" is a deletion

  Scenario: An octopus merge keeps one from-side per parent
    Given a three-parent merge where one file differs from all parents
    When I compute the combined diff
    Then 1 combined path changed
    And the merged path "octo.txt" has 3 parents
    And the change to "octo.txt" against parent 3 is "Modified"

  Scenario: A missing commit fails to find an object
    Given a missing merge commit id
    When I compute the combined diff
    Then the combined diff fails to find an object
