Feature: The commit use case
  gitweb's git_commit resolves a commit-ish (defaulting to HEAD), reads the
  commit, and assembles its authorship, message, parents, and the changed-files
  table: an ordinary diff against the one parent — or the empty tree for a root
  commit — and a combined diff for a merge. A revision that is not a commit is
  the 404 "Unknown commit object".

  Scenario: A single-parent commit exposes its metadata and ordinary changes
    Given a commit "c0ffee" with author "Ada Lovelace <ada@example.com> 1000 +0000"
    And the commit committer is "Linus Torvalds <linus@example.com> 2000 +0000"
    And the commit message is "Add the analytical engine"
    And the commit has parent "dec0de"
    And the commit changes "engine.rs" with status "M"
    And the commit changes "README" with status "A"
    When I assemble the commit view for "HEAD"
    Then the commit author name is "Ada Lovelace"
    And the commit author email is "ada@example.com"
    And the commit committer name is "Linus Torvalds"
    And the commit title is "Add the analytical engine"
    And the commit has 1 parent
    And the commit is not a merge
    And the changed files are ordinary
    And there are 2 changed files
    And ordinary change 1 is "engine.rs"
    And ordinary change 2 is "README"

  Scenario: A root commit has no parent and diffs against the empty tree
    Given a commit "1n1t" with author "Tester <t@example.com> 5 +0000"
    And the commit message is "Initial commit"
    And the commit changes "LICENSE" with status "A"
    When I assemble the commit view for "HEAD"
    Then the commit has 0 parents
    And the commit is not a merge
    And the changed files are ordinary
    And there are 1 changed files

  Scenario: A merge commit shows a combined diff and is flagged as a merge
    Given a commit "mer9e" with author "Tester <t@example.com> 9 +0000"
    And the commit message is "Merge branch topic"
    And the commit has parent "p1"
    And the commit has parent "p2"
    And the merge changes "conflict.txt"
    When I assemble the commit view for "HEAD"
    Then the commit has 2 parents
    And the commit is a merge
    And the changed files are combined
    And there are 1 changed files

  Scenario: An octopus merge shows every parent side
    Given a commit "octo" with author "Tester <t@example.com> 9 +0000"
    And the commit message is "Octopus merge"
    And the commit has parent "p1"
    And the commit has parent "p2"
    And the commit has parent "p3"
    And the merge changes "shared.txt"
    When I assemble the commit view for "HEAD"
    Then the commit has 3 parents
    And the commit is a merge
    And the changed files are combined
    And combined change 1 has 3 parent sides

  Scenario: A merge against an explicit non-first parent shows that parent's ordinary diff
    Given a commit "mer9e" with author "Tester <t@example.com> 9 +0000"
    And the commit message is "Merge branch topic"
    And the commit has parent "p1"
    And the commit has parent "p2"
    And the merge changes "conflict.txt"
    And the commit changes "vs-p2.rs" with status "M"
    When I assemble the commit view for "HEAD" against parent "p2"
    Then the commit has 2 parents
    And the commit is a merge
    And the changed files are ordinary
    And the ordinary base is "p2"
    And there are 1 changed files
    And ordinary change 1 is "vs-p2.rs"

  Scenario: A revision that is not a commit is unknown
    Given a commit "b10b" with author "Tester <t@example.com> 1 +0000"
    And the commit object kind is "blob"
    When I assemble the commit view for "HEAD"
    Then assembling the commit fails with "Unknown commit object"
