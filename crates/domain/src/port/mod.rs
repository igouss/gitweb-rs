//! Ports: the framework-free traits the use cases depend on.
//!
//! Adapters (gix, project discovery) implement these; use cases consume them.
//! Defining the seam here is what lets the use cases be tested against fakes.

pub mod project_store;
pub mod repository;
