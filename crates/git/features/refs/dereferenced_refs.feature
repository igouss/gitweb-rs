Feature: Dereferenced references for ref badges
  The gix adapter resolves every ref to the object it ultimately names, the way
  git show-ref --dereference does, so format_ref_marker can badge a commit row
  with the refs whose tip is that commit. A branch head or a lightweight tag
  points straight at the commit and is reported direct; an annotated tag points
  at a tag object that peels to the commit and is reported indirect.

  Background:
    Given a repository whose commit carries two branches, a lightweight tag and an annotated tag

  Scenario: A branch head points straight at its commit
    When I read the dereferenced references
    Then the ref "refs/heads/topic" targets the commit and is direct

  Scenario: A lightweight tag points straight at its commit
    When I read the dereferenced references
    Then the ref "refs/tags/light" targets the commit and is direct

  Scenario: An annotated tag is peeled to its commit and flagged indirect
    When I read the dereferenced references
    Then the ref "refs/tags/annot" targets the commit and is indirect
    And the ref "refs/tags/annot" does not target the tag object

  Scenario: The references come out in name order
    When I read the dereferenced references
    Then the dereferenced references are "refs/heads/main, refs/heads/topic, refs/tags/annot, refs/tags/light"
