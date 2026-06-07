//! The axum web boundary for gitweb-rs.
//!
//! This is a Boundary (adapter): it turns a raw inbound HTTP request into the
//! domain's validated [`Request`](gitweb_domain::model::request::Request) and,
//! later, dispatches it and renders the response. It depends only on the domain
//! crate — its ports and the done request rules — so all dependencies point
//! inward; the concrete adapters (gix-backed store, render) are wired in by the
//! composition root, never reached for here.
//!
//! The first concern, in [`request`], is the impure half of gitweb's
//! `evaluate_path_info` + `evaluate_query_params`: resolving which PATH_INFO
//! prefix names a project (a filesystem walk, done through the `ProjectStore`
//! port), decomposing the remainder, and merging it with the query string under
//! gitweb's precedence before validating the whole into a `Request`.

pub mod request;
