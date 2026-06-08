Feature: blobdiff_plain golden conformance
  gitweb's blobdiff_plain is format-stable — it is the bare `git diff-tree -r -p`
  of one path (abbreviated index, no mailbox header), prefixed by an X-Git-Url
  self-link line — so its bytes must match the original. The corpus `diffs`
  branch diffs its two commits across the single-file surface this endpoint must
  reproduce: a text modification (a hunk and the abbreviated index line), a pure
  file-mode change (old/new mode, no index, no hunk), a binary modification (the
  `Binary files … differ` notice — bare diff-tree -p emits no base85 patch), and
  an exact rename (`similarity index 100%`, the `a/old b/new` header, no hunk).

  The reference is captured by ../../scripts/capture-goldens.sh over the same
  gix-built corpus, so the commit ids in the X-Git-Url self-link are
  reproducible; the by-hash Expires/Date cache headers are volatile and are not
  compared.

  Scenario Outline: a single-file diff matches gitweb byte for byte
    Given the parity corpus
    When I serve the "<surface>" blobdiff_plain of the corpus diffs branch
    Then the blobdiff_plain body matches gitweb's reference output
    And the blobdiff_plain content type matches gitweb's
    And the blobdiff_plain content disposition matches gitweb's

    Examples:
      | surface |
      | text    |
      | mode    |
      | binary  |
      | rename  |
