Feature: Git file modes
  A git tree entry's octal mode determines its type and the symbolic
  permission string shown in the tree view, mirroring gitweb's
  mode_str / file_type / file_type_long.

  git only records the executable bit for regular files, so the short
  type ("file") collapses executable and non-executable regulars, while
  the long type distinguishes "executable" from "file".

  Scenario Outline: A mode classifies into a short and a long type
    Given a file mode "<mode>"
    When I read the file mode
    Then the short type is "<short>"
    And the long type is "<long>"
    And the permission string is "<perm>"

    Examples:
      | mode   | short     | long       | perm       |
      | 160000 | submodule | submodule  | m--------- |
      | 040000 | directory | directory  | drwxr-xr-x |
      | 120000 | symlink   | symlink    | lrwxrwxrwx |
      | 100755 | file      | executable | -rwxr-xr-x |
      | 100644 | file      | file       | -rw-r--r-- |
      | 030000 | unknown   | unknown    | ---------- |

  Scenario Outline: A mode names the git object type of its tree entry
    Given a file mode "<mode>"
    When I read the file mode
    Then the mode's object kind is "<kind>"

    Examples:
      | mode   | kind   |
      | 040000 | tree   |
      | 160000 | commit |
      | 100644 | blob   |
      | 100755 | blob   |
      | 120000 | blob   |

  Scenario: A mode string that is not octal is rejected
    Given a file mode "100abc"
    When I read the file mode
    Then the file mode is invalid

  Scenario: An empty mode string is rejected
    Given a file mode ""
    When I read the file mode
    Then the file mode is invalid
