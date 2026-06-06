Feature: Shortening ref names for display
  A fully-qualified ref like "refs/heads/main" is shown to humans without
  its namespace prefix, mirroring how gitweb strips refs/heads, refs/tags
  and refs/remotes. A ref in any other namespace keeps its short name but is
  annotated with the namespace so it stays unambiguous.

  Scenario Outline: Branch, tag and remote namespaces are stripped clean
    Given the full ref "<full>"
    When I shorten the ref
    Then the short ref is "<short>"

    Examples:
      | full                      | short       |
      | refs/heads/main           | main        |
      | refs/tags/v1.0            | v1.0        |
      | refs/remotes/origin/main  | origin/main |

  Scenario: An unusual namespace is annotated rather than hidden
    Given the full ref "refs/notes/commits"
    When I shorten the ref
    Then the short ref is "commits (notes)"

  Scenario: A name with no refs/ prefix is left as-is
    Given the full ref "main"
    When I shorten the ref
    Then the short ref is "main"

  Scenario: A single-segment ref under refs/ is left as-is
    Given the full ref "refs/stash"
    When I shorten the ref
    Then the short ref is "refs/stash"
