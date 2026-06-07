Feature: Loading gitweb-rs global config files
  gitweb's config is executable Perl; gitweb-rs reads a declarative TOML format
  instead. The loader reads the files config_chain selected (weakest first),
  parses each into a partial layer, and resolves them over the built-in defaults
  — so a later file overrides an earlier one, and an omitted setting keeps what
  weaker sources resolved. The files here are written to disk and read back, so
  the loader is verified against real files exactly as the composition root runs.

  Scenario: With no files, the built-in defaults are loaded
    Given no config files
    When I load the configured files
    Then loading succeeds
    And the resolved site name is "Untitled Git"
    And the resolved "grep" feature default is "1"

  Scenario: A single file sets scalars and a list
    Given a config file "gitweb.toml":
      """
      projectroot = "/srv/git"
      site_name = "Example Git"
      git_base_url_list = ["git://example.org/", "https://example.org/git/"]
      """
    When I load the configured files
    Then loading succeeds
    And the resolved projectroot is "/srv/git"
    And the resolved site name is "Example Git"
    And the resolved clone base URLs are "git://example.org/, https://example.org/git/"

  Scenario: A later file overrides an earlier one
    Given a config file "common.toml":
      """
      projectroot = "/srv/git"
      site_name = "Common Git"
      """
    And a config file "instance.toml":
      """
      site_name = "Instance Git"
      """
    When I load the configured files
    Then loading succeeds
    And the resolved projectroot is "/srv/git"
    And the resolved site name is "Instance Git"

  Scenario: A feature table sets default and override
    Given a config file "gitweb.toml":
      """
      [features.blame]
      default = ["1"]
      override = true
      """
    When I load the configured files
    Then loading succeeds
    And the resolved "blame" feature default is "1"
    And the resolved "blame" feature is overridable

  Scenario: A multi-option feature default is read as a list
    Given a config file "gitweb.toml":
      """
      [features.snapshot]
      default = ["tgz", "zip"]
      """
    When I load the configured files
    Then loading succeeds
    And the resolved "snapshot" feature default is "tgz, zip"

  Scenario: An unknown feature is rejected
    Given a config file "gitweb.toml":
      """
      [features.teleport]
      default = ["1"]
      """
    When I load the configured files
    Then loading fails

  Scenario: An unknown setting is rejected
    Given a config file "gitweb.toml":
      """
      projetroot = "/srv/git"
      """
    When I load the configured files
    Then loading fails

  Scenario: Malformed TOML is rejected
    Given a config file "gitweb.toml":
      """
      site_name =
      """
    When I load the configured files
    Then loading fails
