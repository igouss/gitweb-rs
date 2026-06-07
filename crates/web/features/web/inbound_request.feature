Feature: Resolving a raw inbound request into a validated Request
  The web boundary turns a raw HTTP request — a query string plus a PATH_INFO —
  into the domain's validated Request, exactly as gitweb's evaluate_query_params
  + evaluate_path_info + evaluate_and_validate_params do. The pure rules
  (Request::from_query, PathInfo::parse) are frozen domain contracts; this is the
  impure glue over them: resolving the project prefix against a real project root
  (gitweb's check_head_link), decomposing the remainder, and merging with the
  query under gitweb's precedence (the query wins).

  The project root is laid out on disk with the gix fixture builder; the gix
  ProjectStore adapter is wired in as the composition root would, so the prefix
  walk hits real repositories.

  # --- project-prefix resolution: zero / one / many components ---

  Scenario: a path with no leading project resolves no project
    Given an empty project root
    When I resolve the path "/" with no query
    Then no project is resolved

  Scenario: a single-component project prefix is resolved
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/summary" with no query
    Then the resolved project is "solo.git"

  Scenario: a nested, slash-containing project prefix is resolved
    Given a project root containing repository "group/nested.git"
    When I resolve the path "/group/nested.git/heads" with no query
    Then the resolved project is "group/nested.git"
    And the resolved action is "heads"

  Scenario: a prefix that names no repository resolves no project
    Given a project root containing repository "real.git"
    When I resolve the path "/ghost.git/commit/main" with no query
    Then no project is resolved
    And no action is resolved

  # --- the path supplies the action when the query does not ---

  Scenario: the action is peeled from the path
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/log" with no query
    Then the resolved project is "solo.git"
    And the resolved action is "log"

  Scenario: a tree action takes its ref as the base, not the hash
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/tree/main" with no query
    Then the resolved action is "tree"
    And the resolved hash base is "main"
    And no hash is resolved

  Scenario: a bare ref-and-file path defaults to the raw blob
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/main:file.txt" with no query
    Then the resolved action is "blob_plain"
    And the resolved hash base is "main"
    And the resolved file is "file.txt"

  # --- the query pins the action: PATH_INFO is then ignored past the project ---

  Scenario: a query action suppresses the path action and path fields
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/tree/main" with the query "a=summary"
    Then the resolved project is "solo.git"
    And the resolved action is "summary"
    And no hash base is resolved

  # --- a query project suppresses PATH_INFO entirely ---

  Scenario: a query project is used and the path is ignored
    Given a project root containing repository "solo.git"
    When I resolve the path "/other.git/log" with the query "p=solo.git"
    Then the resolved project is "solo.git"
    And no action is resolved

  # --- query + PATH_INFO precedence: the query field wins (gitweb's ||=) ---

  Scenario: a query hash overrides the hash decomposed from the path
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/commit/main" with the query "h=feature"
    Then the resolved action is "commit"
    And the resolved hash is "feature"

  # --- view-preference params come only from the query ---

  Scenario: the diff-style, content-tag and javascript params are carried through
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/commitdiff/main" with the query "ds=sidebyside&by_tag=rust&js=1"
    Then the diff style is "sidebyside"
    And the content tag is "rust"
    And the javascript marker is "1"

  # --- an unsafe or invalid field is rejected with the domain rule's status ---

  Scenario: an invalid ref in the path is rejected as the domain rule would
    Given a project root containing repository "solo.git"
    When I resolve the path "/solo.git/commit/feature^" with no query
    Then resolution fails as invalid

  Scenario: an unsafe project in the query is rejected as not found
    Given a project root containing repository "solo.git"
    When I resolve the query "p=../etc"
    Then resolution fails as not found

  Scenario: a too-short search term is rejected as forbidden
    Given a project root containing repository "solo.git"
    When I resolve the query "p=solo.git&a=search&s=x"
    Then resolution fails as forbidden
