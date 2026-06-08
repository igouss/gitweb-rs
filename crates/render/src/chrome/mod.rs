//! Site chrome: the modernized, framework-free page furniture that every view
//! sits inside — the HTML5 document skeleton, the header (logo + breadcrumbs),
//! the search form, the per-page action navigation, and the footer.
//!
//! Each piece renders a finished view-model and builds no URLs of its own:
//! gitweb's `href()` lives at the web boundary, so the chrome's inputs are the
//! contract the boundary fills. Everything is escaped by default via maud; the
//! only raw HTML enters through [`crate::markup::raw`].

pub mod document;
pub mod footer;
pub mod header;
pub mod nav;
pub mod search;

pub use document::{DocumentHead, FeedLink, document};
pub use footer::{FooterLink, PageFooter, footer};
pub use header::{Crumb, Logo, breadcrumbs, page_header};
pub use nav::{MoreLink, NavItem, page_nav};
pub use search::{HiddenField, SearchForm, SearchOption, search_form};
