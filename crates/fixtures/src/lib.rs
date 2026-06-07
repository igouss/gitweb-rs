//! Deterministic git-repo fixtures, built with gix.
//!
//! Conformance and use-case specs need real git repositories whose object ids
//! are stable across runs and machines, so golden differential conformance
//! never flickers. [`RepoBuilder`] writes blobs, trees, commits, and refs with
//! gix — no git binary — pinning every identity and timestamp.
//!
//! It is test-support, consumed as the `Given` of BDD specs across the
//! workspace, so it lives in its own crate rather than inside any production
//! adapter.

mod builder;
mod project_root;
mod spec;

pub use builder::RepoBuilder;
pub use project_root::ProjectRoot;
pub use spec::{CommitSpec, Identity, Mode, TagSpec, TargetKind, TreeEntry};

/// A git object id, re-exported so specs need not depend on gix directly.
pub use gix::ObjectId;
