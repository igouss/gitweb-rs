//! The search form (modernized `print_search_form`).
//!
//! gitweb's form GETs back to itself with a search-type popup
//! (commit/grep/author/committer/pickaxe), a query field, and an
//! extended-regexp toggle, plus hidden params carrying the project and the
//! commit to search from. The set of search types and the hidden params are the
//! web/search layer's contract, so they arrive as data; this only lays out the
//! form. The CGI field names (`s`, `st`, `sr`) are gitweb's request ABI and are
//! preserved verbatim.

use crate::markup::{Markup, html};

/// One choice in the search-type popup.
#[derive(Debug, Clone)]
pub struct SearchOption {
    /// Submitted value (gitweb's `st`, e.g. `commit`, `grep`, `pickaxe`).
    pub value: String,
    /// Visible label.
    pub label: String,
    /// Whether this option is the current selection.
    pub selected: bool,
}

/// A hidden form field carried through the search request.
#[derive(Debug, Clone)]
pub struct HiddenField {
    /// Field name (gitweb ABI, e.g. `p`, `a`, `h`).
    pub name: String,
    /// Field value.
    pub value: String,
}

/// Everything needed to render the search form.
#[derive(Debug, Clone)]
pub struct SearchForm {
    /// Form `action` URL (where the GET goes).
    pub action: String,
    /// Hidden fields preserved across the request.
    pub hidden: Vec<HiddenField>,
    /// Search-type options for the popup.
    pub options: Vec<SearchOption>,
    /// Current query text.
    pub query: String,
    /// Whether the extended-regexp toggle is checked.
    pub use_regexp: bool,
    /// URL of the search-help page.
    pub help_href: String,
}

/// Renders the search form: hidden context fields, the search-type popup, the
/// query field, and the extended-regexp toggle.
#[must_use]
pub fn search_form(form: &SearchForm) -> Markup {
    html! {
        form class="search" method="get" action=(form.action) {
            @for field in &form.hidden {
                input type="hidden" name=(field.name) value=(field.value);
            }
            label {
                select name="st" {
                    @for option in &form.options {
                        option value=(option.value) selected[option.selected] { (option.label) }
                    }
                }
            }
            a class="search-help" href=(form.help_href) title="search help" { "?" }
            input type="search" name="s" value=(form.query);
            label class="regexp" {
                input type="checkbox" name="sr" value="1" checked[form.use_regexp];
                " re"
            }
        }
    }
}
