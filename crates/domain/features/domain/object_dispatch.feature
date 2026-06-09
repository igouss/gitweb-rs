Feature: The no-action default object lookup (gitweb's dispatch git_get_type)
  When a request names no action but carries a hash — or a base ref plus a file
  — gitweb's dispatch sub does NOT redirect. It resolves the referenced object's
  kind with git_get_type and serves the matching view inline, for the same
  request. Which object it asks git_get_type about is a pure decision, gitweb's
  if/elsif: a hash means "the object that hash names"; a base ref plus a file
  means "the object at that path under that base".

  This is gitweb's dispatch ladder, not git_object's: a hash is tested first and
  outranks a base ref, and — unlike git_object — a base ref ALONE names no object
  here (dispatch routes a bare project to the summary, never to object
  resolution), so it does not stand in as an id and no trailing slash is stripped.

  # --- a hash names the object directly ---

  Scenario: A hash looks the object up directly
    Given the object hash is "1c002dd4"
    When I classify the dispatch request
    Then the dispatch lookup is by id "1c002dd4"

  # --- a base ref plus a file names the object at that path ---

  Scenario: A base ref plus a file looks the path up under the base
    Given the object base ref is "main"
    And the object file name is "src/lib.rs"
    When I classify the dispatch request
    Then the dispatch lookup is by base "main" and file "src/lib.rs"

  # --- precedence: a hash outranks a base ref and a file ---

  Scenario: A hash outranks a base ref and a file
    Given the object hash is "1c002dd4"
    And the object base ref is "main"
    And the object file name is "src/lib.rs"
    When I classify the dispatch request
    Then the dispatch lookup is by id "1c002dd4"

  # --- unlike git_object, a base ref alone names no object ---

  Scenario: A base ref with no file names no object to resolve
    Given the object base ref is "main"
    When I classify the dispatch request
    Then the dispatch request names no object
