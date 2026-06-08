Feature: Byte-faithful gitweb URLs for the feeds

  gitweb's format-stable feeds embed absolute links built by href(-full => 1):
  rooted at the site URL, parameters in @cgi_param_mapping order, joined with ';'
  and esc_param-encoded. self_url() is the same but for the request itself, which
  CGI.pm renders with no slash before the query. These must match gitweb to the
  byte, so the syndication output is differentially conformant.

  Scenario: no parameters is the bare site URL
    When I build a full URL at "http://localhost" with no params
    Then the URL is "http://localhost/"

  Scenario: one parameter
    When I build a full URL at "http://localhost" with params "p=repo.git"
    Then the URL is "http://localhost/?p=repo.git"

  Scenario: parameters are emitted in gitweb's canonical order, not call order
    When I build a full URL at "http://localhost" with params "hb=BASE h=TO f=foo.txt a=blobdiff p=repo.git fp=foo.txt hp=FROM"
    Then the URL is "http://localhost/?p=repo.git;a=blobdiff;f=foo.txt;fp=foo.txt;h=TO;hp=FROM;hb=BASE"

  Scenario: a value's space becomes + and reserved characters are percent-encoded
    When I build a full URL at "http://localhost" with param "s" set to "a b&c"
    Then the URL is "http://localhost/?s=a+b%26c"

  Scenario: self_url renders the host with no slash before the query
    When I build a self URL at "http://localhost" with params "p=repo.git a=atom"
    Then the URL is "http://localhost?p=repo.git;a=atom"
