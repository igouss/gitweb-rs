//! Recency styling: domain [`AgeClass`] to a CSS class name.
//!
//! gitweb's `age_class` returns `noage`/`age0`/`age1`/`age2`, which the original
//! stylesheet colours from bright (recent) to faded (old). The *bucketing* is a
//! domain rule and already lives in [`AgeClass`]; this adapter only names the
//! CSS hook the modernized stylesheet targets, so the class names are
//! self-describing rather than gitweb's opaque ordinals.

use gitweb_domain::model::age::AgeClass;

/// Maps an age bucket to the CSS class the fresh stylesheet styles.
#[must_use]
pub fn age_class_name(class: AgeClass) -> &'static str {
    match class {
        AgeClass::Unknown => "age-unknown",
        AgeClass::Fresh => "age-fresh",
        AgeClass::Recent => "age-recent",
        AgeClass::Old => "age-old",
    }
}
