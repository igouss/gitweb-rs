//! Site chrome: the modernized, framework-free page furniture that every view
//! sits inside — the HTML5 document skeleton, the header (logo + breadcrumbs),
//! the search form, the per-page action navigation, and the footer.
//!
//! Each piece renders a finished view-model and builds no URLs of its own:
//! gitweb's `href()` lives at the web boundary, so the chrome's inputs are the
//! contract the boundary fills. Everything is escaped by default via maud; the
//! only raw HTML enters through [`crate::markup::raw`].

pub mod document;
pub mod feed_meta;
pub mod footer;
pub mod header;
pub mod nav;
pub mod search;

pub use document::{DocumentHead, FeedLink, ScriptLink, document};
pub use feed_meta::{FeedHrefs, project_feed_links, project_list_feed_links};
pub use footer::{
    FooterFeedHrefs, FooterLink, PageFooter, footer, project_footer_links,
    project_list_footer_links,
};
pub use header::{Crumb, Logo, breadcrumbs, page_header};
pub use nav::{FormatLink, MoreLink, NavItem, formats_nav, page_nav};
pub use search::{HiddenField, SearchForm, SearchOption, search_form};
