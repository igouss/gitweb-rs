Feature: Git object identifiers
  A git object id is a hexadecimal hash: 40 characters for SHA-1 or 64 for
  SHA-256, mirroring gitweb's $oid_regex. Anything else is not an object id.

  In the UI a full id is abbreviated to its leading characters; gitweb shows
  seven by default (its substr($hash, 0, 7)).

  Scenario Outline: Well-formed hashes are valid object ids
    Given an object id "<hex>"
    When I parse the object id
    Then the object id is valid

    Examples:
      | hex                                                              |
      | 1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131c                         |
      | 0000000000000000000000000000000000000000                         |
      | 1C002DD4B5DC01C595E6C6EC9CCDA4F6F69C131C                         |
      | 1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131c0dd945f8c1234567890abcde |

  Scenario Outline: Malformed strings are not object ids
    Given an object id "<hex>"
    When I parse the object id
    Then the object id is invalid

    Examples:
      | hex                                       |
      |                                           |
      | 1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131  |
      | 1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131cz |
      | refs/heads/main                           |

  Scenario: A full hash abbreviates to seven characters by default
    Given an object id "1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131c"
    When I parse the object id
    Then the abbreviation is "1c002dd"

  Scenario: An abbreviation length can be requested
    Given an object id "1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131c"
    When I parse the object id
    Then the abbreviation to 12 is "1c002dd4b5dc"

  Scenario: Requesting more characters than the hash has yields the full hash
    Given an object id "1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131c"
    When I parse the object id
    Then the abbreviation to 99 is "1c002dd4b5dc01c595e6c6ec9ccda4f6f69c131c"
