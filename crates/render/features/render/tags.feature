Feature: Rendering the tags table
  The tags table (modernized git_tags / git_tags_body) shows one row per tag: its
  creation age coloured by recency (or "unknown"), the tag name linking to the
  tagged object via that object's kind, the annotation subject (chopped, linking
  to the single tag view) for annotated tags only, a "tag" selflink for annotated
  tags only, and reftype-dependent links — shortlog / log for a commit, raw for a
  blob, nothing more for a tree. URLs are built by the web boundary, so this layer
  takes finished hrefs and only decides layout, escaping, chopping, and the
  recency CSS hook.

  Scenario: a commit tag row shows its age, name link, and commit quick links
    Given an annotated commit tag "v1.0" at "/r/v1" subject "Release 1.0" aged 600
    When I render the tags table
    Then the result contains "10 min ago"
    And the result contains ">v1.0<"
    And the result contains "/r/v1"
    And the result contains "commit"
    And the result contains "/r/v1/shortlog"
    And the result contains "/r/v1/log"

  Scenario: the age cell carries a recency class
    Given an annotated commit tag "v1.0" at "/r/v1" subject "Release 1.0" aged 600
    When I render the tags table
    Then the result contains "age-fresh"

  Scenario: an annotated tag links its subject and offers a tag selflink
    Given an annotated commit tag "v1.0" at "/r/v1" subject "Release 1.0" aged 600
    When I render the tags table
    Then the result contains "Release 1.0"
    And the result contains "list subject"
    And the result contains "/r/v1/tag"
    And the result contains ">tag<"

  Scenario: a lightweight tag has no subject and no tag selflink
    Given a lightweight commit tag "rc1" at "/r/rc1" aged 600
    When I render the tags table
    Then the result contains ">rc1<"
    And the result does not contain "list subject"
    And the result does not contain ">tag<"

  Scenario: a blob tag links the blob and offers a raw link
    Given an annotated blob tag "blobtag" at "/r/bt" subject "a blob" aged 600
    When I render the tags table
    Then the result contains "blob"
    And the result contains "/r/bt/raw"
    And the result does not contain "shortlog"

  Scenario: a tree tag offers no commit or raw links
    Given an annotated tree tag "treetag" at "/r/tt" subject "a tree" aged 600
    When I render the tags table
    Then the result contains "tree"
    And the result does not contain "shortlog"
    And the result does not contain "/r/tt/raw"

  Scenario: a tag with no creation time shows an unknown age
    Given an annotated commit tag "v1.0" at "/r/v1" subject "Release 1.0" with an unknown age
    When I render the tags table
    Then the result contains "unknown"

  Scenario: a long subject is chopped and keeps the full text as a title
    Given an annotated commit tag "v1.0" at "/r/v1" subject "This is a very long tag annotation subject that runs well past the limit" aged 600
    When I render the tags table
    Then the result contains "title="
    And the result contains "..."

  Scenario: every tag gets its own row
    Given an annotated commit tag "v1.0" at "/r/v1" subject "one" aged 600
    And a lightweight commit tag "rc1" at "/r/rc1" aged 600
    When I render the tags table
    Then the result contains ">v1.0<"
    And the result contains ">rc1<"

  Scenario: a capped list gets a trailing "more" row linking to the full tags
    Given a lightweight commit tag "rc1" at "/r/rc1" aged 600
    And the tags offer more at "/r/tags" labelled "..."
    When I render the tags table
    Then the result contains "/r/tags"
    And the result contains "..."

  Scenario: an uncapped list has no "more" row
    Given a lightweight commit tag "rc1" at "/r/rc1" aged 600
    When I render the tags table
    Then the result does not contain "more"

  Scenario: an empty tag list shows a note
    When I render the tags page with no tags
    Then the result contains "No tags."
