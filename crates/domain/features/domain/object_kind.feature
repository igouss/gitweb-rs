Feature: Object kind names
  Every git object is one of four kinds. The lowercase type names git reports —
  and gitweb matches in parse_tag's "type (.+)" — map to a kind; anything else
  maps to nothing.

  Scenario Outline: git type names map to a kind
    Given an object type "<name>"
    When I read the object kind
    Then the object kind is "<kind>"

    Examples:
      | name   | kind   |
      | commit | Commit |
      | tree   | Tree   |
      | blob   | Blob   |
      | tag    | Tag    |

  Scenario: An unknown type name has no kind
    Given an object type "wibble"
    When I read the object kind
    Then there is no object kind
