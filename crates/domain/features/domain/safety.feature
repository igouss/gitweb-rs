Feature: Input safety — pathname and refname validation
  Untrusted request parameters (project p=, file f=, ref/hash h=/hb=) are
  validated before any filesystem or repository access, mirroring gitweb's
  is_valid_pathname, is_valid_ref_format and is_valid_refname. This is the
  front line against directory traversal (../) and malformed-ref attacks: a
  value that fails here never reaches a tree, a ref, or the disk.

  Scenario Outline: Well-formed paths are accepted
    Given the candidate path "<path>"
    When I validate it as a path
    Then the path is accepted

    Examples:
      | path        |
      | README      |
      | src/main.rs |
      | ..foo       |
      | a file.txt  |

  Scenario Outline: Malformed or traversing paths are rejected
    Given the candidate path "<path>"
    When I validate it as a path
    Then the path is rejected

    Examples:
      | path          |
      |               |
      | .             |
      | ..            |
      | /etc/passwd   |
      | src/          |
      | a//b          |
      | ../etc/passwd |
      | src/../secret |
      | src/..        |
      | src/./x       |

  Scenario: A path containing a NUL byte is rejected
    Given a candidate path with a NUL byte
    When I validate it as a path
    Then the path is rejected

  Scenario Outline: Plain ref names and full object ids are accepted
    Given the candidate ref "<ref>"
    When I validate it as a ref
    Then the ref is accepted

    Examples:
      | ref                                      |
      | main                                     |
      | refs/heads/main                          |
      | feature/login                            |
      | 0123456789abcdef0123456789abcdef01234567 |

  Scenario: A full SHA-256 object id is accepted as a ref
    Given a candidate ref of 64 hex characters
    When I validate it as a ref
    Then the ref is accepted

  Scenario Outline: Refs violating git-check-ref-format are rejected
    Given the candidate ref "<ref>"
    When I validate it as a ref
    Then the ref is rejected

    Examples:
      | ref          |
      | with space   |
      | ti~lde       |
      | ca^ret       |
      | co:lon       |
      | que?stion    |
      | ast*erisk    |
      | brac[ket     |
      | double..dot  |
      | comp/.hidden |
      | trailing/    |
      | ..           |
