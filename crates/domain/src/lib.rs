//! Framework-free git-domain core for gitweb-rs.
//!
//! This crate holds the Entities (business logic), Ports (traits), and use
//! cases. It must never depend on a web framework, a git library, or any other
//! adapter — all dependencies point inward, toward this crate.

pub mod error;
pub mod model;
pub mod port;
