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

/// Partitions `names` into top-level projects, folding each fork under the
/// shortest project whose directory contains it. Input order is preserved.
#[must_use]
pub fn partition_forks(names: &[String]) -> Vec<ProjectGroup> {
    // Build the prefix tree out of the directories that might contain forks:
    // each project's path with a single trailing `.git` removed.
    let mut trie: Trie = Trie::default();
    for (index, name) in names.iter().enumerate() {
        if let Some(container) = container_path(name) {
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

/// The directory a project might contain forks in: its path with one trailing
/// `.git` stripped. `None` for a non-bare working tree (`repo/.git` → `repo/`)
/// or the bare `.git` itself, neither of which can parent forks.
fn container_path(name: &str) -> Option<String> {
    let stripped: &str = name.strip_suffix(".git").unwrap_or(name);
    if stripped.is_empty() || stripped.ends_with('/') {
        return None;
    }
    Some(stripped.to_owned())
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
