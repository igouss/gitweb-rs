Feature: Assembling a snapshot (snapshot use case)
  The snapshot use case orchestrates gitweb's git_snapshot over the repository
  port: it picks the format from the request and the site's configured formats,
  resolves the hash, dates the archive from the commit behind it, names the
  archive, and produces the bytes wrapped in the project-version directory. The
  format and naming rules are proven as pure rules elsewhere; here we prove the
  orchestration — that the right format is chosen, the archive is named and dated
  from the resolved object, and the failures surface with gitweb's statuses.

  The fake archive echoes the format and the threaded options into its bytes
  ("fmt=<key>;prefix=<prefix>;mtime=<seconds>"), so the use case's choices are
  observable without a real archive.

  Scenario: a commit snapshot is typed, named, dated, and wrapped in its directory
    Given a snapshot commit "feedface00000000000000000000000000000000" dated 1500000000 +0000
    And the snapshot project is "proj.git"
    And the site enables snapshot formats "tgz, zip"
    When I assemble the snapshot of "feedface00000000000000000000000000000000" requesting "zip"
    Then the snapshot content type is "application/x-zip"
    And the snapshot is offered inline as "proj-feedfac.zip"
    And the snapshot is dated
    And the snapshot archive is "fmt=zip;prefix=proj-feedfac;mtime=1500000000"

  Scenario: an unset format defaults to the first configured one
    Given a snapshot commit "feedface00000000000000000000000000000000" dated 1500000000 +0000
    And the snapshot project is "proj.git"
    And the site enables snapshot formats "tgz, zip"
    When I assemble the snapshot of "feedface00000000000000000000000000000000" with no requested format
    Then the snapshot content type is "application/x-gzip"
    And the snapshot is offered inline as "proj-feedfac.tar.gz"

  Scenario: a bare tree snapshot is archived but carries no date
    Given a snapshot tree "abcdef0000000000000000000000000000000000"
    And the snapshot project is "proj.git"
    And the site enables snapshot formats "tgz"
    When I assemble the snapshot of "abcdef0000000000000000000000000000000000" requesting "tgz"
    Then the snapshot content type is "application/x-gzip"
    And the snapshot has no date
    And the snapshot archive is "fmt=tgz;prefix=proj-abcdef0;mtime=0"

  Scenario: a blob hash is not a tree-ish
    Given a snapshot blob "0bada55000000000000000000000000000000000"
    And the snapshot project is "proj.git"
    And the site enables snapshot formats "tgz"
    When I assemble the snapshot of "0bada55000000000000000000000000000000000" requesting "tgz"
    Then assembling the snapshot fails as invalid

  Scenario: a disabled snapshot feature forbids the request
    Given the snapshot project is "proj.git"
    And the site enables snapshot formats ""
    When I assemble the snapshot of "feedface00000000000000000000000000000000" requesting "zip"
    Then assembling the snapshot fails as forbidden

  Scenario: a request with no hash is not found
    Given the snapshot project is "proj.git"
    And the site enables snapshot formats "tgz"
    When I assemble the snapshot with no hash
    Then assembling the snapshot fails as not found
