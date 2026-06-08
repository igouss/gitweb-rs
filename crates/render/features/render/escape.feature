Feature: Escaping for safe HTML and URLs
  The render layer never emits git-derived text into HTML or a URL raw. It
  mirrors gitweb's esc_html / esc_path / esc_url / esc_param family so that
  metacharacters cannot break out of their context and control characters are
  made visible instead of corrupting the page.

  # ---------------------------------------------------------------------------
  # esc_html: HTML text and attribute escaping
  # ---------------------------------------------------------------------------

  Scenario: Empty text escapes to empty text
    Given the text ""
    When I escape it for HTML
    Then the result is ""

  Scenario: Text with no metacharacters is left untouched
    Given the text "hello world"
    When I escape it for HTML
    Then the result is "hello world"

  Scenario: An ampersand becomes an entity
    Given the text "a&b"
    When I escape it for HTML
    Then the result is "a&amp;b"

  Scenario: A less-than sign becomes an entity
    Given the text "a<b"
    When I escape it for HTML
    Then the result is "a&lt;b"

  Scenario: A greater-than sign becomes an entity
    Given the text "a>b"
    When I escape it for HTML
    Then the result is "a&gt;b"

  Scenario: An apostrophe becomes a numeric entity
    Given the text "it's"
    When I escape it for HTML
    Then the result is "it&#39;s"

  Scenario: A double quote becomes an entity
    Given a double-quote character
    When I escape it for HTML
    Then the result is "&quot;"

  Scenario: A script tag in untrusted text cannot break out
    Given the text "<script>alert(1)</script>"
    When I escape it for HTML
    Then the result is "&lt;script&gt;alert(1)&lt;/script&gt;"

  Scenario: A tab is preserved literally in HTML text
    Given a tab character
    When I escape it for HTML
    Then the result is a single tab character

  Scenario: A newline is shown as a visible control escape
    Given a newline character
    When I escape it for HTML
    Then the result is:
      """
      <span class="cntrl">\n</span>
      """

  Scenario: A bell character is shown by its named escape
    Given a bell character
    When I escape it for HTML
    Then the result is:
      """
      <span class="cntrl">\a</span>
      """

  Scenario: A NUL character is shown by its named escape
    Given a NUL character
    When I escape it for HTML
    Then the result is:
      """
      <span class="cntrl">\0</span>
      """

  Scenario: An unnamed control character is shown as space-padded hex
    Given the control byte 0x01
    When I escape it for HTML
    Then the result is:
      """
      <span class="cntrl">\ 1</span>
      """

  # ---------------------------------------------------------------------------
  # esc_html_nbsp: significant whitespace survives
  # ---------------------------------------------------------------------------

  Scenario: Spaces become non-breaking spaces when whitespace is significant
    Given the text "a  b"
    When I escape it for HTML keeping whitespace
    Then the result is "a&nbsp;&nbsp;b"

  # ---------------------------------------------------------------------------
  # esc_path: like esc_html, but tab is also made visible
  # ---------------------------------------------------------------------------

  Scenario: A path with no metacharacters is left untouched
    Given the text "src/main.rs"
    When I escape it as a path
    Then the result is "src/main.rs"

  Scenario: A tab in a path is shown as a visible control escape
    Given a tab character
    When I escape it as a path
    Then the result is:
      """
      <span class="cntrl">\t</span>
      """

  # ---------------------------------------------------------------------------
  # esc_attr: identical to esc_html
  # ---------------------------------------------------------------------------

  Scenario: An attribute value has its double quotes neutralised
    Given a double-quote character
    When I escape it for an HTML attribute
    Then the result is "&quot;"

  # ---------------------------------------------------------------------------
  # esc_url: percent-encode a whole URL, space becomes plus
  # ---------------------------------------------------------------------------

  Scenario: A plain path keeps its slashes
    Given the text "/projects/foo.git"
    When I escape it for a URL
    Then the result is "/projects/foo.git"

  Scenario: URL-structural characters are kept literal
    Given the text "a;b:c@d&e=f?g"
    When I escape it for a URL
    Then the result is "a;b:c@d&e=f?g"

  Scenario: A space in a URL becomes a plus
    Given the text "a b"
    When I escape it for a URL
    Then the result is "a+b"

  Scenario: An unsafe character in a URL is percent-encoded with upper-case hex
    Given the text "a#b"
    When I escape it for a URL
    Then the result is "a%23b"

  Scenario: A non-ASCII character in a URL is percent-encoded per UTF-8 byte
    Given the text "é"
    When I escape it for a URL
    Then the result is "%C3%A9"

  # ---------------------------------------------------------------------------
  # esc_param: a single component, space becomes plus, & is encoded
  # ---------------------------------------------------------------------------

  Scenario: A space in a parameter becomes a plus
    Given the text "a b"
    When I escape it as a URL parameter
    Then the result is "a+b"

  Scenario: An ampersand in a parameter is percent-encoded
    Given the text "a&b"
    When I escape it as a URL parameter
    Then the result is "a%26b"

  Scenario: Path-like component characters are kept literal in a parameter
    Given the text "a/b:c@d"
    When I escape it as a URL parameter
    Then the result is "a/b:c@d"

  # ---------------------------------------------------------------------------
  # esc_index_field: project_index quoting — keep the slash, space becomes plus
  # ---------------------------------------------------------------------------

  Scenario: A project path keeps its slashes and dots in an index field
    Given the text "lib/sub/foo.git"
    When I quote it as a project index field
    Then the result is "lib/sub/foo.git"

  Scenario: A space in an index field becomes a plus
    Given the text "Ada Lovelace"
    When I quote it as a project index field
    Then the result is "Ada+Lovelace"

  Scenario: An unsafe character in an index field is percent-encoded with upper-case hex
    Given the text "a&b"
    When I quote it as a project index field
    Then the result is "a%26b"

  Scenario: A non-ASCII character in an index field is percent-encoded per UTF-8 byte
    Given the text "café"
    When I quote it as a project index field
    Then the result is "caf%C3%A9"

  # ---------------------------------------------------------------------------
  # esc_path_info: space and plus stay literal, ? is escaped
  # ---------------------------------------------------------------------------

  Scenario: A space stays literal in path info
    Given the text "a b"
    When I escape it as path info
    Then the result is "a b"

  Scenario: A plus stays literal in path info
    Given the text "a+b"
    When I escape it as path info
    Then the result is "a+b"

  Scenario: A question mark is escaped in path info
    Given the text "a?b"
    When I escape it as path info
    Then the result is "a%3Fb"
