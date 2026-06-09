Feature: Classifying a tag's listing age (git_get_tags_list)
  gitweb records a tag's creation time only when the ref's top-level object is a
  tag or a commit; from there git_tags_body renders THREE distinct states. A ref
  with a real creation time shows its relative age. A tag or commit ref whose
  creation time is zero shows the literal "unknown". A ref that carries no
  creation time at all — a lightweight tag of a blob or a tree, whose object type
  is neither tag nor commit — has no age field, and gitweb prints an empty cell.
  The use case feeds this rule the creation time (none when the ref carries one
  of the timeless kinds); the rule keeps the three apart.

  Background:
    Given the tag request time is 1000000

  Scenario: a recorded, non-zero creation time gives a relative age
    Given a tag created at 999400
    When I classify the tag age
    Then the tag age is "10 min ago"

  Scenario: a recorded but zero creation time is "unknown"
    Given a tag created at 0
    When I classify the tag age
    Then the tag age is unknown

  Scenario: no recorded creation time has no age cell
    Given a tag with no recorded creation time
    When I classify the tag age
    Then the tag age has no cell
