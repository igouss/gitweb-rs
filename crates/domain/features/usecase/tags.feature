Feature: Listing a project's tags (tags action)
  The tags use case (gitweb's git_tags / git_get_tags_list) lists the refs under
  refs/tags/ and classifies each. An ANNOTATED tag (its ref points at a tag
  object) is peeled to the object it tags — that object's id is the row's refid
  and its kind is the row's reftype — carries the tag message's subject, and ages
  by its tagger time. A LIGHTWEIGHT tag points straight at its object: its reftype
  is that object's kind, it has no subject, and a lightweight tag of a commit ages
  by the commit's committer time. Ordering matches gitweb's --sort=-creatordate:
  newest first. The age has THREE states: a tag or commit ref with a real time
  shows its age; a tag or commit ref with a zero time shows "unknown"; a
  lightweight tag of a blob or tree — kinds that carry no creator date — has no
  age at all. A repository with no tags lists nothing without failing.

  Background:
    Given the current time is 1000000

  # --- discovery (zero / one / many) ---

  Scenario: a repository with no tags lists no tags
    When I assemble the tags
    Then no tags are listed

  Scenario: a single tag is listed
    Given an annotated tag "v1.0" of a commit tagged at 900000 with subject "Release 1.0"
    When I assemble the tags
    Then the listed tags are "v1.0"

  Scenario: tags are ordered by creation date, newest first
    Given an annotated tag "v1.0" of a commit tagged at 100 with subject "one"
    And an annotated tag "v3.0" of a commit tagged at 300 with subject "three"
    And an annotated tag "v2.0" of a commit tagged at 200 with subject "two"
    When I assemble the tags
    Then the listed tags are "v3.0, v2.0, v1.0"

  # --- annotated vs lightweight classification ---

  Scenario: an annotated tag is peeled and carries its subject and commit reftype
    Given an annotated tag "v1.0" of a commit tagged at 900000 with subject "Release 1.0"
    When I assemble the tags
    Then the tag "v1.0" is annotated
    And the tag "v1.0" has subject "Release 1.0"
    And the tag "v1.0" has reftype "commit"

  Scenario: a lightweight tag has no subject and a commit reftype
    Given a lightweight tag "rc1" on a commit at 900000
    When I assemble the tags
    Then the tag "rc1" is not annotated
    And the tag "rc1" has no subject
    And the tag "rc1" has reftype "commit"

  # --- reftype-specific classification (commit / blob / tree) ---

  Scenario: an annotated tag of a blob has a blob reftype
    Given an annotated tag "blobtag" of a blob tagged at 900000 with subject "a blob"
    When I assemble the tags
    Then the tag "blobtag" is annotated
    And the tag "blobtag" has reftype "blob"

  Scenario: an annotated tag of a tree has a tree reftype
    Given an annotated tag "treetag" of a tree tagged at 900000 with subject "a tree"
    When I assemble the tags
    Then the tag "treetag" has reftype "tree"

  Scenario: a lightweight tag of a blob has a blob reftype and no subject
    Given a lightweight tag "rawblob" on a blob
    When I assemble the tags
    Then the tag "rawblob" is not annotated
    And the tag "rawblob" has reftype "blob"
    And the tag "rawblob" has no subject

  # --- age, relative to the request time ---

  Scenario: a lightweight tag's age is measured from the current time
    Given a lightweight tag "rc1" on a commit at 999400
    When I assemble the tags
    Then the tag "rc1" shows the age "10 min ago"

  Scenario: an annotated tag ages by its tagger time
    Given an annotated tag "v1.0" of a commit tagged at 999400 with subject "Release 1.0"
    When I assemble the tags
    Then the tag "v1.0" shows the age "10 min ago"

  Scenario: a commit tag with a zero creation time has an unknown age
    Given a lightweight tag "rc1" on a commit at 0
    When I assemble the tags
    Then the tag "rc1" has an unknown age

  Scenario: a lightweight tag of a blob has no age cell at all
    Given a lightweight tag "rawblob" on a blob
    When I assemble the tags
    Then the tag "rawblob" has no age cell

  Scenario: a lightweight tag of a tree has no age cell at all
    Given a lightweight tag "snap" on a tree
    When I assemble the tags
    Then the tag "snap" has no age cell
