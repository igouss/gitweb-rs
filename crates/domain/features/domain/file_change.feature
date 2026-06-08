Feature: Changed-file status note
  Each row of the commit / commitdiff changed-files table carries a status
  note describing what happened to the path, mirroring the spans
  git_difftree_body emits: a created path's new type and mode, a deleted
  path's old type, a modification's type and mode change, and a rename or
  copy's similarity. The path links are the boundary's job; this rule owns
  only the facts each status implies.

  Scenario Outline: A created path notes its new type, with a mode for regular files
    Given a change with status "added", from mode "000000", to mode "<to>"
    When I derive the file-change note
    Then the note category is "new"
    And the note file type is "<type>"
    And the note mode is "<mode>"
    And the note text is "<text>"

    Examples:
      | to     | type      | mode | text                   |
      | 100644 | file      | 0644 | new file with mode: 0644 |
      | 100755 | file      | 0755 | new file with mode: 0755 |
      | 120000 | symlink   | none | new symlink            |
      | 160000 | submodule | none | new submodule          |

  Scenario: A deleted path notes its old type
    Given a change with status "deleted", from mode "120000", to mode "000000"
    When I derive the file-change note
    Then the note category is "deleted"
    And the note file type is "symlink"
    And the note text is "deleted symlink"

  Scenario: A pure content modification has no note
    Given a change with status "modified", from mode "100644", to mode "100644"
    When I derive the file-change note
    Then there is no note

  Scenario Outline: A modification notes the type and mode change
    Given a change with status "modified", from mode "<from>", to mode "<to>"
    When I derive the file-change note
    Then the note category is "changed"
    And the note text is "<text>"

    Examples:
      | from   | to     | text                                  |
      | 100644 | 100755 | changed mode: 0644->0755              |
      | 100755 | 100644 | changed mode: 0755->0644              |
      | 100644 | 120000 | changed from file to symlink          |
      | 120000 | 100644 | changed from symlink to file mode: 0644 |
      | 100644 | 160000 | changed from file to submodule        |

  Scenario Outline: A rename or copy notes its similarity and any mode change
    Given a change with status "<status>" similarity <sim>, from mode "<from>", to mode "<to>"
    When I derive the file-change note
    Then the note category is "<category>"
    And the note similarity is <sim>
    And the note mode is "<mode>"

    Examples:
      | status  | sim | from   | to     | category | mode |
      | renamed | 100 | 100644 | 100644 | moved    | none |
      | renamed | 90  | 100644 | 100755 | moved    | 0755 |
      | copied  | 75  | 100644 | 100644 | copied   | none |
      | copied  | 80  | 100644 | 120000 | copied   | 0000 |
