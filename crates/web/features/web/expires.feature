Feature: By-oid cache freshness on served responses (Expires +1d)
  gitweb stamps an Expires header — a one-day freshness window — on a response
  whose primary hash is a literal object id, because content addressed by an
  immutable oid may be held by a cache for a day. A symbolic ref (HEAD, a branch
  name) or a request resolved through a file name carries no Expires, since that
  content can change under the same URL. This covers the by-oid sites: git_commit
  / git_commitdiff (html), git_commitdiff_plain, git_blob, and git_blob_plain.

  Scenario: a commit addressed by its full object id is cacheable for a day
    Given a project root containing a commit repository "c.git"
    And the commit action is served
    When I GET the "commit" of "c.git" by its full object id
    Then the response status is 200
    And the response content type is "text/html; charset=utf-8"
    And the response has an expires header

  Scenario: a commit addressed by the HEAD ref is not cached
    Given a project root containing a commit repository "c.git"
    And the commit action is served
    When I GET "/?p=c.git&a=commit&h=HEAD"
    Then the response status is 200
    And the response has no expires header

  Scenario: a commit addressed by no hash at all is not cached
    Given a project root containing a commit repository "c.git"
    And the commit action is served
    When I GET "/?p=c.git&a=commit"
    Then the response status is 200
    And the response has no expires header

  Scenario: a raw commitdiff addressed by its full object id is cacheable for a day
    Given a project root containing a commit repository "c.git"
    And the commitdiff_plain action is served
    When I GET the "commitdiff_plain" of "c.git" by its full object id
    Then the response status is 200
    And the response content type is "text/plain; charset=utf-8"
    And the response has an expires header

  Scenario: a blob addressed by its full object id is cacheable for a day
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET the blob "readme.txt" of "t.git" by its full object id
    Then the response status is 200
    And the response has an expires header

  Scenario: a blob resolved through its file name is not cached
    Given a repository "t.git" with blobs
    And the blob action is served
    When I GET "/?p=t.git&a=blob&hb=HEAD&f=readme.txt"
    Then the response status is 200
    And the response has no expires header
