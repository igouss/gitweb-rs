Feature: Global gitweb config source selection
  gitweb's evaluate_gitweb_config decides which global config files to load and
  in what order. Each of the three slots resolves from its environment override
  or a built-in default ($ENV{X} || "@DEFAULT@"). GITWEB_CONFIG_COMMON is always
  applied first; then the first of GITWEB_CONFIG / GITWEB_CONFIG_SYSTEM whose
  file exists is applied (GITWEB_CONFIG wins, SYSTEM only fills in when CONFIG is
  absent). CONFIG and SYSTEM are de-duplicated against COMMON so no file is read
  twice. Reading the files themselves is the web boundary's job; here we select
  the ordered set, weakest first, given which candidate files exist on disk.

  Scenario: With nothing configured, no files are loaded
    Given no global config is configured
    When I select the global config files
    Then no config files are loaded

  Scenario: A common config that exists is loaded
    Given the common config default is "/etc/gitweb-common.conf"
    And the file "/etc/gitweb-common.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb-common.conf"

  Scenario: A configured common file that is missing is not loaded
    Given the common config default is "/etc/gitweb-common.conf"
    When I select the global config files
    Then no config files are loaded

  Scenario: Common and per-instance config load common first
    Given the common config default is "/etc/gitweb-common.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the file "/etc/gitweb-common.conf" exists
    And the file "/etc/gitweb.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb-common.conf, /etc/gitweb.conf"

  Scenario: The per-instance config wins over the system config
    Given the common config default is "/etc/gitweb-common.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the system config default is "/etc/gitweb-system.conf"
    And the file "/etc/gitweb-common.conf" exists
    And the file "/etc/gitweb.conf" exists
    And the file "/etc/gitweb-system.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb-common.conf, /etc/gitweb.conf"

  Scenario: A missing per-instance config falls through to the system config
    Given the common config default is "/etc/gitweb-common.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the system config default is "/etc/gitweb-system.conf"
    And the file "/etc/gitweb-common.conf" exists
    And the file "/etc/gitweb-system.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb-common.conf, /etc/gitweb-system.conf"

  Scenario: The environment override wins over the per-instance default
    Given the per-instance config default is "/etc/gitweb.conf"
    And the per-instance config env is "/home/u/my.conf"
    And the file "/home/u/my.conf" exists
    And the file "/etc/gitweb.conf" exists
    When I select the global config files
    Then the load order is "/home/u/my.conf"

  Scenario: An empty environment override falls back to the default
    Given the per-instance config default is "/etc/gitweb.conf"
    And the per-instance config env is ""
    And the file "/etc/gitweb.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb.conf"

  Scenario: A per-instance path equal to the common path is not read twice
    Given the common config default is "/etc/gitweb.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the file "/etc/gitweb.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb.conf"

  Scenario: De-duplicating the per-instance path lets the system config through
    Given the common config default is "/etc/gitweb.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the system config default is "/etc/gitweb-system.conf"
    And the file "/etc/gitweb.conf" exists
    And the file "/etc/gitweb-system.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb.conf, /etc/gitweb-system.conf"

  Scenario: A missing common file still lets the per-instance config load
    Given the common config default is "/etc/gitweb-common.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the file "/etc/gitweb.conf" exists
    When I select the global config files
    Then the load order is "/etc/gitweb.conf"

  Scenario: Configured files that do not exist load nothing
    Given the common config default is "/etc/gitweb-common.conf"
    And the per-instance config default is "/etc/gitweb.conf"
    And the system config default is "/etc/gitweb-system.conf"
    When I select the global config files
    Then no config files are loaded
