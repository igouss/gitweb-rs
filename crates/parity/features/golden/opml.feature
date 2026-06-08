Feature: The OPML project outline matches gitweb's reference output

  opml is format-stable, so it is verified by golden differential conformance:
  the bytes our use case + web view-model + render serializer produce over the
  corpus must equal the reference captured once from the original gitweb.perl —
  body, content type, and content disposition alike.

  Scenario: the opml outline is byte-for-byte gitweb's
    Given the parity corpus
    When I serve the opml outline
    Then the opml body matches gitweb's reference output
    And the opml content type matches gitweb's
    And the opml content disposition matches gitweb's
