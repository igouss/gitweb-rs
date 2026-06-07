Feature: Snapshot archive bytes through the gix adapter
  gitweb's `git_snapshot` streams `git archive --format=<tar|zip> --prefix=…
  <hash>`, optionally piped through gzip/bzip2/xz, to serve a downloadable
  snapshot of a tree. gix has no `git archive` subcommand, so the adapter turns
  the tree into a worktree stream and writes it through gix-archive (tar, tar.gz,
  zip) — compressing the raw tar with bzip2 / xz itself for the two formats
  gix-archive does not produce natively. This is the gix adapter honouring the
  Repository port's `archive` operation over deterministic gix-built fixtures.

  Like `git archive`, the snapshot carries only the tree's file content: regular
  files keep their bytes and a 0644 mode, executables a 0755 mode, symlinks are
  stored as symlinks to their target, directories appear only as the prefix of
  the files inside them, and submodules (gitlinks) are omitted — their objects
  are not present to recurse into.

  The download filename and `--prefix` are gitweb presentation concerns resolved
  at the snapshot endpoint, not in this adapter; the bytes here therefore carry
  no path prefix. Byte-exact equality with real `git archive` output — including
  its depth-first entry order — is a separate golden-conformance concern for that
  endpoint; gix streams entries breadth-first (a tree's own files before its
  subtrees'), which differs but is just as deterministic. What this slice pins is
  that the bytes are well-formed in every format, faithfully represent every
  entry kind, and are reproducible: the same tree always yields the same bytes.

  The "tree of mixed entries" fixture is one commit whose tree holds:

    greeting.txt        text       "hello\n"
    link                symlink     target "greeting.txt"
    nested/inner.txt    text        "deep\n"  (inside a subtree)
    run.sh              executable  "#!/bin/sh\necho hi\n"
    submod              gitlink     a submodule commit pointer

  so the archive has every entry kind to include, recurse into, or skip. Archived
  in gix's breadth-first order, it holds four files — greeting.txt, link, run.sh,
  nested/inner.txt — with submod skipped.

  # --- a well-formed archive in every enabled format ---

  Scenario Outline: a tree archives to a well-formed <format>
    Given a tree of mixed entries
    When I archive the tree as <format>
    Then the archive is a well-formed <format>

    Examples:
      | format |
      | tgz    |
      | tbz2   |
      | txz    |
      | zip    |

  # --- reproducible bytes: the core parity edge ---

  Scenario Outline: the same tree yields identical <format> bytes every time
    Given a tree of mixed entries
    When I archive the tree as <format> twice
    Then the two archives are byte-identical

    Examples:
      | format |
      | tgz    |
      | tbz2   |
      | txz    |
      | zip    |

  # --- entry representation, read back from the canonical tar (via tgz) ---

  Scenario: the archive carries the tree's files, subtree recursed, submodule omitted
    Given a tree of mixed entries
    When I archive the tree as tgz
    Then the archive holds 4 files
    And the archived files are "greeting.txt, link, run.sh, nested/inner.txt"

  Scenario: a regular file keeps its bytes and a 0644 mode
    Given a tree of mixed entries
    When I archive the tree as tgz
    Then archived file "greeting.txt" is a regular file of mode 644 holding "hello\n"

  Scenario: an executable keeps its 0755 mode
    Given a tree of mixed entries
    When I archive the tree as tgz
    Then archived file "run.sh" is a regular file of mode 755

  Scenario: a symlink is stored as a symlink to its target
    Given a tree of mixed entries
    When I archive the tree as tgz
    Then archived file "link" is a symlink to "greeting.txt"

  Scenario: a submodule (gitlink) is omitted from the archive
    Given a tree of mixed entries
    When I archive the tree as tgz
    Then the archive has no entry named "submod"

  # --- the bzip2 and xz containers wrap the very same tar ---

  Scenario: the bzip2 archive decompresses to the same tar as gzip
    Given a tree of mixed entries
    When I archive the tree as tgz and as tbz2
    Then both decompress to identical tar bytes

  Scenario: the xz archive decompresses to the same tar as gzip
    Given a tree of mixed entries
    When I archive the tree as tgz and as txz
    Then both decompress to identical tar bytes

  # --- the empty-tree edge ---

  Scenario: an empty tree archives to a well-formed, file-less tgz
    Given an empty tree
    When I archive the tree as tgz
    Then the archive is a well-formed tgz
    And the archive holds 0 files

  Scenario: an empty tree archives to a well-formed zip
    Given an empty tree
    When I archive the tree as zip
    Then the archive is a well-formed zip

  # --- a non-tree object is rejected, as gitweb rejects a non-tree-ish ---

  Scenario: a blob id is not a tree and cannot be archived
    Given a tree of mixed entries
    When I archive the blob as tgz
    Then archiving fails as invalid
