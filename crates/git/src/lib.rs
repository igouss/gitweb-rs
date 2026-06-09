//! The gix-backed adapter for gitweb-rs.
//!
//! This crate is a Boundary in the ECB sense: it implements the domain's
//! [`Repository`] and [`ProjectStore`] ports by driving gix and the filesystem.
//! All dependencies point inward — it knows about [`gitweb_domain`] and gix, and
//! nothing knows about it but the composition root.
//!
//! The translation between gix's read types and the domain's entities lives in
//! [`conv`], so [`repository`] reads as a faithful rendering of the port into
//! gix calls rather than a pile of conversion noise.
//!
//! [`Repository`]: gitweb_domain::port::repository::Repository
//! [`ProjectStore`]: gitweb_domain::port::project_store::ProjectStore

mod archive;
mod conv;
mod project_store;
mod repository;
mod user_directory;

pub use project_store::GixProjectStore;
pub use repository::GixRepository;
pub use user_directory::{SystemUserDirectory, UserDirectory};
