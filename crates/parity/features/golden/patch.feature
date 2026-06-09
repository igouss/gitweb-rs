Feature: patch golden conformance
  gitweb's patch action streams `git format-patch --stdout` verbatim, so its
  bytes are git's mailbox format and must match the original. It is captured over
  the corpus `texts` branch root commit — a text-only `--root` create of two
  files — so the whole mail is reproducible: the `From <id> Mon Sep 17` magic
  separator, the From / Date / Subject headers, the body, the diffstat (column
  alignment and `create mode` lines), the create diff, and the `-- ` / git
  version signature. git's binary patch body is its own zlib output, out of reach
  of the gix-only no-unsafe port, so it is deliberately not in this corpus.

  Scenario: corpus texts-commit patch matches byte for byte
    Given the parity corpus
    When I serve the patch of the corpus texts commit
    Then the patch body matches gitweb's reference output
    And the patch content type matches gitweb's
    And the patch content disposition matches gitweb's
