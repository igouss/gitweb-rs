Feature: Serving a downloadable snapshot archive (snapshot action)
  gitweb's git_snapshot streams a compressed archive of a tree-ish. It picks the
  format from the request and the site's enabled formats, resolves the hash,
  rejects a non-tree-ish, and serves the bytes under the format's media type,
  offered inline under a project-and-version filename, dated with the commit time.
  A site with no enabled formats, a statically disabled format (txz), and a
  format the site does not enable are each refused; an unknown format is a bad
  request; a missing ref or project is not found.

  Background:
    Given a repository "t.git" with blobs
    And the repository "t.git" has an annotated tag "v1.0" of a commit at 1500000000 with subject "release"

  Scenario: a tag snapshot is served as a gzip named for the project and tag
    Given the snapshot action is served with formats "tgz, zip"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0&sf=tgz"
    Then the response status is 200
    And the response content type is "application/x-gzip"
    And the response is offered inline as "t-v1.0.tar.gz"
    And the response has a last-modified header
    And the response body begins with the gzip magic

  Scenario: a zip snapshot is served under the zip type and suffix
    Given the snapshot action is served with formats "tgz, zip"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0&sf=zip"
    Then the response status is 200
    And the response content type is "application/x-zip"
    And the response is offered inline as "t-v1.0.zip"
    And the response body begins with the zip magic

  Scenario: an unset format defaults to the first enabled one
    Given the snapshot action is served with formats "tgz, zip"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0"
    Then the response status is 200
    And the response content type is "application/x-gzip"
    And the response is offered inline as "t-v1.0.tar.gz"

  Scenario: a HEAD snapshot is named for the abbreviated commit
    Given the snapshot action is served with formats "tgz"
    When I GET "/?p=t.git&a=snapshot&h=HEAD&sf=tgz"
    Then the response status is 200
    And the response content type is "application/x-gzip"
    And the response content disposition contains "filename="t-HEAD-"
    And the response content disposition contains ".tar.gz""

  Scenario: a disabled snapshot feature forbids the download
    Given the snapshot action is served with formats ""
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0&sf=tgz"
    Then the response status is 403

  Scenario: a statically disabled format is forbidden
    Given the snapshot action is served with formats "tgz, zip"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0&sf=txz"
    Then the response status is 403

  Scenario: a format the site does not enable is forbidden
    Given the snapshot action is served with formats "tgz"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0&sf=zip"
    Then the response status is 403

  Scenario: an unknown format token is a bad request
    Given the snapshot action is served with formats "tgz"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/v1.0&sf=rar"
    Then the response status is 400

  Scenario: a snapshot of a missing ref is not found
    Given the snapshot action is served with formats "tgz"
    When I GET "/?p=t.git&a=snapshot&h=refs/tags/nope&sf=tgz"
    Then the response status is 404

  Scenario: a snapshot of a project that does not exist is not found
    Given the snapshot action is served with formats "tgz"
    When I GET "/?p=ghost.git&a=snapshot&h=HEAD&sf=tgz"
    Then the response status is 404
