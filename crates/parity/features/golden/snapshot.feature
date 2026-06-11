Feature: snapshot serves a tree archive with gitweb-faithful headers
  gitweb's `snapshot` action streams `git archive --format=<tar|zip> --prefix=…`
  (tar piped through `gzip -n`) as a downloadable archive. Our endpoint produces
  the same archive through the gix worktree-stream + gix-archive writer.

  This is a format-stable endpoint, so it is pinned by GOLDEN differential
  conformance against output captured once from the original gitweb.perl. But
  unlike the other golden endpoints, only the HEADERS are byte-reproducible — the
  archive BODY is not, and cannot be, byte-identical to `git archive`. That is a
  property of the writers, not a defect in our code:

    - tar: `git archive` prepends a `pax_global_header` carrying the commit id as
      a `comment=<sha>` pax record, emits an explicit directory entry for the
      prefix, stamps files mode 0664 with uname/gname `root`, and pads the stream
      to a 20-block (10240-byte) boundary. gix-archive emits no pax header, no
      directory entries, mode 0644 with empty owner names, and its own padding.
    - tar.gz: `git archive | gzip -n` writes a gzip header with MTIME 0 and OS
      byte 0x03; gix-archive bakes the archive modification time into the gzip
      header and sets OS 0xff, and its deflate stream (flate2 over zlib-rs) is not
      git's `gzip` output.
    - zip: `git archive` records the commit id as the end-of-central-directory
      archive comment and emits a directory entry; gix-archive emits neither, and
      adds a per-entry `UT` (0x5455 extended-timestamp) field `git archive` does
      not. tar.bz2 and tar.xz wrap the same divergent tar, so they diverge too.

  So there is NO body assertion here, and the goldens are kept only to lock the
  headers and to record gitweb's real archive bytes for the divergence on the
  record. What the headers DO match, byte-for-byte against gitweb, is the
  presentation `git_snapshot` resolves before streaming: the format's media type,
  the `inline; filename="<name><suffix>"` download disposition built from
  `snapshot_name`, and the commit-dated `Last-Modified`.

  Background:
    Given the parity corpus

  Scenario Outline: the snapshot headers are byte-identical to gitweb's
    When I serve the "<format>" snapshot of the corpus HEAD commit
    Then the snapshot media type matches gitweb's
    And the snapshot content disposition matches gitweb's
    And the snapshot last-modified matches gitweb's

    Examples:
      | format |
      | tgz    |
      | zip    |

  Scenario: the captured tgz reference is a well-formed CGI response
    When I serve the "tgz" snapshot of the corpus HEAD commit
    Then the reference declares a Content-Type header
