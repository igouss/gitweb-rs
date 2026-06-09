//! Use cases: the application logic that drives the [ports](crate::port) and
//! assembles framework-free view-models for the boundaries to render.
//!
//! A use case is the Control in ECB terms: it orchestrates a request against the
//! ports (a [`ProjectStore`](crate::port::project_store::ProjectStore), a
//! [`Repository`](crate::port::repository::Repository)) and the pure entity
//! rules, and returns a view-model. It depends only inward — it knows nothing of
//! HTTP, gix, or HTML — so the web and render adapters consume its output
//! without the domain knowing they exist.

pub mod blob;
pub mod blob_plain;
pub mod blobdiff;
pub mod blobdiff_plain;
pub mod commit;
pub mod commitdiff;
pub mod commitdiff_plain;
pub mod feed;
pub mod heads;
pub mod history;
pub mod log;
pub mod log_generic;
pub mod object;
pub mod object_dispatch;
pub mod opml;
pub mod patch;
pub mod project_index;
pub mod project_list;
pub mod ref_markers;
pub mod remotes;
pub mod shortlog;
pub mod snapshot;
pub mod summary;
pub mod tag;
pub mod tags;
pub mod tree;
