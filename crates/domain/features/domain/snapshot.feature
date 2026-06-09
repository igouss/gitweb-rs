Feature: Snapshot formats, format selection, and archive naming
  gitweb's git_snapshot serves a downloadable archive of a tree-ish. Three pure
  rules sit in front of the bytes: the fixed format table (each format's media
  type, suffix, and label, with txz statically disabled), the filter that turns a
  site's configured formats into the effective enabled set, and the validation
  cascade that picks the format for one request. A fourth rule, snapshot_name,
  builds the project-version base name the download filename and the archive's
  top-level directory share. These are the rules; the bytes are the adapter's job.

  # --- the format table (%known_snapshot_formats) ---

  Scenario Outline: a snapshot format reports gitweb's media type, suffix, and label
    Given the snapshot format "<key>"
    Then its content type is "<type>"
    And its filename suffix is "<suffix>"
    And its display name is "<display>"

    Examples:
      | key  | type                | suffix   | display |
      | tgz  | application/x-gzip  | .tar.gz  | tar.gz  |
      | tbz2 | application/x-bzip2 | .tar.bz2 | tar.bz2 |
      | txz  | application/x-xz    | .tar.xz  | tar.xz  |
      | zip  | application/x-zip   | .zip     | zip     |

  Scenario: txz is the one statically disabled format
    Then the snapshot format "txz" is disabled
    And the snapshot format "tgz" is not disabled
    And the snapshot format "zip" is not disabled

  # --- the configured-list filter (filter_snapshot_fmts) ---

  Scenario: the configured list resolves aliases and drops unknown and disabled formats
    Given the configured snapshot formats "gzip, bzip2, txz, zip, bogus, x-gzip"
    When I compute the enabled snapshot formats
    Then the enabled formats are "tgz, tbz2, zip"

  Scenario: a configured list of only disabled or junk tokens enables nothing
    Given the configured snapshot formats "txz, gz, x-zip"
    When I compute the enabled snapshot formats
    Then no formats are enabled

  # --- the selection cascade (git_snapshot) ---

  Scenario: with no formats enabled, every snapshot is forbidden
    Given no snapshot formats are enabled
    When I select the snapshot format requested "zip"
    Then snapshot selection is forbidden as "Snapshots not allowed"

  Scenario: an unset format defaults to the first enabled format
    Given the enabled snapshot formats "tbz2, tgz"
    When I select the snapshot format with no request
    Then the selected snapshot format is "tbz2"

  Scenario: an empty format request defaults to the first enabled format
    Given the enabled snapshot formats "zip, tgz"
    When I select the snapshot format requested ""
    Then the selected snapshot format is "zip"

  Scenario: a format with illegal characters is a bad request
    Given the enabled snapshot formats "tgz"
    When I select the snapshot format requested "tar.gz"
    Then snapshot selection is invalid as "Invalid snapshot format parameter"

  Scenario: an uppercase format token is a bad request
    Given the enabled snapshot formats "tgz"
    When I select the snapshot format requested "ZIP"
    Then snapshot selection is invalid as "Invalid snapshot format parameter"

  Scenario: a well-formed but unknown format token is a bad request
    Given the enabled snapshot formats "tgz"
    When I select the snapshot format requested "rar"
    Then snapshot selection is invalid as "Unknown snapshot format"

  Scenario: a known but statically disabled format is forbidden
    Given the enabled snapshot formats "tgz"
    When I select the snapshot format requested "txz"
    Then snapshot selection is forbidden as "Snapshot format not allowed"

  Scenario: a known format the site does not configure is forbidden
    Given the enabled snapshot formats "tgz"
    When I select the snapshot format requested "zip"
    Then snapshot selection is forbidden as "Unsupported snapshot format"

  Scenario: a configured format is selected
    Given the enabled snapshot formats "tgz, zip"
    When I select the snapshot format requested "zip"
    Then the selected snapshot format is "zip"

  # --- the archive base name (snapshot_name) ---

  Scenario Outline: the snapshot name joins the project basename and a version string
    Given the project path "<project>"
    And the snapshot hash "<hash>" abbreviating to "<short>"
    When I build the snapshot name
    Then the snapshot name is "<name>"

    Examples:
      | project                  | hash                                     | short   | name                  |
      | foo.git                  | a1b2c3d4e5f60718293a4b5c6d7e8f9012345678 | a1b2c3d | foo-a1b2c3d           |
      | path/to/foo.git          | a1b2c3d4e5f60718293a4b5c6d7e8f9012345678 | a1b2c3d | foo-a1b2c3d           |
      | foo/.git                 | a1b2c3d4e5f60718293a4b5c6d7e8f9012345678 | a1b2c3d | foo-a1b2c3d           |
      | foo.git                  | abc1234                                  | abc1234 | foo-abc1234           |
      | foo.git                  | refs/tags/v1.0                           | a1b2c3d | foo-v1.0              |
      | foo.git                  | refs/heads/main                          | a1b2c3d | foo-main-a1b2c3d      |
      | foo.git                  | refs/heads/feature/x                     | a1b2c3d | foo-feature.x-a1b2c3d |
      | foo.git                  | refs/remotes/origin/main                 | a1b2c3d | foo-origin.main-a1b2c3d |
      | foo.git                  | HEAD                                     | a1b2c3d | foo-HEAD-a1b2c3d      |
      | my repo!.git             | refs/tags/v2                             | a1b2c3d | myrepo-v2             |
