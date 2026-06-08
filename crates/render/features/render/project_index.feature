Feature: Machine-readable project index serialization (gitweb git_project_index)

  The project index is format-stable, so the serializer reproduces gitweb's
  "index.aux" to the byte: one "path owner" line per project, each field
  CGI-quoted (the slash kept, a space turned into '+'), each line newline-ended.
  The parity crate's golden test proves byte-exactness against real gitweb.

  Scenario: a project renders as a CGI-quoted, newline-ended line
    Given a project index entry "repo.git" owned by "Ada Lovelace"
    When I render the project index
    Then the rendered body is the newline-ended lines:
      """
      repo.git Ada+Lovelace
      """

  Scenario: every project is on its own line, in order
    Given a project index entry "zlib.git" owned by "z"
    And a project index entry "make.git" owned by "m"
    When I render the project index
    Then the rendered body is the newline-ended lines:
      """
      zlib.git z
      make.git m
      """

  Scenario: an empty index renders an empty body
    Given an empty project index
    When I render the project index
    Then the index body is empty
