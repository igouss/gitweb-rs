Feature: Deterministic git fixtures
  Conformance specs need real git repositories whose object ids are stable
  across runs and across machines, so that golden differential conformance
  never flickers. The fixture builder writes objects and refs with gix — no git
  binary — and pins every author, committer, and tagger identity and timestamp.

  Every object id below is a git SHA-1. The empty blob and the empty tree match
  git's own well-known canonical ids, which proves the hashing is genuinely
  git's; every other id here is therefore a real git id, frozen as a golden so a
  regression in the builder (a wrong mode byte, an unsorted tree, an unpinned
  date) changes the hash and trips the spec.

  Background:
    Given a fresh fixture repository
    And the fixture identity "Ada Lovelace <ada@example.com>" at epoch 1700000000
    And a blob containing "hello\n" stored as "hello"
    And an empty tree stored as "empty"

  # --- blobs: zero bytes, hashed bytes, byte-exact round trip ---

  Scenario: An empty blob has git's canonical empty-blob id
    When I write a blob with no bytes
    Then the new object id is "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"

  Scenario: A blob's id is git's hash of its bytes
    When I write a blob containing "hello\n"
    Then the new object id is "ce013625030ba8dba906f756967f9e9ca394464a"

  Scenario: A blob round-trips its exact bytes, including non-UTF-8
    When I write a blob of the bytes "00 ff 80 0a"
    Then reading that blob back yields the bytes "00 ff 80 0a"

  # --- trees: zero / one / many entries, every special mode, canonical order ---

  Scenario: An empty tree has git's canonical empty-tree id
    When I write a tree with no entries
    Then the new object id is "4b825dc642cb6eb9a060e54bf8d69288fbee4904"

  Scenario: A one-entry tree hashes to its git id
    When I write a tree with the file "hello.txt" pointing at "hello"
    Then the new object id is "aaa96ced2d9a1c8e72c56b253a0e2fe78393feb7"

  Scenario: A tree carries every special git file mode
    Given a placeholder object id "1111111111111111111111111111111111111111" stored as "submodule"
    When I write a tree with entries:
      | name      | mode       | target    |
      | plain.txt | file       | hello     |
      | exec.sh   | executable | hello     |
      | link      | symlink    | hello     |
      | sub       | directory  | empty     |
      | mod       | gitlink    | submodule |
    Then the new object id is "a653ab46a2e785dbaf53e299ae618ac73cd9f266"

  Scenario: Tree entries are written in git's canonical order regardless of input order
    Given a placeholder object id "1111111111111111111111111111111111111111" stored as "submodule"
    When I write a tree with entries:
      | name      | mode       | target    |
      | mod       | gitlink    | submodule |
      | sub       | directory  | empty     |
      | link      | symlink    | hello     |
      | exec.sh   | executable | hello     |
      | plain.txt | file       | hello     |
    Then the new object id is "a653ab46a2e785dbaf53e299ae618ac73cd9f266"

  # --- commits: root, merge; pinned identity makes the id stable ---

  Scenario: A root commit with pinned identity has a stable id
    Given a tree with the file "hello.txt" pointing at "hello" stored as "one-file"
    When I write a commit with tree "one-file" and message "root commit\n"
    Then the new object id is "b4cef8dd6f3bc03b4d810dd6526be39948a1b86b"

  Scenario: A merge commit records all of its parents
    Given a tree with the file "hello.txt" pointing at "hello" stored as "one-file"
    And a commit "root" with tree "one-file" and message "root commit\n"
    And a commit "second" with tree "empty" and message "second\n"
    When I write a commit with tree "one-file", parents "root" and "second", and message "merge\n"
    Then the new object id is "ccf7aa881b83641f69b0251da0a3be7c8d470a50"

  # --- refs: unborn HEAD, branch, lightweight tag, annotated tag ---

  Scenario: A fresh repository's HEAD is unborn
    When I set HEAD to branch "main"
    Then "HEAD" does not resolve

  Scenario: HEAD resolves once its branch is created
    Given a commit "root" with tree "empty" and message "root commit\n"
    And HEAD is set to branch "main"
    When I create branch "main" at "root"
    Then "HEAD" resolves to "root"

  Scenario: A branch points straight at its commit
    Given a commit "root" with tree "empty" and message "root commit\n"
    When I create branch "topic" at "root"
    Then "refs/heads/topic" resolves to "root"

  Scenario: A lightweight tag points straight at its target
    Given a commit "root" with tree "empty" and message "root commit\n"
    When I create lightweight tag "v0" at "root"
    Then "refs/tags/v0" resolves to "root"

  Scenario: An annotated tag wraps its target in a tag object
    Given a tree with the file "hello.txt" pointing at "hello" stored as "one-file"
    And a commit "root" with tree "one-file" and message "root commit\n"
    When I create annotated tag "v1.0" on "root" with message "release\n"
    Then the new object id is "eee0f51ece1ca2b07b909466949eddc91d3f076e"
    And "refs/tags/v1.0" resolves to the new object id
