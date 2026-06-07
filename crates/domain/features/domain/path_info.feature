Feature: PATH_INFO decomposition
  gitweb lets a request be written as a path instead of query parameters:
  /project/action/ref:path or /project/oldref..newref. evaluate_path_info
  decomposes the part after the project into action, hash/hash_base,
  hash_parent/hash_parent_base, file_name/file_parent, and (for snapshots) a
  format stripped off the ref's extension.

  Resolving which prefix is the project walks the filesystem, so that is the
  web boundary's job; here we decompose only the remainder after the project,
  and only when the query string did not already pin an action.

  Scenario: An empty remainder decomposes to nothing
    Given the request path "/"
    When I decompose the request path
    Then the decomposed request is empty

  Scenario: A bare ref becomes a hash for type dispatch
    Given the request path "master"
    When I decompose the request path
    Then the decomposed request has no action
    And the decomposed hash is "master"

  Scenario: A non-action first segment is a ref, not an action
    Given the request path "feature/login"
    When I decompose the request path
    Then the decomposed request has no action
    And the decomposed hash is "feature/login"

  Scenario: A known first segment is peeled as the action
    Given the request path "commit/0123456789abcdef0123456789abcdef01234567"
    When I decompose the request path
    Then the decomposed action is "commit"
    And the decomposed hash is "0123456789abcdef0123456789abcdef01234567"

  Scenario: A lone action segment carries nothing else
    Given the request path "summary"
    When I decompose the request path
    Then the decomposed action is "summary"
    And the decomposed request has no hash

  Scenario: A tree action takes the ref as its base, not its hash
    Given the request path "tree/master"
    When I decompose the request path
    Then the decomposed action is "tree"
    And the decomposed hash base is "master"
    And the decomposed request has no hash

  Scenario: A ref and file path default to viewing the raw blob
    Given the request path "master:src/main.rs"
    When I decompose the request path
    Then the decomposed action is "blob_plain"
    And the decomposed hash base is "master"
    And the decomposed file name is "src/main.rs"

  Scenario: A ref and a trailing-slash path default to a tree
    Given the request path "master:src/"
    When I decompose the request path
    Then the decomposed action is "tree"
    And the decomposed hash base is "master"
    And the decomposed file name is "src"

  Scenario: An explicit action is not overridden by the path defaults
    Given the request path "blob/master:src/main.rs"
    When I decompose the request path
    Then the decomposed action is "blob"
    And the decomposed hash base is "master"
    And the decomposed file name is "src/main.rs"

  Scenario: A two-dot range defaults to a shortlog of new since old
    Given the request path "v1.0..v2.0"
    When I decompose the request path
    Then the decomposed action is "shortlog"
    And the decomposed hash is "v2.0"
    And the decomposed hash parent is "v1.0"

  Scenario: A two-dot range with file paths defaults to a blob diff
    Given the request path "v1:old.txt..v2:new.txt"
    When I decompose the request path
    Then the decomposed action is "blobdiff_plain"
    And the decomposed hash base is "v2"
    And the decomposed file name is "new.txt"
    And the decomposed hash parent base is "v1"
    And the decomposed file parent is "old.txt"

  Scenario: A snapshot ref carries its format in the extension
    Given the request path "snapshot/v1.0.tar.gz"
    When I decompose the request path
    Then the decomposed action is "snapshot"
    And the decomposed snapshot format is "tgz"
    And the decomposed hash is "v1.0"

  Scenario: The short snapshot extension is recognised too
    Given the request path "snapshot/v1.0.tgz"
    When I decompose the request path
    Then the decomposed action is "snapshot"
    And the decomposed snapshot format is "tgz"
    And the decomposed hash is "v1.0"

  Scenario: A zip snapshot is recognised by its extension
    Given the request path "snapshot/v1.0.zip"
    When I decompose the request path
    Then the decomposed action is "snapshot"
    And the decomposed snapshot format is "zip"
    And the decomposed hash is "v1.0"
