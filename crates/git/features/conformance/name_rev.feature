Feature: name-rev --tags ancestor naming through the gix adapter
  The Repository port's rev_name_tag is gitweb's git_get_rev_name_tags
  (git name-rev --tags <oid>), the source of the commitdiff_plain / patch
  X-Git-Tag line. The naming rule itself is the pure model::name_rev port; this
  is the gix adapter honouring it — resolving each refs/tags ref to its peeled
  commit, name, tagger date and dereference flag, and collecting the ancestry
  the rule walks — over a deterministic fixture built with gix.

  The distance-zero cases (a tag whose tip IS the commit) live in
  object_reads.feature on the populated fixture; here a dedicated tagged history
  exercises ancestor distance and the multi-tag tie-break, off its own roots so
  it perturbs no other scenario's fixture.

  Background:
    Given a tagged history

  # --- ancestor distance: ~N first-parent generations behind a tag ---

  Scenario: a commit one generation behind an annotated tag is named with ~1
    When I read the rev-name tag of "p1"
    Then the rev-name tag is "rel~1"

  Scenario: a commit two generations behind strips the annotated ^0 before ~N
    When I read the rev-name tag of "p0"
    Then the rev-name tag is "rel~2"

  # --- multi-tag tie-break: equidistant tags, the older tag wins ---

  Scenario: equidistant tags break the tie in favour of the older tag
    When I read the rev-name tag of "t0"
    Then the rev-name tag is "older~1"
