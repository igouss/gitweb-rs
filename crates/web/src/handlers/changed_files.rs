//! Shared changed-files row construction for the diff-bearing views.
//!
//! gitweb's `git_difftree_body` builds a changed-file row's per-file links from
//! the file's status and the *action* it is rendered under: the `commit` view
//! links a modified file to its blobdiff (`diff`), while the `commitdiff` view
//! links every row to its in-page patch anchor instead (and drops the blobdiff
//! link). Everything else — the path link, the status note, the `blob` / `blame`
//! / `history` affordances, and the combined-diff per-parent columns — is shared.
//!
//! So the URL builders and the status-note mapping live here once, and the two
//! views differ only by a [`Context`] flag threaded into the row mappers. This is
//! the boundary's faithful port of `git_difftree_body`'s `$action` branch.

use gitweb_domain::model::change::ChangeKind;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::usecase::commit::{ChangedFiles, CombinedChange, CommitView, OrdinaryChange};
use gitweb_render::chrome::FormatLink;
use gitweb_render::commit::{ChangeNoteView, ChangedRow};

use crate::url::href;

/// Which diff-bearing view a changed-files row is built for — gitweb's `$action`
/// branch in `git_difftree_body`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Context {
    /// The `commit` view: a modified file links to its blobdiff (`diff`).
    Commit,
    /// The `commitdiff` view: every row links to its in-page patch anchor.
    Commitdiff,
}

/// The changed-files rows for a commit-or-commitdiff view, mapped from the use
/// case's ordinary or combined diff under the given [`Context`].
pub(crate) fn rows(
    context: Context,
    project: &str,
    view: &CommitView,
    blame_on: bool,
) -> Vec<ChangedRow> {
    let hash: &str = view.id().as_str();
    match view.changes() {
        ChangedFiles::Ordinary(changes) => {
            let parent: Option<&str> = view.parents().first().map(ObjectId::as_str);
            changes
                .iter()
                .enumerate()
                .map(|(index, change): (usize, &OrdinaryChange)| {
                    ordinary_row(context, project, hash, parent, index + 1, blame_on, change)
                })
                .collect()
        }
        ChangedFiles::Combined(changes) => changes
            .iter()
            .enumerate()
            .map(|(index, change): (usize, &CombinedChange)| {
                combined_row(context, project, hash, view.parents(), index + 1, change)
            })
            .collect(),
    }
}

/// The in-page anchor link to a file's patch in the client-rendered diff —
/// gitweb's `href(-anchor => "patch$patchno")`, the leading link of every
/// `commitdiff` row.
fn patch_anchor(patchno: usize) -> FormatLink {
    format_link("patch", format!("#patch{patchno}"))
}

/// The leading link(s) of a row: the patch anchor under the `commitdiff` view,
/// or — under the `commit` view — the optional blobdiff `diff` link a modified
/// or renamed file carries.
fn lead(context: Context, patchno: usize, commit_diff: Option<FormatLink>) -> Vec<FormatLink> {
    match context {
        Context::Commitdiff => vec![patch_anchor(patchno)],
        Context::Commit => commit_diff.into_iter().collect(),
    }
}

/// One ordinary changed-files row, with the status note and per-file links
/// gitweb's `git_difftree_body` builds for the given view context.
fn ordinary_row(
    context: Context,
    project: &str,
    hash: &str,
    parent: Option<&str>,
    patchno: usize,
    blame_on: bool,
    change: &OrdinaryChange,
) -> ChangedRow {
    let to_id: &str = change.to_oid().as_str();
    let from_id: &str = change.from_oid().as_str();
    match change.status().kind() {
        ChangeKind::Added => {
            let blob: String = blob_href(project, to_id, hash, change.to_path());
            let mut links: Vec<FormatLink> = lead(context, patchno, None);
            links.push(format_link("blob", blob.clone()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(blob),
                note: text_note(change),
                links,
            }
        }
        ChangeKind::Deleted => {
            let base: &str = parent.unwrap_or(hash);
            let blob: String = blob_href(project, from_id, base, change.to_path());
            let mut links: Vec<FormatLink> = lead(context, patchno, None);
            links.push(format_link("blob", blob.clone()));
            push_blame(&mut links, blame_on, project, base, change.to_path());
            links.push(history_link(project, base, change.to_path()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(blob),
                note: text_note(change),
                links,
            }
        }
        ChangeKind::Modified | ChangeKind::TypeChanged => {
            let base: &str = parent.unwrap_or(hash);
            let blob: String = blob_href(project, to_id, hash, change.to_path());
            let diff: Option<FormatLink> = (to_id != from_id).then(|| {
                format_link(
                    "diff",
                    blobdiff_href(project, to_id, from_id, hash, base, change.to_path(), None),
                )
            });
            let mut links: Vec<FormatLink> = lead(context, patchno, diff);
            links.push(format_link("blob", blob.clone()));
            push_blame(&mut links, blame_on, project, hash, change.to_path());
            links.push(history_link(project, hash, change.to_path()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(blob),
                note: text_note(change),
                links,
            }
        }
        ChangeKind::Renamed | ChangeKind::Copied => {
            let base: &str = parent.unwrap_or(hash);
            let path_link: String = blob_href(project, to_id, hash, change.to_path());
            let note: ChangeNoteView = rename_note(project, base, change);
            let diff: Option<FormatLink> = (to_id != from_id).then(|| {
                format_link(
                    "diff",
                    blobdiff_href(
                        project,
                        to_id,
                        from_id,
                        hash,
                        base,
                        change.to_path(),
                        Some(change.from_path()),
                    ),
                )
            });
            let mut links: Vec<FormatLink> = lead(context, patchno, diff);
            links.push(format_link(
                "blob",
                blob_href(project, to_id, base, change.to_path()),
            ));
            push_blame(&mut links, blame_on, project, hash, change.to_path());
            links.push(history_link(project, hash, change.to_path()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(path_link),
                note,
                links,
            }
        }
    }
}

/// One combined (merge) changed-files row: the optional leading patch anchor
/// (`commitdiff` only), the per-parent `diffN` / `blobN` links, then the result
/// `blob` and `history`, mirroring gitweb's combined branch of
/// `git_difftree_body`.
fn combined_row(
    context: Context,
    project: &str,
    hash: &str,
    parents: &[ObjectId],
    patchno: usize,
    change: &CombinedChange,
) -> ChangedRow {
    let to_id: &str = change.to_oid().as_str();
    let to_path: &str = change.to_path();
    let mut links: Vec<FormatLink> = lead(context, patchno, None);
    for (index, side) in change.parents().iter().enumerate() {
        let parent: &str = parents.get(index).map(ObjectId::as_str).unwrap_or(hash);
        match side.status().kind() {
            ChangeKind::Added => {}
            ChangeKind::Deleted => links.push(format_link(
                format!("blob{}", index + 1),
                blob_href(project, side.from_oid().as_str(), hash, to_path),
            )),
            _ => links.push(format_link(
                format!("diff{}", index + 1),
                blobdiff_href(
                    project,
                    to_id,
                    side.from_oid().as_str(),
                    hash,
                    parent,
                    to_path,
                    Some(to_path),
                ),
            )),
        }
    }
    if change.not_deleted() {
        links.push(format_link(
            "blob",
            blob_href(project, to_id, hash, to_path),
        ));
    }
    if change.has_history() {
        links.push(history_link(project, hash, to_path));
    }
    ChangedRow {
        path: to_path.to_owned(),
        path_href: (!change.is_deleted()).then(|| blob_href(project, to_id, hash, to_path)),
        note: ChangeNoteView::None,
        links,
    }
}

/// The status note view for a new / deleted / modified row: the domain's
/// link-free note text, or no note for a pure content modification.
fn text_note(change: &OrdinaryChange) -> ChangeNoteView {
    match change.note() {
        None => ChangeNoteView::None,
        Some(note) => ChangeNoteView::Text {
            category: note.category().to_owned(),
            text: note.text().unwrap_or_default(),
        },
    }
}

/// The rename/copy note view: the category, a link to the old path's blob (at the
/// parent), the similarity, and any mode change.
fn rename_note(project: &str, base: &str, change: &OrdinaryChange) -> ChangeNoteView {
    let note = change
        .note()
        .expect("a rename or copy carries a status note");
    ChangeNoteView::Rename {
        category: note.category().to_owned(),
        from_path: change.from_path().to_owned(),
        from_href: blob_href(
            project,
            change.from_oid().as_str(),
            base,
            change.from_path(),
        ),
        similarity: note.similarity().unwrap_or_default(),
        mode: note.mode().map(str::to_owned),
    }
}

/// Appends a `blame` link for `path` at `base` when the `blame` feature is on.
fn push_blame(links: &mut Vec<FormatLink>, blame_on: bool, project: &str, base: &str, path: &str) {
    if blame_on {
        links.push(format_link(
            "blame",
            href(&[("p", project), ("a", "blame"), ("hb", base), ("f", path)]),
        ));
    }
}

/// A `blob` URL: the object id, the base revision it is viewed at, and the path.
fn blob_href(project: &str, oid: &str, base: &str, path: &str) -> String {
    href(&[
        ("p", project),
        ("a", "blob"),
        ("h", oid),
        ("hb", base),
        ("f", path),
    ])
}

/// A `blobdiff` URL between two blobs of a path, optionally a rename's old path.
fn blobdiff_href(
    project: &str,
    to_id: &str,
    from_id: &str,
    hash: &str,
    parent: &str,
    path: &str,
    file_parent: Option<&str>,
) -> String {
    let mut params: Vec<(&str, &str)> = vec![
        ("p", project),
        ("a", "blobdiff"),
        ("h", to_id),
        ("hp", from_id),
        ("hb", hash),
        ("hpb", parent),
        ("f", path),
    ];
    if let Some(from_path) = file_parent {
        params.push(("fp", from_path));
    }
    href(&params)
}

/// A `history` action link for `path` at `base`.
fn history_link(project: &str, base: &str, path: &str) -> FormatLink {
    format_link(
        "history",
        href(&[("p", project), ("a", "history"), ("hb", base), ("f", path)]),
    )
}

/// A labelled link.
fn format_link(label: impl Into<String>, href: String) -> FormatLink {
    FormatLink {
        label: label.into(),
        href,
    }
}
