Feature: commitdiff_plain golden conformance
  gitweb's commitdiff_plain is format-stable — git am parses it — so its bytes
  must match the original. The corpus root commit exercises the format's whole
  surface in one capture: the mailbox header with the request's own X-Git-Url,
  the comment and `---` separator, the bare commit-id line git's diff-tree
  prints for the --root form, then a creation patch per blob — abbreviated index
  lines, the binary-file notice, the non-UTF-8 line re-encoded the way gitweb's
  :utf8 output layer does, and the missing-trailing-newline marker.

  The reference is captured by ../../scripts/capture-goldens.sh over the same
  gix-built corpus, so the hash in the body and filename is reproducible; the
  by-hash Expires/Date cache headers are volatile and are not compared.

  Scenario: the corpus root commitdiff_plain matches gitweb byte for byte
    Given the parity corpus
    When I serve the commitdiff_plain of the corpus root commit
    Then the commitdiff_plain body matches gitweb's reference output
    And the commitdiff_plain content type matches gitweb's
    And the commitdiff_plain content disposition matches gitweb's

  # The `anchored` branch's middle commit is named only as an ancestor of the
  # annotated v2.0 tag, so gitweb stamps `X-Git-Tag: v2.0~1` — the byte-exact
  # proof of name-rev ancestor-distance naming (the `^0` dereference marker
  # stripped before the `~N` suffix), the case the old subset omitted entirely.
  Scenario: an ancestor-named commit's commitdiff_plain matches gitweb byte for byte
    Given the parity corpus
    When I serve the commitdiff_plain of the anchored ancestor commit
    Then the commitdiff_plain body matches gitweb's reference output
    And the commitdiff_plain content type matches gitweb's
    And the commitdiff_plain content disposition matches gitweb's
