Feature: Site chrome
  The modernized page furniture every view sits inside: the HTML5 document
  skeleton, the header (logo + breadcrumbs), the search form, the per-page action
  navigation, and the footer. Each renders a finished view-model, builds no URLs,
  and escapes git-derived text by default. Verified behaviourally (what links and
  information appear), since the markup is modernized rather than byte-identical.

  # ---- document skeleton ----------------------------------------------------

  Scenario: the document is modern HTML5 carrying the title, stylesheet and body
    When I render a document titled "Tom & Jerry" with stylesheet "/static/style.css" and body "<main>hi</main>"
    Then the result contains "<!DOCTYPE html>"
    And the result contains "<html lang="en">"
    And the result contains "<meta charset="utf-8">"
    And the result contains "<meta name="viewport" content="width=device-width, initial-scale=1">"
    And the result contains "<meta name="robots" content="index, nofollow">"
    And the result contains "<title>Tom &amp; Jerry</title>"
    And the result contains "<link rel="stylesheet" href="/static/style.css">"
    And the result contains "<main>hi</main>"

  Scenario: a document without a favicon omits the icon link
    When I render a document titled "x" with stylesheet "/s" and body "y"
    Then the result does not contain "rel="icon""

  Scenario: a document with a favicon links it
    When I render a document with a favicon "/favicon.svg"
    Then the result contains "<link rel="icon" href="/favicon.svg">"

  Scenario: a document advertises syndication feeds in the head
    When I render a document with an RSS feed "/?p=r;a=rss" titled "r - log - RSS feed"
    Then the result contains "<link rel="alternate" type="application/rss+xml" title="r - log - RSS feed" href="/?p=r;a=rss">"

  # ---- client-side script wiring (gitweb's javascript-* features) -----------
  # gitweb appends its <script> tags at the end of <body>; we modernize: each is
  # an ES module loaded in the <head> (type="module" defers until the document
  # parses, so the DOM exists when it runs). The behaviour preserved is *which*
  # scripts a page carries — gated by the javascript-timezone / javascript-actions
  # features at the boundary — not where in the document they sit. The in-browser
  # effect of each module (timezone conversion, action-link rewiring, progressive
  # blame fill) is a documented manual check, not a headless assertion.

  Scenario: a document with no client scripts emits no module script tags
    When I render a document titled "x" with stylesheet "/s" and body "y"
    Then the result does not contain "<script"

  Scenario: a document wires each client script as a deferred ES module in the head
    Given a client script at "/static/gitweb-timezone.js"
    And a client script at "/static/gitweb-actions.js"
    When I render a document titled "x" with stylesheet "/s" and body "y"
    Then the result contains "<script type="module" src="/static/gitweb-timezone.js" defer></script>"
    And the result contains "<script type="module" src="/static/gitweb-actions.js" defer></script>"

  Scenario: the document closes the body with the footer chrome below the content
    Given a footer link "RSS" to "/?p=r;a=rss" titled "log RSS feed"
    When I render a document titled "x" with stylesheet "/s.css" and body "<main>hi</main>"
    Then the result contains "<main>hi</main>"
    And the result contains "<footer class="page-footer">"
    And the result contains "<a class="feed" href="/?p=r;a=rss" title="log RSS feed">RSS</a>"

  # ---- breadcrumbs ----------------------------------------------------------

  Scenario: an empty breadcrumb trail renders the nav element only
    When I render the breadcrumbs
    Then the result contains "<nav class="breadcrumbs">"
    And the result does not contain "<a "
    And the result does not contain "<span class="sep">"

  Scenario: a single current crumb is plain text, not a link
    Given a current breadcrumb "main"
    When I render the breadcrumbs
    Then the result contains "<span class="current">main</span>"
    And the result does not contain "<a "
    And the result does not contain "<span class="sep">"

  Scenario: many crumbs are linked and separated, with the last current
    Given a breadcrumb link "projects" to "/?a=project_list"
    And a breadcrumb link "myrepo" to "/?p=myrepo;a=summary"
    And a current breadcrumb "shortlog"
    When I render the breadcrumbs
    Then the result contains "<a href="/?a=project_list">projects</a>"
    And the result contains "<a href="/?p=myrepo;a=summary">myrepo</a>"
    And the result contains "<span class="current">shortlog</span>"
    And the result contains "<span class="sep">/</span>"

  Scenario: crumb labels are escaped
    Given a current breadcrumb "a & b"
    When I render the breadcrumbs
    Then the result contains "a &amp; b"

  # ---- page header ----------------------------------------------------------

  Scenario: the page header wraps the breadcrumbs and shows no logo by default
    Given a current breadcrumb "main"
    When I render the page header without a logo
    Then the result contains "<header class="page-header">"
    And the result contains "<span class="current">main</span>"
    And the result does not contain "<img"

  Scenario: the page header shows the logo when one is given
    Given a current breadcrumb "main"
    When I render the page header with a logo linking "/" image "/logo.svg" labelled "git"
    Then the result contains "class="logo""
    And the result contains "<img src="/logo.svg" alt="git">"

  # ---- search form ----------------------------------------------------------

  Scenario: the search form carries query, hidden params, type and regexp
    When I render the standard search form with query "fix bug" type "grep" regexp on
    Then the result contains "<form class="search" method="get" action="/myrepo/search">"
    And the result contains "<input type="hidden" name="p" value="myrepo">"
    And the result contains "<input type="hidden" name="a" value="search">"
    And the result contains "<option value="grep" selected>"
    And the result contains "name="s" value="fix bug""
    And the result contains "name="sr""
    And the result contains "checked"
    And the result contains "search help"

  Scenario: an off regexp toggle and unselected types are not marked
    When I render the standard search form with query "" type "commit" regexp off
    Then the result does not contain "checked"
    And the result contains "<option value="commit" selected>"
    And the result does not contain "<option value="grep" selected>"

  # ---- page navigation ------------------------------------------------------

  Scenario: an empty action bar renders the nav element only
    When I render the page navigation
    Then the result contains "<nav class="page-nav">"
    And the result does not contain "<a "

  Scenario: a single current action is plain text
    Given a navigation item "summary" current
    When I render the page navigation
    Then the result contains "<span class="current">summary</span>"
    And the result does not contain "<a "
    And the result does not contain "<span class="sep">"

  Scenario: many actions are separated, current as text and the rest linked
    Given a navigation link "summary" to "/?p=r;a=summary"
    And a navigation item "shortlog" current
    And a navigation link "log" to "/?p=r;a=log"
    When I render the page navigation
    Then the result contains "<a href="/?p=r;a=summary">summary</a>"
    And the result contains "<span class="current">shortlog</span>"
    And the result contains "<a href="/?p=r;a=log">log</a>"
    And the result contains "<span class="sep">|</span>"

  Scenario: the action bar carries a trailing extra
    Given a navigation item "log" current
    When I render the page navigation with extra "<span class="pager">next</span>"
    Then the result contains "<div class="nav-extra"><span class="pager">next</span></div>"

  # ---- footer ---------------------------------------------------------------

  Scenario: the footer shows the description and links
    Given a footer link "RSS" to "/?p=r;a=rss" titled "log RSS feed"
    And a footer link "Atom" to "/?p=r;a=atom" titled "log Atom feed"
    When I render the footer with description "A test repo & more"
    Then the result contains "<footer class="page-footer">"
    And the result contains "A test repo &amp; more"
    And the result contains "<a class="feed" href="/?p=r;a=rss" title="log RSS feed">RSS</a>"
    And the result contains "<a class="feed" href="/?p=r;a=atom" title="log Atom feed">Atom</a>"

  Scenario: the footer omits the description when there is none
    Given a footer link "OPML" to "/?a=opml" titled "OPML"
    When I render the footer without a description
    Then the result contains "<a class="feed" href="/?a=opml" title="OPML">OPML</a>"
    And the result does not contain "footer-desc"

  # ---- footer link assembly (git_footer_html) -------------------------------

  Scenario: a project footer links the RSS and Atom feeds, titled per the feed
    When I assemble the project footer titled "log" with rss "/?p=r;a=rss" and atom "/?p=r;a=atom"
    Then the result contains "<a class="feed" href="/?p=r;a=rss" title="log RSS feed">RSS</a>"
    And the result contains "<a class="feed" href="/?p=r;a=atom" title="log Atom feed">Atom</a>"

  Scenario: a branch-scoped project footer feed title names the branch
    When I assemble the project footer titled "log of topic" with rss "/?p=r;a=rss;h=refs/heads/topic" and atom "/?p=r;a=atom;h=refs/heads/topic"
    Then the result contains "title="log of topic RSS feed""
    And the result contains "title="log of topic Atom feed""

  Scenario: the projects-list footer links OPML then the plain-text index
    When I assemble the projects-list footer with opml "/?a=opml" and index "/?a=project_index"
    Then the result contains "<a class="feed" href="/?a=opml" title="OPML">OPML</a>"
    And the result contains "<a class="feed" href="/?a=project_index" title="TXT">TXT</a>"
