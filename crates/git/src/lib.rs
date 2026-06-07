//! The gix-backed adapter for gitweb-rs.
//!
//! This crate is a Boundary in the ECB sense: it implements the domain's
//! [`Repository`] (and, in later slices, `ProjectStore`) port by driving gix.
//! All dependencies point inward — it knows about [`gitweb_domain`] and gix, and
//! nothing knows about it but the composition root.
//!
//! The translation between gix's read types and the domain's entities lives in
//! [`conv`], so [`repository`] reads as a faithful rendering of the port into
//! gix calls rather than a pile of conversion noise.
//!
//! [`Repository`]: gitweb_domain::port::repository::Repository

mod conv;
mod repository;

pub use repository::GixRepository;
