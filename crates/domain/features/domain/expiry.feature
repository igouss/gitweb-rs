Feature: By-oid cache freshness (Expires +1d)

  gitweb stamps an Expires header — a one-day freshness window — on a response
  whose primary hash is a literal object id, because content addressed by an
  immutable oid may be held by a cache for a day: the
  "if ($hash =~ /^$oid_regex$/) { $expires = '+1d'; }" sites. A symbolic ref, an
  abbreviated id, or an absent hash earns no window: such content can change
  under the same URL and must be revalidated. The oid_regex matches only a full
  40-character (SHA-1) or 64-character (SHA-256) hex id.

  Scenario: a full SHA-1 object id earns a one-day window
    Given a view addressed by hash "1c002dd4b536e7479fe34593e72e6c6c1819e53b"
    When I evaluate its cache freshness
    Then the freshness window is one day
    And the freshness window is 86400 seconds

  Scenario: a full SHA-256 object id earns a one-day window
    Given a view addressed by hash "18114c2b5780cc40bef4d20f0d5fa1b772d44d75ee36f73c6f5e6f5c34c0c4e4"
    When I evaluate its cache freshness
    Then the freshness window is one day

  Scenario: a symbolic HEAD earns no window
    Given a view addressed by hash "HEAD"
    When I evaluate its cache freshness
    Then there is no freshness window

  Scenario: a branch name earns no window
    Given a view addressed by hash "master"
    When I evaluate its cache freshness
    Then there is no freshness window

  Scenario: an abbreviated object id earns no window
    Given a view addressed by hash "1c002dd"
    When I evaluate its cache freshness
    Then there is no freshness window

  Scenario: a forty-character non-hex string earns no window
    Given a view addressed by hash "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
    When I evaluate its cache freshness
    Then there is no freshness window

  Scenario: a view addressed by no hash earns no window
    Given a view addressed by no hash
    When I evaluate its cache freshness
    Then there is no freshness window

  # gitweb's git_blobdiff (html and plain alike) gates Expires on a DUAL-oid rule:
  # "if ($hash_base =~ /^$oid_regex$/ && $hash_parent_base =~ /^$oid_regex$/)".
  # BOTH the base and the parent base must be literal object ids before a
  # single-file diff is cacheable; if either side is a ref name or absent, no
  # window — that content can move under the same URL.

  Scenario: a single-file diff between two full object ids earns a one-day window
    Given a single-file diff with base "1c002dd4b536e7479fe34593e72e6c6c1819e53b" and parent base "8f94139338f9404f26296befa88755fc2598c289"
    When I evaluate the single-file diff cache freshness
    Then the freshness window is one day
    And the freshness window is 86400 seconds

  Scenario: a single-file diff whose base is a ref earns no window
    Given a single-file diff with base "HEAD" and parent base "8f94139338f9404f26296befa88755fc2598c289"
    When I evaluate the single-file diff cache freshness
    Then there is no freshness window

  Scenario: a single-file diff whose parent base is a ref earns no window
    Given a single-file diff with base "1c002dd4b536e7479fe34593e72e6c6c1819e53b" and parent base "master"
    When I evaluate the single-file diff cache freshness
    Then there is no freshness window

  Scenario: a single-file diff missing its parent base earns no window
    Given a single-file diff with base "1c002dd4b536e7479fe34593e72e6c6c1819e53b" and no parent base
    When I evaluate the single-file diff cache freshness
    Then there is no freshness window
