//! Golden differential conformance for gitweb-rs's format-stable endpoints.
//!
//! gitweb's HTML pages are modernized, so they are verified behaviourally. Its
//! machine-readable, byte-stable endpoints — `blob_plain`, `commitdiff_plain`,
//! `patch`, `project_index`, `opml`, the feeds, `blame_data` — are not: other
//! tools parse them, so they must match the original gitweb byte for byte. Those
//! are pinned by GOLDEN differential conformance: reference output is captured
//! ONCE from the upstream `gitweb.perl` (see `scripts/capture-goldens.sh`) into
//! committed files under `goldens/`, and our output is diffed against it. No
//! perl, and no git binary, run at test time.
//!
//! Two halves live here:
//! - [`corpus`] builds the deterministic fixture repository the goldens were
//!   captured over. The SAME builder runs at capture time (via the `build-corpus`
//!   binary) and at test time, so every object id matches and the goldens never
//!   flicker.
//! - [`golden`] loads a captured CGI response and splits its header block from
//!   its raw body, binary-safe, so each endpoint can assert the exact bytes.

pub mod corpus;
pub mod golden;

pub use corpus::{Corpus, build};
pub use golden::Golden;
