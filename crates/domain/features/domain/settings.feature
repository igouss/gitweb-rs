Feature: Global gitweb settings value precedence
  config_chain.feature decides WHICH global config files load and in what order
  (weakest first). This is the other half: resolving the VALUES the rest of the
  app reads across that ordered list of sources — built-in defaults first, then
  each loaded source overlaid on top (and, later, per-project strongest of all).

  gitweb's config files are executable Perl that re-assign `our` variables and
  %feature entries; the strongest source that sets a value wins. We do not run
  Perl, so a source is a partial layer and the rule overlays them. Settings
  compose by three kinds: a scalar's value replaces; a list replaces wholesale
  (no append); a feature's `default` and `override` flag overlay independently.

  Scenario: With no sources, the built-in defaults stand
    Given no config sources
    When I resolve the settings
    Then the site name is "Untitled Git"
    And the logo is "static/git-logo.png"
    And the favicon is "static/git-favicon.png"
    And the default projects order is "project"
    And the fallback encoding is "latin1"
    And the "grep" feature default is "1"
    And the "blame" feature default is "0"
    And the "blame" feature is not overridable

  # --- scalar: replace -------------------------------------------------------

  Scenario: A single source sets a scalar
    Given a config source
    And it sets the projectroot to "/srv/git"
    When I resolve the settings
    Then the projectroot is "/srv/git"

  Scenario: A scalar unset by a source keeps the built-in default
    Given a config source
    And it sets the projectroot to "/srv/git"
    When I resolve the settings
    Then the site name is "Untitled Git"

  Scenario: The strongest source wins for a scalar
    Given a config source
    And it sets the site name to "Common Git"
    And a config source
    And it sets the site name to "Instance Git"
    When I resolve the settings
    Then the site name is "Instance Git"

  Scenario: A weaker source still wins where the stronger one is silent
    Given a config source
    And it sets the projectroot to "/srv/git"
    And it sets the site name to "Common Git"
    And a config source
    And it sets the site name to "Instance Git"
    When I resolve the settings
    Then the projectroot is "/srv/git"
    And the site name is "Instance Git"

  Scenario: A source overrides the logo and favicon
    Given a config source
    And it sets the logo to "static/our-logo.png"
    And it sets the favicon to "static/our-favicon.ico"
    When I resolve the settings
    Then the logo is "static/our-logo.png"
    And the favicon is "static/our-favicon.ico"

  # --- list: replace wholesale (never append) --------------------------------

  Scenario: A source replaces a list wholesale
    Given a config source
    And it sets the clone base URLs to "git://a/, https://b/"
    When I resolve the settings
    Then the clone base URLs are "git://a/, https://b/"

  Scenario: A stronger source's list replaces a weaker one's, not appends
    Given a config source
    And it sets the clone base URLs to "git://a/, git://b/"
    And a config source
    And it sets the clone base URLs to "https://c/"
    When I resolve the settings
    Then the clone base URLs are "https://c/"

  # --- feature: default and override overlay independently -------------------

  Scenario: A source raises a feature default
    Given a config source
    And it sets the "blame" feature default to "1"
    When I resolve the settings
    Then the "blame" feature default is "1"
    And the "blame" feature is not overridable

  Scenario: A source enables per-project override without touching the default
    Given a config source
    And it makes the "blame" feature overridable
    When I resolve the settings
    Then the "blame" feature default is "0"
    And the "blame" feature is overridable

  Scenario: A feature's default and override flag compose from different sources
    Given a config source
    And it sets the "blame" feature default to "1"
    And a config source
    And it makes the "blame" feature overridable
    When I resolve the settings
    Then the "blame" feature default is "1"
    And the "blame" feature is overridable

  Scenario: A multi-option feature default replaces wholesale
    Given a config source
    And it sets the "snapshot" feature default to "tgz, zip"
    When I resolve the settings
    Then the "snapshot" feature default is "tgz, zip"

  # --- feature: boolean reading (gitweb feature_bool / gitweb_check_feature) --
  # A boolean feature is on when its first default option is Perl-truthy, the way
  # gitweb_check_feature reads it: "1" is on, "0" is off, and a feature with no
  # options at all (a list feature read in boolean context) is off.

  Scenario: A feature defaulting to "0" is disabled
    Given no config sources
    When I resolve the settings
    Then the "remote_heads" feature is disabled

  Scenario: A source switching a feature default to "1" enables it
    Given a config source
    And it sets the "remote_heads" feature default to "1"
    When I resolve the settings
    Then the "remote_heads" feature is enabled

  Scenario: A feature with no default options is disabled
    Given no config sources
    When I resolve the settings
    Then the "actions" feature is disabled

  # --- per-project override: the repository's gitweb.<key> (strongest of all) --
  # gitweb_get_feature consults the repository's own gitweb.<key> config ONLY when
  # the site has made the feature overridable AND we are inside a project. A
  # boolean feature reads it through gitweb's config_to_bool: a non-zero number,
  # "true"/"yes" (any case), or a valueless key is on; "0", "false"/"no"/"off",
  # the empty string, or anything else is off. A key the repository never sets
  # leaves the site default standing.

  Scenario: A non-overridable feature ignores the repository's gitweb config
    Given a config source
    And it sets the "blame" feature default to "0"
    And the repository sets gitweb."blame" to "true"
    When I resolve the settings for the project
    Then the project "blame" feature default is "0"

  Scenario: An overridable boolean feature reads the repository to turn on
    Given a config source
    And it sets the "blame" feature default to "0"
    And it makes the "blame" feature overridable
    And the repository sets gitweb."blame" to "true"
    When I resolve the settings for the project
    Then the project "blame" feature default is "1"

  Scenario: An overridable boolean feature reads the repository to turn off
    Given a config source
    And it sets the "blame" feature default to "1"
    And it makes the "blame" feature overridable
    And the repository sets gitweb."blame" to "0"
    When I resolve the settings for the project
    Then the project "blame" feature default is "0"

  Scenario: An overridable feature the repository never sets keeps the site default
    Given a config source
    And it sets the "blame" feature default to "1"
    And it makes the "blame" feature overridable
    And the repository sets no gitweb config
    When I resolve the settings for the project
    Then the project "blame" feature default is "1"

  Scenario: An overridable boolean feature reads a valueless key as on
    Given a config source
    And it sets the "blame" feature default to "0"
    And it makes the "blame" feature overridable
    And the repository sets a valueless gitweb."blame"
    When I resolve the settings for the project
    Then the project "blame" feature default is "1"

  # Every boolean feature gitweb exposes a feature_bool sub for reads its key the
  # same way; one scenario each pins the wiring (and a different truthy spelling).

  Scenario: An overridable grep is turned off by the repository
    Given a config source
    And it makes the "grep" feature overridable
    And the repository sets gitweb."grep" to "0"
    When I resolve the settings for the project
    Then the project "grep" feature default is "0"

  Scenario: An overridable pickaxe is turned off by "false"
    Given a config source
    And it makes the "pickaxe" feature overridable
    And the repository sets gitweb."pickaxe" to "false"
    When I resolve the settings for the project
    Then the project "pickaxe" feature default is "0"

  Scenario: An overridable show-sizes is turned off by the repository
    Given a config source
    And it makes the "show-sizes" feature overridable
    And the repository sets gitweb."show-sizes" to "0"
    When I resolve the settings for the project
    Then the project "show-sizes" feature default is "0"

  Scenario: An overridable highlight is turned on by "true"
    Given a config source
    And it makes the "highlight" feature overridable
    And the repository sets gitweb."highlight" to "true"
    When I resolve the settings for the project
    Then the project "highlight" feature default is "1"

  Scenario: An overridable remote_heads is turned on by "yes"
    Given a config source
    And it makes the "remote_heads" feature overridable
    And the repository sets gitweb."remote_heads" to "yes"
    When I resolve the settings for the project
    Then the project "remote_heads" feature default is "1"

  # snapshot is a list feature (gitweb feature_snapshot): the repository value is
  # a comma- or whitespace-separated format list, "none" means none, and a falsy
  # value (empty, "0", absent) leaves the site default. Site default is "tgz".

  Scenario: An overridable snapshot takes a single repository format
    Given a config source
    And it makes the "snapshot" feature overridable
    And the repository sets gitweb."snapshot" to "zip"
    When I resolve the settings for the project
    Then the project "snapshot" feature default is "zip"

  Scenario: An overridable snapshot splits a comma-separated repository list
    Given a config source
    And it makes the "snapshot" feature overridable
    And the repository sets gitweb."snapshot" to "tgz,zip"
    When I resolve the settings for the project
    Then the project "snapshot" feature default is "tgz, zip"

  Scenario: An overridable snapshot splits a whitespace-separated repository list
    Given a config source
    And it makes the "snapshot" feature overridable
    And the repository sets gitweb."snapshot" to "tgz zip tbz2"
    When I resolve the settings for the project
    Then the project "snapshot" feature default is "tgz, zip, tbz2"

  Scenario: An overridable snapshot set to "none" offers no formats
    Given a config source
    And it makes the "snapshot" feature overridable
    And the repository sets gitweb."snapshot" to "none"
    When I resolve the settings for the project
    Then the project "snapshot" feature default is ""

  Scenario: An overridable snapshot set to the empty string keeps the site default
    Given a config source
    And it makes the "snapshot" feature overridable
    And the repository sets gitweb."snapshot" to ""
    When I resolve the settings for the project
    Then the project "snapshot" feature default is "tgz"

  # patches is an integer feature (gitweb feature_patches over config_to_int): the
  # repository value is read as an integer, honouring git's k/m/g unit suffixes.
  # Site default is "16".

  Scenario: An overridable patches takes the repository's count
    Given a config source
    And it makes the "patches" feature overridable
    And the repository sets gitweb."patches" to "100"
    When I resolve the settings for the project
    Then the project "patches" feature default is "100"

  Scenario: An overridable patches set to zero disables the link
    Given a config source
    And it makes the "patches" feature overridable
    And the repository sets gitweb."patches" to "0"
    When I resolve the settings for the project
    Then the project "patches" feature default is "0"

  Scenario: An overridable patches honours a git k/m/g unit suffix
    Given a config source
    And it makes the "patches" feature overridable
    And the repository sets gitweb."patches" to "2k"
    When I resolve the settings for the project
    Then the project "patches" feature default is "2048"

  # avatar is a single-value feature (gitweb feature_avatar): the repository value
  # is the avatar provider, taken verbatim. Site default is empty (no avatars).

  Scenario: An overridable avatar takes the repository's provider
    Given a config source
    And it makes the "avatar" feature overridable
    And the repository sets gitweb."avatar" to "gravatar"
    When I resolve the settings for the project
    Then the project "avatar" feature default is "gravatar"

  Scenario: An overridable avatar overrides the site provider
    Given a config source
    And it sets the "avatar" feature default to "gravatar"
    And it makes the "avatar" feature overridable
    And the repository sets gitweb."avatar" to "picon"
    When I resolve the settings for the project
    Then the project "avatar" feature default is "picon"

  # extra-branch-refs is a whitespace-list feature (gitweb
  # feature_extra_branch_refs): the repository value splits on runs of whitespace
  # into ref namespaces. Site default is empty.

  Scenario: An overridable extra-branch-refs takes a single repository namespace
    Given a config source
    And it makes the "extra-branch-refs" feature overridable
    And the repository sets gitweb."extra-branch-refs" to "sandbox"
    When I resolve the settings for the project
    Then the project "extra-branch-refs" feature default is "sandbox"

  Scenario: An overridable extra-branch-refs splits on whitespace
    Given a config source
    And it makes the "extra-branch-refs" feature overridable
    And the repository sets gitweb."extra-branch-refs" to "wip   review"
    When I resolve the settings for the project
    Then the project "extra-branch-refs" feature default is "wip, review"
