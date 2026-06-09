Feature: Serving a commit as a format-patch mail (patch action)
  gitweb's patch action streams `git format-patch` for one commit: a mailbox
  header (From / Date / Subject), the body, a `---`, the diffstat, the diff, and
  a `-- ` / git-version signature. It is served as text/plain and offered inline
  under `<project basename>-<hash>.patch`. The view is gated on the `patches`
  feature — off, gitweb answers 403 "Patch view not allowed".

  Scenario: the commit is served as an inline mailbox patch
    Given a project root containing a commit repository "c.git"
    And the patch action is served
    When I GET "/?p=c.git&a=patch"
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response is offered inline as "c.git-HEAD.patch"
    And the response body contains " Mon Sep 17 00:00:00 2001"
    And the response body contains "From: Ada Lovelace"
    And the response body contains "Subject: [PATCH] Rework the engine"
    And the response body contains "Detailed body line."
    And the response body contains "diff --git a/README b/README"
    And the response body contains "2.54.0"

  Scenario: the filename uses an explicit hash request value
    Given a project root containing a commit repository "c.git"
    And the patch action is served
    When I GET "/?p=c.git&a=patch&h=HEAD"
    Then the response status is 200
    And the response is offered inline as "c.git-HEAD.patch"

  Scenario: a patch for a revision that resolves to nothing is not found
    Given a project root containing a commit repository "c.git"
    And the patch action is served
    When I GET "/?p=c.git&a=patch&h=0000000000000000000000000000000000000000"
    Then the response status is 404

  Scenario: the patch view is forbidden when the patches feature is off
    Given a project root containing a commit repository "c.git"
    And the patch action is served with the patches feature off
    When I GET "/?p=c.git&a=patch"
    Then the response status is 403
