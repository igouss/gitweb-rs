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
pub mod feed;
pub mod heads;
pub mod history;
pub mod log;
pub mod opml;
pub mod project_index;
pub mod project_list;
mod refs;
pub mod remotes;
pub mod shortlog;
pub mod summary;
pub mod tag;
pub mod tags;
pub mod tree;

pub use blob::BlobHandler;
pub use blob_plain::BlobPlainHandler;
pub use feed::FeedHandler;
pub use heads::HeadsHandler;
pub use history::HistoryHandler;
pub use log::LogHandler;
pub use opml::OpmlHandler;
pub use project_index::ProjectIndexHandler;
pub use project_list::ProjectListHandler;
pub use remotes::RemotesHandler;
pub use shortlog::ShortlogHandler;
pub use summary::SummaryHandler;
pub use tag::TagHandler;
pub use tags::TagsHandler;
pub use tree::TreeHandler;
