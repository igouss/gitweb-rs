Feature: Assembling the machine-readable project index
  The project_index use case (gitweb's git_project_index) lists every project
  under the root as a "path owner" pair — the data behind the text/plain
  "index.aux" — and dies "404 No projects found" when the root is empty. It does
  not sort: it preserves discovery order, and the render layer does the CGI
  quoting. The owner is resolved through the same precedence as the listing
  (file, then config, then the filesystem fallback), and is empty when none can
  be found.

  Scenario: an empty project root has no index
    When I assemble the project index
    Then assembling the index fails as not found

  Scenario: a single project is indexed with its path and owner
    Given the store has project "only.git" described as "x" owned by "Ada Lovelace"
    When I assemble the project index
    Then the index row for "only.git" has owner "Ada Lovelace"

  Scenario: the index lists every project in discovery order
    Given the store has project "zlib.git"
    And the store has project "make.git"
    And the store has project "acl.git"
    When I assemble the project index
    Then the indexed projects are "zlib.git, make.git, acl.git"

  Scenario: a project with no resolvable owner is indexed with an empty owner
    Given the store has project "void.git"
    When I assemble the project index
    Then the index row for "void.git" has owner ""

  # --- the project filter (pf subdirectory scoping) ---

  Scenario: a filter scopes the index to its subdirectory
    Given the store has project "group/alpha.git"
    And the store has project "other/beta.git"
    When I assemble the project index filtered by "group"
    Then the indexed projects are "group/alpha.git"

  Scenario: a filter matching no project is still the not-found error
    Given the store has project "group/alpha.git"
    When I assemble the project index filtered by "elsewhere"
    Then assembling the index fails as not found
