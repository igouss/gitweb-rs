Feature: Viewing a single annotated tag (tag action)
  The tag use case (gitweb's git_tag / parse_tag) resolves the requested hash,
  insists it names a tag OBJECT, and assembles that tag for display: the tag's
  own name, the object it points at and that object's kind (so the view can link
  the object to the right action), the tagger identity and the absolute date it
  was made when the tag carries one, and the message body. A hash that resolves
  to anything other than a tag object — a commit, or a lightweight tag's object —
  is gitweb's "Unknown tag object" (404); a hash that resolves to nothing is
  simply not found.

  # --- the happy path: an annotated tag of a commit ---

  Scenario: an annotated tag of a commit is shown with its object and tagger
    Given an annotated tag "v1.0" of a commit tagged at 999400 with subject "Release 1.0"
    When I show the tag "v1.0"
    Then the tag view name is "v1.0"
    And the tag view points at a "commit"
    And the tag view has a tagger
    And the tag view tagger shows the date "Mon, 12 Jan 1970 13:36:40 +0000"
    And the tag view message is "Release 1.0"

  # --- the tagged object's kind decides the object link (commit / blob / tree) ---

  Scenario: an annotated tag of a blob points at a blob
    Given an annotated tag "blobtag" of a blob tagged at 999400 with subject "a blob"
    When I show the tag "blobtag"
    Then the tag view points at a "blob"

  Scenario: an annotated tag of a tree points at a tree
    Given an annotated tag "treetag" of a tree tagged at 999400 with subject "a tree"
    When I show the tag "treetag"
    Then the tag view points at a "tree"

  # --- tagger present vs absent ---

  Scenario: an annotated tag with no tagger omits the authorship
    Given an annotated tag "anon" of a commit with no tagger
    When I show the tag "anon"
    Then the tag view has no tagger

  # --- the message body ---

  Scenario: a multi-line message is preserved
    Given an annotated tag "notes" of a commit with a two-line message
    When I show the tag "notes"
    Then the tag view message has 2 lines
    And the tag view message line 1 is "First line"
    And the tag view message line 2 is "Second line"

  # --- not a tag object: the "Unknown tag object" 404 ---

  Scenario: a lightweight tag's object is not a tag object
    Given a lightweight tag "rc1" on a commit at 999400
    When I show the tag "rc1"
    Then showing the tag fails with "Unknown tag object"

  Scenario: a plain commit is not a tag object
    Given the repository has branch "main" committed at 999400
    When I show the tag "main"
    Then showing the tag fails with "Unknown tag object"

  # --- the object simply does not exist ---

  Scenario: an unknown hash is not found
    When I show the tag "ghost"
    Then showing the tag fails as not found
