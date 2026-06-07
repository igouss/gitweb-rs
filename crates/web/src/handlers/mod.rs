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

pub mod heads;
pub mod project_list;
pub mod tag;
pub mod tags;

pub use heads::HeadsHandler;
pub use project_list::ProjectListHandler;
pub use tag::TagHandler;
pub use tags::TagsHandler;
