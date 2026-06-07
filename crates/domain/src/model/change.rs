//! The change a diff records for a single path.
//!
//! Mirrors the status token in gitweb's `parse_difftree_raw_line`
//! (`(.)([0-9]{0,3})`): a one-letter status optionally followed by a 0–100
//! similarity score. `git_difftree_body` renders exactly six of those letters —
//! `A`, `D`, `M`, `T`, `R`, `C` — and reads the score only for renames (`R`)
//! and copies (`C`). Any other letter is something gitweb does not display, so
//! it parses to nothing here.

use crate::model::file_mode::FileMode;

/// What happened to a path between two trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// The path was created (`A`).
    Added,
    /// The path was removed (`D`).
    Deleted,
    /// The path's content changed (`M`).
    Modified,
    /// The path's type changed, e.g. file → symlink (`T`).
    TypeChanged,
    /// The path was moved from another path (`R`).
    Renamed,
    /// The path was copied from another path (`C`).
    Copied,
}

/// A diff status: what changed, plus the rename/copy similarity score.
///
/// The score is meaningful only for [`ChangeKind::Renamed`] and
/// [`ChangeKind::Copied`]; it is `0` for every other kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeStatus {
    kind: ChangeKind,
    similarity: u8,
}

impl ChangeStatus {
    /// Parses a diff-tree status token (`"M"`, `"R100"`, `"C75"`, …).
    ///
    /// Returns `None` for an unrecognised status letter, for a similarity score
    /// on a non-rename/non-copy status, or for a malformed score.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        let first: char = token.chars().next()?;
        let (letter, tail): (&str, &str) = token.split_at(first.len_utf8());
        let kind: ChangeKind = match letter {
            "A" => ChangeKind::Added,
            "D" => ChangeKind::Deleted,
            "M" => ChangeKind::Modified,
            "T" => ChangeKind::TypeChanged,
            "R" => ChangeKind::Renamed,
            "C" => ChangeKind::Copied,
            _ => return None,
        };
        let carries_score: bool = matches!(kind, ChangeKind::Renamed | ChangeKind::Copied);
        let similarity: u8 = match (carries_score, tail) {
            // No score is the norm for plain statuses, and the default for a
            // bare rename/copy.
            (_, "") => 0,
            // Only renames and copies may carry a 1–3 digit similarity score.
            (true, digits) => parse_score(digits)?,
            // A score on any other status is not something git emits.
            (false, _) => return None,
        };
        Some(Self { kind, similarity })
    }

    /// A path that was created.
    #[must_use]
    pub fn added() -> Self {
        Self {
            kind: ChangeKind::Added,
            similarity: 0,
        }
    }

    /// A path that was removed.
    #[must_use]
    pub fn deleted() -> Self {
        Self {
            kind: ChangeKind::Deleted,
            similarity: 0,
        }
    }

    /// A path that was moved from another path, with its similarity score.
    #[must_use]
    pub fn renamed(similarity: u8) -> Self {
        Self {
            kind: ChangeKind::Renamed,
            similarity,
        }
    }

    /// A path that was copied from another path, with its similarity score.
    #[must_use]
    pub fn copied(similarity: u8) -> Self {
        Self {
            kind: ChangeKind::Copied,
            similarity,
        }
    }

    /// Classifies a content change between two file modes.
    ///
    /// A raw diff-tree line hands gitweb a ready-made status letter, but gix
    /// reports a modification as a pair of modes, so we re-derive git's
    /// `M`-vs-`T` rule: flipping the executable bit leaves a regular file a
    /// regular file (`M`), while changing between file, symlink, and gitlink is
    /// a type change (`T`).
    #[must_use]
    pub fn from_modification(from: FileMode, to: FileMode) -> Self {
        let kind: ChangeKind = if from.same_type(to) {
            ChangeKind::Modified
        } else {
            ChangeKind::TypeChanged
        };
        Self {
            kind,
            similarity: 0,
        }
    }

    /// The kind of change.
    #[must_use]
    pub fn kind(self) -> ChangeKind {
        self.kind
    }

    /// The rename/copy similarity score (`0` for other kinds).
    #[must_use]
    pub fn similarity(self) -> u8 {
        self.similarity
    }
}

/// Parses a similarity score: a 1–3 digit percentage, mirroring gitweb's
/// `[0-9]{0,3}` capture. Wider or non-numeric input is not a valid token.
fn parse_score(digits: &str) -> Option<u8> {
    if digits.len() > 3 {
        return None;
    }
    digits.parse::<u8>().ok()
}
