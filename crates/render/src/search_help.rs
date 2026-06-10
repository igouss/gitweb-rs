//! The search-help page (modernized `git_search_help`).
//!
//! gitweb's search-help page is pure help prose: an intro paragraph explaining
//! how patterns are matched (precise and case-insensitive by default, a POSIX
//! extended regular expression when the `re` box is checked) followed by a
//! definition list documenting each search type. Which types appear is the
//! domain's [`help_topics`](gitweb_domain::model::search_help::help_topics) rule;
//! the prose for each — gitweb's exact descriptive text, the page's information —
//! is this view's copy, laid out as a `<dl>` and escaped by maud.
//!
//! The boundary supplies the breadcrumbs and the per-project action navigation
//! (it owns the URLs); this lays out the chrome around the help body.

use gitweb_domain::model::search_help::SearchHelpTopic;

use crate::chrome::{Crumb, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};

/// The search-help page: the breadcrumb header, the per-project action
/// navigation, and the search types to document (in gitweb's fixed order).
#[derive(Debug, Clone)]
pub struct SearchHelpPage {
    /// Breadcrumb trail (home / project / search help).
    pub crumbs: Vec<Crumb>,
    /// The per-project action bar; on this page nothing is the current view, so
    /// every item links.
    pub nav: Vec<NavItem>,
    /// The search types to document, already filtered by the site's feature
    /// configuration.
    pub topics: Vec<SearchHelpTopic>,
}

/// Renders the search-help page body: the breadcrumb header, the action nav, the
/// pattern-matching intro, and the per-type definition list, then the footer.
/// The caller wraps this in the document (`gitweb_render::chrome::document`).
#[must_use]
pub fn search_help_body(page: &SearchHelpPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            p class="search-help-intro" {
                strong { "Pattern" }
                " is by default a normal string that is matched precisely (but without "
                "regard to case, except in the case of pickaxe). However, when you check the "
                em { "re" }
                " checkbox, the pattern entered is recognized as the POSIX extended "
                a href="https://en.wikipedia.org/wiki/Regular_expression" { "regular expression" }
                " (also case insensitive)."
            }
            dl class="search-help" {
                @for topic in &page.topics {
                    dt { (topic.name()) }
                    dd { (describe(*topic)) }
                }
            }
        }
    }
}

/// gitweb's exact descriptive prose for each search type (the page's
/// information). The matched string in the pickaxe text is quoted with literal
/// double quotes, which maud escapes for HTML.
fn describe(topic: SearchHelpTopic) -> &'static str {
    match topic {
        SearchHelpTopic::Commit => {
            "The commit messages and authorship information will be scanned for the given pattern."
        }
        SearchHelpTopic::Grep => {
            "All files in the currently selected tree (HEAD unless you are explicitly browsing a \
             different one) are searched for the given pattern. On large trees, this search can \
             take a while and put some strain on the server, so please use it with some \
             consideration. Note that due to git-grep peculiarity, currently if regexp mode is \
             turned off, the matches are case-sensitive."
        }
        SearchHelpTopic::Author => {
            "Name and e-mail of the change author and date of birth of the patch will be scanned \
             for the given pattern."
        }
        SearchHelpTopic::Committer => {
            "Name and e-mail of the committer and date of commit will be scanned for the given \
             pattern."
        }
        SearchHelpTopic::Pickaxe => {
            "All commits that caused the string to appear or disappear from any file (changes that \
             added, removed or \"modified\" the string) will be listed. This search can take a \
             while and takes a lot of strain on the server, so please use it wisely. Note that \
             since you may be interested even in changes just changing the case as well, this \
             search is case sensitive."
        }
    }
}
