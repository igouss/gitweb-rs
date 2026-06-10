Feature: The branch-ref directory set (get_branch_refs)
  gitweb's get_branch_refs names the directories under refs/ it treats as
  branches: always 'heads', plus the validated entries of the 'extra-branch-refs'
  feature. filter_and_validate_refs runs over the configured entries first — it
  rejects a malformed ref (gitweb dies with a 500), drops a literal 'heads' (it is
  added back implicitly), and returns the survivors sorted and de-duplicated. The
  result is 'heads' followed by that set. Wiring this is what lets a snapshot of a
  ref under an extra branch directory be named with the directory prefix.

  Scenario: with no extra branch refs configured, only heads counts
    Given no extra branch refs
    When I resolve the branch refs
    Then the branch refs are "heads"

  Scenario: one extra branch ref is appended after heads
    Given the extra branch refs "sandbox"
    When I resolve the branch refs
    Then the branch refs are "heads, sandbox"

  Scenario Outline: many extra branch refs are validated, sorted, and de-duplicated
    Given the extra branch refs "<configured>"
    When I resolve the branch refs
    Then the branch refs are "<resolved>"

    Examples:
      | configured          | resolved            |
      | wip, sandbox        | heads, sandbox, wip |
      | wip, wip, sandbox   | heads, sandbox, wip |
      | heads, wip          | heads, wip          |
      | sandbox, heads, wip | heads, sandbox, wip |

  Scenario Outline: a malformed extra branch ref takes the site down with a 500
    Given the extra branch refs "<ref>"
    When I resolve the branch refs
    Then resolving the branch refs fails as "Invalid ref '<ref>' in 'extra-branch-refs' feature"

    Examples:
      | ref       |
      | bad ref   |
      | foo/.bar  |
      | a..b      |
      | trailing/ |
      | wip~1     |
      | wip^      |
      | wip:tip   |
      | wip?      |
      | wip*      |
      | wip[      |
