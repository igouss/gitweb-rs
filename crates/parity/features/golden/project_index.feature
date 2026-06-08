Feature: The machine-readable project index matches gitweb's reference output

  project_index is format-stable, so it is verified by golden differential
  conformance: the bytes our use case + web view-model + render serializer
  produce over the corpus must equal the reference captured once from the
  original gitweb.perl — body, content type, and content disposition alike.

  Scenario: the project index is byte-for-byte gitweb's
    Given the parity corpus
    When I serve the project index
    Then the project index body matches gitweb's reference output
    And the project index content type matches gitweb's
    And the project index content disposition matches gitweb's
