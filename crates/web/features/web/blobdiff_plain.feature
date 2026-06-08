Feature: Serving the single-file raw diff (blobdiff_plain action)
  gitweb's blobdiff_plain streams the raw unified diff of one file between two
  revisions: an `X-Git-Url` self-link line, a blank line, then the bare
  `git diff-tree -p` patch for that path. It is served as text/plain and offered
  inline under `<file_name>.patch`. The X-Git-Url line carries the request's own
  absolute self URL, with the parameters in gitweb's order (p, a, f, fp, hb, hpb).
  A missing base is a 404; a missing file is a 400; a path the diff does not
  touch is a 404.

  Scenario: a single file's modification is served inline as a raw patch
    Given a project root containing a commit repository "c.git"
    And the blobdiff_plain action is served
    When I GET the blobdiff_plain of "README" in "c.git"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response is offered inline as "README.patch"
    And the response body contains "diff --git a/README b/README"
    And the response body contains "+second"
    And the response body contains "X-Git-Url: http://localhost?p=c.git;a=blobdiff_plain;f=README;hb="

  Scenario: a rename carries its from-path in the patch and the self-link
    Given a project root containing a commit repository "c.git"
    And the blobdiff_plain action is served
    When I GET the blobdiff_plain of "new.txt" renamed from "old.txt" in "c.git"
    Then the response status is 200
    And the response is offered inline as "new.txt.patch"
    And the response body contains "rename from old.txt"
    And the response body contains "rename to new.txt"
    And the response body contains "X-Git-Url: http://localhost?p=c.git;a=blobdiff_plain;f=new.txt;fp=old.txt;hb="

  Scenario: a missing base is not found
    Given a project root containing a commit repository "c.git"
    And the blobdiff_plain action is served
    When I GET "/?p=c.git&a=blobdiff_plain&f=README"
    Then the response status is 404

  Scenario: present bases but a missing file is a bad request
    Given a project root containing a commit repository "c.git"
    And the blobdiff_plain action is served
    When I GET "/?p=c.git&a=blobdiff_plain&hb=HEAD&hpb=HEAD"
    Then the response status is 400

  Scenario: a path the diff does not touch is not found
    Given a project root containing a commit repository "c.git"
    And the blobdiff_plain action is served
    When I GET the blobdiff_plain of "absent.txt" in "c.git"
    Then the response status is 404
