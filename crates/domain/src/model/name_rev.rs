//! `git name-rev --tags` ancestor naming — gitweb's `git_get_rev_name_tags`.
//!
//! gitweb stamps a commit's `X-Git-Tag` line (on `commitdiff_plain` / `patch`)
//! with `git name-rev --tags <oid>`: the nearest tag that reaches the commit,
//! suffixed by the path name-rev walked from that tag's tip down to it. Each
//! first-parent generation is `~1`; stepping onto the Nth parent (N > 1) of a
//! merge is `^N` and resets the generation counter. A tag whose tip *is* the
//! commit names it at distance zero — bare for a lightweight tag, `<name>^0` for
//! an annotated one (name-rev's one-dereference marker) — and that `^0` is
//! stripped before any `~N` is appended to an ancestor (so a commit three
//! first-parent generations behind annotated `v1.0` is `v1.0~3`, not
//! `v1.0^0~3`).
//!
//! This is the pure half of `Repository::rev_name_tag`, ported byte-for-byte
//! from git's `builtin/name-rev.c` (`name_rev`, `is_better_name`,
//! `get_parent_name`, `get_rev_name`). The adapter resolves each tag ref to its
//! peeled commit, captured name, tagger date (the commit's own committer date
//! for a lightweight tag) and dereference flag, collects the commit ancestry
//! reachable from the tips, and hands both to [`name_by_tags`].
//!
//! Two simplifications, both output-identical to git for naming a single commit:
//! every tip is a tag (gitweb only ever passes `--tags`), so `is_better_name`'s
//! tag-vs-non-tag branches are dead and the comparison reduces to "strictly
//! nearer effective-distance wins; on a tie keep the earlier-processed tip"; and
//! git's date/generation *cutoff* (a search-pruning optimisation) is omitted —
//! every commit on a tag→target path is newer than the target, so a full walk
//! reaches the same names a cut-off walk would.

use std::collections::HashMap;

use crate::model::object_id::ObjectId;

/// git's `MERGE_TRAVERSAL_WEIGHT`: how many first-parent generations are
/// maximally preferred over a single merge (non-first-parent) traversal.
const MERGE_TRAVERSAL_WEIGHT: i64 = 65535;

/// A tag tip name-rev considers — git's `tip_table_entry` for a `refs/tags/*`
/// ref, already resolved by the adapter.
#[derive(Debug, Clone)]
pub struct TagTip {
    /// The commit the tag peels to.
    oid: ObjectId,
    /// The gitweb-captured tag name (the `tags/(.*)` body — no `refs/tags/`).
    name: String,
    /// git's `taggerdate`: an annotated tag's tagger date, or the commit's own
    /// committer date for a lightweight tag. Only the tip *ordering* uses it.
    taggerdate: i64,
    /// Whether this is an annotated tag, whose distance-zero name carries the
    /// `^0` dereference marker.
    deref: bool,
}

impl TagTip {
    /// A resolved tag tip.
    #[must_use]
    pub fn new(oid: ObjectId, name: String, taggerdate: i64, deref: bool) -> Self {
        Self {
            oid,
            name,
            taggerdate,
            deref,
        }
    }

    /// The commit this tag peels to — the seed the adapter walks ancestry from.
    #[must_use]
    pub fn oid(&self) -> &ObjectId {
        &self.oid
    }
}

/// The commit ancestry the walk needs: each commit mapped to its parents, in
/// parent order (first parent first).
#[derive(Debug, Default, Clone)]
pub struct CommitGraph {
    parents: HashMap<ObjectId, Vec<ObjectId>>,
}

impl CommitGraph {
    /// An empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a commit's parents (first parent first); a root commit has none.
    pub fn insert(&mut self, oid: ObjectId, parents: Vec<ObjectId>) {
        self.parents.insert(oid, parents);
    }

    /// The parents of `oid`, or an empty slice for a root or unknown commit.
    fn parents_of(&self, oid: &ObjectId) -> &[ObjectId] {
        self.parents.get(oid).map_or(&[], Vec::as_slice)
    }
}

/// One commit's best name so far — git's `struct rev_name`, reduced to the
/// fields the all-tags case needs (`from_tag` is always set, `taggerdate` only
/// orders the tips).
struct RevName {
    tip_name: String,
    generation: i64,
    distance: i64,
}

/// git's `effective_distance`: a name through a merge parent costs a whole
/// `MERGE_TRAVERSAL_WEIGHT` block, so a shorter first-parent path is preferred
/// over a merge hop. (Ported verbatim, counterintuitive sign and all.)
fn effective_distance(distance: i64, generation: i64) -> i64 {
    distance
        + if generation > 0 {
            MERGE_TRAVERSAL_WEIGHT
        } else {
            0
        }
}

/// git's `is_better_name` for two tags (the only case here): a candidate beats
/// the incumbent only when it is strictly nearer in effective distance. A tie
/// keeps the incumbent — the earlier-processed (older, then lower-named) tip.
fn is_better(incumbent: Option<&RevName>, generation: i64, distance: i64) -> bool {
    match incumbent {
        None => true,
        Some(existing) => {
            effective_distance(existing.distance, existing.generation)
                > effective_distance(distance, generation)
        }
    }
}

/// git's `get_parent_name`: the name of the `parent_number`-th parent (N > 1) of
/// a commit named `tip_name` at `generation`, the `^N` merge-traversal form.
fn parent_name_via(tip_name: &str, generation: i64, parent_number: usize) -> String {
    let base: &str = tip_name.strip_suffix("^0").unwrap_or(tip_name);
    if generation > 0 {
        format!("{base}~{generation}^{parent_number}")
    } else {
        format!("{base}^{parent_number}")
    }
}

/// git's `get_rev_name`: render the final string, stripping the `^0` dereference
/// marker before appending a `~N` generation suffix.
fn render_name(name: &RevName) -> String {
    if name.generation == 0 {
        return name.tip_name.clone();
    }
    let base: &str = name.tip_name.strip_suffix("^0").unwrap_or(&name.tip_name);
    format!("{base}~{}", name.generation)
}

/// git's `name_rev`: propagate one tip's name down the first-parent chain
/// (`~N`) and onto merge parents (`^N`), keeping the better name at each commit.
fn name_rev_from_tip(tip: &TagTip, graph: &CommitGraph, names: &mut HashMap<ObjectId, RevName>) {
    if !is_better(names.get(&tip.oid), 0, 0) {
        return;
    }
    let start_tip_name: String = if tip.deref {
        format!("{}^0", tip.name)
    } else {
        tip.name.clone()
    };
    names.insert(
        tip.oid.clone(),
        RevName {
            tip_name: start_tip_name,
            generation: 0,
            distance: 0,
        },
    );

    let mut stack: Vec<ObjectId> = vec![tip.oid.clone()];
    while let Some(commit) = stack.pop() {
        let (commit_tip_name, commit_gen, commit_dist): (String, i64, i64) = {
            let current: &RevName = &names[&commit];
            (
                current.tip_name.clone(),
                current.generation,
                current.distance,
            )
        };
        let mut to_queue: Vec<ObjectId> = Vec::new();
        for (index, parent) in graph.parents_of(&commit).iter().enumerate() {
            let parent_number: usize = index + 1;
            let (generation, distance): (i64, i64) = if parent_number == 1 {
                (commit_gen + 1, commit_dist + 1)
            } else {
                (0, commit_dist + MERGE_TRAVERSAL_WEIGHT)
            };
            if !is_better(names.get(parent), generation, distance) {
                continue;
            }
            let parent_tip_name: String = if parent_number == 1 {
                commit_tip_name.clone()
            } else {
                parent_name_via(&commit_tip_name, commit_gen, parent_number)
            };
            names.insert(
                parent.clone(),
                RevName {
                    tip_name: parent_tip_name,
                    generation,
                    distance,
                },
            );
            to_queue.push(parent.clone());
        }
        // git pops the first parent first; push in reverse so it lands on top.
        for parent in to_queue.into_iter().rev() {
            stack.push(parent);
        }
    }
}

/// Names `target` after the best tag tip that reaches it, or `None` when no tag
/// does — git's `git name-rev --tags <target>` output, with the `tags/` prefix
/// already dropped (gitweb's captured body).
#[must_use]
pub fn name_by_tags(tips: &[TagTip], graph: &CommitGraph, target: &ObjectId) -> Option<String> {
    // git's `name_tips`: set better names first so worse ones spread less —
    // older tags first (`cmp_by_tag_and_age`). git's qsort is unstable for an
    // equal-taggerdate pair (its result then matches ref-name iteration order);
    // we make that deterministic with a lower-name secondary key.
    let mut order: Vec<&TagTip> = tips.iter().collect();
    order.sort_by(|a: &&TagTip, b: &&TagTip| {
        a.taggerdate
            .cmp(&b.taggerdate)
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut names: HashMap<ObjectId, RevName> = HashMap::new();
    for tip in order {
        name_rev_from_tip(tip, graph, &mut names);
    }
    names.get(target).map(render_name)
}
