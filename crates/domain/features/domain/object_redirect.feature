Feature: Routing the object action to a typed view (git_object)
  gitweb's git_object is the generic object dispatcher: given a hash, or a base
  ref alone, or a base ref plus a file, it works out the referenced object's
  kind and 302-redirects to the view for that kind. Two pure rules drive it.

  The first is which lookup the request asks for, gitweb's if/elsif/else: a hash
  (the base ref standing in when no hash is given) means "look the id up"; a base
  ref plus a file means "look the path up under the base"; neither is the 400
  "Not enough information to find object". The base+file form strips trailing
  slashes off the file the way gitweb does (s,/+$,,).

  The second is the kind-to-action map: a commit object redirects to the commit
  view, a tree to the tree view, a blob to the blob view, a tag to the tag view.

  The repository lookups themselves are the use case's job; these rules only
  decide the shape of the request and the destination action.

  # --- which lookup: by hash ---

  Scenario: A hash means look the id up directly
    Given the object hash is "1c002dd4"
    When I classify the object request
    Then the lookup is by id "1c002dd4"

  Scenario: A base ref with no file falls back to looking the base up as an id
    Given the object base ref is "main"
    When I classify the object request
    Then the lookup is by id "main"

  Scenario: A hash outranks a base ref when both are present
    Given the object hash is "1c002dd4"
    And the object base ref is "main"
    When I classify the object request
    Then the lookup is by id "1c002dd4"

  # --- which lookup: by base and path ---

  Scenario: A base ref plus a file means look the path up under the base
    Given the object base ref is "main"
    And the object file name is "src/lib.rs"
    When I classify the object request
    Then the lookup is by base "main" and file "src/lib.rs"

  Scenario: A trailing slash on the file is stripped before the path lookup
    Given the object base ref is "main"
    And the object file name is "src/"
    When I classify the object request
    Then the lookup is by base "main" and file "src"

  # --- neither: not enough information (a 400) ---

  Scenario: Neither a hash nor a base ref is not enough information
    When I classify the object request
    Then classification fails as not enough information

  Scenario: A file with no base ref is not enough information
    Given the object file name is "src/lib.rs"
    When I classify the object request
    Then classification fails as not enough information

  # --- kind to action: every object kind has its view ---

  Scenario: A commit object redirects to the commit view
    Given the resolved object kind is "commit"
    When I map the object kind to its action
    Then the redirect action is "commit"

  Scenario: A tree object redirects to the tree view
    Given the resolved object kind is "tree"
    When I map the object kind to its action
    Then the redirect action is "tree"

  Scenario: A blob object redirects to the blob view
    Given the resolved object kind is "blob"
    When I map the object kind to its action
    Then the redirect action is "blob"

  Scenario: A tag object redirects to the tag view
    Given the resolved object kind is "tag"
    When I map the object kind to its action
    Then the redirect action is "tag"
