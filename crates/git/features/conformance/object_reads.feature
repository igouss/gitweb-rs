Feature: Reading git objects through the gix adapter
  The Repository port is a frozen contract; this is the gix adapter honouring it
  over a real, deterministic fixture repository (built with gix, no git binary).
  Object ids are stable, so "points at <name>" compares against the very object
  the fixture wrote.

  The fixture is one small history: a root commit on a five-entry tree that
  exercises every git file mode, a child commit, and a merge of the two; HEAD on
  branch "main"; a lightweight tag and an annotated tag.

  Background:
    Given a populated repository

  # --- HEAD: present, and the unborn (zero-commit) edge ---

  Scenario: HEAD names the checked-out branch and its commit
    When I read HEAD
    Then the head ref is "refs/heads/main"
    And the head points at "c2"

  Scenario: an unborn repository has no HEAD
    Given an unborn repository
    When I read HEAD
    Then reading HEAD fails as not found

  # --- references(prefix): zero / one / many ---

  Scenario: listing one namespace returns its single head
    When I list references under "refs/heads/"
    Then 1 reference is listed
    And the reference "refs/heads/main" points at "c2"

  Scenario: listing tags returns both the lightweight and the annotated tag
    When I list references under "refs/tags/"
    Then 2 references are listed
    And the reference "refs/tags/lw" points at "c1"
    And the reference "refs/tags/v1" points at "tagobj"

  Scenario: listing an empty namespace returns nothing
    When I list references under "refs/remotes/"
    Then 0 references are listed

  # --- resolve(rev): ref, HEAD, and the missing edge ---

  Scenario Outline: a revision resolves to its commit
    When I resolve "<rev>"
    Then it resolves to "c2"

    Examples:
      | rev             |
      | main            |
      | HEAD            |
      | refs/heads/main |

  Scenario: an unknown revision does not resolve
    When I resolve "refs/heads/missing"
    Then resolving fails as not found

  # --- object_kind(oid): each kind, plus the missing edge ---

  Scenario Outline: the kind of each object is reported
    When I read the kind of "<object>"
    Then the kind is "<kind>"

    Examples:
      | object | kind   |
      | c1     | Commit |
      | t_full | Tree   |
      | hello  | Blob   |
      | tagobj | Tag    |

  Scenario: the kind of an absent object is not found
    When I read the kind of an absent object
    Then reading the kind fails as not found

  # --- find_commit(oid): fields, merge edge, wrong-kind edge ---

  Scenario: a root commit exposes its parsed fields
    When I read the commit "c1"
    Then the commit author is "Ada Lovelace"
    And the commit author email is "ada@example.com"
    And the commit timestamp is 1700000000
    And the commit title is "first commit"
    And the commit snapshots the tree "t_full"
    And the commit has 0 parents
    And the commit is not a merge

  Scenario: a merge commit records every parent
    When I read the commit "merge"
    Then the commit has 2 parents
    And the commit is a merge

  Scenario: reading a blob as a commit is invalid
    When I read the commit "hello"
    Then reading the commit fails as invalid

  # --- find_tree(oid): every mode, empty edge, wrong-kind edge ---

  Scenario: a tree exposes one entry per child, with its mode
    When I read the tree "t_full"
    Then the tree has 5 entries
    And the tree entry "file.txt" has long type "file"
    And the tree entry "file.txt" points at "hello"
    And the tree entry "run.sh" has long type "executable"
    And the tree entry "link" has long type "symlink"
    And the tree entry "sub" has long type "directory"
    And the tree entry "mod" has long type "submodule"

  Scenario: an empty tree has no entries
    When I read the tree "empty"
    Then the tree has 0 entries

  Scenario: reading a commit as a tree is invalid
    When I read the tree "c1"
    Then reading the tree fails as invalid

  # --- find_blob(oid): text, binary edge, wrong-kind edge ---

  Scenario: a text blob exposes its bytes and reads as text
    When I read the blob "hello"
    Then the blob is 6 bytes
    And the blob is text

  Scenario: a blob with NUL bytes reads as binary
    When I read the blob "bin"
    Then the blob is binary

  Scenario: reading a tree as a blob is invalid
    When I read the blob "t_full"
    Then reading the blob fails as invalid

  # --- find_tag(oid): annotated-tag fields, wrong-kind edge ---

  Scenario: an annotated tag exposes its target and tagger
    When I read the tag "tagobj"
    Then the tag name is "v1"
    And the tag points at "c2"
    And the tagged kind is "Commit"
    And the tag tagger is "Ada Lovelace"

  Scenario: reading a commit as a tag is invalid
    When I read the tag "c1"
    Then reading the tag fails as invalid

  # --- path_id(tree-ish, path): file, subtree, absent, peels a commit ---
  # gitweb's git_get_hash_by_path: the per-path history walk resolves a file's
  # type and its blob at each commit through this. c1 snapshots the full tree;
  # c2 snapshots the empty tree, so the same path is absent there.

  Scenario: a file path in a commit resolves to its blob
    When I read the path "file.txt" at "c1"
    Then the path resolves to "hello"

  Scenario: a directory path in a commit resolves to its subtree
    When I read the path "sub" at "c1"
    Then the path resolves to "empty"

  Scenario: a tree-ish that is a tree is peeled and looked up directly
    When I read the path "file.txt" at "t_full"
    Then the path resolves to "hello"

  Scenario: a path absent from the tree resolves to nothing
    When I read the path "file.txt" at "c2"
    Then the path resolves to nothing

  Scenario: a path that never existed resolves to nothing
    When I read the path "nope.txt" at "c1"
    Then the path resolves to nothing
