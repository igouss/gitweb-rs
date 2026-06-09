Feature: Search-help topic gating
  gitweb's search-help page (git_search_help) documents each search type in a
  fixed list. Three types are always documented — commit, author, committer —
  because they need no extra feature. The grep type is documented only when the
  grep feature is enabled, and the pickaxe type only when the pickaxe feature is
  enabled, since each documents a search the user could not otherwise run. The
  order gitweb prints them is fixed: commit, grep, author, committer, pickaxe.

  Scenario: neither grep nor pickaxe enabled lists only the always-available types
    When the search help topics are listed with grep off and pickaxe off
    Then the topics are "commit, author, committer"

  Scenario: grep enabled documents the grep type right after commit
    When the search help topics are listed with grep on and pickaxe off
    Then the topics are "commit, grep, author, committer"

  Scenario: pickaxe enabled documents the pickaxe type last
    When the search help topics are listed with grep off and pickaxe on
    Then the topics are "commit, author, committer, pickaxe"

  Scenario: both enabled lists every type in gitweb's order
    When the search help topics are listed with grep on and pickaxe on
    Then the topics are "commit, grep, author, committer, pickaxe"
