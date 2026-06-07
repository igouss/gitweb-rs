//! The request clock, read at the boundary.
//!
//! gitweb computes relative ages against `time` (the request's wall clock). That
//! "now" is an input to the use cases, not something the framework-free domain
//! should reach for, so the boundary reads it once per request here and hands it
//! inward. Keeping it in one place means every handler measures ages against the
//! same clock.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current time as a Unix epoch in whole seconds (gitweb's `time`). A clock
/// before the Unix epoch is clamped to 0 rather than panicking, and a clock far
/// enough past it to overflow `i64` is clamped to `i64::MAX`.
#[must_use]
pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed: std::time::Duration| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
