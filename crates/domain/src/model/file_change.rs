//! The per-row status note `git_difftree_body` attaches to a changed path.
//!
//! Each row of the commit / commitdiff changed-files table carries a small
//! annotation describing *what* happened to the path: `[new file with mode:
//! 0755]`, `[deleted symlink]`, `[changed from file to symlink]`, `[moved …
//! with 90% similarity]`. gitweb computes this inline in `git_difftree_body`
//! from the path's status letter and its from/to modes; here it is the pure rule
//! that decides which facts a status implies, so the render layer only places
//! them and the web boundary only builds the rename/copy from-path link.
//!
//! Only the rename/copy note carries a link (to the old path's blob), and that
//! link's URL is the boundary's job — so this entity holds the *similarity* and
//! the *mode*, not the from-path. Everything else (new/deleted/modified) is
//! link-free text, exposed as a ready [`FileChangeNote::text`] the render wraps
//! in a span.

use crate::model::change::{ChangeKind, ChangeStatus};
use crate::model::file_mode::FileMode;

/// The structured status note for one changed path, mirroring the spans
/// `git_difftree_body` emits. A pure content modification (mode unchanged) has
/// no note, so [`FileChangeNote::derive`] returns `None` for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeNote {
    /// A created path (`A`): its new file type, plus the permission bits when it
    /// is a regular file (gitweb shows `mode_str` only under `S_ISREG`).
    New {
        /// The short file type of the created path (`file` / `symlink` / …).
        file_type: &'static str,
        /// The permission bits (`"0755"`), present only for a regular file.
        mode: Option<String>,
    },
    /// A removed path (`D`): the file type it had.
    Deleted {
        /// The short file type of the removed path.
        file_type: &'static str,
    },
    /// A modified or type-changed path (`M` / `T`) whose mode changed: the type
    /// change when the file types differ, and the permission change when the
    /// permission bits differ and the new side is a regular file.
    Changed {
        /// The `(from, to)` short file types, present only when they differ.
        type_change: Option<(&'static str, &'static str)>,
        /// The permission change, present only when the bits differ and the new
        /// side is a regular file.
        mode: Option<PermissionChange>,
    },
    /// A moved path (`R`): the similarity score, and the new permission bits when
    /// the mode changed across the move.
    Moved {
        /// The rename similarity percentage (gitweb's score).
        similarity: u8,
        /// The new permission bits, present only when the mode changed.
        mode: Option<String>,
    },
    /// A copied path (`C`): the similarity score, and the new permission bits
    /// when the mode changed across the copy.
    Copied {
        /// The copy similarity percentage.
        similarity: u8,
        /// The new permission bits, present only when the mode changed.
        mode: Option<String>,
    },
}

/// The permission part of a modified path's note: the new bits always, the old
/// bits only when the old side was also a regular file — gitweb's
/// `mode: $from->$to` vs `mode: $to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionChange {
    /// The old permission bits, present only when the old side is a regular file.
    pub from: Option<String>,
    /// The new permission bits.
    pub to: String,
}

impl FileChangeNote {
    /// Derives the note for a changed path from its status and from/to modes.
    /// Returns `None` for a pure content modification (the modes are identical),
    /// which `git_difftree_body` annotates with nothing.
    #[must_use]
    pub fn derive(status: ChangeStatus, from: FileMode, to: FileMode) -> Option<Self> {
        match status.kind() {
            ChangeKind::Added => Some(Self::New {
                file_type: to.short_type(),
                mode: regular_mode(to),
            }),
            ChangeKind::Deleted => Some(Self::Deleted {
                file_type: from.short_type(),
            }),
            ChangeKind::Modified | ChangeKind::TypeChanged => modified(from, to),
            ChangeKind::Renamed => Some(Self::Moved {
                similarity: status.similarity(),
                mode: moved_mode(from, to),
            }),
            ChangeKind::Copied => Some(Self::Copied {
                similarity: status.similarity(),
                mode: moved_mode(from, to),
            }),
        }
    }

    /// The CSS class / status word gitweb tags the span with: `new`, `deleted`,
    /// `changed`, `moved`, or `copied`.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            Self::New { .. } => "new",
            Self::Deleted { .. } => "deleted",
            Self::Changed { .. } => "changed",
            Self::Moved { .. } => "moved",
            Self::Copied { .. } => "copied",
        }
    }

    /// The created/removed path's file type, for the `New` and `Deleted` notes.
    #[must_use]
    pub fn file_type(&self) -> Option<&'static str> {
        match self {
            Self::New { file_type, .. } | Self::Deleted { file_type } => Some(file_type),
            _ => None,
        }
    }

    /// The rename/copy similarity percentage, for the `Moved` and `Copied` notes.
    #[must_use]
    pub fn similarity(&self) -> Option<u8> {
        match self {
            Self::Moved { similarity, .. } | Self::Copied { similarity, .. } => Some(*similarity),
            _ => None,
        }
    }

    /// The permission bits a `New`, `Moved`, or `Copied` note shows — the value
    /// after a `with mode:` / `, mode:` separator. A `Changed` note's richer
    /// from→to permission lives in its [`PermissionChange`] and is rendered via
    /// [`Self::text`].
    #[must_use]
    pub fn mode(&self) -> Option<&str> {
        match self {
            Self::New { mode, .. } | Self::Moved { mode, .. } | Self::Copied { mode, .. } => {
                mode.as_deref()
            }
            _ => None,
        }
    }

    /// The link-free annotation text for the `New`, `Deleted`, and `Changed`
    /// notes — exactly the span body gitweb prints, ready for the render layer to
    /// wrap (the brackets and the CSS class are the render's). Returns `None` for
    /// the rename/copy notes, whose text embeds a link the boundary must build.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        match self {
            Self::New { file_type, mode } => Some(match mode {
                Some(bits) => format!("new {file_type} with mode: {bits}"),
                None => format!("new {file_type}"),
            }),
            Self::Deleted { file_type } => Some(format!("deleted {file_type}")),
            Self::Changed { type_change, mode } => Some(changed_text(*type_change, mode.as_ref())),
            Self::Moved { .. } | Self::Copied { .. } => None,
        }
    }
}

/// The permission bits of `mode` when it is a regular file, else `None` —
/// gitweb's `$to_mode_str = sprintf(…) if S_ISREG(...)`.
fn regular_mode(mode: FileMode) -> Option<String> {
    mode.is_regular().then(|| mode.permission_bits())
}

/// The new permission bits for a rename/copy, shown only when the full mode
/// changed — gitweb's `, mode: %04o` guard on `from_mode != to_mode`. Unlike a
/// modification, a rename/copy shows the bits for any new type (directories
/// included), so this does not gate on `S_ISREG`.
fn moved_mode(from: FileMode, to: FileMode) -> Option<String> {
    (from != to).then(|| to.permission_bits())
}

/// The note for a modification or type change: `None` when the full mode is
/// unchanged (a pure content edit), otherwise the type and/or permission change
/// the new mode introduces.
fn modified(from: FileMode, to: FileMode) -> Option<FileChangeNote> {
    if from == to {
        return None;
    }
    let type_change: Option<(&'static str, &'static str)> =
        (from.short_type() != to.short_type()).then(|| (from.short_type(), to.short_type()));
    let mode: Option<PermissionChange> = permission_change(from, to);
    Some(FileChangeNote::Changed { type_change, mode })
}

/// The permission change for a modification: present only when the permission
/// bits differ and the new side is a regular file (gitweb shows the bits under
/// `$to_mode_str`). The old bits ride along only when the old side is also a
/// regular file.
fn permission_change(from: FileMode, to: FileMode) -> Option<PermissionChange> {
    if from.permission_bits() == to.permission_bits() || !to.is_regular() {
        return None;
    }
    Some(PermissionChange {
        from: from.is_regular().then(|| from.permission_bits()),
        to: to.permission_bits(),
    })
}

/// Composes a `Changed` note's link-free text — gitweb's `[changed …]` body
/// without the brackets: the optional `from <a> to <b>` type clause followed by
/// the optional `mode: …` permission clause.
fn changed_text(
    type_change: Option<(&'static str, &'static str)>,
    mode: Option<&PermissionChange>,
) -> String {
    let mut text: String = "changed".to_owned();
    if let Some((from, to)) = type_change {
        text.push_str(&format!(" from {from} to {to}"));
    }
    if let Some(change) = mode {
        match &change.from {
            Some(from) => text.push_str(&format!(" mode: {from}->{}", change.to)),
            None => text.push_str(&format!(" mode: {}", change.to)),
        }
    }
    text
}
