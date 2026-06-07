Feature: Request actions
  Every gitweb request names an action — the key into gitweb's %actions
  dispatch table that decides which view is served. A wire name maps to an
  action and renders back unchanged; anything that is not a known action maps
  to nothing (gitweb's is_valid_action, which dies 400 on an unknown action).

  Three actions are served without a project (gitweb's "those below don't need
  $project": project_list, project_index, opml); every other action needs one.

  Scenario Outline: Wire names map to an action and round-trip
    Given a request action "<name>"
    When I read the action
    Then the action's wire name is "<name>"

    Examples:
      | name              |
      | blame             |
      | blame_incremental |
      | blame_data        |
      | blobdiff          |
      | blobdiff_plain    |
      | blob              |
      | blob_plain        |
      | commitdiff        |
      | commitdiff_plain  |
      | commit            |
      | forks             |
      | heads             |
      | history           |
      | log               |
      | patch             |
      | patches           |
      | remotes           |
      | rss               |
      | atom              |
      | search            |
      | search_help       |
      | shortlog          |
      | summary           |
      | tag               |
      | tags              |
      | tree              |
      | snapshot          |
      | object            |
      | opml              |
      | project_list      |
      | project_index     |

  Scenario: An unknown wire name is not an action
    Given a request action "wibble"
    When I read the action
    Then there is no action

  Scenario: An empty wire name is not an action
    Given a request action ""
    When I read the action
    Then there is no action

  Scenario Outline: Some actions are served without a project
    Given a request action "<name>"
    When I read the action
    Then needing a project reads "<needs>"

    Examples:
      | name          | needs |
      | summary       | yes   |
      | tree          | yes   |
      | snapshot      | yes   |
      | opml          | no    |
      | project_list  | no    |
      | project_index | no    |
