Feature: Rendering the single annotated-tag view
  The single-tag view (modernized git_tag) shows the tag's name as a heading, an
  object-header table whose object row links the tagged object — by its id and by
  its kind label (commit / blob / tree) — to that object's view, a tagger row
  with the tagger's name, email and the absolute date the tag was made when the
  tag carries a tagger, and the message body with one line per message line. URLs
  are built by the web boundary, so this layer takes a finished object href and
  only decides layout, escaping, and the timestamp markup.

  The tagger date is gitweb's format_timestamp_html: the UTC date as the visible
  text, carried in a machine-readable <time datetime> so the javascript-timezone
  feature can localize it, plus a "(HH:MM tz)" hint in the commit's own zone that
  is flagged when the local hour is before 06:00.

  Scenario: a commit tag shows its name, object link and kind, tagger and message
    Given a commit tag "v1.0" pointing at "abc123" at "/r/obj" with message "Release 1.0"
    And tagged by "Ada Lovelace <ada@example.com>" at epoch 1780842645 +0200
    When I render the tag page
    Then the result contains ">v1.0<"
    And the result contains "abc123"
    And the result contains "/r/obj"
    And the result contains ">commit<"
    And the result contains "Ada Lovelace"
    And the result contains "ada@example.com"
    And the result contains "Sun, 7 Jun 2026 14:30:45 +0000"
    And the result contains "Release 1.0"

  Scenario: the tagger date is machine-readable and zone-aware
    Given a commit tag "v1.0" pointing at "abc123" at "/r/obj" with message "Release 1.0"
    And tagged by "Ada Lovelace <ada@example.com>" at epoch 1780842645 +0200
    When I render the tag page
    Then the result contains "2026-06-07T14:30:45Z"
    And the result contains ">16:30<"
    And the result contains "+0200"
    And the result does not contain "atnight"

  Scenario: a wee-hours tag is flagged at night
    Given a commit tag "v1.0" pointing at "abc123" at "/r/obj" with message "Release 1.0"
    And tagged by "Ada Lovelace <ada@example.com>" at epoch 1719790200 +0200
    When I render the tag page
    Then the result contains "Sun, 30 Jun 2024 23:30:00 +0000"
    And the result contains ">01:30<"
    And the result contains "local-time atnight"

  Scenario: a blob tag links the blob by its blob kind
    Given a blob tag "blobtag" pointing at "def456" at "/r/blob" with message "a blob"
    And tagged by "Ada Lovelace <ada@example.com>" at epoch 1780842645 +0200
    When I render the tag page
    Then the result contains ">blob<"
    And the result contains "/r/blob"

  Scenario: a tree tag links the tree by its tree kind
    Given a tree tag "treetag" pointing at "789abc" at "/r/tree" with message "a tree"
    And tagged by "Ada Lovelace <ada@example.com>" at epoch 1780842645 +0200
    When I render the tag page
    Then the result contains ">tree<"
    And the result contains "/r/tree"

  Scenario: a tag with no tagger omits the tagger row
    Given a commit tag "anon" pointing at "abc123" at "/r/obj" with message "unsigned"
    When I render the tag page
    Then the result contains ">anon<"
    And the result does not contain "tagger"

  Scenario: a multi-line message renders one line per message line
    Given a commit tag "notes" pointing at "abc123" at "/r/obj" with a two-line message
    And tagged by "Ada Lovelace <ada@example.com>" at epoch 1780842645 +0200
    When I render the tag page
    Then the result contains "First line"
    And the result contains "Second line"
    And the result contains "<br"
