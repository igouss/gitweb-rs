Feature: Rendering the single annotated-tag view
  The single-tag view (modernized git_tag) shows the tag's name as a heading, an
  object-header table whose object row links the tagged object — by its id and by
  its kind label (commit / blob / tree) — to that object's view, a tagger row
  with the tagger's name, email and recency-coloured age when the tag carries a
  tagger, and the message body with one line per message line. URLs are built by
  the web boundary, so this layer takes a finished object href and only decides
  layout, escaping, and the age hook.

  Scenario: a commit tag shows its name, object link and kind, tagger and message
    Given a commit tag "v1.0" pointing at "abc123" at "/r/obj" tagged by "Ada Lovelace <ada@example.com>" aged 600 with message "Release 1.0"
    When I render the tag page
    Then the result contains ">v1.0<"
    And the result contains "abc123"
    And the result contains "/r/obj"
    And the result contains ">commit<"
    And the result contains "Ada Lovelace"
    And the result contains "ada@example.com"
    And the result contains "10 min ago"
    And the result contains "Release 1.0"

  Scenario: the tagger age carries a recency class
    Given a commit tag "v1.0" pointing at "abc123" at "/r/obj" tagged by "Ada Lovelace <ada@example.com>" aged 600 with message "Release 1.0"
    When I render the tag page
    Then the result contains "age-fresh"

  Scenario: a blob tag links the blob by its blob kind
    Given a blob tag "blobtag" pointing at "def456" at "/r/blob" tagged by "Ada Lovelace <ada@example.com>" aged 600 with message "a blob"
    When I render the tag page
    Then the result contains ">blob<"
    And the result contains "/r/blob"

  Scenario: a tree tag links the tree by its tree kind
    Given a tree tag "treetag" pointing at "789abc" at "/r/tree" tagged by "Ada Lovelace <ada@example.com>" aged 600 with message "a tree"
    When I render the tag page
    Then the result contains ">tree<"
    And the result contains "/r/tree"

  Scenario: a tag with no tagger omits the tagger row
    Given a commit tag "anon" pointing at "abc123" at "/r/obj" with no tagger and message "unsigned"
    When I render the tag page
    Then the result contains ">anon<"
    And the result does not contain "tagger"

  Scenario: a multi-line message renders one line per message line
    Given a commit tag "notes" pointing at "abc123" at "/r/obj" tagged by "Ada Lovelace <ada@example.com>" aged 600 with a two-line message
    When I render the tag page
    Then the result contains "First line"
    And the result contains "Second line"
    And the result contains "<br"
