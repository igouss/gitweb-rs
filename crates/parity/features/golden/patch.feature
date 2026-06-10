Feature: patch golden conformance
  gitweb's patch action streams `git format-patch --stdout` verbatim, so its
  bytes are git's mailbox format and must match the original. It is captured over
  the corpus `texts` branch root commit — a text-only `--root` create of two
  files — so the whole mail is reproducible: the `From <id> Mon Sep 17` magic
  separator, the From / Date / Subject headers, the body, the diffstat (column
  alignment and `create mode` lines), the create diff, and the `-- ` / git
  version signature. A binary file's body is git's own base85 `GIT binary patch`
  (a deflated literal or delta, whichever is shorter), reproduced byte-for-byte by
  model::binary_patch — see the binmix scenario below.

  Scenario: corpus texts-commit patch matches byte for byte
    Given the parity corpus
    When I serve the patch of the corpus texts commit
    Then the patch body matches gitweb's reference output
    And the patch content type matches gitweb's
    And the patch content disposition matches gitweb's

  # The binmix head modifies a text file and a binary file. gitweb streams
  # `git format-patch`, which embeds the binary file as a base85 `GIT binary patch`
  # with a FULL `index` (git implies --full-index for an embedded blob), while the
  # text file keeps its abbreviated index. The whole mail — header, the diffstat
  # (incl. the `Bin <old> -> <new> bytes` row), the text diff, the binary file's
  # full index and base85 body (the forward and reverse blocks), and the signature
  # — is byte-for-byte gitweb's output. This is the af6 deliverable, superseding the
  # earlier `--no-binary` divergence.
  Scenario: corpus binmix-commit binary patch matches gitweb byte for byte
    Given the parity corpus
    When I serve the patch of the corpus binmix commit
    Then the binary patch matches gitweb byte for byte
