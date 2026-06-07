//! Gherkin-driven BDD harness for domain rules.
//!
//! Runs every `.feature` under `features/domain`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_domain::model::age::{Age, AgeClass};
use gitweb_domain::model::binary::is_binary;
use gitweb_domain::model::change::ChangeStatus;
use gitweb_domain::model::chop::{ChopMode, chop_str};
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::{CombinedDiffEntry, CombinedParent};
use gitweb_domain::model::email_privacy::redact;
use gitweb_domain::model::encoding::{FallbackEncoding, to_utf8};
use gitweb_domain::model::export::{ExportPolicy, RepoFacts};
use gitweb_domain::model::file_mode::FileMode;
use gitweb_domain::model::forks::{ProjectGroup, partition_forks};
use gitweb_domain::model::grep::{GrepMatch, file_matches};
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::patch::{FileContent, FilePatch, Hunk, HunkLine, Patch};
use gitweb_domain::model::project_info::{CategoryGroup, ProjectInfo, group_by_category};
use gitweb_domain::model::projects_list::{ProjectListEntry, parse_project_line};
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::url::unescape;

#[derive(Debug, Default, World)]
struct DomainWorld {
    age: Option<Age>,
    humanized: Option<String>,
    class: Option<AgeClass>,
    raw_mode: String,
    file_mode: Option<FileMode>,
    raw_oid: String,
    object_id: Option<ObjectId>,
    raw_ident: String,
    signature: Option<Signature>,
    chop_text: String,
    chopped: Option<String>,
    ref_name: Option<RefName>,
    short_ref: Option<String>,
    privacy_line: String,
    redacted: Option<String>,
    bytes: Vec<u8>,
    decoded: Option<String>,
    binary: Option<bool>,
    commit: Option<Commit>,
    is_merge: Option<bool>,
    commit_title: Option<String>,
    commit_title_short: Option<String>,
    status_token: String,
    change: Option<ChangeStatus>,
    mod_from: Option<FileMode>,
    mod_to: Option<FileMode>,
    type_name: String,
    object_kind: Option<ObjectKind>,
    path_input: String,
    path_valid: Option<bool>,
    ref_input: String,
    ref_valid: Option<bool>,
    export_policy: ExportPolicy,
    repo_facts: RepoFacts,
    visible: Option<bool>,
    patch_under_test: Option<Patch>,
    rendered: Option<String>,
    encoded_token: String,
    decoded_token: Option<String>,
    fork_input: Vec<String>,
    fork_groups: Option<Vec<ProjectGroup>>,
    subject_project: Option<ProjectInfo>,
    short_description: Option<String>,
    resolved_category: Option<String>,
    projects_to_group: Vec<ProjectInfo>,
    category_groups: Option<Vec<CategoryGroup>>,
    projects_list_line: String,
    parsed_entry: Option<Option<ProjectListEntry>>,
    combined_entry: Option<CombinedDiffEntry>,
    combined_nparents: Option<usize>,
    combined_is_deleted: Option<bool>,
    combined_has_history: Option<bool>,
    combined_not_deleted: Option<bool>,
    grep_path: String,
    grep_content: Vec<u8>,
    grep_results: Option<Vec<GrepMatch>>,
}

fn dummy_oid() -> ObjectId {
    ObjectId::parse(&"0".repeat(40)).expect("forty zeros is a valid object id")
}

fn dummy_sig() -> Signature {
    Signature::parse("Tester <t@example.com> 1700000000 +0000").expect("a valid ident line")
}

fn make_commit(parents: usize, message: &str) -> Commit {
    let parent_oids: Vec<ObjectId> = vec![dummy_oid(); parents];
    Commit::new(
        dummy_oid(),
        dummy_oid(),
        parent_oids,
        dummy_sig(),
        dummy_sig(),
        message.to_owned(),
    )
}

fn parse_hex_bytes(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .map(|byte: &str| u8::from_str_radix(byte, 16).expect("valid hex byte"))
        .collect()
}

#[given(regex = r"^an age of (-?\d+) seconds$")]
fn given_age(world: &mut DomainWorld, seconds: i64) {
    world.age = Some(Age::from_seconds(seconds));
}

#[given("an unknown age")]
fn given_unknown_age(world: &mut DomainWorld) {
    world.age = None;
}

#[when("I humanize it")]
fn humanize(world: &mut DomainWorld) {
    let age: Age = world.age.expect("the age must be set first");
    world.humanized = Some(age.humanized());
}

#[when("I classify it")]
fn classify(world: &mut DomainWorld) {
    world.class = Some(AgeClass::from_age(world.age));
}

#[then(regex = r#"^the age reads "(.*)"$"#)]
fn age_reads(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.humanized.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the class is "(\w+)"$"#)]
fn class_is(world: &mut DomainWorld, expected: String) {
    let actual: AgeClass = world.class.expect("classify before asserting");
    assert_eq!(format!("{actual:?}"), expected);
}

#[given(regex = r#"^a file mode "(.*)"$"#)]
fn given_file_mode(world: &mut DomainWorld, text: String) {
    world.raw_mode = text;
}

#[when("I read the file mode")]
fn read_file_mode(world: &mut DomainWorld) {
    world.file_mode = FileMode::from_octal(&world.raw_mode);
}

#[then(regex = r#"^the short type is "(.*)"$"#)]
fn short_type_is(world: &mut DomainWorld, expected: String) {
    let mode: FileMode = world.file_mode.expect("a valid mode");
    assert_eq!(mode.short_type(), expected);
}

#[then(regex = r#"^the long type is "(.*)"$"#)]
fn long_type_is(world: &mut DomainWorld, expected: String) {
    let mode: FileMode = world.file_mode.expect("a valid mode");
    assert_eq!(mode.long_type(), expected);
}

#[then(regex = r#"^the permission string is "(.*)"$"#)]
fn permission_string_is(world: &mut DomainWorld, expected: String) {
    let mode: FileMode = world.file_mode.expect("a valid mode");
    assert_eq!(mode.permission_string(), expected);
}

#[then("the file mode is invalid")]
fn file_mode_is_invalid(world: &mut DomainWorld) {
    assert_eq!(world.file_mode, None);
}

#[given(regex = r#"^an object id "(.*)"$"#)]
fn given_object_id(world: &mut DomainWorld, text: String) {
    world.raw_oid = text;
}

#[when("I parse the object id")]
fn parse_object_id(world: &mut DomainWorld) {
    world.object_id = ObjectId::parse(&world.raw_oid);
}

#[then("the object id is valid")]
fn object_id_is_valid(world: &mut DomainWorld) {
    assert!(world.object_id.is_some());
}

#[then("the object id is invalid")]
fn object_id_is_invalid(world: &mut DomainWorld) {
    assert_eq!(world.object_id, None);
}

#[then(regex = r#"^the abbreviation is "(.*)"$"#)]
fn abbreviation_is(world: &mut DomainWorld, expected: String) {
    let oid: &ObjectId = world.object_id.as_ref().expect("a valid object id");
    assert_eq!(oid.abbreviated(), expected);
}

#[then(regex = r#"^the abbreviation to (\d+) is "(.*)"$"#)]
fn abbreviation_to_is(world: &mut DomainWorld, len: usize, expected: String) {
    let oid: &ObjectId = world.object_id.as_ref().expect("a valid object id");
    assert_eq!(oid.abbreviated_to(len), expected);
}

#[given(regex = r#"^the ident line "(.*)"$"#)]
fn given_ident_line(world: &mut DomainWorld, line: String) {
    world.raw_ident = line;
}

#[when("I parse the signature")]
fn parse_signature(world: &mut DomainWorld) {
    world.signature = Signature::parse(&world.raw_ident);
}

#[then(regex = r#"^the author name is "(.*)"$"#)]
fn author_name_is(world: &mut DomainWorld, expected: String) {
    let sig: &Signature = world.signature.as_ref().expect("a valid signature");
    assert_eq!(sig.name(), expected);
}

#[then(regex = r#"^the email is "(.*)"$"#)]
fn email_is(world: &mut DomainWorld, expected: String) {
    let sig: &Signature = world.signature.as_ref().expect("a valid signature");
    assert_eq!(sig.email(), Some(expected.as_str()));
}

#[then("there is no email")]
fn there_is_no_email(world: &mut DomainWorld) {
    let sig: &Signature = world.signature.as_ref().expect("a valid signature");
    assert_eq!(sig.email(), None);
}

#[then(regex = r"^the timestamp is (-?\d+)$")]
fn timestamp_is(world: &mut DomainWorld, expected: i64) {
    let sig: &Signature = world.signature.as_ref().expect("a valid signature");
    assert_eq!(sig.epoch(), expected);
}

#[then(regex = r#"^the timezone is "(.*)"$"#)]
fn timezone_is(world: &mut DomainWorld, expected: String) {
    let sig: &Signature = world.signature.as_ref().expect("a valid signature");
    assert_eq!(sig.timezone(), expected);
}

#[then("the signature is invalid")]
fn signature_is_invalid(world: &mut DomainWorld) {
    assert_eq!(world.signature, None);
}

#[given(regex = r#"^the text "(.*)"$"#)]
fn given_text(world: &mut DomainWorld, text: String) {
    world.chop_text = text;
}

#[when(regex = r"^I right-chop it to (\d+) keeping (\d+) extra$")]
fn right_chop(world: &mut DomainWorld, len: usize, add_len: usize) {
    world.chopped = Some(chop_str(&world.chop_text, len, add_len, ChopMode::Right));
}

#[when(regex = r"^I left-chop it to (\d+) keeping (\d+) extra$")]
fn left_chop(world: &mut DomainWorld, len: usize, add_len: usize) {
    world.chopped = Some(chop_str(&world.chop_text, len, add_len, ChopMode::Left));
}

#[when(regex = r"^I center-chop it to (\d+) keeping (\d+) extra$")]
fn center_chop(world: &mut DomainWorld, len: usize, add_len: usize) {
    world.chopped = Some(chop_str(&world.chop_text, len, add_len, ChopMode::Center));
}

#[then(regex = r#"^the chopped text is "(.*)"$"#)]
fn chopped_text_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.chopped.as_deref(), Some(expected.as_str()));
}

#[given(regex = r#"^the full ref "(.*)"$"#)]
fn given_full_ref(world: &mut DomainWorld, full: String) {
    world.ref_name = Some(RefName::new(full));
}

#[when("I shorten the ref")]
fn shorten_ref(world: &mut DomainWorld) {
    let ref_name: &RefName = world.ref_name.as_ref().expect("a ref must be set");
    world.short_ref = Some(ref_name.short().into_owned());
}

#[then(regex = r#"^the short ref is "(.*)"$"#)]
fn short_ref_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.short_ref.as_deref(), Some(expected.as_str()));
}

#[given(regex = r#"^the message line "(.*)"$"#)]
fn given_message_line(world: &mut DomainWorld, line: String) {
    world.privacy_line = line;
}

#[when("I redact private emails")]
fn redact_private_emails(world: &mut DomainWorld) {
    world.redacted = Some(redact(&world.privacy_line).into_owned());
}

#[then(regex = r#"^the redacted line is "(.*)"$"#)]
fn redacted_line_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.redacted.as_deref(), Some(expected.as_str()));
}

#[given(regex = r#"^the bytes "(.*)"$"#)]
fn given_bytes(world: &mut DomainWorld, hex: String) {
    world.bytes = parse_hex_bytes(&hex);
}

#[when("I decode them with the latin1 fallback")]
fn decode_bytes(world: &mut DomainWorld) {
    world.decoded = Some(to_utf8(&world.bytes, FallbackEncoding::Latin1));
}

#[then(regex = r#"^the text is "(.*)"$"#)]
fn text_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.decoded.as_deref(), Some(expected.as_str()));
}

#[when("I check whether it is binary")]
fn check_binary(world: &mut DomainWorld) {
    world.binary = Some(is_binary(&world.bytes));
}

#[then("it is binary")]
fn it_is_binary(world: &mut DomainWorld) {
    assert_eq!(world.binary, Some(true));
}

#[then("it is text")]
fn it_is_text(world: &mut DomainWorld) {
    assert_eq!(world.binary, Some(false));
}

#[given(regex = r"^a commit with (\d+) parents$")]
fn given_commit_with_parents(world: &mut DomainWorld, parents: usize) {
    world.commit = Some(make_commit(parents, ""));
}

#[given(regex = r#"^a commit with the message "(.*)"$"#)]
fn given_commit_with_message(world: &mut DomainWorld, message: String) {
    world.commit = Some(make_commit(0, &message));
}

#[given(regex = r#"^a commit whose message starts with a blank line then "(.*)"$"#)]
fn given_commit_blank_then(world: &mut DomainWorld, subject: String) {
    world.commit = Some(make_commit(0, &format!("\n{subject}")));
}

#[given("a commit with an empty message")]
fn given_commit_empty_message(world: &mut DomainWorld) {
    world.commit = Some(make_commit(0, ""));
}

#[given("a commit whose message is two blank lines")]
fn given_commit_two_blank_lines(world: &mut DomainWorld) {
    world.commit = Some(make_commit(0, "\n\n"));
}

#[given(regex = r#"^a commit whose first message line is "(.)" repeated (\d+) times$"#)]
fn given_commit_repeated_line(world: &mut DomainWorld, letter: String, count: usize) {
    world.commit = Some(make_commit(0, &letter.repeat(count)));
}

#[when("I check whether it is a merge")]
fn check_merge(world: &mut DomainWorld) {
    let commit: &Commit = world.commit.as_ref().expect("a commit must be set");
    world.is_merge = Some(commit.is_merge());
}

#[when("I read its title")]
fn read_title(world: &mut DomainWorld) {
    let commit: &Commit = world.commit.as_ref().expect("a commit must be set");
    world.commit_title = Some(commit.title());
}

#[when("I read its short title")]
fn read_short_title(world: &mut DomainWorld) {
    let commit: &Commit = world.commit.as_ref().expect("a commit must be set");
    world.commit_title_short = Some(commit.title_short());
}

#[then("it is a merge")]
fn it_is_a_merge(world: &mut DomainWorld) {
    assert_eq!(world.is_merge, Some(true));
}

#[then("it is not a merge")]
fn it_is_not_a_merge(world: &mut DomainWorld) {
    assert_eq!(world.is_merge, Some(false));
}

#[then(regex = r#"^the title is "(.*)"$"#)]
fn title_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.commit_title.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the short title is "(.*)"$"#)]
fn short_title_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.commit_title_short.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the title is the letter "(.)" repeated (\d+) times then "(.*)"$"#)]
fn title_is_repeated(world: &mut DomainWorld, letter: String, count: usize, suffix: String) {
    let expected: String = letter.repeat(count) + &suffix;
    assert_eq!(world.commit_title.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the short title is the letter "(.)" repeated (\d+) times then "(.*)"$"#)]
fn short_title_is_repeated(world: &mut DomainWorld, letter: String, count: usize, suffix: String) {
    let expected: String = letter.repeat(count) + &suffix;
    assert_eq!(world.commit_title_short.as_deref(), Some(expected.as_str()));
}

#[given(regex = r#"^a diff status token "(.*)"$"#)]
fn given_status_token(world: &mut DomainWorld, token: String) {
    world.status_token = token;
}

#[when("I read the change status")]
fn read_change_status(world: &mut DomainWorld) {
    world.change = ChangeStatus::parse(&world.status_token);
}

#[given(regex = r#"^a modification from mode "(.*)" to mode "(.*)"$"#)]
fn given_modification(world: &mut DomainWorld, from: String, to: String) {
    world.mod_from = FileMode::from_octal(&from);
    world.mod_to = FileMode::from_octal(&to);
}

#[when("I classify the modification")]
fn classify_modification(world: &mut DomainWorld) {
    let from: FileMode = world.mod_from.expect("a valid from-mode");
    let to: FileMode = world.mod_to.expect("a valid to-mode");
    world.change = Some(ChangeStatus::from_modification(from, to));
}

#[then(regex = r#"^the change is "(\w+)"$"#)]
fn change_is(world: &mut DomainWorld, expected: String) {
    let change: ChangeStatus = world.change.expect("a change status must be parsed");
    assert_eq!(format!("{:?}", change.kind()), expected);
}

#[then(regex = r"^the similarity is (\d+)$")]
fn similarity_is(world: &mut DomainWorld, expected: u8) {
    let change: ChangeStatus = world.change.expect("a change status must be parsed");
    assert_eq!(change.similarity(), expected);
}

#[then("there is no change")]
fn there_is_no_change(world: &mut DomainWorld) {
    assert_eq!(world.change, None);
}

#[given(regex = r#"^an object type "(.*)"$"#)]
fn given_object_type(world: &mut DomainWorld, name: String) {
    world.type_name = name;
}

#[when("I read the object kind")]
fn read_object_kind(world: &mut DomainWorld) {
    world.object_kind = ObjectKind::parse(&world.type_name);
}

#[then(regex = r#"^the object kind is "(\w+)"$"#)]
fn object_kind_is(world: &mut DomainWorld, expected: String) {
    let kind: ObjectKind = world.object_kind.expect("an object kind must be parsed");
    assert_eq!(format!("{kind:?}"), expected);
}

#[then("there is no object kind")]
fn there_is_no_object_kind(world: &mut DomainWorld) {
    assert_eq!(world.object_kind, None);
}

#[given(regex = r#"^the candidate path "(.*)"$"#)]
fn given_candidate_path(world: &mut DomainWorld, path: String) {
    world.path_input = path;
}

#[given("a candidate path with a NUL byte")]
fn given_candidate_path_with_nul(world: &mut DomainWorld) {
    world.path_input = "src/\0evil".to_owned();
}

#[when("I validate it as a path")]
fn validate_as_path(world: &mut DomainWorld) {
    world.path_valid = Some(SafePath::parse(&world.path_input).is_some());
}

#[then("the path is accepted")]
fn path_is_accepted(world: &mut DomainWorld) {
    assert_eq!(world.path_valid, Some(true));
}

#[then("the path is rejected")]
fn path_is_rejected(world: &mut DomainWorld) {
    assert_eq!(world.path_valid, Some(false));
}

#[given(regex = r#"^the candidate ref "(.*)"$"#)]
fn given_candidate_ref(world: &mut DomainWorld, reference: String) {
    world.ref_input = reference;
}

#[given("a candidate ref of 64 hex characters")]
fn given_candidate_ref_64_hex(world: &mut DomainWorld) {
    world.ref_input = "0".repeat(64);
}

#[when("I validate it as a ref")]
fn validate_as_ref(world: &mut DomainWorld) {
    world.ref_valid = Some(SafeRef::parse(&world.ref_input).is_some());
}

#[then("the ref is accepted")]
fn ref_is_accepted(world: &mut DomainWorld) {
    assert_eq!(world.ref_valid, Some(true));
}

#[then("the ref is rejected")]
fn ref_is_rejected(world: &mut DomainWorld) {
    assert_eq!(world.ref_valid, Some(false));
}

#[given("a repository whose HEAD is linked")]
fn given_head_linked(world: &mut DomainWorld) {
    world.repo_facts.head_linked = true;
}

#[given("a repository whose HEAD is not linked")]
fn given_head_not_linked(world: &mut DomainWorld) {
    world.repo_facts.head_linked = false;
}

#[given("a permissive export policy")]
fn given_permissive_policy(world: &mut DomainWorld) {
    world.export_policy = ExportPolicy::default();
}

#[given("an export marker is required")]
fn given_marker_required(world: &mut DomainWorld) {
    world.export_policy.require_marker = true;
}

#[given("the export marker is present")]
fn given_marker_present(world: &mut DomainWorld) {
    world.repo_facts.marker_present = true;
}

#[given("the export marker is absent")]
fn given_marker_absent(world: &mut DomainWorld) {
    world.repo_facts.marker_present = false;
}

#[given("strict export is enabled")]
fn given_strict_enabled(world: &mut DomainWorld) {
    world.export_policy.strict = true;
}

#[given("the repository is in the projects list")]
fn given_in_projects_list(world: &mut DomainWorld) {
    world.repo_facts.in_projects_list = true;
}

#[given("the repository is not in the projects list")]
fn given_not_in_projects_list(world: &mut DomainWorld) {
    world.repo_facts.in_projects_list = false;
}

#[given("an auth hook is configured")]
fn given_auth_hook_configured(world: &mut DomainWorld) {
    world.export_policy.has_auth_hook = true;
}

#[given("the auth hook allows the repository")]
fn given_auth_hook_allows(world: &mut DomainWorld) {
    world.repo_facts.auth_hook_allows = true;
}

#[given("the auth hook denies the repository")]
fn given_auth_hook_denies(world: &mut DomainWorld) {
    world.repo_facts.auth_hook_allows = false;
}

#[when("I evaluate visibility")]
fn evaluate_visibility(world: &mut DomainWorld) {
    world.visible = Some(world.export_policy.permits(&world.repo_facts));
}

#[then("the repository is visible")]
fn repository_is_visible(world: &mut DomainWorld) {
    assert_eq!(world.visible, Some(true));
}

#[then("the repository is hidden")]
fn repository_is_hidden(world: &mut DomainWorld) {
    assert_eq!(world.visible, Some(false));
}

// --- Patch (unified diff) formatting -----------------------------------------

/// An object id that is the given hex digit repeated forty times — a valid,
/// recognisable SHA-1 for pinning `index` lines in specs.
fn oid_of(digit: char) -> ObjectId {
    ObjectId::parse(&digit.to_string().repeat(40)).expect("forty hex digits is a valid object id")
}

fn mode_of(octal: &str) -> FileMode {
    FileMode::from_octal(octal).expect("a valid octal mode")
}

fn modified_patch(path: &str) -> FilePatch {
    let hunk: Hunk = Hunk::new(
        1,
        1,
        1,
        1,
        vec![HunkLine::deletion("old"), HunkLine::addition("new")],
    );
    FilePatch::new(
        ChangeStatus::from_modification(mode_of("100644"), mode_of("100644")),
        mode_of("100644"),
        mode_of("100644"),
        oid_of('1'),
        oid_of('2'),
        path,
        path,
        FileContent::Text(vec![hunk]),
    )
}

fn created_patch(path: &str) -> FilePatch {
    let hunk: Hunk = Hunk::new(
        0,
        0,
        1,
        2,
        vec![
            HunkLine::addition("line one"),
            HunkLine::addition("line two"),
        ],
    );
    FilePatch::new(
        ChangeStatus::added(),
        mode_of("000000"),
        mode_of("100644"),
        oid_of('0'),
        oid_of('2'),
        path,
        path,
        FileContent::Text(vec![hunk]),
    )
}

fn deleted_patch(path: &str) -> FilePatch {
    let hunk: Hunk = Hunk::new(1, 1, 0, 0, vec![HunkLine::deletion("was here")]);
    FilePatch::new(
        ChangeStatus::deleted(),
        mode_of("100644"),
        mode_of("000000"),
        oid_of('1'),
        oid_of('0'),
        path,
        path,
        FileContent::Text(vec![hunk]),
    )
}

fn one_file(patch: FilePatch) -> Patch {
    Patch::new(vec![patch])
}

#[given(regex = r#"^a modified file patch for "([^"]+)"$"#)]
fn given_modified_patch(world: &mut DomainWorld, path: String) {
    world.patch_under_test = Some(one_file(modified_patch(&path)));
}

#[given(regex = r#"^a created file patch for "([^"]+)" with two lines$"#)]
fn given_created_patch(world: &mut DomainWorld, path: String) {
    world.patch_under_test = Some(one_file(created_patch(&path)));
}

#[given(regex = r#"^a deleted file patch for "([^"]+)"$"#)]
fn given_deleted_patch(world: &mut DomainWorld, path: String) {
    world.patch_under_test = Some(one_file(deleted_patch(&path)));
}

#[given(regex = r#"^an executable-bit file patch for "([^"]+)"$"#)]
fn given_mode_change_patch(world: &mut DomainWorld, path: String) {
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::from_modification(mode_of("100644"), mode_of("100755")),
        mode_of("100644"),
        mode_of("100755"),
        oid_of('1'),
        oid_of('1'),
        path.clone(),
        path,
        FileContent::Text(vec![]),
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(regex = r#"^an executable-bit-and-content file patch for "([^"]+)"$"#)]
fn given_mode_and_content_patch(world: &mut DomainWorld, path: String) {
    let hunk: Hunk = Hunk::new(
        1,
        1,
        1,
        1,
        vec![HunkLine::deletion("old"), HunkLine::addition("new")],
    );
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::from_modification(mode_of("100644"), mode_of("100755")),
        mode_of("100644"),
        mode_of("100755"),
        oid_of('1'),
        oid_of('2'),
        path.clone(),
        path,
        FileContent::Text(vec![hunk]),
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(regex = r#"^an exact rename file patch from "([^"]+)" to "([^"]+)"$"#)]
fn given_exact_rename_patch(world: &mut DomainWorld, from: String, to: String) {
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::renamed(100),
        mode_of("100644"),
        mode_of("100644"),
        oid_of('1'),
        oid_of('1'),
        from,
        to,
        FileContent::Text(vec![]),
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(regex = r#"^an inexact rename file patch from "([^"]+)" to "([^"]+)" at (\d+)%$"#)]
fn given_inexact_rename_patch(world: &mut DomainWorld, from: String, to: String, score: u8) {
    let hunk: Hunk = Hunk::new(
        1,
        2,
        1,
        2,
        vec![
            HunkLine::context("kept"),
            HunkLine::deletion("old"),
            HunkLine::addition("new"),
        ],
    );
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::renamed(score),
        mode_of("100644"),
        mode_of("100644"),
        oid_of('1'),
        oid_of('2'),
        from,
        to,
        FileContent::Text(vec![hunk]),
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(regex = r#"^an exact copy file patch from "([^"]+)" to "([^"]+)"$"#)]
fn given_exact_copy_patch(world: &mut DomainWorld, from: String, to: String) {
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::copied(100),
        mode_of("100644"),
        mode_of("100644"),
        oid_of('1'),
        oid_of('1'),
        from,
        to,
        FileContent::Text(vec![]),
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(regex = r#"^a binary modification file patch for "([^"]+)"$"#)]
fn given_binary_modification_patch(world: &mut DomainWorld, path: String) {
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::from_modification(mode_of("100644"), mode_of("100644")),
        mode_of("100644"),
        mode_of("100644"),
        oid_of('1'),
        oid_of('2'),
        path.clone(),
        path,
        FileContent::Binary,
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(regex = r#"^a binary creation file patch for "([^"]+)"$"#)]
fn given_binary_creation_patch(world: &mut DomainWorld, path: String) {
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::added(),
        mode_of("000000"),
        mode_of("100644"),
        oid_of('0'),
        oid_of('2'),
        path.clone(),
        path,
        FileContent::Binary,
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given(
    regex = r#"^a created file patch for "([^"]+)" whose single line has no trailing newline$"#
)]
fn given_created_no_newline_patch(world: &mut DomainWorld, path: String) {
    let hunk: Hunk = Hunk::new(
        0,
        0,
        1,
        1,
        vec![HunkLine::addition("target/path").without_trailing_newline()],
    );
    let patch: FilePatch = FilePatch::new(
        ChangeStatus::added(),
        mode_of("000000"),
        mode_of("120000"),
        oid_of('0'),
        oid_of('2'),
        path.clone(),
        path,
        FileContent::Text(vec![hunk]),
    );
    world.patch_under_test = Some(one_file(patch));
}

#[given("a patch over no files")]
fn given_empty_patch(world: &mut DomainWorld) {
    world.patch_under_test = Some(Patch::new(vec![]));
}

#[given("a patch over a created file and a deleted file")]
fn given_multi_file_patch(world: &mut DomainWorld) {
    world.patch_under_test = Some(Patch::new(vec![
        created_patch("added.txt"),
        deleted_patch("gone.txt"),
    ]));
}

#[when("I render the patch")]
fn render_patch(world: &mut DomainWorld) {
    let patch: &Patch = world
        .patch_under_test
        .as_ref()
        .expect("a patch must be built before rendering");
    world.rendered = Some(patch.render());
}

fn rendered(world: &DomainWorld) -> &str {
    world
        .rendered
        .as_deref()
        .expect("the patch must be rendered before asserting on it")
}

#[then(regex = r#"^the patch contains "(.*)"$"#)]
fn then_patch_contains(world: &mut DomainWorld, fragment: String) {
    assert!(
        rendered(world).contains(&fragment),
        "expected patch to contain {fragment:?}, got:\n{}",
        rendered(world)
    );
}

#[then(regex = r#"^the patch does not contain "(.*)"$"#)]
fn then_patch_not_contains(world: &mut DomainWorld, fragment: String) {
    assert!(
        !rendered(world).contains(&fragment),
        "expected patch not to contain {fragment:?}, got:\n{}",
        rendered(world)
    );
}

#[then(regex = r#"^the patch has a line "(.*)"$"#)]
fn then_patch_has_line(world: &mut DomainWorld, expected: String) {
    assert!(
        rendered(world).lines().any(|line: &str| line == expected),
        "expected patch to have the exact line {expected:?}, got:\n{}",
        rendered(world)
    );
}

#[then("the patch is:")]
fn then_patch_is(world: &mut DomainWorld, step: &Step) {
    let expected: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    assert_eq!(rendered(world).trim_end_matches('\n'), expected);
}

#[then("the patch is empty")]
fn then_patch_is_empty(world: &mut DomainWorld) {
    assert_eq!(rendered(world), "");
}

// --- URL decoding (unescape) -------------------------------------------------

#[given(regex = r#"^the encoded token "(.*)"$"#)]
fn given_encoded_token(world: &mut DomainWorld, token: String) {
    world.encoded_token = token;
}

#[when("I unescape it")]
fn unescape_token(world: &mut DomainWorld) {
    world.decoded_token = Some(unescape(&world.encoded_token));
}

#[then(regex = r#"^the decoded token is "(.*)"$"#)]
fn decoded_token_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.decoded_token.as_deref(), Some(expected.as_str()));
}

// --- Fork detection ----------------------------------------------------------

fn fork_group<'a>(world: &'a DomainWorld, name: &str) -> &'a ProjectGroup {
    world
        .fork_groups
        .as_ref()
        .expect("partition the forks first")
        .iter()
        .find(|group: &&ProjectGroup| group.name() == name)
        .unwrap_or_else(|| panic!("no top-level project named {name}"))
}

#[given(regex = r#"^a project path "([^"]*)"$"#)]
fn given_project_path(world: &mut DomainWorld, path: String) {
    world.fork_input.push(path);
}

#[when("I partition forks")]
fn partition_the_forks(world: &mut DomainWorld) {
    world.fork_groups = Some(partition_forks(&world.fork_input));
}

#[then(regex = r"^(\d+) top-level projects? (?:remain|remains)$")]
fn top_level_count(world: &mut DomainWorld, count: usize) {
    let groups: &[ProjectGroup] = world.fork_groups.as_ref().expect("partition first");
    assert_eq!(groups.len(), count);
}

#[then(regex = r#"^"([^"]*)" is a top-level project$"#)]
fn is_top_level(world: &mut DomainWorld, name: String) {
    let found: bool = world
        .fork_groups
        .as_ref()
        .expect("partition first")
        .iter()
        .any(|group: &ProjectGroup| group.name() == name);
    assert!(found, "expected {name} at the top level");
}

#[then(regex = r#"^"([^"]*)" is not a top-level project$"#)]
fn is_not_top_level(world: &mut DomainWorld, name: String) {
    let found: bool = world
        .fork_groups
        .as_ref()
        .expect("partition first")
        .iter()
        .any(|group: &ProjectGroup| group.name() == name);
    assert!(!found, "expected {name} to be folded away as a fork");
}

#[then(regex = r#"^"([^"]*)" has (\d+) forks?$"#)]
fn has_n_forks(world: &mut DomainWorld, name: String, count: usize) {
    assert_eq!(fork_group(world, &name).forks().len(), count);
}

#[then(regex = r#"^"([^"]*)" has the fork "([^"]*)"$"#)]
fn has_the_fork(world: &mut DomainWorld, name: String, fork: String) {
    let found: bool = fork_group(world, &name)
        .forks()
        .iter()
        .any(|candidate: &String| candidate == &fork);
    assert!(found, "expected {name} to own the fork {fork}");
}

// --- Per-project metadata (ProjectInfo) --------------------------------------

fn subject(world: &DomainWorld) -> &ProjectInfo {
    world
        .subject_project
        .as_ref()
        .expect("a project must be set first")
}

fn category_group<'a>(world: &'a DomainWorld, name: &str) -> &'a CategoryGroup {
    world
        .category_groups
        .as_ref()
        .expect("group by category first")
        .iter()
        .find(|group: &&CategoryGroup| group.name() == name)
        .unwrap_or_else(|| panic!("no category named {name}"))
}

#[given(regex = r#"^a project "([^"]*)" with no description$"#)]
fn given_project_no_description(world: &mut DomainWorld, name: String) {
    world.subject_project = Some(ProjectInfo::named(name));
}

#[given(regex = r#"^a project "([^"]*)" described as "(.*)"$"#)]
fn given_project_described(world: &mut DomainWorld, name: String, description: String) {
    world.subject_project = Some(ProjectInfo::named(name).with_description(description));
}

#[given(regex = r#"^a project "([^"]*)" with no category$"#)]
fn given_project_no_category(world: &mut DomainWorld, name: String) {
    world.subject_project = Some(ProjectInfo::named(name));
}

#[given(regex = r#"^a project "([^"]*)" in category "([^"]*)"$"#)]
fn given_project_in_category(world: &mut DomainWorld, name: String, category: String) {
    world.subject_project = Some(ProjectInfo::named(name).with_category(category));
}

#[when(regex = r"^I shorten its description to width (\d+)$")]
fn shorten_description(world: &mut DomainWorld, width: usize) {
    world.short_description = Some(subject(world).descr_short(width));
}

#[when(regex = r#"^I read its category with default "([^"]*)"$"#)]
fn read_category(world: &mut DomainWorld, default: String) {
    world.resolved_category = Some(subject(world).category_or(&default).to_owned());
}

#[then(regex = r#"^the short description is "(.*)"$"#)]
fn short_description_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.short_description.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the category is "([^"]*)"$"#)]
fn category_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.resolved_category.as_deref(), Some(expected.as_str()));
}

#[given("no projects to group")]
fn given_no_projects_to_group(world: &mut DomainWorld) {
    world.projects_to_group.clear();
}

#[given(regex = r#"^a project "([^"]*)" in category "([^"]*)" to group$"#)]
fn given_project_in_category_to_group(world: &mut DomainWorld, name: String, category: String) {
    world
        .projects_to_group
        .push(ProjectInfo::named(name).with_category(category));
}

#[given(regex = r#"^a project "([^"]*)" with no category to group$"#)]
fn given_project_no_category_to_group(world: &mut DomainWorld, name: String) {
    world.projects_to_group.push(ProjectInfo::named(name));
}

#[when(regex = r#"^I group by category with default "([^"]*)"$"#)]
fn group_projects(world: &mut DomainWorld, default: String) {
    world.category_groups = Some(group_by_category(&world.projects_to_group, &default));
}

#[then(regex = r"^(\d+) categor(?:y|ies) results?$")]
fn category_count(world: &mut DomainWorld, count: usize) {
    let groups: &[CategoryGroup] = world.category_groups.as_ref().expect("group first");
    assert_eq!(groups.len(), count);
}

#[then(regex = r#"^the category "([^"]*)" holds (\d+) projects?$"#)]
fn category_holds_count(world: &mut DomainWorld, name: String, count: usize) {
    assert_eq!(category_group(world, &name).projects().len(), count);
}

#[then(regex = r#"^the category "([^"]*)" holds "([^"]*)"$"#)]
fn category_holds_project(world: &mut DomainWorld, name: String, project: String) {
    let found: bool = category_group(world, &name)
        .projects()
        .iter()
        .any(|info: &ProjectInfo| info.name() == project);
    assert!(found, "expected category {name} to hold {project}");
}

// --- Projects-list file lines ------------------------------------------------

fn parsed(world: &DomainWorld) -> &ProjectListEntry {
    world
        .parsed_entry
        .as_ref()
        .expect("parse the line first")
        .as_ref()
        .expect("the line carried a project")
}

#[given(regex = r#"^the projects-list line "(.*)"$"#)]
fn given_projects_list_line(world: &mut DomainWorld, line: String) {
    world.projects_list_line = line;
}

#[when("I parse the projects-list line")]
fn parse_projects_list_line(world: &mut DomainWorld) {
    world.parsed_entry = Some(parse_project_line(&world.projects_list_line));
}

#[then("the line carries no project")]
fn line_carries_no_project(world: &mut DomainWorld) {
    let result: &Option<ProjectListEntry> =
        world.parsed_entry.as_ref().expect("parse the line first");
    assert_eq!(result, &None);
}

#[then(regex = r#"^the parsed path is "(.*)"$"#)]
fn parsed_path_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(parsed(world).path(), expected);
}

#[then(regex = r#"^the parsed owner is "(.*)"$"#)]
fn parsed_owner_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(parsed(world).owner(), Some(expected.as_str()));
}

#[then("the parsed line has no owner")]
fn parsed_line_has_no_owner(world: &mut DomainWorld) {
    assert_eq!(parsed(world).owner(), None);
}

// --- combined merge diff entry -----------------------------------------------

/// A non-null object id, so a combined entry's result side reads as present.
fn nonzero_oid() -> ObjectId {
    ObjectId::parse(&"a".repeat(40)).expect("forty a's is a valid object id")
}

/// A regular-file mode, the only mode these pure-rule scenarios care about.
fn regular_mode() -> FileMode {
    FileMode::from_octal("100644").expect("100644 is a valid mode")
}

/// One parent's from-side carrying `status` over a regular file.
fn parent_with(status: ChangeStatus) -> CombinedParent {
    CombinedParent::new(status, regular_mode(), nonzero_oid())
}

/// A modification status over an unchanged regular-file mode.
fn modified_status() -> ChangeStatus {
    ChangeStatus::from_modification(regular_mode(), regular_mode())
}

#[given("a combined entry modified against two parents")]
fn given_combined_modified_two(world: &mut DomainWorld) {
    world.combined_entry = Some(CombinedDiffEntry::new(
        vec![
            parent_with(modified_status()),
            parent_with(modified_status()),
        ],
        regular_mode(),
        nonzero_oid(),
        "merged.txt".to_owned(),
    ));
}

#[given("a combined entry added against two parents")]
fn given_combined_added_two(world: &mut DomainWorld) {
    world.combined_entry = Some(CombinedDiffEntry::new(
        vec![
            parent_with(ChangeStatus::added()),
            parent_with(ChangeStatus::added()),
        ],
        regular_mode(),
        nonzero_oid(),
        "merged.txt".to_owned(),
    ));
}

#[given("a combined entry deleted against two parents")]
fn given_combined_deleted_two(world: &mut DomainWorld) {
    world.combined_entry = Some(CombinedDiffEntry::new(
        vec![
            parent_with(ChangeStatus::deleted()),
            parent_with(ChangeStatus::deleted()),
        ],
        regular_mode(),
        dummy_oid(),
        "merged.txt".to_owned(),
    ));
}

#[given("a combined entry modified against three parents")]
fn given_combined_modified_three(world: &mut DomainWorld) {
    world.combined_entry = Some(CombinedDiffEntry::new(
        vec![
            parent_with(modified_status()),
            parent_with(modified_status()),
            parent_with(modified_status()),
        ],
        regular_mode(),
        nonzero_oid(),
        "merged.txt".to_owned(),
    ));
}

#[when("I read its combined-diff flags")]
fn read_combined_flags(world: &mut DomainWorld) {
    let entry: &CombinedDiffEntry = world
        .combined_entry
        .as_ref()
        .expect("a combined entry must be built first");
    world.combined_nparents = Some(entry.nparents());
    world.combined_is_deleted = Some(entry.is_deleted());
    world.combined_has_history = Some(entry.has_history());
    world.combined_not_deleted = Some(entry.not_deleted());
}

#[then(regex = r"^the combined entry has (\d+) parents$")]
fn combined_has_parents(world: &mut DomainWorld, count: usize) {
    assert_eq!(world.combined_nparents, Some(count));
}

#[then("the combined entry is a deletion")]
fn combined_is_deletion(world: &mut DomainWorld) {
    assert_eq!(world.combined_is_deleted, Some(true));
}

#[then("the combined entry is not a deletion")]
fn combined_is_not_deletion(world: &mut DomainWorld) {
    assert_eq!(world.combined_is_deleted, Some(false));
}

#[then("the combined entry has history")]
fn combined_has_history(world: &mut DomainWorld) {
    assert_eq!(world.combined_has_history, Some(true));
}

#[then("the combined entry has no history")]
fn combined_has_no_history(world: &mut DomainWorld) {
    assert_eq!(world.combined_has_history, Some(false));
}

#[then("the combined entry survives against some parent")]
fn combined_survives(world: &mut DomainWorld) {
    assert_eq!(world.combined_not_deleted, Some(true));
}

#[then("the combined entry is gone from every parent")]
fn combined_gone(world: &mut DomainWorld) {
    assert_eq!(world.combined_not_deleted, Some(false));
}

// --- content grep (file_matches) ---------------------------------------------

/// Expands the feature files' byte escapes: `\n` to a newline and `\0` to a NUL,
/// so a fixture pins a file's exact bytes — including a trailing newline or an
/// embedded NUL that makes the content binary.
fn expand_escapes(text: &str) -> String {
    text.replace("\\0", "\0").replace("\\n", "\n")
}

fn grep_matches(world: &DomainWorld) -> &[GrepMatch] {
    world
        .grep_results
        .as_ref()
        .expect("run a grep first")
        .as_slice()
}

#[given(regex = r#"^a file "([^"]*)" with content "(.*)"$"#)]
fn given_grep_file(world: &mut DomainWorld, path: String, content: String) {
    world.grep_path = path;
    world.grep_content = expand_escapes(&content).into_bytes();
}

#[given(regex = r#"^a file "([^"]*)" with latin1 content "(.*)"$"#)]
fn given_grep_latin1_file(world: &mut DomainWorld, path: String, content: String) {
    world.grep_path = path;
    world.grep_content = expand_escapes(&content)
        .chars()
        .map(|character: char| character as u8)
        .collect();
}

#[when(regex = r#"^I grep "(.*)"$"#)]
fn run_grep(world: &mut DomainWorld, pattern: String) {
    world.grep_results = Some(file_matches(
        &world.grep_path,
        &world.grep_content,
        &pattern,
    ));
}

#[then(regex = r"^(\d+) grep match(?:es)? (?:is|are) found$")]
fn grep_count(world: &mut DomainWorld, count: usize) {
    assert_eq!(grep_matches(world).len(), count);
}

#[then(regex = r#"^grep match (\d+) is line (\d+) "(.*)" in "([^"]*)"$"#)]
fn grep_line_is(world: &mut DomainWorld, index: usize, line: usize, text: String, path: String) {
    let hit: &GrepMatch = &grep_matches(world)[index];
    assert_eq!(hit.path(), path);
    assert_eq!(hit.line_no(), Some(line));
    assert_eq!(hit.text(), Some(text.as_str()));
}

#[then(regex = r#"^grep match (\d+) is binary file "([^"]*)"$"#)]
fn grep_binary_is(world: &mut DomainWorld, index: usize, path: String) {
    let hit: &GrepMatch = &grep_matches(world)[index];
    assert!(hit.is_binary());
    assert_eq!(hit.path(), path);
    assert_eq!(hit.text(), None);
}

#[tokio::main]
async fn main() {
    DomainWorld::run("features/domain").await;
}
