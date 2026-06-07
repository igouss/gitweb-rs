Feature: Raw-HTML safe sink
  The render layer escapes by default and only emits raw HTML through one
  explicit, documented sink. This protects gitweb's deliberate raw-HTML spots
  (README.html, the control-character spans from the esc_* family) from being
  double-escaped, while keeping untrusted git-derived text un-injectable.

  Scenario: untrusted text is auto-escaped on the default path
    # Anything interpolated into a template without the safe sink is escaped,
    # so git-derived text can never break out of its HTML context.
    Given the text "<script>x</script>&"foo"
    When I render it as untrusted template content
    Then the result is "&lt;script&gt;x&lt;/script&gt;&amp;&quot;foo"

  Scenario: trusted raw HTML passes through the safe sink unchanged
    Given the text "<b class="x">bold &amp; raw</b>"
    When I render it through the raw-HTML safe sink
    Then the result is "<b class="x">bold &amp; raw</b>"

  Scenario: gitweb-escaped text is not double-escaped by the safe sink
    # esc_html already produced safe HTML (a visible control-char span); the
    # sink must emit it verbatim, not turn its tags back into entities.
    Given a NUL character
    When I escape it for HTML and render it through the raw-HTML safe sink
    Then the result is:
      """
      <span class="cntrl">\0</span>
      """

  Scenario: routing escaped text back through the default path double-escapes it
    # The contrast case: this is the footgun the sink exists to avoid. Escaped
    # HTML on the default (auto-escaping) path comes out double-escaped.
    Given a NUL character
    When I escape it for HTML and render it as untrusted template content
    Then the result is:
      """
      &lt;span class=&quot;cntrl&quot;&gt;\0&lt;/span&gt;
      """
