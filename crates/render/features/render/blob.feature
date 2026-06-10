Feature: Rendering one file (blob)
  The blob body (modernized git_blob) shows a text file as numbered, anchorable
  lines; an image inline as <img>; and any other binary as a raw download link
  only. An empty text file shows a short note instead of an empty table. URLs are
  built by the web boundary, so this layer takes finished hrefs.

  Scenario: a text blob numbers and anchors each line
    Given a blob line 1 reading "fn main() {}"
    And a blob line 2 reading "let x = 1;"
    When I render the blob content
    Then the result contains "id="l1""
    And the result contains "#l1"
    And the result contains "fn main() {}"
    And the result contains "id="l2""
    And the result contains "let x = 1;"

  Scenario: an empty text blob shows a note, not an empty table
    When I render the blob content
    Then the result contains "Empty file."
    And the result does not contain "<table"

  Scenario: an image blob is shown inline as an img
    Given the blob is an image at "/r/HEAD/logo.png/raw" described as "logo.png"
    When I render the blob content
    Then the result contains "<img"
    And the result contains "/r/HEAD/logo.png/raw"
    And the result contains "logo.png"
    And the result does not contain "<table"

  Scenario: a binary blob offers a raw download only
    Given the blob is a binary download at "/r/HEAD/data.bin/raw"
    When I render the blob content
    Then the result contains "/r/HEAD/data.bin/raw"
    And the result contains "Download raw"
    And the result does not contain "<table"

  Scenario: the blob page body wraps the content in the page chrome
    Given a blob line 1 reading "fn main() {}"
    When I render the blob page
    Then the result contains "class="page-header""
    And the result contains "readme.txt"
    And the result contains "fn main() {}"
    And the result contains "class="page-footer""
