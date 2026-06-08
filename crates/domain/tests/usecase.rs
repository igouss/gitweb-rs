//! Gherkin-driven BDD harness for the domain use cases.
//!
//! Use cases orchestrate the ports, so the `Given` builds an in-memory fake
//! [`ProjectStore`] (no adapter, no gix) holding hand-written [`ProjectInfo`],
//! the `When` runs the use case against it, and each `Then` asserts one fact
//! about the resulting view-model or the failure. Verifying the orchestration
//! through a fake keeps it fast and isolated; the gix adapter realizes the same
//! port contract under its own conformance specs, and the web layer exercises
//! the two together end-to-end. cucumber supplies its own `main`, so this target
//! sets `harness = false`.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blame::Blame;
use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::{CombinedDiff, Diff};
use gitweb_domain::model::grep::GrepResults;
use gitweb_domain::model::message_body::LogLine;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::patch::Patch;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::model::remote::{Remote, RemoteUrl};
use gitweb_domain::model::settings::{FeatureLayer, FeatureName, Settings, SettingsLayer};
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tree::Tree;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::{
    ArchiveFormat, Page, RenameDetection, Repository, SearchQuery,
};
use gitweb_domain::usecase::heads::{HeadRow, HeadsView, assemble_heads};
use gitweb_domain::usecase::history::{HistoryRow, HistoryView, assemble_history};
use gitweb_domain::usecase::log::{LogRow, LogView, assemble_log};
use gitweb_domain::usecase::project_list::{
    ProjectListRow, ProjectListView, assemble_project_list,
};
use gitweb_domain::usecase::remotes::{RemoteBlock, RemotesView, assemble_remotes};
use gitweb_domain::usecase::shortlog::{ShortlogRow, ShortlogView, assemble_shortlog};
use gitweb_domain::usecase::summary::{SummaryView, assemble_summary};
use gitweb_domain::usecase::tag::{TagView, show_tag};
use gitweb_domain::usecase::tags::{TagRow, TagsView, assemble_tags};

/// An in-memory [`ProjectStore`] over a fixed set of projects. It serves
/// `list` and `info` from its metadata; `open` is never reached by the
/// project-list use case, so it is left unimplemented.
struct FakeStore {
    projects: Vec<ProjectInfo>,
}

impl ProjectStore for FakeStore {
    fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self
            .projects
            .iter()
            .map(|info: &ProjectInfo| Project::new(info.name().to_owned()))
            .collect())
    }

    fn open(&self, _name: &str) -> Result<Box<dyn Repository>, DomainError> {
        unimplemented!("the project-list use case never opens a repository")
    }

    fn info(&self, name: &str) -> Result<ProjectInfo, DomainError> {
        self.projects
            .iter()
            .find(|info: &&ProjectInfo| info.name() == name)
            .cloned()
            .ok_or_else(|| DomainError::NotFound(format!("no project {name}")))
    }

    fn readme_html(&self, _name: &str) -> Result<Option<String>, DomainError> {
        // The summary use case takes its README as a direct input (the boundary
        // reads it from the store), so no use case drives this through the fake.
        Ok(None)
    }
}

#[derive(Debug, Default, World)]
struct UsecaseWorld {
    projects: Vec<ProjectInfo>,
    now: i64,
    settings: Settings,
    result: Option<Result<ProjectListView, DomainError>>,
    head: Option<String>,
    branches: Vec<FakeBranch>,
    heads_result: Option<Result<HeadsView, DomainError>>,
    tags: Vec<FakeTag>,
    tags_result: Option<Result<TagsView, DomainError>>,
    tag_result: Option<Result<TagView, DomainError>>,
    commits: Vec<FakeCommit>,
    head_commit: Option<ObjectId>,
    shortlog_result: Option<Result<ShortlogView, DomainError>>,
    log_result: Option<Result<LogView, DomainError>>,
    history_result: Option<Result<HistoryView, DomainError>>,
    summary_name: Option<String>,
    summary_description: Option<String>,
    summary_owner: Option<String>,
    summary_clone_urls: Vec<String>,
    summary_readme: Option<String>,
    summary_omit_owner: bool,
    summary_prevent_xss: bool,
    summary_base_urls: Vec<String>,
    summary_result: Option<Result<SummaryView, DomainError>>,
    remotes: Vec<Remote>,
    remote_branches: Vec<FakeRemoteBranch>,
    remote_heads_enabled: bool,
    remotes_result: Option<Result<RemotesView, DomainError>>,
}

/// One remote-tracking branch in the fake repository: which remote it belongs to,
/// its branch name (without the remote prefix), the id of its tip commit, and
/// that commit's committer epoch. Listed under `refs/remotes/<remote>/<name>`.
#[derive(Debug, Clone)]
struct FakeRemoteBranch {
    remote: String,
    name: String,
    tip: ObjectId,
    epoch: i64,
}

/// One branch in the fake repository: its short name, the id of its tip commit,
/// and that commit's committer epoch.
#[derive(Debug, Clone)]
struct FakeBranch {
    name: String,
    tip: ObjectId,
    epoch: i64,
}

/// One tag in the fake repository. `ref_target` is what `refs/tags/<name>`
/// points at directly — a tag object for an annotated tag, the tagged object
/// itself for a lightweight one. `object_kind` is the kind of the object the tag
/// ultimately names, and `epoch` is its creation time (the tagger time for an
/// annotated tag, the committer time for a lightweight tag of a commit, ignored
/// for a lightweight tag of a blob or tree). `message` is the full annotated-tag
/// message (its first non-empty line is the listing subject); `has_tagger` is
/// whether the tag object carries a tagger line.
#[derive(Debug, Clone)]
struct FakeTag {
    full_name: String,
    ref_target: ObjectId,
    object: ObjectId,
    object_kind: ObjectKind,
    annotated: bool,
    epoch: i64,
    message: String,
    has_tagger: bool,
}

/// One commit in the fake repository's linear history (newest first as declared):
/// its id, its committer epoch, the author name, and the subject line. For the
/// per-path history, `touches` names the paths this commit changed (the
/// path-limited walk keeps it for any of these) and `present` is the object each
/// path resolves to in this commit's tree — a deleted path is touched but absent.
#[derive(Debug, Clone)]
struct FakeCommit {
    id: ObjectId,
    epoch: i64,
    author: String,
    title: String,
    touches: Vec<String>,
    present: Vec<FakePathEntry>,
}

/// One path present in a fake commit's tree: the path, the object it resolves to,
/// and that object's kind (a blob for a file, a tree for a directory).
#[derive(Debug, Clone)]
struct FakePathEntry {
    path: String,
    oid: ObjectId,
    kind: ObjectKind,
}

/// A deterministic 40-hex object id derived from `seed`, so distinct branch
/// names get distinct tips while a test can still alias two branches onto one
/// commit by deriving both from the same seed.
fn fake_oid(seed: &str) -> ObjectId {
    let mut hex: String = seed.bytes().map(|byte: u8| format!("{byte:02x}")).collect();
    hex.truncate(40);
    while hex.len() < 40 {
        hex.push('0');
    }
    ObjectId::parse(&hex).expect("a 40-character hex object id")
}

/// An in-memory [`Repository`] over a fixed set of branches and tags. It serves
/// the reads the heads and tags use cases make — `head`, `references`,
/// `object_kind`, `find_commit`, `find_tag` — and leaves every other port method
/// unimplemented, since those use cases never reach them.
struct FakeRepository {
    head: Option<String>,
    head_commit: Option<ObjectId>,
    branches: Vec<FakeBranch>,
    tags: Vec<FakeTag>,
    commits: Vec<FakeCommit>,
    remotes: Vec<Remote>,
    remote_branches: Vec<FakeRemoteBranch>,
}

impl FakeRepository {
    fn branch_ref(branch: &FakeBranch) -> Reference {
        Reference::new(
            RefName::new(format!("refs/heads/{}", branch.name)),
            branch.tip.clone(),
        )
    }

    fn remote_branch_ref(branch: &FakeRemoteBranch) -> Reference {
        Reference::new(
            RefName::new(format!("refs/remotes/{}/{}", branch.remote, branch.name)),
            branch.tip.clone(),
        )
    }

    fn tag_ref(tag: &FakeTag) -> Reference {
        Reference::new(RefName::new(tag.full_name.clone()), tag.ref_target.clone())
    }

    /// The committer epoch of whatever commit `oid` names — a branch tip, a
    /// lightweight tag's commit, or a commit declared in this fake's history (so
    /// a HEAD pointed straight at such a commit resolves, as the summary's
    /// last-change read needs).
    fn commit_epoch(&self, oid: &ObjectId) -> Option<i64> {
        self.branches
            .iter()
            .find(|branch: &&FakeBranch| &branch.tip == oid)
            .map(|branch: &FakeBranch| branch.epoch)
            .or_else(|| {
                self.remote_branches
                    .iter()
                    .find(|branch: &&FakeRemoteBranch| &branch.tip == oid)
                    .map(|branch: &FakeRemoteBranch| branch.epoch)
            })
            .or_else(|| {
                self.tags
                    .iter()
                    .find(|tag: &&FakeTag| !tag.annotated && &tag.ref_target == oid)
                    .map(|tag: &FakeTag| tag.epoch)
            })
            .or_else(|| {
                self.commits
                    .iter()
                    .find(|commit: &&FakeCommit| &commit.id == oid)
                    .map(|commit: &FakeCommit| commit.epoch)
            })
    }
}

impl Repository for FakeRepository {
    fn head(&self) -> Result<Reference, DomainError> {
        if let Some(oid) = &self.head_commit {
            return Ok(Reference::new(RefName::new("HEAD".to_owned()), oid.clone()));
        }
        let name: &String = self
            .head
            .as_ref()
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?;
        let branch: &FakeBranch = self
            .branches
            .iter()
            .find(|branch: &&FakeBranch| &branch.name == name)
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?;
        Ok(Self::branch_ref(branch))
    }

    fn references(&self, prefix: &str) -> Result<Vec<Reference>, DomainError> {
        let branches: Vec<Reference> = self.branches.iter().map(Self::branch_ref).collect();
        let tags: Vec<Reference> = self.tags.iter().map(Self::tag_ref).collect();
        let remote_branches: Vec<Reference> = self
            .remote_branches
            .iter()
            .map(Self::remote_branch_ref)
            .collect();
        Ok(branches
            .into_iter()
            .chain(tags)
            .chain(remote_branches)
            .filter(|reference: &Reference| reference.name().full().starts_with(prefix))
            .collect())
    }

    fn remotes(&self) -> Result<Vec<Remote>, DomainError> {
        Ok(self.remotes.clone())
    }

    fn find_commit(&self, oid: &ObjectId) -> Result<Commit, DomainError> {
        let epoch: i64 = self
            .commit_epoch(oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let who: Signature = Signature::parse(&format!("Tester <t@example.com> {epoch} +0000"))
            .expect("a valid fixture signature");
        Ok(Commit::new(
            oid.clone(),
            fake_oid("tree"),
            Vec::new(),
            who.clone(),
            who,
            "msg\n".to_owned(),
        ))
    }

    fn resolve(&self, rev: &str) -> Result<ObjectId, DomainError> {
        let full: String = format!("refs/tags/{rev}");
        if let Some(tag) = self
            .tags
            .iter()
            .find(|tag: &&FakeTag| tag.full_name == full)
        {
            return Ok(tag.ref_target.clone());
        }
        if let Some(branch) = self
            .branches
            .iter()
            .find(|branch: &&FakeBranch| branch.name == rev)
        {
            return Ok(branch.tip.clone());
        }
        Err(DomainError::NotFound(rev.to_owned()))
    }

    fn object_kind(&self, oid: &ObjectId) -> Result<ObjectKind, DomainError> {
        if let Some(tag) = self
            .tags
            .iter()
            .find(|tag: &&FakeTag| &tag.ref_target == oid)
        {
            return Ok(if tag.annotated {
                ObjectKind::Tag
            } else {
                tag.object_kind
            });
        }
        if self
            .branches
            .iter()
            .any(|branch: &FakeBranch| &branch.tip == oid)
        {
            return Ok(ObjectKind::Commit);
        }
        if let Some(entry) = self
            .commits
            .iter()
            .flat_map(|commit: &FakeCommit| &commit.present)
            .find(|entry: &&FakePathEntry| &entry.oid == oid)
        {
            return Ok(entry.kind);
        }
        Err(DomainError::NotFound(oid.as_str().to_owned()))
    }

    fn find_tree(&self, _oid: &ObjectId) -> Result<Tree, DomainError> {
        unimplemented!("the heads use case never reads a tree")
    }

    fn find_blob(&self, _oid: &ObjectId) -> Result<Blob, DomainError> {
        unimplemented!("the heads use case never reads a blob")
    }

    fn find_tag(&self, oid: &ObjectId) -> Result<Tag, DomainError> {
        let tag: &FakeTag = self
            .tags
            .iter()
            .find(|tag: &&FakeTag| tag.annotated && &tag.ref_target == oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let tagger: Option<Signature> = tag.has_tagger.then(|| {
            Signature::parse(&format!("Tagger <tagger@example.com> {} +0000", tag.epoch))
                .expect("a valid fixture tagger signature")
        });
        let name: String = RefName::new(tag.full_name.clone()).short().into_owned();
        Ok(Tag::new(
            tag.ref_target.clone(),
            tag.object.clone(),
            tag.object_kind,
            name,
            tagger,
            tag.message.clone(),
        ))
    }

    fn history(
        &self,
        _start: &ObjectId,
        path: Option<&str>,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError> {
        let commits: Vec<Commit> = self
            .commits
            .iter()
            .filter(|commit: &&FakeCommit| match path {
                None => true,
                Some(wanted) => commit.touches.iter().any(|p: &String| p == wanted),
            })
            .skip(page.skip)
            .take(page.limit)
            .map(|commit: &FakeCommit| {
                let who: Signature = Signature::parse(&format!(
                    "{} <a@example.com> {} +0000",
                    commit.author, commit.epoch
                ))
                .expect("a valid fixture signature");
                Commit::new(
                    commit.id.clone(),
                    fake_oid("tree"),
                    Vec::new(),
                    who.clone(),
                    who,
                    format!("{}\n", commit.title),
                )
            })
            .collect();
        Ok(commits)
    }

    fn path_id(&self, at: &ObjectId, path: &str) -> Result<Option<ObjectId>, DomainError> {
        let entry: Option<ObjectId> = self
            .commits
            .iter()
            .find(|commit: &&FakeCommit| &commit.id == at)
            .and_then(|commit: &FakeCommit| {
                commit
                    .present
                    .iter()
                    .find(|entry: &&FakePathEntry| entry.path == path)
                    .map(|entry: &FakePathEntry| entry.oid.clone())
            });
        Ok(entry)
    }

    fn diff(
        &self,
        _from: Option<&ObjectId>,
        _to: &ObjectId,
        _detection: RenameDetection,
    ) -> Result<Diff, DomainError> {
        unimplemented!("the heads use case never diffs")
    }

    fn patch(
        &self,
        _from: Option<&ObjectId>,
        _to: &ObjectId,
        _detection: RenameDetection,
    ) -> Result<Patch, DomainError> {
        unimplemented!("the heads use case never builds a patch")
    }

    fn combined_diff(&self, _commit: &ObjectId) -> Result<CombinedDiff, DomainError> {
        unimplemented!("the heads use case never builds a combined diff")
    }

    fn blame(&self, _at: &ObjectId, _path: &str) -> Result<Blame, DomainError> {
        unimplemented!("the heads use case never blames")
    }

    fn archive(&self, _tree: &ObjectId, _format: ArchiveFormat) -> Result<Vec<u8>, DomainError> {
        unimplemented!("the heads use case never archives")
    }

    fn search(&self, _query: &SearchQuery, _page: Page) -> Result<Vec<Commit>, DomainError> {
        unimplemented!("the heads use case never searches")
    }

    fn grep(&self, _revision: &ObjectId, _pattern: &str) -> Result<GrepResults, DomainError> {
        unimplemented!("the heads use case never greps")
    }
}

/// The successful view, or a panic if the scenario produced an error.
fn view(world: &UsecaseWorld) -> &ProjectListView {
    world
        .result
        .as_ref()
        .expect("assemble the list first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The failure, or a panic if the scenario produced a success.
fn error(world: &UsecaseWorld) -> &DomainError {
    match world.result.as_ref().expect("assemble the list first") {
        Ok(_) => panic!("expected assembly to fail"),
        Err(failure) => failure,
    }
}

/// The assembled row for `name`, or a panic if it is absent.
fn row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a ProjectListRow {
    view(world)
        .rows()
        .iter()
        .find(|row: &&ProjectListRow| row.name() == name)
        .unwrap_or_else(|| panic!("no row for {name}"))
}

// --- Givens ------------------------------------------------------------------

#[given(regex = r#"^the store has project "([^"]*)"$"#)]
fn store_has_project(world: &mut UsecaseWorld, name: String) {
    world.projects.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the store has project "([^"]*)" last changed at (\d+)$"#)]
fn store_has_aged_project(world: &mut UsecaseWorld, name: String, epoch: i64) {
    world
        .projects
        .push(ProjectInfo::named(name).with_last_activity(epoch));
}

#[given(regex = r#"^the store has project "([^"]*)" with no commits$"#)]
fn store_has_uncommitted_project(world: &mut UsecaseWorld, name: String) {
    world.projects.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the store has project "([^"]*)" described as "(.*)" owned by "([^"]*)"$"#)]
fn store_has_described_project(
    world: &mut UsecaseWorld,
    name: String,
    description: String,
    owner: String,
) {
    world.projects.push(
        ProjectInfo::named(name)
            .with_description(description)
            .with_owner(owner),
    );
}

#[given(regex = r"^the current time is (\d+)$")]
fn current_time_is(world: &mut UsecaseWorld, now: i64) {
    world.now = now;
}

// --- Whens -------------------------------------------------------------------

#[when("I assemble the project list")]
fn assemble_default(world: &mut UsecaseWorld) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
    };
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        None,
        world.now,
    ));
}

#[when(regex = r#"^I assemble the project list ordered by "([^"]*)"$"#)]
fn assemble_ordered(world: &mut UsecaseWorld, order: String) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
    };
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        Some(&order),
        world.now,
    ));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r#"^the listed projects are "(.*)"$"#)]
fn listed_projects_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = view(world)
        .rows()
        .iter()
        .map(|row: &ProjectListRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("assembling fails as not found")]
fn fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(error(world), DomainError::NotFound(_)));
}

#[then("assembling fails as invalid")]
fn fails_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(error(world), DomainError::Invalid(_)));
}

#[then(regex = r#"^the project "([^"]*)" shows description "(.*)"$"#)]
fn project_shows_description(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(row(world, &name).description(), Some(expected.as_str()));
}

#[then(regex = r#"^the project "([^"]*)" shows owner "([^"]*)"$"#)]
fn project_shows_owner(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(row(world, &name).owner(), Some(expected.as_str()));
}

#[then(regex = r#"^the project "([^"]*)" shows the age "([^"]*)"$"#)]
fn project_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = row(world, &name)
        .age()
        .expect("the project has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the project "([^"]*)" has no age$"#)]
fn project_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(row(world, &name).age(), None);
}

// --- heads: accessors --------------------------------------------------------

/// The assembled heads view, or a panic if the scenario produced an error.
fn heads_view(world: &UsecaseWorld) -> &HeadsView {
    world
        .heads_result
        .as_ref()
        .expect("assemble the heads first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled head row for `name`, or a panic if it is absent.
fn head_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a HeadRow {
    heads_view(world)
        .rows()
        .iter()
        .find(|row: &&HeadRow| row.name() == name)
        .unwrap_or_else(|| panic!("no head row for {name}"))
}

/// The committer epoch of an already-declared branch, for aliasing a second
/// branch onto the same commit.
fn branch_epoch(world: &UsecaseWorld, name: &str) -> i64 {
    world
        .branches
        .iter()
        .find(|branch: &&FakeBranch| branch.name == name)
        .unwrap_or_else(|| panic!("no branch {name} declared yet"))
        .epoch
}

// --- heads: Givens -----------------------------------------------------------

#[given(regex = r#"^the repository HEAD is branch "([^"]*)"$"#)]
fn head_is_branch(world: &mut UsecaseWorld, name: String) {
    world.head = Some(name);
}

#[given(regex = r#"^the repository HEAD is the unborn branch "([^"]*)"$"#)]
fn head_is_unborn(world: &mut UsecaseWorld, name: String) {
    world.head = Some(name);
}

#[given(regex = r#"^the repository has branch "([^"]*)" committed at (\d+)$"#)]
fn repo_has_branch(world: &mut UsecaseWorld, name: String, epoch: i64) {
    let tip: ObjectId = fake_oid(&name);
    world.branches.push(FakeBranch { name, tip, epoch });
}

#[given(regex = r#"^the repository has branch "([^"]*)" at the same commit as "([^"]*)"$"#)]
fn repo_has_aliased_branch(world: &mut UsecaseWorld, name: String, other: String) {
    let tip: ObjectId = fake_oid(&other);
    let epoch: i64 = branch_epoch(world, &other);
    world.branches.push(FakeBranch { name, tip, epoch });
}

// --- heads: When -------------------------------------------------------------

#[when("I assemble the heads")]
fn assemble_the_heads(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.heads_result = Some(assemble_heads(&repo, world.now));
}

// --- heads: Thens ------------------------------------------------------------

#[then("no heads are listed")]
fn no_heads_listed(world: &mut UsecaseWorld) {
    assert!(heads_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed heads are "(.*)"$"#)]
fn listed_heads_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = heads_view(world)
        .rows()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the head "([^"]*)" is current$"#)]
fn head_is_current(world: &mut UsecaseWorld, name: String) {
    assert!(head_row(world, &name).current());
}

#[then(regex = r#"^the head "([^"]*)" is not current$"#)]
fn head_is_not_current(world: &mut UsecaseWorld, name: String) {
    assert!(!head_row(world, &name).current());
}

#[then(regex = r#"^the head "([^"]*)" shows the age "([^"]*)"$"#)]
fn head_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = head_row(world, &name)
        .age()
        .expect("the head has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the head "([^"]*)" has no age$"#)]
fn head_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(head_row(world, &name).age(), None);
}

// --- tags: accessors ---------------------------------------------------------

/// The assembled tags view, or a panic if the scenario produced an error.
fn tags_view(world: &UsecaseWorld) -> &TagsView {
    world
        .tags_result
        .as_ref()
        .expect("assemble the tags first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled tag row for `name`, or a panic if it is absent.
fn tag_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a TagRow {
    tags_view(world)
        .rows()
        .iter()
        .find(|row: &&TagRow| row.name() == name)
        .unwrap_or_else(|| panic!("no tag row for {name}"))
}

/// Derives the two ids an annotated tag needs — the tag object the ref points at
/// and the object that tag peels to — from its name, so each stays distinct.
fn annotated_ids(name: &str) -> (ObjectId, ObjectId) {
    (
        fake_oid(&format!("tagobj-{name}")),
        fake_oid(&format!("target-{name}")),
    )
}

// --- tags: Givens ------------------------------------------------------------

#[given(
    regex = r#"^an annotated tag "([^"]*)" of a (commit|blob|tree) tagged at (\d+) with subject "(.*)"$"#
)]
fn repo_has_annotated_tag(
    world: &mut UsecaseWorld,
    name: String,
    kind: String,
    epoch: i64,
    subject: String,
) {
    let (ref_target, object): (ObjectId, ObjectId) = annotated_ids(&name);
    let object_kind: ObjectKind = ObjectKind::parse(&kind).expect("a valid object kind");
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target,
        object,
        object_kind,
        annotated: true,
        epoch,
        message: format!("{subject}\n"),
        has_tagger: true,
    });
}

#[given(regex = r#"^an annotated tag "([^"]*)" of a commit with no tagger$"#)]
fn repo_has_taggerless_annotated_tag(world: &mut UsecaseWorld, name: String) {
    let (ref_target, object): (ObjectId, ObjectId) = annotated_ids(&name);
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target,
        object,
        object_kind: ObjectKind::Commit,
        annotated: true,
        epoch: 0,
        message: "anonymous tag\n".to_owned(),
        has_tagger: false,
    });
}

#[given(regex = r#"^an annotated tag "([^"]*)" of a commit with a two-line message$"#)]
fn repo_has_multiline_annotated_tag(world: &mut UsecaseWorld, name: String) {
    let (ref_target, object): (ObjectId, ObjectId) = annotated_ids(&name);
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target,
        object,
        object_kind: ObjectKind::Commit,
        annotated: true,
        epoch: 999_400,
        message: "First line\nSecond line\n".to_owned(),
        has_tagger: true,
    });
}

#[given(regex = r#"^a lightweight tag "([^"]*)" on a commit at (\d+)$"#)]
fn repo_has_lightweight_commit_tag(world: &mut UsecaseWorld, name: String, epoch: i64) {
    let target: ObjectId = fake_oid(&format!("lw-{name}"));
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target: target.clone(),
        object: target,
        object_kind: ObjectKind::Commit,
        annotated: false,
        epoch,
        message: String::new(),
        has_tagger: false,
    });
}

#[given(regex = r#"^a lightweight tag "([^"]*)" on a (blob|tree)$"#)]
fn repo_has_lightweight_object_tag(world: &mut UsecaseWorld, name: String, kind: String) {
    let target: ObjectId = fake_oid(&format!("lw-{name}"));
    let object_kind: ObjectKind = ObjectKind::parse(&kind).expect("a valid object kind");
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target: target.clone(),
        object: target,
        object_kind,
        annotated: false,
        epoch: 0,
        message: String::new(),
        has_tagger: false,
    });
}

// --- tags: When --------------------------------------------------------------

#[when("I assemble the tags")]
fn assemble_the_tags(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.tags_result = Some(assemble_tags(&repo, world.now));
}

// --- tags: Thens -------------------------------------------------------------

#[then("no tags are listed")]
fn no_tags_listed(world: &mut UsecaseWorld) {
    assert!(tags_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed tags are "(.*)"$"#)]
fn listed_tags_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = tags_view(world)
        .rows()
        .iter()
        .map(|row: &TagRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the tag "([^"]*)" is annotated$"#)]
fn tag_is_annotated(world: &mut UsecaseWorld, name: String) {
    assert!(tag_row(world, &name).annotated());
}

#[then(regex = r#"^the tag "([^"]*)" is not annotated$"#)]
fn tag_is_not_annotated(world: &mut UsecaseWorld, name: String) {
    assert!(!tag_row(world, &name).annotated());
}

#[then(regex = r#"^the tag "([^"]*)" has subject "(.*)"$"#)]
fn tag_has_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(tag_row(world, &name).subject(), Some(expected.as_str()));
}

#[then(regex = r#"^the tag "([^"]*)" has no subject$"#)]
fn tag_has_no_subject(world: &mut UsecaseWorld, name: String) {
    assert_eq!(tag_row(world, &name).subject(), None);
}

#[then(regex = r#"^the tag "([^"]*)" has reftype "([^"]*)"$"#)]
fn tag_has_reftype(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(tag_row(world, &name).reftype().as_str(), expected);
}

#[then(regex = r#"^the tag "([^"]*)" shows the age "([^"]*)"$"#)]
fn tag_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = tag_row(world, &name)
        .age()
        .expect("the tag has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the tag "([^"]*)" has no age$"#)]
fn tag_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(tag_row(world, &name).age(), None);
}

// --- single tag view: accessors ----------------------------------------------

/// The resolved single-tag view, or a panic if the scenario produced an error.
fn tag_show_view(world: &UsecaseWorld) -> &TagView {
    world
        .tag_result
        .as_ref()
        .expect("show the tag first")
        .as_ref()
        .expect("the tag resolved")
}

/// The single-tag failure, or a panic if the scenario produced a success.
fn tag_show_error(world: &UsecaseWorld) -> &DomainError {
    match world.tag_result.as_ref().expect("show the tag first") {
        Ok(_) => panic!("expected showing the tag to fail"),
        Err(failure) => failure,
    }
}

// --- single tag view: When ---------------------------------------------------

#[when(regex = r#"^I show the tag "([^"]*)"$"#)]
fn show_the_single_tag(world: &mut UsecaseWorld, hash: String) {
    let repo: FakeRepository = fake_repo(world);
    world.tag_result = Some(show_tag(&repo, &hash));
}

// --- single tag view: Thens --------------------------------------------------

#[then(regex = r#"^the tag view name is "([^"]*)"$"#)]
fn tag_view_name_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_view(world).name(), expected);
}

#[then(regex = r#"^the tag view points at a "([^"]*)"$"#)]
fn tag_view_points_at(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_view(world).object_kind().as_str(), expected);
}

#[then("the tag view has a tagger")]
fn tag_view_has_tagger(world: &mut UsecaseWorld) {
    assert!(tag_show_view(world).tagger().is_some());
}

#[then("the tag view has no tagger")]
fn tag_view_has_no_tagger(world: &mut UsecaseWorld) {
    assert!(tag_show_view(world).tagger().is_none());
}

#[then(regex = r#"^the tag view tagger shows the date "([^"]*)"$"#)]
fn tag_view_tagger_date(world: &mut UsecaseWorld, expected: String) {
    let date: String = tag_show_view(world)
        .tagger()
        .expect("the tag view has a tagger")
        .timestamp()
        .rfc2822();
    assert_eq!(date, expected);
}

#[then(regex = r#"^the tag view message is "([^"]*)"$"#)]
fn tag_view_message_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_view(world).message().trim_end(), expected);
}

#[then(regex = r"^the tag view message has (\d+) lines$")]
fn tag_view_message_line_count(world: &mut UsecaseWorld, expected: usize) {
    assert_eq!(tag_show_view(world).message().lines().count(), expected);
}

#[then(regex = r#"^the tag view message line (\d+) is "([^"]*)"$"#)]
fn tag_view_message_line_is(world: &mut UsecaseWorld, number: usize, expected: String) {
    let line: &str = tag_show_view(world)
        .message()
        .lines()
        .nth(number - 1)
        .expect("the message has that line");
    assert_eq!(line, expected);
}

#[then(regex = r#"^showing the tag fails with "([^"]*)"$"#)]
fn tag_view_fails_with(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_error(world).message(), expected);
}

#[then("showing the tag fails as not found")]
fn tag_view_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(tag_show_error(world), DomainError::NotFound(_)));
}

// --- shortlog: accessors -----------------------------------------------------

/// The assembled shortlog view, or a panic if the scenario produced an error.
fn shortlog_view(world: &UsecaseWorld) -> &ShortlogView {
    world
        .shortlog_result
        .as_ref()
        .expect("assemble the shortlog first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled shortlog row for the commit declared as `name`, or a panic if
/// it is absent. The fake derives each commit id from its declared name.
fn shortlog_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a ShortlogRow {
    let id: ObjectId = fake_oid(name);
    shortlog_view(world)
        .rows()
        .iter()
        .find(|row: &&ShortlogRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no shortlog row for {name}"))
}

// --- shortlog: Givens --------------------------------------------------------

#[given(regex = r#"^the repository HEAD is at commit "([^"]*)"$"#)]
fn head_is_at_commit(world: &mut UsecaseWorld, name: String) {
    world.head_commit = Some(fake_oid(&name));
}

#[given(regex = r#"^a commit "([^"]*)" at epoch (\d+) by "([^"]*)" titled "(.*)"$"#)]
fn repo_has_commit(
    world: &mut UsecaseWorld,
    name: String,
    epoch: i64,
    author: String,
    title: String,
) {
    world.commits.push(FakeCommit {
        id: fake_oid(&name),
        epoch,
        author,
        title,
        touches: Vec::new(),
        present: Vec::new(),
    });
}

// --- shortlog: Whens ---------------------------------------------------------

#[when(regex = r"^I assemble the shortlog of the default branch with page size (\d+)$")]
fn assemble_default_shortlog(world: &mut UsecaseWorld, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.shortlog_result = Some(assemble_shortlog(
        &repo,
        None,
        world.now,
        Page::new(0, size),
    ));
}

#[when(regex = r#"^I assemble the shortlog of "([^"]*)" with page size (\d+)$"#)]
fn assemble_rev_shortlog(world: &mut UsecaseWorld, rev: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.shortlog_result = Some(assemble_shortlog(
        &repo,
        Some(&rev),
        world.now,
        Page::new(0, size),
    ));
}

// --- shortlog: Thens ---------------------------------------------------------

#[then("no commits are listed")]
fn no_commits_listed(world: &mut UsecaseWorld) {
    assert!(shortlog_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed commits are "(.*)"$"#)]
fn listed_commits_are(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = shortlog_view(world)
        .rows()
        .iter()
        .map(|row: &ShortlogRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then("the shortlog has a further page")]
fn shortlog_has_further_page(world: &mut UsecaseWorld) {
    assert!(shortlog_view(world).has_more());
}

#[then("the shortlog has no further page")]
fn shortlog_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!shortlog_view(world).has_more());
}

#[then(regex = r#"^the commit "([^"]*)" is by "([^"]*)"$"#)]
fn commit_is_by(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).author(), expected);
}

#[then(regex = r#"^the commit "([^"]*)" author shortens to "(.*)"$"#)]
fn commit_author_shortens_to(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).author_short(), expected);
}

#[then(regex = r#"^the commit "([^"]*)" shows the subject "(.*)"$"#)]
fn commit_shows_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).title(), expected);
}

#[then(regex = r#"^the commit "([^"]*)" date cell shows "(.*)"$"#)]
fn commit_date_cell_shows(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).date().displayed(), expected);
}

// --- log: accessors ----------------------------------------------------------

/// The assembled log view, or a panic if the scenario produced an error.
fn log_view(world: &UsecaseWorld) -> &LogView {
    world
        .log_result
        .as_ref()
        .expect("assemble the log first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled log row for the commit declared as `name`, or a panic if it is
/// absent. The fake derives each commit id from its declared name.
fn log_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a LogRow {
    let id: ObjectId = fake_oid(name);
    log_view(world)
        .rows()
        .iter()
        .find(|row: &&LogRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no log row for {name}"))
}

/// Builds the fake repository from the world's declared refs and commits.
fn fake_repo(world: &UsecaseWorld) -> FakeRepository {
    FakeRepository {
        head: world.head.clone(),
        head_commit: world.head_commit.clone(),
        branches: world.branches.clone(),
        tags: world.tags.clone(),
        commits: world.commits.clone(),
        remotes: world.remotes.clone(),
        remote_branches: world.remote_branches.clone(),
    }
}

// --- log: Whens --------------------------------------------------------------

#[when(regex = r"^I assemble the log of the default branch with page size (\d+)$")]
fn assemble_default_log(world: &mut UsecaseWorld, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.log_result = Some(assemble_log(&repo, None, world.now, Page::new(0, size)));
}

#[when(regex = r#"^I assemble the log of "([^"]*)" with page size (\d+)$"#)]
fn assemble_rev_log(world: &mut UsecaseWorld, rev: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.log_result = Some(assemble_log(
        &repo,
        Some(&rev),
        world.now,
        Page::new(0, size),
    ));
}

// --- log: Thens --------------------------------------------------------------

#[then("no log entries are listed")]
fn no_log_entries(world: &mut UsecaseWorld) {
    assert!(log_view(world).rows().is_empty());
}

#[then(regex = r#"^the logged commits are "(.*)"$"#)]
fn logged_commits_are(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = log_view(world)
        .rows()
        .iter()
        .map(|row: &LogRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then("the log has a further page")]
fn log_has_further_page(world: &mut UsecaseWorld) {
    assert!(log_view(world).has_more());
}

#[then("the log has no further page")]
fn log_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!log_view(world).has_more());
}

#[then(regex = r#"^the log entry "([^"]*)" is by "([^"]*)"$"#)]
fn log_entry_is_by(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).author(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" shows the title "(.*)"$"#)]
fn log_entry_shows_title(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).title(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" shows the age "(.*)"$"#)]
fn log_entry_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).age(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" was authored on "([^"]*)"$"#)]
fn log_entry_authored_on(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).timestamp().utc_date(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" body begins "(.*)"$"#)]
fn log_entry_body_begins(world: &mut UsecaseWorld, name: String, expected: String) {
    let first: &LogLine = log_row(world, &name)
        .comment()
        .first()
        .expect("the log body has at least one line");
    assert_eq!(first, &LogLine::Text(expected));
}

// --- summary: accessors ------------------------------------------------------

/// The assembled summary view, or a panic if the scenario produced an error.
fn summary_view(world: &UsecaseWorld) -> &SummaryView {
    world
        .summary_result
        .as_ref()
        .expect("assemble the summary first")
        .as_ref()
        .expect("assembly succeeded")
}

// --- summary: Givens ---------------------------------------------------------

#[given(regex = r#"^the project is named "([^"]*)"$"#)]
fn summary_project_named(world: &mut UsecaseWorld, name: String) {
    world.summary_name = Some(name);
}

#[given(regex = r#"^the project is described as "(.*)"$"#)]
fn summary_project_described(world: &mut UsecaseWorld, text: String) {
    world.summary_description = Some(text);
}

#[given(regex = r#"^the project is owned by "([^"]*)"$"#)]
fn summary_project_owned_by(world: &mut UsecaseWorld, owner: String) {
    world.summary_owner = Some(owner);
}

#[given(regex = r#"^the project has clone url "([^"]*)"$"#)]
fn summary_project_clone_url(world: &mut UsecaseWorld, url: String) {
    world.summary_clone_urls.push(url);
}

#[given(regex = r#"^the project README is "(.*)"$"#)]
fn summary_project_readme(world: &mut UsecaseWorld, contents: String) {
    world.summary_readme = Some(contents);
}

#[given("owner display is omitted")]
fn summary_omit_owner(world: &mut UsecaseWorld) {
    world.summary_omit_owner = true;
}

#[given("XSS prevention is on")]
fn summary_prevent_xss(world: &mut UsecaseWorld) {
    world.summary_prevent_xss = true;
}

#[given(regex = r#"^the git base URL is "([^"]*)"$"#)]
fn summary_base_url(world: &mut UsecaseWorld, base: String) {
    world.summary_base_urls.push(base);
}

// --- summary: When -----------------------------------------------------------

#[when("I assemble the summary")]
fn assemble_the_summary(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    let name: String = world
        .summary_name
        .clone()
        .unwrap_or_else(|| "repo".to_owned());
    let mut info: ProjectInfo = ProjectInfo::named(name);
    if let Some(description) = &world.summary_description {
        info = info.with_description(description.clone());
    }
    if let Some(owner) = &world.summary_owner {
        info = info.with_owner(owner.clone());
    }
    for url in &world.summary_clone_urls {
        info = info.with_clone_url(url.clone());
    }
    let layer: SettingsLayer = SettingsLayer {
        omit_owner: Some(world.summary_omit_owner),
        prevent_xss: Some(world.summary_prevent_xss),
        git_base_url_list: Some(world.summary_base_urls.clone()),
        ..SettingsLayer::default()
    };
    let settings: Settings = Settings::resolve(&[layer]);
    world.summary_result = Some(assemble_summary(
        &repo,
        &info,
        world.summary_readme.as_deref(),
        &settings,
        world.now,
    ));
}

// --- summary: Thens ----------------------------------------------------------

#[then(regex = r#"^the summary description is "(.*)"$"#)]
fn summary_description_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).description(), expected);
}

#[then(regex = r#"^the summary shows the owner "([^"]*)"$"#)]
fn summary_shows_owner(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).owner(), Some(expected.as_str()));
}

#[then("the summary shows no owner")]
fn summary_shows_no_owner(world: &mut UsecaseWorld) {
    assert_eq!(summary_view(world).owner(), None);
}

#[then(regex = r#"^the summary last change date is "([^"]*)"$"#)]
fn summary_last_change_date(world: &mut UsecaseWorld, expected: String) {
    let date: String = summary_view(world)
        .last_change()
        .expect("the summary has a last change")
        .utc_date();
    assert_eq!(date, expected);
}

#[then("the summary shows no last change")]
fn summary_shows_no_last_change(world: &mut UsecaseWorld) {
    assert_eq!(summary_view(world).last_change(), None);
}

#[then(regex = r#"^the summary clone urls are "(.*)"$"#)]
fn summary_clone_urls_are(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).clone_urls().join(", "), expected);
}

#[then("the summary advertises no clone urls")]
fn summary_advertises_no_clone_urls(world: &mut UsecaseWorld) {
    assert!(summary_view(world).clone_urls().is_empty());
}

#[then(regex = r#"^the summary README is "(.*)"$"#)]
fn summary_readme_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).readme(), Some(expected.as_str()));
}

#[then("the summary includes no README")]
fn summary_includes_no_readme(world: &mut UsecaseWorld) {
    assert_eq!(summary_view(world).readme(), None);
}

#[then(regex = r#"^the summary shortlog lists "(.*)"$"#)]
fn summary_shortlog_lists(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = summary_view(world)
        .shortlog()
        .rows()
        .iter()
        .map(|row: &ShortlogRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then("the summary shortlog is empty")]
fn summary_shortlog_is_empty(world: &mut UsecaseWorld) {
    assert!(summary_view(world).shortlog().rows().is_empty());
}

#[then(regex = r#"^the summary lists tags "(.*)"$"#)]
fn summary_lists_tags(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = summary_view(world)
        .tags()
        .shown()
        .iter()
        .map(|row: &TagRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("the summary tags section is not truncated")]
fn summary_tags_not_truncated(world: &mut UsecaseWorld) {
    assert!(!summary_view(world).tags().is_truncated());
}

#[then(regex = r#"^the summary lists heads "(.*)"$"#)]
fn summary_lists_heads(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = summary_view(world)
        .heads()
        .shown()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("the summary heads section is not truncated")]
fn summary_heads_not_truncated(world: &mut UsecaseWorld) {
    assert!(!summary_view(world).heads().is_truncated());
}

// --- remotes: accessors ------------------------------------------------------

/// The assembled remotes view, or a panic if the scenario produced an error.
fn remotes_view(world: &UsecaseWorld) -> &RemotesView {
    world
        .remotes_result
        .as_ref()
        .expect("assemble the remotes first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The remotes failure, or a panic if the scenario produced a success.
fn remotes_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .remotes_result
        .as_ref()
        .expect("assemble the remotes first")
    {
        Ok(_) => panic!("expected assembling the remotes to fail"),
        Err(failure) => failure,
    }
}

/// The assembled block for remote `name`, or a panic if it is absent.
fn remote_block<'a>(world: &'a UsecaseWorld, name: &str) -> &'a RemoteBlock {
    remotes_view(world)
        .blocks()
        .iter()
        .find(|block: &&RemoteBlock| block.name() == name)
        .unwrap_or_else(|| panic!("no remote block for {name}"))
}

/// The tracking-branch row named `branch` in remote `remote`, or a panic.
fn tracking_row<'a>(world: &'a UsecaseWorld, remote: &str, branch: &str) -> &'a HeadRow {
    remote_block(world, remote)
        .heads()
        .iter()
        .find(|row: &&HeadRow| row.name() == branch)
        .unwrap_or_else(|| panic!("no tracking branch {branch} on remote {remote}"))
}

/// Serializes one URL line to its `role value` form (`missing` for the
/// placeholder), so a scenario asserts a block's whole URL set with one string.
fn url_line_string(line: &RemoteUrl) -> String {
    match line {
        RemoteUrl::Combined(url) => format!("combined {url}"),
        RemoteUrl::Fetch(url) => format!("fetch {url}"),
        RemoteUrl::Push(url) => format!("push {url}"),
        RemoteUrl::Missing => "missing".to_owned(),
    }
}

/// The settings the remotes use case reads: the `remote_heads` feature on or off,
/// matching gitweb's `gitweb_check_feature('remote_heads')` gate.
fn remotes_settings(world: &UsecaseWorld) -> Settings {
    if !world.remote_heads_enabled {
        return Settings::builtin();
    }
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::RemoteHeads,
        FeatureLayer {
            default: Some(vec!["1".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

// --- remotes: Givens ---------------------------------------------------------

#[given("the remote_heads feature is enabled")]
fn remote_heads_enabled(world: &mut UsecaseWorld) {
    world.remote_heads_enabled = true;
}

#[given("the remote_heads feature is disabled")]
fn remote_heads_disabled(world: &mut UsecaseWorld) {
    world.remote_heads_enabled = false;
}

#[given(regex = r#"^a remote "([^"]*)" fetching from "([^"]*)" pushing to "([^"]*)"$"#)]
fn given_usecase_remote_fetch_push(
    world: &mut UsecaseWorld,
    name: String,
    fetch: String,
    push: String,
) {
    world
        .remotes
        .push(Remote::new(name, Some(fetch), Some(push)));
}

#[given(regex = r#"^a remote "([^"]*)" fetching from "([^"]*)"$"#)]
fn given_usecase_remote_fetch(world: &mut UsecaseWorld, name: String, fetch: String) {
    world.remotes.push(Remote::new(name, Some(fetch), None));
}

#[given(regex = r#"^the remote "([^"]*)" tracks branch "([^"]*)" committed at (\d+)$"#)]
fn remote_tracks_branch(world: &mut UsecaseWorld, remote: String, branch: String, epoch: i64) {
    let tip: ObjectId = fake_oid(&format!("{remote}/{branch}"));
    world.remote_branches.push(FakeRemoteBranch {
        remote,
        name: branch,
        tip,
        epoch,
    });
}

#[given(regex = r#"^the remote "([^"]*)" tracks branch "([^"]*)" at commit "([^"]*)"$"#)]
fn remote_tracks_branch_at_commit(
    world: &mut UsecaseWorld,
    remote: String,
    branch: String,
    commit: String,
) {
    let tip: ObjectId = fake_oid(&commit);
    world.remote_branches.push(FakeRemoteBranch {
        remote,
        name: branch,
        tip,
        epoch: 500_000,
    });
}

// --- remotes: Whens ----------------------------------------------------------

#[when("I assemble the remotes")]
fn assemble_the_remotes(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    let settings: Settings = remotes_settings(world);
    world.remotes_result = Some(assemble_remotes(&repo, &settings, None, world.now));
}

#[when(regex = r#"^I assemble the remote "([^"]*)"$"#)]
fn assemble_one_remote(world: &mut UsecaseWorld, name: String) {
    let repo: FakeRepository = fake_repo(world);
    let settings: Settings = remotes_settings(world);
    world.remotes_result = Some(assemble_remotes(&repo, &settings, Some(&name), world.now));
}

// --- remotes: Thens ----------------------------------------------------------

#[then(regex = r#"^the shown remotes are "(.*)"$"#)]
fn shown_remotes_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = remotes_view(world)
        .blocks()
        .iter()
        .map(|block: &RemoteBlock| block.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the remote "([^"]*)" URL lines are "(.*)"$"#)]
fn remote_url_lines_are(world: &mut UsecaseWorld, name: String, expected: String) {
    let rendered: String = remote_block(world, &name)
        .urls()
        .iter()
        .map(url_line_string)
        .collect::<Vec<String>>()
        .join(", ");
    assert_eq!(rendered, expected);
}

#[then(regex = r#"^the remote "([^"]*)" tracks no branches$"#)]
fn remote_tracks_no_branches(world: &mut UsecaseWorld, name: String) {
    assert!(remote_block(world, &name).heads().is_empty());
}

#[then(regex = r#"^the remote "([^"]*)" tracks "(.*)"$"#)]
fn remote_tracks(world: &mut UsecaseWorld, name: String, expected: String) {
    let names: Vec<&str> = remote_block(world, &name)
        .heads()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the remote "([^"]*)" tracking branch "([^"]*)" shows the age "([^"]*)"$"#)]
fn remote_tracking_age(world: &mut UsecaseWorld, remote: String, branch: String, expected: String) {
    let humanized: String = tracking_row(world, &remote, &branch)
        .age()
        .expect("the tracking branch has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the remote "([^"]*)" tracking branch "([^"]*)" is current$"#)]
fn remote_tracking_current(world: &mut UsecaseWorld, remote: String, branch: String) {
    assert!(tracking_row(world, &remote, &branch).current());
}

#[then("the remotes view is the single-remote view")]
fn remotes_view_is_single(world: &mut UsecaseWorld) {
    assert!(remotes_view(world).is_single());
}

#[then("assembling the remotes fails as forbidden")]
fn remotes_fails_forbidden(world: &mut UsecaseWorld) {
    assert!(matches!(remotes_error(world), DomainError::Forbidden(_)));
}

#[then("assembling the remotes fails as not found")]
fn remotes_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(remotes_error(world), DomainError::NotFound(_)));
}

// --- history: accessors ------------------------------------------------------

/// The assembled history view, or a panic if the scenario produced an error.
fn history_view(world: &UsecaseWorld) -> &HistoryView {
    world
        .history_result
        .as_ref()
        .expect("assemble the history first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The history failure, or a panic if the scenario produced a success.
fn history_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .history_result
        .as_ref()
        .expect("assemble the history first")
    {
        Ok(_) => panic!("expected assembling the history to fail"),
        Err(failure) => failure,
    }
}

/// The history row for the commit declared as `name`, or a panic if it is absent.
fn history_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a HistoryRow {
    let id: ObjectId = fake_oid(name);
    history_view(world)
        .rows()
        .iter()
        .find(|row: &&HistoryRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no history row for {name}"))
}

/// The fake commit declared as `name`, mutable so a later step can record the
/// paths it changed.
fn commit_named_mut<'a>(world: &'a mut UsecaseWorld, name: &str) -> &'a mut FakeCommit {
    let id: ObjectId = fake_oid(name);
    world
        .commits
        .iter_mut()
        .find(|commit: &&mut FakeCommit| commit.id == id)
        .unwrap_or_else(|| panic!("no commit named {name}"))
}

// --- history: Givens ---------------------------------------------------------

#[given(regex = r#"^commit "([^"]*)" changes file "([^"]*)" to blob "([^"]*)"$"#)]
fn commit_changes_file(world: &mut UsecaseWorld, name: String, path: String, blob: String) {
    let oid: ObjectId = fake_oid(&blob);
    let commit: &mut FakeCommit = commit_named_mut(world, &name);
    commit.touches.push(path.clone());
    commit.present.push(FakePathEntry {
        path,
        oid,
        kind: ObjectKind::Blob,
    });
}

#[given(regex = r#"^commit "([^"]*)" changes directory "([^"]*)" to tree "([^"]*)"$"#)]
fn commit_changes_directory(world: &mut UsecaseWorld, name: String, path: String, tree: String) {
    let oid: ObjectId = fake_oid(&tree);
    let commit: &mut FakeCommit = commit_named_mut(world, &name);
    commit.touches.push(path.clone());
    commit.present.push(FakePathEntry {
        path,
        oid,
        kind: ObjectKind::Tree,
    });
}

#[given(regex = r#"^commit "([^"]*)" deletes "([^"]*)"$"#)]
fn commit_deletes(world: &mut UsecaseWorld, name: String, path: String) {
    commit_named_mut(world, &name).touches.push(path);
}

// --- history: Whens ----------------------------------------------------------

#[when(
    regex = r#"^I assemble the history of "([^"]*)" from the default branch with page size (\d+)$"#
)]
fn assemble_default_history(world: &mut UsecaseWorld, path: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.history_result = Some(assemble_history(
        &repo,
        None,
        &path,
        world.now,
        Page::new(0, size),
    ));
}

#[when(regex = r#"^I assemble the history of "([^"]*)" from "([^"]*)" with page size (\d+)$"#)]
fn assemble_rev_history(world: &mut UsecaseWorld, path: String, rev: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.history_result = Some(assemble_history(
        &repo,
        Some(&rev),
        &path,
        world.now,
        Page::new(0, size),
    ));
}

// --- history: Thens ----------------------------------------------------------

#[then("assembling the history fails")]
fn history_fails(world: &mut UsecaseWorld) {
    assert!(matches!(history_error(world), DomainError::Backend(_)));
}

#[then(regex = r#"^the history lists "(.*)"$"#)]
fn history_lists(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = history_view(world)
        .rows()
        .iter()
        .map(|row: &HistoryRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then(regex = r#"^the history file name is "([^"]*)"$"#)]
fn history_file_name_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(history_view(world).file_name(), expected);
}

#[then(regex = r#"^the history file type is "([^"]*)"$"#)]
fn history_file_type_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(history_view(world).file_type().as_str(), expected);
}

#[then(regex = r#"^the history current blob is "([^"]*)"$"#)]
fn history_current_blob_is(world: &mut UsecaseWorld, blob: String) {
    assert_eq!(history_view(world).file_hash(), fake_oid(&blob).as_str());
}

#[then("the history has a further page")]
fn history_has_further_page(world: &mut UsecaseWorld) {
    assert!(history_view(world).has_more());
}

#[then("the history has no further page")]
fn history_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!history_view(world).has_more());
}

#[then(regex = r#"^the history row "([^"]*)" is by "([^"]*)"$"#)]
fn history_row_is_by(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).author(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" shows the subject "(.*)"$"#)]
fn history_row_shows_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).title(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" date cell shows "(.*)"$"#)]
fn history_row_date_cell_shows(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).date().displayed(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" author shortens to "(.*)"$"#)]
fn history_row_author_shortens_to(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).author_short(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" offers a diff to current "([^"]*)"$"#)]
fn history_row_offers_diff(world: &mut UsecaseWorld, name: String, blob: String) {
    assert_eq!(
        history_row(world, &name).diff_to_current(),
        Some(fake_oid(&blob).as_str())
    );
}

#[then(regex = r#"^the history row "([^"]*)" offers no diff to current$"#)]
fn history_row_offers_no_diff(world: &mut UsecaseWorld, name: String) {
    assert_eq!(history_row(world, &name).diff_to_current(), None);
}

#[tokio::main]
async fn main() {
    UsecaseWorld::run("features/usecase").await;
}
