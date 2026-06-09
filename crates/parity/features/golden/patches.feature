Feature: patches golden conformance
  gitweb's patches action streams `git format-patch --stdout` for a commit range
  verbatim, so its bytes are git's numbered mailbox format and must match the
  original. It is captured over the whole two-commit `texts` branch — a text-only
  root create plus a modify-and-add — so the whole stream is reproducible: the
  `[PATCH 1/2]` / `[PATCH 2/2]` Subjects, both mailbox headers, both diffstats
  (column alignment, `create mode` lines, the mixed insertion/deletion summary),
  the create and modification diffs, and each `-- ` / git version signature. git's
  binary patch body is its own zlib output, out of reach of the gix-only no-unsafe
  port, so this corpus is deliberately text-only.

  Scenario: corpus texts-range patches match byte for byte
    Given the parity corpus
    When I serve the patches of the corpus texts range
    Then the patches body matches gitweb's reference output
    And the patches content type matches gitweb's
    And the patches content disposition matches gitweb's
