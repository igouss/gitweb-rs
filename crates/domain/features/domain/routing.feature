Feature: Dispatch routing
  Once a request is validated, gitweb's dispatch sub decides which %actions
  handler runs. It first defaults the action when none was named, then refuses
  any action that needs a project without one (gitweb's "Project needed", a
  400). The defaulting is gitweb's exact ladder: a bare hash — or a base ref
  plus a file — means "resolve the object's kind and show its view"; a project
  alone means summary; nothing at all means the project list.

  Resolving an object's kind needs the repository, so this pure rule only flags
  that case; the object-resolution use case carries it out.

  # --- an explicit action passes straight through ---

  Scenario: An explicit action dispatches to that action
    Given a request parameter "a" is "log"
    And a request parameter "p" is "proj.git"
    When I route the request
    Then the dispatched action is "log"

  # --- defaulting: project-list vs summary (zero / one project) ---

  Scenario: A bare request defaults to the project list
    Given a bare request
    When I route the request
    Then the dispatched action is "project_list"

  Scenario: A request with only a project defaults to the summary
    Given a request parameter "p" is "proj.git"
    When I route the request
    Then the dispatched action is "summary"

  # --- defaulting: a bare hash (or base ref + file) needs the object's kind ---

  Scenario: A request with only a hash must resolve the object kind
    Given a request parameter "p" is "proj.git"
    And a request parameter "h" is "deadbeef"
    When I route the request
    Then the object kind must be resolved

  Scenario: A request with only a base ref and a file must resolve the object kind
    Given a request parameter "p" is "proj.git"
    And a request parameter "hb" is "main"
    And a request parameter "f" is "src/lib.rs"
    When I route the request
    Then the object kind must be resolved

  # --- the project gate (gitweb's "Project needed") ---

  Scenario: An action that needs a project is rejected without one
    Given a request parameter "a" is "log"
    When I route the request
    Then routing fails as project needed

  Scenario: A no-project action routes without a project
    Given a request parameter "a" is "project_index"
    When I route the request
    Then the dispatched action is "project_index"
