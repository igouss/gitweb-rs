Feature: Age recency CSS class
  gitweb colours each "last changed" value by how recently it changed
  (age_class: noage/age0/age1/age2). The bucketing rule is a domain concern
  (AgeClass); the render layer only maps each bucket to the CSS class the fresh
  stylesheet targets.

  Scenario: an unknown age has the unknown class
    Given an age classification of unknown
    When I ask for its CSS class
    Then the result is "age-unknown"

  Scenario: a fresh age has the fresh class
    Given an age classification of fresh
    When I ask for its CSS class
    Then the result is "age-fresh"

  Scenario: a recent age has the recent class
    Given an age classification of recent
    When I ask for its CSS class
    Then the result is "age-recent"

  Scenario: an old age has the old class
    Given an age classification of old
    When I ask for its CSS class
    Then the result is "age-old"
