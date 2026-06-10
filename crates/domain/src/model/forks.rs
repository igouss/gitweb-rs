//! Detecting forks among a set of projects (gitweb's
//! `filter_forks_from_projects_list`).
//!
//! gitweb's fork convention: the forks of `repo.git` live in the sibling
//! directory `repo/`, as `repo/whatever.git`. The list page shows `repo.git`
//! once and folds its forks underneath it. The rule builds a prefix tree of
//! project paths with the trailing `.git` stripped, then removes any project
//! that sits under a shorter project's directory, attaching it to that parent.
//!
//! Matching is by whole path component, so `foobar.git` is not a fork of
//! `foo.git`.

use std::num::NonZeroUsize;

/// One project that survived fork-filtering, with any forks folded under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGroup {
    name: String,
    forks: Vec<String>,
}

impl ProjectGroup {
    /// The canonical project's store-relative path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The store-relative paths of this project's forks, in list order.
    #[must_use]
    pub fn forks(&self) -> &[String] {
        &self.forks
    }
}

/// The subdirectory the forks of `project` live in: its path with one trailing
/// `.git` removed (gitweb's `$filter =~ s/\.git$//` in `git_forks`). The forks
/// of `repo.git` live in `repo/`, so the forks action scopes the project
/// listing to this subdirectory.
#[must_use]
pub fn forks_subdirectory(project: &str) -> &str {
    project.strip_suffix(".git").unwrap_or(project)
}

/// The directory a project might *contain* forks in — the path it must own on
/// disk for gitweb to treat it as fork-capable. This is gitweb's
/// `filter_forks_from_projects_list` name-side guards: strip one trailing `.git`
/// (`forks of 'repo.git' are in 'repo/'`), then reject a non-bare working tree
/// (`repo/.git` strips to `repo/`, which ends in `/`) and the bare `.git`
/// repository itself (strips to the empty string). `None` means the project can
/// never parent forks, whatever the filesystem holds; `Some(dir)` is the
/// directory the caller still has to confirm exists (gitweb's `-d` test, the
/// filesystem half this pure rule cannot decide).
#[must_use]
pub fn fork_container(name: &str) -> Option<&str> {
    let stripped: &str = forks_subdirectory(name);
    // gitweb: `next if ($path =~ m!/$!)` (non-bare repo) and `next unless ($path)`
    // (the bare `.git` itself) — both leave the project unable to parent forks.
    if stripped.is_empty() || stripped.ends_with('/') {
        return None;
    }
    Some(stripped)
}

/// The leading fork-column state for one project on the listing — gitweb's
/// `forks` field, whose three shapes drive three distinct renderings:
///
/// - [`NotForkable`](Self::NotForkable): gitweb's `forks` is undef. The project
///   cannot parent forks, or its container directory does not exist on disk.
/// - [`EmptyContainer`](Self::EmptyContainer): gitweb's `forks => []`. The
///   container directory exists but holds zero forks.
/// - [`Forked`](Self::Forked): gitweb's `forks => [N>0]`. Forks are folded
///   under the project.
///
/// The empty-container state exists precisely because gitweb sets `forks => []`
/// on any fork-capable project whose container directory exists, *before* it
/// folds any forks in — so a project with a real but fork-less sibling directory
/// is distinguishable from one that can never parent forks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkState {
    /// Not fork-capable: no `+` affordance, no `forks` quick link (gitweb's
    /// undefined `forks`).
    NotForkable,
    /// A fork-capable container directory exists but folds zero forks: an
    /// *unlinked* `+` titled "0 forks", plus the `forks` quick link (gitweb's
    /// `forks => []`).
    EmptyContainer,
    /// Forks are folded under this project: a `+` *linked* to the forks view,
    /// titled with the count, plus the `forks` quick link (gitweb's
    /// `forks => [N>0]`).
    Forked(NonZeroUsize),
}

impl ForkState {
    /// The number of forks folded under the project (gitweb's
    /// `scalar @{$pr->{forks}}`): the count for [`Forked`](Self::Forked), and `0`
    /// for both fork-less states.
    #[must_use]
    pub fn fork_count(self) -> usize {
        match self {
            ForkState::Forked(forks) => forks.get(),
            ForkState::NotForkable | ForkState::EmptyContainer => 0,
        }
    }

    /// Whether the project shows any fork affordance — the `+` and the `forks`
    /// quick link (gitweb's truthy `$pr->{forks}`): true for both the empty
    /// container and a forked project, false only when not fork-capable.
    #[must_use]
    pub fn is_forkable(self) -> bool {
        !matches!(self, ForkState::NotForkable)
    }
}

/// Partitions `names` into top-level projects, folding each fork under the
/// shortest project whose directory contains it. Input order is preserved.
#[must_use]
pub fn partition_forks(names: &[String]) -> Vec<ProjectGroup> {
    // Build the prefix tree out of the directories that might contain forks:
    // each project's path with a single trailing `.git` removed.
    let mut trie: Trie = Trie::default();
    for (index, name) in names.iter().enumerate() {
        if let Some(container) = fork_container(name) {
            let components: Vec<&str> = container.split('/').collect();
            trie.insert(&components, index);
        }
    }

    // Walk each project's *original* path through the tree: hitting a shorter
    // project's end marker means this project is a fork of it.
    let mut groups: Vec<ProjectGroup> = names
        .iter()
        .map(|name: &String| ProjectGroup {
            name: name.clone(),
            forks: Vec::new(),
        })
        .collect();
    let mut folded: Vec<bool> = vec![false; names.len()];
    for (index, name) in names.iter().enumerate() {
        if let Some(parent) = trie.shortest_prefix(name) {
            groups[parent].forks.push(name.clone());
            folded[index] = true;
        }
    }

    groups
        .into_iter()
        .zip(folded)
        .filter_map(|(group, fork): (ProjectGroup, bool)| (!fork).then_some(group))
        .collect()
}

/// A prefix tree of project-path components, with an end marker recording which
/// project (by index) terminates at each node.
#[derive(Default)]
struct Trie {
    children: std::collections::HashMap<String, Trie>,
    /// The index of the project whose container path ends here, if any. The
    /// first project to claim a node keeps it, matching gitweb.
    project: Option<usize>,
}

impl Trie {
    /// Records that `project`'s container path runs through `components`.
    fn insert(&mut self, components: &[&str], project: usize) {
        match components.split_first() {
            None => {
                if self.project.is_none() {
                    self.project = Some(project);
                }
            }
            Some((head, rest)) => {
                self.children
                    .entry((*head).to_owned())
                    .or_default()
                    .insert(rest, project);
            }
        }
    }

    /// The index of the shortest project whose container directory is a
    /// component-wise prefix of `name` — the project `name` is a fork of — or
    /// `None` when `name` is itself a top-level project.
    fn shortest_prefix(&self, name: &str) -> Option<usize> {
        let mut node: &Trie = self;
        for component in name.split('/') {
            if let Some(parent) = node.project {
                return Some(parent);
            }
            node = node.children.get(component)?;
        }
        None
    }
}
