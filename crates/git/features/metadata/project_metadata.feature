Feature: Per-project metadata through the gix ProjectStore adapter
  gitweb shows each repository's description, owner, category and clone URLs,
  reading each either from a file in the repository (`description`, `category`,
  `cloneurl`) or, when that file is absent, from a `gitweb.*` git config value
  (`git_get_file_or_project_config`). This is the gix adapter resolving that
  metadata over real repositories laid out on disk.

  The file always wins over config; a value present in neither is simply absent.

  # --- description: file, config, precedence, absent ---

  Scenario: a description is read from the description file
    Given a project root containing repository "solo.git"
    And "solo.git" has the description file "A neat little project"
    When I read the metadata of "solo.git"
    Then the description is "A neat little project"

  Scenario: a description falls back to gitweb config when there is no file
    Given a project root containing repository "solo.git"
    And "solo.git" has no description file
    And "solo.git" has gitweb config "description" set to "Configured description"
    When I read the metadata of "solo.git"
    Then the description is "Configured description"

  Scenario: the description file wins over the gitweb config value
    Given a project root containing repository "solo.git"
    And "solo.git" has the description file "From the file"
    And "solo.git" has gitweb config "description" set to "From the config"
    When I read the metadata of "solo.git"
    Then the description is "From the file"

  Scenario: a project with neither a description file nor config has no description
    Given a project root containing repository "solo.git"
    And "solo.git" has no description file
    When I read the metadata of "solo.git"
    Then there is no description

  # --- owner: file / config win, then the filesystem owner (get_file_owner) ---
  # gitweb resolves an owner from the projects-list file, then the gitweb.owner
  # config, and only as a last resort from the operating-system owner of the
  # repository directory (get_file_owner: the user that owns the directory on
  # disk). The uid -> name step is seamed so this pins a name deterministically
  # instead of leaning on the host's passwd database.

  Scenario: an owner is read from gitweb config
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "owner" set to "Ada Lovelace"
    When I read the metadata of "solo.git"
    Then the owner is "Ada Lovelace"

  Scenario: with no file or config owner, the filesystem owner is used
    Given a project root containing repository "solo.git"
    And the repository directory's operating-system owner resolves to "Grace Hopper"
    When I read the metadata of "solo.git"
    Then the owner is "Grace Hopper"

  Scenario: the gitweb config owner wins over the filesystem owner
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "owner" set to "Ada Lovelace"
    And the repository directory's operating-system owner resolves to "Grace Hopper"
    When I read the metadata of "solo.git"
    Then the owner is "Ada Lovelace"

  Scenario: with no owner anywhere and no passwd entry, there is no owner
    Given a project root containing repository "solo.git"
    When I read the metadata of "solo.git"
    Then there is no owner

  # --- category: file and config ---

  Scenario: a category is read from the category file
    Given a project root containing repository "solo.git"
    And "solo.git" has the category file "tools"
    When I read the metadata of "solo.git"
    Then the category is "tools"

  Scenario: a category falls back to gitweb config when there is no file
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "category" set to "libraries"
    When I read the metadata of "solo.git"
    Then the category is "libraries"

  # --- clone URLs: zero / one / many ---

  Scenario: a project with no clone URLs has none
    Given a project root containing repository "solo.git"
    When I read the metadata of "solo.git"
    Then there are 0 clone URLs

  Scenario: a single clone URL comes from gitweb config
    Given a project root containing repository "solo.git"
    And "solo.git" has gitweb config "url" set to "git://example.org/solo.git"
    When I read the metadata of "solo.git"
    Then there is 1 clone URL
    And the clone URLs include "git://example.org/solo.git"

  Scenario: several clone URLs come from the cloneurl file in order
    Given a project root containing repository "solo.git"
    And "solo.git" has the clone URLs "git://example.org/solo.git" and "https://example.org/solo.git"
    When I read the metadata of "solo.git"
    Then there are 2 clone URLs
    And the clone URLs include "git://example.org/solo.git"
    And the clone URLs include "https://example.org/solo.git"

  # --- last activity: the most recent branch commit (git_get_last_activity) ---
  # gitweb reports a project's "last change" as the committer timestamp of its
  # most recently updated branch (`git for-each-ref --sort=-committerdate
  # --count=1 refs/heads`). The adapter surfaces the raw timestamp; turning it
  # into an age is the view's job, so conformance can pin exact epoch seconds.

  Scenario: an unborn repository with no commits has no last activity
    Given a project root containing an empty repository "fresh.git"
    When I read the metadata of "fresh.git"
    Then there is no last activity

  Scenario: a single-branch repository reports that branch's commit time
    Given a project root containing an empty repository "single.git"
    And "single.git" has a branch "main" committed at 1700000000
    When I read the metadata of "single.git"
    Then the last activity timestamp is 1700000000

  Scenario: with several branches the most recent commit wins, whatever the order
    Given a project root containing an empty repository "busy.git"
    And "busy.git" has a branch "main" committed at 1700000000
    And "busy.git" has a branch "feature" committed at 1700002000
    And "busy.git" has a branch "topic" committed at 1700001000
    When I read the metadata of "busy.git"
    Then the last activity timestamp is 1700002000

  Scenario: only branch refs count, so a newer tag does not move the last activity
    Given a project root containing an empty repository "tagged.git"
    And "tagged.git" has a branch "main" committed at 1700000000
    And "tagged.git" has a tag "v1" committed at 1700009000
    When I read the metadata of "tagged.git"
    Then the last activity timestamp is 1700000000

  # gitweb's git_get_last_activity scans `map { "refs/$_" } get_branch_refs()`,
  # so a commit on a configured extra-branch-refs directory moves last activity,
  # but a ref under a directory the feature does not list never counts.

  Scenario: a newer commit on a configured extra branch directory wins
    Given a project root containing an empty repository "extra.git"
    And the "extra-branch-refs" feature lists "sandbox"
    And "extra.git" has a branch "main" committed at 1700000000
    And "extra.git" has a ref "refs/sandbox/wip" committed at 1700005000
    When I read the metadata of "extra.git"
    Then the last activity timestamp is 1700005000

  Scenario: a ref under an unlisted directory does not move the last activity
    Given a project root containing an empty repository "plain.git"
    And "plain.git" has a branch "main" committed at 1700000000
    And "plain.git" has a ref "refs/sandbox/wip" committed at 1700005000
    When I read the metadata of "plain.git"
    Then the last activity timestamp is 1700000000

  # --- the missing and unsafe edges, shared with open() ---

  Scenario: reading metadata for a name that does not exist fails as not found
    Given an empty project root
    When I read the metadata of "ghost.git"
    Then reading metadata fails as not found

  Scenario: a path-traversal name is rejected as invalid
    Given an empty project root
    When I read the metadata of "../etc"
    Then reading metadata fails as invalid
