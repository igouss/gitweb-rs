//! The action handlers: gitweb's `%actions` subs as [`Handler`] implementations.
//!
//! Each capability registers one handler per action it serves. A handler is the
//! boundary glue between the inbound [`Request`] and the response: it reads the
//! request, drives a domain use case over the ports it was wired with, renders
//! the resulting view-model through the render layer, and returns a [`View`].
//! The composition root constructs each handler with its concrete adapters and
//! registers it on the [`Dispatcher`].
//!
//! [`Handler`]: crate::dispatch::Handler
//! [`Request`]: gitweb_domain::model::request::Request
//! [`View`]: crate::response::View
//! [`Dispatcher`]: crate::dispatch::Dispatcher

pub mod blob;
pub mod blob_plain;
pub mod blobdiff;
pub mod blobdiff_plain;
mod changed_files;
pub mod commit;
pub mod commitdiff;
pub mod commitdiff_plain;
pub mod feed;
pub mod heads;
pub mod history;
pub mod log;
pub mod object;
pub mod opml;
pub mod patch;
pub mod patches;
pub mod project_index;
pub mod project_list;
mod ref_markers;
mod refs;
pub mod remotes;
pub mod shortlog;
pub mod snapshot;
pub mod summary;
pub mod tag;
pub mod tags;
pub mod tree;

pub use blob::BlobHandler;
pub use blob_plain::BlobPlainHandler;
pub use blobdiff::BlobdiffHandler;
pub use blobdiff_plain::BlobdiffPlainHandler;
pub use commit::CommitHandler;
pub use commitdiff::CommitdiffHandler;
pub use commitdiff_plain::CommitdiffPlainHandler;
pub use feed::FeedHandler;
pub use heads::HeadsHandler;
pub use history::HistoryHandler;
pub use log::LogHandler;
pub use object::ObjectHandler;
pub use opml::OpmlHandler;
pub use patch::PatchHandler;
pub use patches::PatchesHandler;
pub use project_index::ProjectIndexHandler;
pub use project_list::ProjectListHandler;
pub use remotes::RemotesHandler;
pub use shortlog::ShortlogHandler;
pub use snapshot::SnapshotHandler;
pub use summary::SummaryHandler;
pub use tag::TagHandler;
pub use tags::TagsHandler;
pub use tree::TreeHandler;
