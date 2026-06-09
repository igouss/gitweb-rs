Feature: Assembling the OPML project outline
  The opml use case (gitweb's git_opml) lists every project under the root that
  has a resolvable HEAD — gitweb skips a project whose git_get_head_hash is
  undefined. It dies "404 No projects found" when the root is empty, before the
  HEAD filter; a non-empty root whose projects all lack a HEAD is not an error,
  it just yields an empty outline. It preserves discovery order; the link
  building and the title are the boundary's job.

  Scenario: an empty project root has no outline
    When I assemble the opml
    Then assembling the opml fails as not found

  Scenario: a project with a HEAD is included
    Given the opml store has project "live.git" with a head
    When I assemble the opml
    Then the opml projects are "live.git"

  Scenario: a project without a HEAD is skipped
    Given the opml store has project "live.git" with a head
    And the opml store has project "void.git" without a head
    When I assemble the opml
    Then the opml projects are "live.git"

  Scenario: the outline lists every head-bearing project in discovery order
    Given the opml store has project "zlib.git" with a head
    And the opml store has project "make.git" with a head
    And the opml store has project "acl.git" with a head
    When I assemble the opml
    Then the opml projects are "zlib.git, make.git, acl.git"

  Scenario: a non-empty root of only headless projects yields an empty outline
    Given the opml store has project "void.git" without a head
    When I assemble the opml
    Then the opml has no projects

  # --- the project filter (pf subdirectory scoping) ---

  Scenario: a filter scopes the outline to its subdirectory
    Given the opml store has project "group/live.git" with a head
    And the opml store has project "other/live.git" with a head
    When I assemble the opml filtered by "group"
    Then the opml projects are "group/live.git"

  Scenario: a filter matching no project is still the not-found error
    Given the opml store has project "group/live.git" with a head
    When I assemble the opml filtered by "elsewhere"
    Then assembling the opml fails as not found
