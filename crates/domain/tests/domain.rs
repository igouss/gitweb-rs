//! Gherkin-driven BDD harness for domain rules.
//!
//! Runs every `.feature` under `features/domain`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_domain::error::DomainError;
use gitweb_domain::model::accept::prefer_text_xml_feed;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::age::{Age, AgeClass};
use gitweb_domain::model::binary::is_binary;
use gitweb_domain::model::blob::{Blob, BlobDisplay};
use gitweb_domain::model::blobdiff_plain::BlobdiffPlain;
use gitweb_domain::model::change::ChangeStatus;
use gitweb_domain::model::chop::{ChopMode, chop_str};
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::commit_date::CommitDate;
use gitweb_domain::model::commitdiff::{DiffBase, diff_base};
use gitweb_domain::model::commitdiff_plain::CommitdiffPlain;
use gitweb_domain::model::conditional::{Freshness, freshness};
use gitweb_domain::model::config_chain::{ConfigChain, ConfigSlot};
use gitweb_domain::model::content_type::PlainHeaders;
use gitweb_domain::model::diff::{CombinedDiffEntry, CombinedParent};
use gitweb_domain::model::diffstat::{Diffstat, DiffstatEntry, StatChange};
use gitweb_domain::model::email_privacy::redact;
use gitweb_domain::model::encoding::{FallbackEncoding, to_utf8};
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::export::{ExportPolicy, RepoFacts};
use gitweb_domain::model::feed::{comment_lines, feed_title, feed_window};
use gitweb_domain::model::file_change::FileChangeNote;
use gitweb_domain::model::file_mode::FileMode;
use gitweb_domain::model::forks::{ProjectGroup, partition_forks};
use gitweb_domain::model::format_patch::{FormatPatch, PatchEntry};
use gitweb_domain::model::grep::{GrepMatch, file_matches};
use gitweb_domain::model::message_body::{LogLine, log_lines};
use gitweb_domain::model::object_dispatch::{DispatchLookup, dispatch_lookup};
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::object_redirect::{Resolution, resolution, target_action};
use gitweb_domain::model::patch::{FileContent, FilePatch, FileSelection, Hunk, HunkLine, Patch};
use gitweb_domain::model::path_info::PathInfo;
use gitweb_domain::model::project_info::{CategoryGroup, ProjectInfo, group_by_category};
use gitweb_domain::model::project_order::ProjectOrder;
use gitweb_domain::model::projects_list::{ProjectListEntry, parse_project_line};
use gitweb_domain::model::ref_marker::{MarkerView, RefMarker, markers_for};
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::DereferencedRef;
use gitweb_domain::model::remote::{Remote, RemoteUrl};
use gitweb_domain::model::request::Request;
use gitweb_domain::model::routing::{Dispatch, route};
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::search_help::{SearchHelpTopic, help_topics};
use gitweb_domain::model::section::Section;
use gitweb_domain::model::settings::{FeatureName, Settings, SettingsLayer};
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::snapshot::{
    ArchiveFormat, enabled_formats, select_format, snapshot_name,
};
use gitweb_domain::model::tag_age::TagAge;
use gitweb_domain::model::timestamp::Timestamp;
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
    search_help_topics: Option<Vec<SearchHelpTopic>>,
    ref_name: Option<RefName>,
    short_ref: Option<String>,
    deref_refs: Vec<DereferencedRef>,
    markers: Option<Vec<RefMarker>>,
    privacy_line: String,
    redacted: Option<String>,
    bytes: Vec<u8>,
    decoded: Option<String>,
    binary: Option<bool>,
    blob_display: Option<BlobDisplay>,
    plain_headers: Option<PlainHeaders>,
    commit: Option<Commit>,
    is_merge: Option<bool>,
    commit_title: Option<String>,
    commit_title_short: Option<String>,
    status_token: String,
    change: Option<ChangeStatus>,
    mod_from: Option<FileMode>,
    mod_to: Option<FileMode>,
    note_status: Option<ChangeStatus>,
    note_from: Option<FileMode>,
    note_to: Option<FileMode>,
    derived_note: Option<Option<FileChangeNote>>,
    type_name: String,
    object_kind: Option<ObjectKind>,
    action_name: String,
    parsed_action: Option<Action>,
    request_params: Vec<(String, String)>,
    request_result: Option<Result<Request, DomainError>>,
    path_info_input: String,
    path_info: Option<PathInfo>,
    path_input: String,
    path_valid: Option<bool>,
    ref_input: String,
    ref_valid: Option<bool>,
    export_policy: ExportPolicy,
    repo_facts: RepoFacts,
    visible: Option<bool>,
    patch_under_test: Option<Patch>,
    diffstat_entries: Vec<DiffstatEntry>,
    diffstat_text: Option<String>,
    fp_commit_id: String,
    fp_author: String,
    fp_date: Option<Timestamp>,
    fp_subject: String,
    fp_number: Option<(usize, usize)>,
    fp_body: Vec<String>,
    fp_diff_body: String,
    fp_entries: Vec<PatchEntry>,
    fp_text: Option<String>,
    rendered: Option<String>,
    /// The single-file selection result: `None` until the When runs, then the
    /// inner `Option` distinguishes a found file's text from no match.
    selected_file: Option<Option<String>>,
    /// The `blobdiff` file-resolution outcome, reduced to an owned form so the
    /// borrow of the patch does not outlive the When.
    file_resolution: Option<FileResolution>,
    bdp_patch_body: Option<String>,
    bdp_rendered: Option<String>,
    cdp_author: Option<String>,
    cdp_subject: Option<String>,
    cdp_tag: Option<String>,
    cdp_comment: Vec<String>,
    cdp_commit_id: Option<String>,
    cdp_patch_body: Option<String>,
    cdp_rendered: Option<String>,
    commitdiff_parents: Vec<ObjectId>,
    commitdiff_explicit: Option<ObjectId>,
    commitdiff_base: Option<DiffBase>,
    encoded_token: String,
    decoded_token: Option<String>,
    fork_input: Vec<String>,
    fork_groups: Option<Vec<ProjectGroup>>,
    subject_project: Option<ProjectInfo>,
    short_description: Option<String>,
    resolved_category: Option<String>,
    projects_to_group: Vec<ProjectInfo>,
    category_groups: Option<Vec<CategoryGroup>>,
    order_listing: Vec<ProjectInfo>,
    parsed_order: Option<Result<ProjectOrder, DomainError>>,
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
    cfg_common_env: Option<String>,
    cfg_common_default: Option<String>,
    cfg_primary_env: Option<String>,
    cfg_primary_default: Option<String>,
    cfg_system_env: Option<String>,
    cfg_system_default: Option<String>,
    cfg_existing: Vec<String>,
    cfg_load_order: Option<Vec<String>>,
    settings_layers: Vec<SettingsLayer>,
    resolved_settings: Option<Settings>,
    routed: Option<Result<Dispatch, DomainError>>,
    timestamp: Option<Timestamp>,
    timestamp_text: Option<String>,
    at_night: Option<bool>,
    commit_date_input: Option<(i64, i64)>,
    commit_date: Option<CommitDate>,
    log_message: String,
    log_body: Option<Vec<LogLine>>,
    section_cap: usize,
    section_items: Vec<String>,
    section: Option<Section<String>>,
    remote: Option<Remote>,
    remote_url_lines: Option<Vec<RemoteUrl>>,
    feed_now: i64,
    feed_epochs: Vec<i64>,
    feed_kept: Option<usize>,
    feed_title: Option<String>,
    feed_comment: Option<Vec<String>>,
    obj_hash: Option<String>,
    obj_base: Option<String>,
    obj_file: Option<String>,
    obj_lookup: Option<Result<Resolution, DomainError>>,
    dispatch_lookup: Option<Option<DispatchLookup>>,
    obj_kind_in: Option<ObjectKind>,
    obj_action_out: Option<Action>,
    snapshot_format: Option<ArchiveFormat>,
    configured_formats: Vec<String>,
    computed_formats: Option<Vec<ArchiveFormat>>,
    selection_enabled: Vec<ArchiveFormat>,
    selection_result: Option<Result<ArchiveFormat, DomainError>>,
    snapshot_project: String,
    snapshot_hash: String,
    snapshot_short: String,
    snapshot_name_out: Option<String>,
    tag_age_now: i64,
    tag_age_creation: Option<i64>,
    tag_age_result: Option<TagAge>,
    cond_epoch: i64,
    cond_result: Option<Freshness>,
    accept_feed_type: String,
    accept_result: Option<bool>,
    expiry_hash: Option<String>,
    expiry_base: Option<String>,
    expiry_parent_base: Option<String>,
    expiry_result: Option<Expiry>,
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

// --- tag listing age (TagAge::classify) --------------------------------------

#[given(regex = r"^the tag request time is (-?\d+)$")]
fn given_tag_age_now(world: &mut DomainWorld, now: i64) {
    world.tag_age_now = now;
}

#[given(regex = r"^a tag created at (-?\d+)$")]
fn given_tag_created_at(world: &mut DomainWorld, epoch: i64) {
    world.tag_age_creation = Some(epoch);
}

#[given("a tag with no recorded creation time")]
fn given_tag_no_creation(world: &mut DomainWorld) {
    world.tag_age_creation = None;
}

#[when("I classify the tag age")]
fn classify_tag_age(world: &mut DomainWorld) {
    world.tag_age_result = Some(TagAge::classify(world.tag_age_creation, world.tag_age_now));
}

#[then(regex = r#"^the tag age is "([^"]*)"$"#)]
fn tag_age_is_known(world: &mut DomainWorld, expected: String) {
    let TagAge::Known(age) = world.tag_age_result.expect("classify the tag age first") else {
        panic!("expected a known tag age");
    };
    assert_eq!(age.humanized(), expected);
}

#[then("the tag age is unknown")]
fn tag_age_is_unknown(world: &mut DomainWorld) {
    assert_eq!(
        world.tag_age_result.expect("classify the tag age first"),
        TagAge::Unknown
    );
}

#[then("the tag age has no cell")]
fn tag_age_is_absent(world: &mut DomainWorld) {
    assert_eq!(
        world.tag_age_result.expect("classify the tag age first"),
        TagAge::Absent
    );
}

#[given(regex = r#"^a timestamp at epoch (-?\d+) with timezone "(.*)"$"#)]
fn given_timestamp(world: &mut DomainWorld, epoch: i64, tz: String) {
    world.timestamp = Some(Timestamp::new(epoch, &tz));
}

#[given("there is no timestamp")]
fn given_no_timestamp(world: &mut DomainWorld) {
    world.timestamp = None;
}

#[when("I render its RFC-2822 form")]
fn render_rfc2822(world: &mut DomainWorld) {
    world.timestamp_text = world.timestamp.as_ref().map(|t: &Timestamp| t.rfc2822());
}

#[when("I render its ISO-8601 form")]
fn render_iso8601(world: &mut DomainWorld) {
    world.timestamp_text = world.timestamp.as_ref().map(|t: &Timestamp| t.iso8601());
}

#[when("I render its RFC-2822 local form")]
fn render_rfc2822_local(world: &mut DomainWorld) {
    world.timestamp_text = world
        .timestamp
        .as_ref()
        .map(|t: &Timestamp| t.rfc2822_local());
}

#[when("I render its local form")]
fn render_local(world: &mut DomainWorld) {
    world.timestamp_text = world.timestamp.as_ref().map(|t: &Timestamp| t.iso_tz());
}

#[when("I check whether it is at night")]
fn check_at_night(world: &mut DomainWorld) {
    world.at_night = world
        .timestamp
        .as_ref()
        .map(|t: &Timestamp| t.is_at_night());
}

#[then(regex = r#"^the timestamp reads "(.*)"$"#)]
fn timestamp_reads(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.timestamp_text.as_deref(), Some(expected.as_str()));
}

#[then("nothing is rendered")]
fn nothing_is_rendered(world: &mut DomainWorld) {
    assert_eq!(world.timestamp_text, None);
}

#[then(regex = r"^at-night is (true|false)$")]
fn at_night_is(world: &mut DomainWorld, expected: bool) {
    assert_eq!(world.at_night, Some(expected));
}

#[then(regex = r"^the local hour is (\d+)$")]
fn local_hour_is(world: &mut DomainWorld, expected: u8) {
    let timestamp: &Timestamp = world.timestamp.as_ref().expect("a timestamp");
    assert_eq!(timestamp.local_hour(), expected);
}

#[then(regex = r"^the local minute is (\d+)$")]
fn local_minute_is(world: &mut DomainWorld, expected: u8) {
    let timestamp: &Timestamp = world.timestamp.as_ref().expect("a timestamp");
    assert_eq!(timestamp.local_minute(), expected);
}

#[then(regex = r#"^the displayed timezone is "(.*)"$"#)]
fn displayed_timezone_is(world: &mut DomainWorld, expected: String) {
    let timestamp: &Timestamp = world.timestamp.as_ref().expect("a timestamp");
    assert_eq!(timestamp.timezone(), expected);
}

#[given(regex = r"^a commit dated at epoch (-?\d+) viewed at (-?\d+)$")]
fn given_commit_dated(world: &mut DomainWorld, epoch: i64, now: i64) {
    world.commit_date_input = Some((epoch, now));
}

#[when("I form its date cell")]
fn form_date_cell(world: &mut DomainWorld) {
    let (epoch, now): (i64, i64) = world.commit_date_input.expect("a commit date input");
    world.commit_date = Some(CommitDate::new(epoch, now));
}

#[then(regex = r#"^the date cell shows "(.*)"$"#)]
fn date_cell_shows(world: &mut DomainWorld, expected: String) {
    let date: &CommitDate = world.commit_date.as_ref().expect("a formed date cell");
    assert_eq!(date.displayed(), expected);
}

#[then(regex = r#"^the date cell tooltip is "(.*)"$"#)]
fn date_cell_tooltip_is(world: &mut DomainWorld, expected: String) {
    let date: &CommitDate = world.commit_date.as_ref().expect("a formed date cell");
    assert_eq!(date.tooltip(), expected);
}

// --- Log comment body (git_print_log) ----------------------------------------

/// The processed log body, or a panic if it was not split yet.
fn log_body(world: &DomainWorld) -> &[LogLine] {
    world.log_body.as_deref().expect("split the log body first")
}

/// One processed log line by its 1-based position, or a panic if absent.
fn log_body_line(world: &DomainWorld, number: usize) -> &LogLine {
    log_body(world)
        .get(number - 1)
        .unwrap_or_else(|| panic!("no log line {number}"))
}

#[given("an empty commit message")]
fn given_empty_commit_message(world: &mut DomainWorld) {
    world.log_message = String::new();
}

#[given(regex = r#"^the commit message "(.*)"$"#)]
fn given_inline_commit_message(world: &mut DomainWorld, message: String) {
    world.log_message = message;
}

#[given("the commit message:")]
fn given_block_commit_message(world: &mut DomainWorld, step: &Step) {
    world.log_message = step
        .docstring
        .clone()
        .expect("scenario must supply a docstring");
}

#[when("I split the log body")]
fn split_the_log_body(world: &mut DomainWorld) {
    world.log_body = Some(log_lines(&world.log_message));
}

#[then(regex = r"^the log body has (\d+) lines?$")]
fn log_body_has_lines(world: &mut DomainWorld, count: usize) {
    assert_eq!(log_body(world).len(), count);
}

#[then(regex = r#"^log line (\d+) is text "(.*)"$"#)]
fn log_line_is_text(world: &mut DomainWorld, number: usize, expected: String) {
    assert_eq!(log_body_line(world, number), &LogLine::Text(expected));
}

#[then(regex = r"^log line (\d+) is blank$")]
fn log_line_is_blank(world: &mut DomainWorld, number: usize) {
    assert_eq!(log_body_line(world, number), &LogLine::Text(String::new()));
}

#[then(regex = r#"^log line (\d+) is a sign-off "(.*)"$"#)]
fn log_line_is_signoff(world: &mut DomainWorld, number: usize, expected: String) {
    assert_eq!(log_body_line(world, number), &LogLine::Signoff(expected));
}

#[then(regex = r#"^log line (\d+) is an autolink labelled "(.*)" to "(.*)"$"#)]
fn log_line_is_autolink(world: &mut DomainWorld, number: usize, label: String, url: String) {
    assert_eq!(
        log_body_line(world, number),
        &LogLine::Autolink { label, url }
    );
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

#[then(regex = r#"^the mode's object kind is "(.*)"$"#)]
fn mode_object_kind_is(world: &mut DomainWorld, expected: String) {
    let mode: FileMode = world.file_mode.expect("a valid mode");
    assert_eq!(mode.object_kind().as_str(), expected);
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

#[when(regex = r"^the search help topics are listed with grep (on|off) and pickaxe (on|off)$")]
fn list_search_help_topics(world: &mut DomainWorld, grep: String, pickaxe: String) {
    world.search_help_topics = Some(help_topics(grep == "on", pickaxe == "on"));
}

#[then(regex = r#"^the topics are "(.*)"$"#)]
fn topics_are(world: &mut DomainWorld, expected: String) {
    let names: Vec<&str> = world
        .search_help_topics
        .as_ref()
        .expect("list the topics first")
        .iter()
        .map(|topic: &SearchHelpTopic| topic.name())
        .collect();
    assert_eq!(names.join(", "), expected);
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

#[when(regex = r#"^I classify the blob as "([^"]*)"$"#)]
fn classify_blob_named(world: &mut DomainWorld, file_name: String) {
    let blob: Blob = Blob::new(world.bytes.clone());
    world.blob_display = Some(blob.display_kind(Some(&file_name)));
}

#[when("I classify the blob with no file name")]
fn classify_blob_unnamed(world: &mut DomainWorld) {
    let blob: Blob = Blob::new(world.bytes.clone());
    world.blob_display = Some(blob.display_kind(None));
}

#[then("the blob displays as text")]
fn blob_displays_text(world: &mut DomainWorld) {
    assert_eq!(world.blob_display, Some(BlobDisplay::Text));
}

#[then("the blob displays as an image")]
fn blob_displays_image(world: &mut DomainWorld) {
    assert_eq!(world.blob_display, Some(BlobDisplay::Image));
}

#[then("the blob displays as binary")]
fn blob_displays_binary(world: &mut DomainWorld) {
    assert_eq!(world.blob_display, Some(BlobDisplay::Binary));
}

/// The resolved blob id the blob_plain "save as" name falls back to when no file
/// name is given — matches the id named in `content_type.feature`.
const RAW_BLOB_ID: &str = "0a1b2c3";

/// Resolves the raw-serve headers for the given bytes under the named file (or
/// none), charset, and XSS-prevention flag, and stashes them for assertion.
fn serve_raw(
    world: &mut DomainWorld,
    file_name: Option<&str>,
    charset: Option<&str>,
    prevent_xss: bool,
) {
    let is_binary: bool = Blob::new(world.bytes.clone()).is_binary();
    world.plain_headers = Some(PlainHeaders::resolve(
        file_name,
        is_binary,
        RAW_BLOB_ID,
        charset,
        prevent_xss,
    ));
}

#[when(regex = r#"^I serve it raw as "([^"]*)"$"#)]
fn serve_raw_named(world: &mut DomainWorld, file_name: String) {
    serve_raw(world, Some(&file_name), None, false);
}

#[when("I serve it raw with no file name")]
fn serve_raw_unnamed(world: &mut DomainWorld) {
    serve_raw(world, None, None, false);
}

#[when(regex = r#"^I serve it raw as "([^"]*)" with charset "([^"]*)"$"#)]
fn serve_raw_with_charset(world: &mut DomainWorld, file_name: String, charset: String) {
    serve_raw(world, Some(&file_name), Some(&charset), false);
}

#[when(regex = r#"^I serve it raw as "([^"]*)" with XSS prevention$"#)]
fn serve_raw_with_xss(world: &mut DomainWorld, file_name: String) {
    serve_raw(world, Some(&file_name), None, true);
}

fn served_headers(world: &DomainWorld) -> &PlainHeaders {
    world
        .plain_headers
        .as_ref()
        .expect("serve the blob raw before asserting")
}

#[then(regex = r#"^it is served as "([^"]*)"$"#)]
fn served_as(world: &mut DomainWorld, expected: String) {
    assert_eq!(served_headers(world).content_type(), expected);
}

#[then(regex = r#"^it is offered inline as "([^"]*)"$"#)]
fn offered_inline_as(world: &mut DomainWorld, file_name: String) {
    assert_eq!(
        served_headers(world).content_disposition(),
        format!(r#"inline; filename="{file_name}""#)
    );
}

#[then(regex = r#"^it is offered as an attachment named "([^"]*)"$"#)]
fn offered_attachment_named(world: &mut DomainWorld, file_name: String) {
    assert_eq!(
        served_headers(world).content_disposition(),
        format!(r#"attachment; filename="{file_name}""#)
    );
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

#[then(regex = r#"^the permission bits are "(.*)"$"#)]
fn permission_bits_are(world: &mut DomainWorld, expected: String) {
    let mode: FileMode = world.file_mode.expect("a valid mode");
    assert_eq!(mode.permission_bits(), expected);
}

#[then(regex = r"^the mode is regular is (true|false)$")]
fn mode_is_regular_is(world: &mut DomainWorld, expected: bool) {
    let mode: FileMode = world.file_mode.expect("a valid mode");
    assert_eq!(mode.is_regular(), expected);
}

#[given(
    regex = r#"^a change with status "(added|deleted|modified)", from mode "(.*)", to mode "(.*)"$"#
)]
fn given_plain_change(world: &mut DomainWorld, status: String, from: String, to: String) {
    let from_mode: FileMode = mode_of(&from);
    let to_mode: FileMode = mode_of(&to);
    world.note_from = Some(from_mode);
    world.note_to = Some(to_mode);
    world.note_status = Some(plain_status(&status, from_mode, to_mode));
}

#[given(
    regex = r#"^a change with status "(renamed|copied)" similarity (\d+), from mode "(.*)", to mode "(.*)"$"#
)]
fn given_rename_change(world: &mut DomainWorld, status: String, sim: u8, from: String, to: String) {
    world.note_from = Some(mode_of(&from));
    world.note_to = Some(mode_of(&to));
    world.note_status = Some(rename_status(&status, sim));
}

#[when("I derive the file-change note")]
fn derive_file_change_note(world: &mut DomainWorld) {
    let status: ChangeStatus = world.note_status.expect("a change status");
    let from: FileMode = world.note_from.expect("a from mode");
    let to: FileMode = world.note_to.expect("a to mode");
    world.derived_note = Some(FileChangeNote::derive(status, from, to));
}

#[then(regex = r#"^the note category is "(.*)"$"#)]
fn note_category_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(note(world).category(), expected);
}

#[then(regex = r#"^the note file type is "(.*)"$"#)]
fn note_file_type_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(note(world).file_type().expect("a file type"), expected);
}

#[then(regex = r#"^the note mode is "(.*)"$"#)]
fn note_mode_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(note(world).mode().unwrap_or("none"), expected);
}

#[then(regex = r#"^the note text is "(.*)"$"#)]
fn note_text_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(note(world).text().expect("link-free note text"), expected);
}

#[then(regex = r"^the note similarity is (\d+)$")]
fn note_similarity_is(world: &mut DomainWorld, expected: u8) {
    assert_eq!(note(world).similarity().expect("a similarity"), expected);
}

#[then("there is no note")]
fn there_is_no_note(world: &mut DomainWorld) {
    let result: &Option<FileChangeNote> =
        world.derived_note.as_ref().expect("derive the note first");
    assert!(result.is_none(), "expected no note, got {result:?}");
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

#[given(regex = r#"^a request action "(.*)"$"#)]
fn given_request_action(world: &mut DomainWorld, name: String) {
    world.action_name = name;
}

#[when("I read the action")]
fn read_action(world: &mut DomainWorld) {
    world.parsed_action = Action::parse(&world.action_name);
}

#[then(regex = r#"^the action's wire name is "(.*)"$"#)]
fn action_wire_name_is(world: &mut DomainWorld, expected: String) {
    let action: Action = world.parsed_action.expect("an action must be parsed");
    assert_eq!(action.as_str(), expected);
}

#[then("there is no action")]
fn there_is_no_action(world: &mut DomainWorld) {
    assert_eq!(world.parsed_action, None);
}

#[then(regex = r#"^needing a project reads "(yes|no)"$"#)]
fn needing_a_project_reads(world: &mut DomainWorld, expected: String) {
    let action: Action = world.parsed_action.expect("an action must be parsed");
    let reads: &str = if action.needs_project() { "yes" } else { "no" };
    assert_eq!(reads, expected);
}

#[given("a bare request")]
fn given_a_bare_request(_world: &mut DomainWorld) {}

#[given(regex = r#"^a request parameter "(.*)" is "(.*)"$"#)]
fn given_a_request_parameter(world: &mut DomainWorld, key: String, value: String) {
    world.request_params.push((key, value));
}

#[when("I parse the request parameters")]
fn parse_the_request_parameters(world: &mut DomainWorld) {
    world.request_result = Some(Request::from_query(&world.request_params));
}

fn parsed_request(world: &DomainWorld) -> &Request {
    match world
        .request_result
        .as_ref()
        .expect("the request must be parsed")
    {
        Ok(request) => request,
        Err(error) => panic!("expected the request to parse, but it was rejected: {error}"),
    }
}

#[then("the request parses")]
fn the_request_parses(world: &mut DomainWorld) {
    let _request: &Request = parsed_request(world);
}

#[then("the request has no action")]
fn the_request_has_no_action(world: &mut DomainWorld) {
    assert_eq!(parsed_request(world).action, None);
}

#[then("the request has no project")]
fn the_request_has_no_project(world: &mut DomainWorld) {
    assert_eq!(parsed_request(world).project, None);
}

#[then("the request project is absent")]
fn the_request_project_is_absent(world: &mut DomainWorld) {
    assert_eq!(parsed_request(world).project, None);
}

#[then(regex = r#"^the request action is "(.*)"$"#)]
fn the_request_action_is(world: &mut DomainWorld, expected: String) {
    let action: Action = parsed_request(world)
        .action
        .expect("the request must have an action");
    assert_eq!(action.as_str(), expected);
}

#[then(regex = r#"^the request hash base is "(.*)"$"#)]
fn the_request_hash_base_is(world: &mut DomainWorld, expected: String) {
    let request: &Request = parsed_request(world);
    let hash_base: &SafeRef = request
        .hash_base
        .as_ref()
        .expect("the request must have a hash base");
    assert_eq!(hash_base.as_str(), expected);
}

#[then(regex = r#"^the request hash is "(.*)"$"#)]
fn the_request_hash_is(world: &mut DomainWorld, expected: String) {
    let request: &Request = parsed_request(world);
    let hash: &SafeRef = request.hash.as_ref().expect("the request must have a hash");
    assert_eq!(hash.as_str(), expected);
}

#[then(regex = r#"^the request file name is "(.*)"$"#)]
fn the_request_file_name_is(world: &mut DomainWorld, expected: String) {
    let request: &Request = parsed_request(world);
    let file_name: &SafePath = request
        .file_name
        .as_ref()
        .expect("the request must have a file name");
    assert_eq!(file_name.as_str(), expected);
}

#[then(regex = r#"^the request project is "(.*)"$"#)]
fn the_request_project_is(world: &mut DomainWorld, expected: String) {
    let request: &Request = parsed_request(world);
    let project: &str = request
        .project
        .as_deref()
        .expect("the request must have a project");
    assert_eq!(project, expected);
}

#[then(regex = r#"^the request page is (\d+)$"#)]
fn the_request_page_is(world: &mut DomainWorld, expected: u32) {
    assert_eq!(parsed_request(world).page, Some(expected));
}

#[then(regex = r#"^the request search type is "(.*)"$"#)]
fn the_request_search_type_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        parsed_request(world).search_type.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the request search text is "(.*)"$"#)]
fn the_request_search_text_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        parsed_request(world).search_text.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the request keeps the extra option "(.*)"$"#)]
fn the_request_keeps_the_extra_option(world: &mut DomainWorld, expected: String) {
    let request: &Request = parsed_request(world);
    assert!(
        request
            .extra_options
            .iter()
            .any(|option: &String| option == &expected)
    );
}

fn rejection(world: &DomainWorld) -> &DomainError {
    match world
        .request_result
        .as_ref()
        .expect("the request must be parsed")
    {
        Ok(_) => panic!("expected the request to be rejected, but it parsed"),
        Err(error) => error,
    }
}

#[then("the request is rejected as invalid")]
fn the_request_is_rejected_as_invalid(world: &mut DomainWorld) {
    assert!(matches!(rejection(world), DomainError::Invalid(_)));
}

#[then("the request is rejected as not found")]
fn the_request_is_rejected_as_not_found(world: &mut DomainWorld) {
    assert!(matches!(rejection(world), DomainError::NotFound(_)));
}

#[then("the request is rejected as forbidden")]
fn the_request_is_rejected_as_forbidden(world: &mut DomainWorld) {
    assert!(matches!(rejection(world), DomainError::Forbidden(_)));
}

#[given(regex = r#"^the request path "(.*)"$"#)]
fn given_the_request_path(world: &mut DomainWorld, path: String) {
    world.path_info_input = path;
}

#[when("I decompose the request path")]
fn decompose_the_request_path(world: &mut DomainWorld) {
    world.path_info = Some(PathInfo::parse(&world.path_info_input));
}

fn decomposed(world: &DomainWorld) -> &PathInfo {
    world
        .path_info
        .as_ref()
        .expect("the request path must be decomposed")
}

#[then("the decomposed request is empty")]
fn the_decomposed_request_is_empty(world: &mut DomainWorld) {
    assert_eq!(decomposed(world), &PathInfo::default());
}

#[then("the decomposed request has no action")]
fn the_decomposed_request_has_no_action(world: &mut DomainWorld) {
    assert_eq!(decomposed(world).action, None);
}

#[then("the decomposed request has no hash")]
fn the_decomposed_request_has_no_hash(world: &mut DomainWorld) {
    assert_eq!(decomposed(world).hash, None);
}

#[then(regex = r#"^the decomposed action is "(.*)"$"#)]
fn the_decomposed_action_is(world: &mut DomainWorld, expected: String) {
    let action: Action = decomposed(world)
        .action
        .expect("the path must yield an action");
    assert_eq!(action.as_str(), expected);
}

#[then(regex = r#"^the decomposed hash is "(.*)"$"#)]
fn the_decomposed_hash_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(decomposed(world).hash.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the decomposed hash base is "(.*)"$"#)]
fn the_decomposed_hash_base_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        decomposed(world).hash_base.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the decomposed hash parent is "(.*)"$"#)]
fn the_decomposed_hash_parent_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        decomposed(world).hash_parent.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the decomposed hash parent base is "(.*)"$"#)]
fn the_decomposed_hash_parent_base_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        decomposed(world).hash_parent_base.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the decomposed file name is "(.*)"$"#)]
fn the_decomposed_file_name_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        decomposed(world).file_name.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the decomposed file parent is "(.*)"$"#)]
fn the_decomposed_file_parent_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        decomposed(world).file_parent.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the decomposed snapshot format is "(.*)"$"#)]
fn the_decomposed_snapshot_format_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(
        decomposed(world).snapshot_format.as_deref(),
        Some(expected.as_str())
    );
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

/// The change status a plain (non-rename) note scenario names: a created or
/// removed path, or a modification re-derived from its two modes.
fn plain_status(word: &str, from: FileMode, to: FileMode) -> ChangeStatus {
    match word {
        "added" => ChangeStatus::added(),
        "deleted" => ChangeStatus::deleted(),
        _ => ChangeStatus::from_modification(from, to),
    }
}

/// The change status a rename/copy note scenario names, carrying its similarity.
fn rename_status(word: &str, similarity: u8) -> ChangeStatus {
    match word {
        "copied" => ChangeStatus::copied(similarity),
        _ => ChangeStatus::renamed(similarity),
    }
}

/// The file-change note the current scenario derived (asserting it is present).
fn note(world: &DomainWorld) -> &FileChangeNote {
    world
        .derived_note
        .as_ref()
        .expect("derive the note first")
        .as_ref()
        .expect("a note")
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

/// The owned form of a [`FileSelection`], so the resolution outcome can outlive
/// the borrow of the patch it was selected from.
#[derive(Debug, PartialEq, Eq)]
enum FileResolution {
    Found { from: String, to: String },
    Missing,
    Ambiguous,
}

/// Reduces a [`FileSelection`] (which borrows the patch) to the owned
/// [`FileResolution`] the world stores.
fn owned_selection(selection: FileSelection<'_>) -> FileResolution {
    match selection {
        FileSelection::One(file) => FileResolution::Found {
            from: file.from_path().to_owned(),
            to: file.to_path().to_owned(),
        },
        FileSelection::Missing => FileResolution::Missing,
        FileSelection::Ambiguous => FileResolution::Ambiguous,
    }
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

#[given("a patch over two files added with identical content")]
fn given_twin_adds_patch(world: &mut DomainWorld) {
    // Both created from nothing with the same two lines, so they share a new-side
    // blob id (created_patch's `to_oid` is the same digit fill) — the only way a
    // by-id blobdiff resolution is ambiguous.
    world.patch_under_test = Some(Patch::new(vec![
        created_patch("twin-a.txt"),
        created_patch("twin-b.txt"),
    ]));
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

#[when(regex = r#"^I render the patch abbreviated to (\d+)$"#)]
fn render_patch_abbreviated(world: &mut DomainWorld, len: usize) {
    let patch: &Patch = world
        .patch_under_test
        .as_ref()
        .expect("a patch must be built before rendering");
    world.rendered = Some(patch.render_abbreviated(len));
}

fn rendered(world: &DomainWorld) -> &str {
    world
        .rendered
        .as_deref()
        .expect("the patch must be rendered before asserting on it")
}

#[when(regex = r#"^I select the file "([^"]+)" abbreviated to (\d+)$"#)]
fn select_file_abbreviated(world: &mut DomainWorld, path: String, len: usize) {
    let patch: &Patch = world
        .patch_under_test
        .as_ref()
        .expect("a patch must be built before selecting");
    world.selected_file = Some(patch.render_file_abbreviated(&path, len));
}

/// The text of the selected file patch, asserting one was found.
fn selected(world: &DomainWorld) -> &str {
    world
        .selected_file
        .as_ref()
        .expect("a file must be selected before asserting on it")
        .as_deref()
        .expect("expected a file patch to be selected, found none")
}

#[then(regex = r#"^the selected patch contains "(.*)"$"#)]
fn then_selected_contains(world: &mut DomainWorld, fragment: String) {
    assert!(
        selected(world).contains(&fragment),
        "expected selection to contain {fragment:?}, got:\n{}",
        selected(world)
    );
}

#[then(regex = r#"^the selected patch does not contain "(.*)"$"#)]
fn then_selected_not_contains(world: &mut DomainWorld, fragment: String) {
    assert!(
        !selected(world).contains(&fragment),
        "expected selection not to contain {fragment:?}, got:\n{}",
        selected(world)
    );
}

#[then(regex = r#"^the selected patch has a line "(.*)"$"#)]
fn then_selected_has_line(world: &mut DomainWorld, expected: String) {
    assert!(
        selected(world).lines().any(|line: &str| line == expected),
        "expected selection to have the exact line {expected:?}, got:\n{}",
        selected(world)
    );
}

#[then("no file patch is selected")]
fn then_no_file_selected(world: &mut DomainWorld) {
    assert_eq!(
        world
            .selected_file
            .as_ref()
            .expect("the selection must have run"),
        &None
    );
}

#[when(regex = r#"^I render the file "([^"]+)"$"#)]
fn render_file(world: &mut DomainWorld, path: String) {
    let patch: &Patch = world
        .patch_under_test
        .as_ref()
        .expect("a patch must be built before rendering");
    world.selected_file = Some(patch.render_file(&path));
}

#[when(regex = r#"^I resolve the file by path "([^"]+)"$"#)]
fn resolve_by_path(world: &mut DomainWorld, path: String) {
    let patch: &Patch = world
        .patch_under_test
        .as_ref()
        .expect("a patch must be built before resolving");
    world.file_resolution = Some(owned_selection(patch.select_by_to_path(&path)));
}

#[when(regex = r#"^I resolve the file by new-side id "([^"]+)"$"#)]
fn resolve_by_id(world: &mut DomainWorld, id: String) {
    let patch: &Patch = world
        .patch_under_test
        .as_ref()
        .expect("a patch must be built before resolving");
    world.file_resolution = Some(owned_selection(patch.select_by_to_oid(&id)));
}

/// The file-resolution outcome, asserting one was produced.
fn file_resolution(world: &DomainWorld) -> &FileResolution {
    world
        .file_resolution
        .as_ref()
        .expect("a file must be resolved before asserting on it")
}

#[then(regex = r#"^the resolution finds the file from "([^"]+)" to "([^"]+)"$"#)]
fn then_resolution_finds(world: &mut DomainWorld, from: String, to: String) {
    assert_eq!(file_resolution(world), &FileResolution::Found { from, to });
}

#[then("the resolution finds no file")]
fn then_resolution_missing(world: &mut DomainWorld) {
    assert_eq!(file_resolution(world), &FileResolution::Missing);
}

#[then("the resolution is ambiguous")]
fn then_resolution_ambiguous(world: &mut DomainWorld) {
    assert_eq!(file_resolution(world), &FileResolution::Ambiguous);
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

// --- Diffstat (git format-patch --stat --summary) ----------------------------

/// The zero (all-octal-zeros) mode of a side that does not exist, git's `0` mode
/// (`is_set()` false): a created file has no old mode, a deleted file no new one.
fn zero_mode() -> FileMode {
    FileMode::from_octal("000000").expect("the zero mode is octal")
}

/// An octal mode token, parsed the way the diff feeds modes in.
fn mode(octal: &str) -> FileMode {
    FileMode::from_octal(octal).unwrap_or_else(|| panic!("{octal:?} is an octal mode"))
}

#[given("a diffstat")]
fn given_diffstat(world: &mut DomainWorld) {
    world.diffstat_entries = Vec::new();
}

#[given(regex = r#"^a created file "([^"]+)" mode "([^"]+)" with (\d+) added (\d+) deleted$"#)]
fn given_diffstat_created(
    world: &mut DomainWorld,
    path: String,
    octal: String,
    added: u32,
    deleted: u32,
) {
    world.diffstat_entries.push(DiffstatEntry::new(
        ChangeStatus::added(),
        zero_mode(),
        mode(&octal),
        path.clone(),
        path,
        StatChange::Text { added, deleted },
    ));
}

#[given(regex = r#"^a deleted file "([^"]+)" mode "([^"]+)" with (\d+) added (\d+) deleted$"#)]
fn given_diffstat_deleted(
    world: &mut DomainWorld,
    path: String,
    octal: String,
    added: u32,
    deleted: u32,
) {
    world.diffstat_entries.push(DiffstatEntry::new(
        ChangeStatus::deleted(),
        mode(&octal),
        zero_mode(),
        path.clone(),
        path,
        StatChange::Text { added, deleted },
    ));
}

#[given(regex = r#"^a modified file "([^"]+)" mode "([^"]+)" with (\d+) added (\d+) deleted$"#)]
fn given_diffstat_modified(
    world: &mut DomainWorld,
    path: String,
    octal: String,
    added: u32,
    deleted: u32,
) {
    let file_mode: FileMode = mode(&octal);
    world.diffstat_entries.push(DiffstatEntry::new(
        ChangeStatus::from_modification(file_mode, file_mode),
        file_mode,
        file_mode,
        path.clone(),
        path,
        StatChange::Text { added, deleted },
    ));
}

#[given(regex = r#"^a mode change on "([^"]+)" from "([^"]+)" to "([^"]+)"$"#)]
fn given_diffstat_mode_change(world: &mut DomainWorld, path: String, from: String, to: String) {
    let from_mode: FileMode = mode(&from);
    let to_mode: FileMode = mode(&to);
    world.diffstat_entries.push(DiffstatEntry::new(
        ChangeStatus::from_modification(from_mode, to_mode),
        from_mode,
        to_mode,
        path.clone(),
        path,
        StatChange::Text {
            added: 0,
            deleted: 0,
        },
    ));
}

#[given(regex = r#"^a renamed file from "([^"]+)" to "([^"]+)" similarity (\d+) mode "([^"]+)"$"#)]
fn given_diffstat_renamed(
    world: &mut DomainWorld,
    from: String,
    to: String,
    similarity: u8,
    octal: String,
) {
    let file_mode: FileMode = mode(&octal);
    world.diffstat_entries.push(DiffstatEntry::new(
        ChangeStatus::renamed(similarity),
        file_mode,
        file_mode,
        from,
        to,
        StatChange::Text {
            added: 0,
            deleted: 0,
        },
    ));
}

#[given(regex = r#"^a binary file "([^"]+)" mode "([^"]+)" sized (\d+) to (\d+)$"#)]
fn given_diffstat_binary(
    world: &mut DomainWorld,
    path: String,
    octal: String,
    old_size: u64,
    new_size: u64,
) {
    let file_mode: FileMode = mode(&octal);
    world.diffstat_entries.push(DiffstatEntry::new(
        ChangeStatus::from_modification(file_mode, file_mode),
        file_mode,
        file_mode,
        path.clone(),
        path,
        StatChange::Binary { old_size, new_size },
    ));
}

#[when("I render the diffstat")]
fn render_diffstat(world: &mut DomainWorld) {
    let diffstat: Diffstat = Diffstat::new(std::mem::take(&mut world.diffstat_entries));
    world.diffstat_text = Some(diffstat.render());
}

// --- FormatPatch (git format-patch mailbox framing) --------------------------

#[given(
    regex = r#"^a patch mail for commit "([^"]+)" by "([^"]+)" at epoch (-?\d+) zone "([^"]*)"$"#
)]
fn given_patch_mail(
    world: &mut DomainWorld,
    commit_id: String,
    author: String,
    epoch: i64,
    zone: String,
) {
    world.fp_commit_id = commit_id;
    world.fp_author = author;
    world.fp_date = Some(Timestamp::new(epoch, &zone));
    world.fp_subject = String::new();
    world.fp_number = None;
    world.fp_body = Vec::new();
    world.fp_diff_body = String::new();
    world.diffstat_entries = Vec::new();
}

#[given(regex = r#"^the patch subject is "([^"]*)"$"#)]
fn given_patch_subject(world: &mut DomainWorld, subject: String) {
    world.fp_subject = subject;
}

#[given(regex = r"^it is patch (\d+) of (\d+)$")]
fn given_patch_number(world: &mut DomainWorld, index: usize, total: usize) {
    world.fp_number = Some((index, total));
}

#[given(regex = r#"^the patch body line is "([^"]*)"$"#)]
fn given_patch_body_line(world: &mut DomainWorld, line: String) {
    world.fp_body.push(line);
}

#[given("the patch diff body is:")]
fn given_patch_diff_body(world: &mut DomainWorld, step: &Step) {
    world.fp_diff_body = format!(
        "{}\n",
        step.docstring
            .as_deref()
            .expect("a diff body docstring")
            .trim_matches('\n')
    );
}

#[given("the mail is complete")]
fn given_mail_complete(world: &mut DomainWorld) {
    let diffstat: Diffstat = Diffstat::new(std::mem::take(&mut world.diffstat_entries));
    let entry: PatchEntry = PatchEntry::new(
        std::mem::take(&mut world.fp_commit_id),
        std::mem::take(&mut world.fp_author),
        world.fp_date.take().expect("a patch mail date"),
        std::mem::take(&mut world.fp_subject),
        world.fp_number.take(),
        std::mem::take(&mut world.fp_body),
        diffstat,
        std::mem::take(&mut world.fp_diff_body),
    );
    world.fp_entries.push(entry);
}

#[when(regex = r#"^I render the format-patch stream with version "([^"]*)"$"#)]
fn render_format_patch(world: &mut DomainWorld, version: String) {
    let stream: FormatPatch = FormatPatch::new(std::mem::take(&mut world.fp_entries), version);
    world.fp_text = Some(stream.render());
}

#[then("the format-patch stream is:")]
fn then_format_patch_is(world: &mut DomainWorld, step: &Step) {
    let doc: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    // The doc-string drops the trailing space of git's `-- ` signature delimiter
    // (and the stream's trailing blank); restore the delimiter to compare bytes.
    let expected: String = doc
        .lines()
        .map(|line: &str| {
            if line == "--" {
                "-- ".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
    let actual: &str = world.fp_text.as_deref().expect("render the stream first");
    assert_eq!(actual.trim_end_matches('\n'), expected);
}

#[then("the diffstat is:")]
fn then_diffstat_is(world: &mut DomainWorld, step: &Step) {
    let doc: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    // git leads every diffstat line with a single space; the doc-string dedent
    // strips that uniform column, so re-add it to compare against git's bytes.
    let expected: String = doc
        .lines()
        .map(|line: &str| format!(" {line}"))
        .collect::<Vec<String>>()
        .join("\n");
    let actual: &str = world
        .diffstat_text
        .as_deref()
        .expect("render the diffstat first");
    assert_eq!(actual.trim_end_matches('\n'), expected);
}

// --- commitdiff diff base ----------------------------------------------------

/// A deterministic object id for a short test label (e.g. `"aaaa"`): its hex
/// digits cycled to the full forty, so the same label always names the same id.
fn commitdiff_oid(label: &str) -> ObjectId {
    let hex: String = label.chars().filter(char::is_ascii_hexdigit).collect();
    let full: String = hex.chars().cycle().take(40).collect();
    ObjectId::parse(&full).expect("a label yields a valid object id")
}

#[given("a commit with no parents")]
fn given_commitdiff_no_parents(world: &mut DomainWorld) {
    world.commitdiff_parents = Vec::new();
}

#[given(regex = r#"^a commit with parent "([^"]*)"$"#)]
fn given_commitdiff_parent(world: &mut DomainWorld, label: String) {
    world.commitdiff_parents = vec![commitdiff_oid(&label)];
}

#[given(regex = r#"^the commit also has parent "([^"]*)"$"#)]
fn given_commitdiff_also_parent(world: &mut DomainWorld, label: String) {
    world.commitdiff_parents.push(commitdiff_oid(&label));
}

#[given(regex = r#"^an explicit parent "([^"]*)" is given$"#)]
fn given_commitdiff_explicit(world: &mut DomainWorld, label: String) {
    world.commitdiff_explicit = Some(commitdiff_oid(&label));
}

#[when("I pick the commitdiff base")]
fn pick_commitdiff_base(world: &mut DomainWorld) {
    world.commitdiff_base = Some(diff_base(
        &world.commitdiff_parents,
        world.commitdiff_explicit.as_ref(),
    ));
}

#[then("the commitdiff base is the empty tree")]
fn then_commitdiff_base_empty(world: &mut DomainWorld) {
    assert_eq!(
        world.commitdiff_base.as_ref().expect("a base was picked"),
        &DiffBase::EmptyTree
    );
}

#[then(regex = r#"^the commitdiff base is commit "([^"]*)"$"#)]
fn then_commitdiff_base_commit(world: &mut DomainWorld, label: String) {
    assert_eq!(
        world.commitdiff_base.as_ref().expect("a base was picked"),
        &DiffBase::Commit(commitdiff_oid(&label))
    );
}

#[then("the patch is empty")]
fn then_patch_is_empty(world: &mut DomainWorld) {
    assert_eq!(rendered(world), "");
}

// --- commitdiff_plain body format --------------------------------------------

/// The fixed author timestamp the commitdiff_plain scenarios render against:
/// epoch 1_700_000_000 at `+0000`, which is `Tue, 14 Nov 2023 22:13:20 +0000`.
const CDP_EPOCH: i64 = 1_700_000_000;

#[given(regex = r#"^a commitdiff_plain by "([^"]*)" titled "([^"]*)"$"#)]
fn given_commitdiff_plain(world: &mut DomainWorld, author: String, subject: String) {
    world.cdp_author = Some(author);
    world.cdp_subject = Some(subject);
}

#[given(regex = r#"^its comment line is "([^"]*)"$"#)]
fn given_commitdiff_plain_comment(world: &mut DomainWorld, line: String) {
    world.cdp_comment.push(line);
}

#[given(regex = r#"^it is tag-named "([^"]*)"$"#)]
fn given_commitdiff_plain_tag(world: &mut DomainWorld, tag: String) {
    world.cdp_tag = Some(tag);
}

#[given(regex = r#"^it carries the commit-id line "([^"]*)"$"#)]
fn given_commitdiff_plain_commit_id(world: &mut DomainWorld, id: String) {
    world.cdp_commit_id = Some(id);
}

#[given(regex = r#"^its patch body is "([^"]*)"$"#)]
fn given_commitdiff_plain_body(world: &mut DomainWorld, body: String) {
    world.cdp_patch_body = Some(format!("{body}\n"));
}

#[when(regex = r#"^I render the commitdiff_plain at "([^"]*)"$"#)]
fn render_commitdiff_plain(world: &mut DomainWorld, self_url: String) {
    let plain: CommitdiffPlain = CommitdiffPlain::new(
        world.cdp_author.clone().expect("an author"),
        Timestamp::new(CDP_EPOCH, "+0000"),
        world.cdp_subject.clone().expect("a subject"),
        world.cdp_tag.clone(),
        world.cdp_comment.clone(),
        world.cdp_commit_id.clone(),
        world.cdp_patch_body.clone().unwrap_or_default(),
    );
    world.cdp_rendered = Some(plain.render(&self_url));
}

#[then("the commitdiff_plain body is:")]
fn then_commitdiff_plain_body(world: &mut DomainWorld, step: &Step) {
    let expected: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    let actual: &str = world
        .cdp_rendered
        .as_deref()
        .expect("render the commitdiff_plain first");
    assert_eq!(actual.trim_end_matches('\n'), expected);
}

// --- blobdiff_plain body format ----------------------------------------------

#[given(regex = r#"^a blobdiff_plain whose patch body is "([^"]*)"$"#)]
fn given_blobdiff_plain_body_inline(world: &mut DomainWorld, body: String) {
    world.bdp_patch_body = Some(format!("{body}\n"));
}

#[given("a blobdiff_plain whose patch body is:")]
fn given_blobdiff_plain_body_block(world: &mut DomainWorld, step: &Step) {
    let body: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    world.bdp_patch_body = Some(format!("{body}\n"));
}

#[when(regex = r#"^I render the blobdiff_plain at "([^"]*)"$"#)]
fn render_blobdiff_plain(world: &mut DomainWorld, self_url: String) {
    let plain: BlobdiffPlain = BlobdiffPlain::new(world.bdp_patch_body.clone().unwrap_or_default());
    world.bdp_rendered = Some(plain.render(&self_url));
}

#[then("the blobdiff_plain body is:")]
fn then_blobdiff_plain_body(world: &mut DomainWorld, step: &Step) {
    let expected: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    let actual: &str = world
        .bdp_rendered
        .as_deref()
        .expect("render the blobdiff_plain first");
    assert_eq!(actual.trim_end_matches('\n'), expected);
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

// --- Projects-list ordering (ProjectOrder) -----------------------------------

#[given(regex = r#"^the listing has a project "([^"]*)"$"#)]
fn listing_project(world: &mut DomainWorld, name: String) {
    world.order_listing.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the listing has a project "([^"]*)" owned by "([^"]*)"$"#)]
fn listing_project_owned(world: &mut DomainWorld, name: String, owner: String) {
    world
        .order_listing
        .push(ProjectInfo::named(name).with_owner(owner));
}

#[given(regex = r#"^the listing has a project "([^"]*)" with no owner$"#)]
fn listing_project_no_owner(world: &mut DomainWorld, name: String) {
    world.order_listing.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the listing has a project "([^"]*)" described as "(.*)"$"#)]
fn listing_project_described(world: &mut DomainWorld, name: String, description: String) {
    world
        .order_listing
        .push(ProjectInfo::named(name).with_description(description));
}

#[given(regex = r#"^the listing has a project "([^"]*)" last changed at (\d+)$"#)]
fn listing_project_aged(world: &mut DomainWorld, name: String, epoch: i64) {
    world
        .order_listing
        .push(ProjectInfo::named(name).with_last_activity(epoch));
}

#[given(regex = r#"^the listing has a project "([^"]*)" with no commits$"#)]
fn listing_project_no_commits(world: &mut DomainWorld, name: String) {
    world.order_listing.push(ProjectInfo::named(name));
}

#[when(regex = r#"^I parse the project order "([^"]*)"$"#)]
fn parse_project_order(world: &mut DomainWorld, token: String) {
    world.parsed_order = Some(ProjectOrder::parse(&token));
}

#[then(regex = r#"^the parsed order is "([^"]*)"$"#)]
fn parsed_order_is(world: &mut DomainWorld, expected: String) {
    let order: ProjectOrder = world
        .parsed_order
        .as_ref()
        .expect("parse the order first")
        .clone()
        .expect("a valid order");
    assert_eq!(order.as_str(), expected);
}

#[then("parsing the order is rejected as invalid")]
fn parsing_order_rejected(world: &mut DomainWorld) {
    let result: &Result<ProjectOrder, DomainError> =
        world.parsed_order.as_ref().expect("parse the order first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

#[when(regex = r#"^I order the listing by "([^"]*)"$"#)]
fn order_the_listing(world: &mut DomainWorld, token: String) {
    let order: ProjectOrder = ProjectOrder::parse(&token).expect("the order under test is valid");
    order.sort(&mut world.order_listing);
}

#[then(regex = r#"^the listing order is "(.*)"$"#)]
fn listing_order_is(world: &mut DomainWorld, expected: String) {
    let names: Vec<&str> = world
        .order_listing
        .iter()
        .map(|info: &ProjectInfo| info.name())
        .collect();
    assert_eq!(names.join(", "), expected);
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

// --- Global config source selection (ConfigChain) ----------------------------

#[given("no global config is configured")]
fn given_no_global_config(_world: &mut DomainWorld) {}

#[given(regex = r#"^the common config default is "(.*)"$"#)]
fn given_common_default(world: &mut DomainWorld, path: String) {
    world.cfg_common_default = Some(path);
}

#[given(regex = r#"^the common config env is "(.*)"$"#)]
fn given_common_env(world: &mut DomainWorld, path: String) {
    world.cfg_common_env = Some(path);
}

#[given(regex = r#"^the per-instance config default is "(.*)"$"#)]
fn given_primary_default(world: &mut DomainWorld, path: String) {
    world.cfg_primary_default = Some(path);
}

#[given(regex = r#"^the per-instance config env is "(.*)"$"#)]
fn given_primary_env(world: &mut DomainWorld, path: String) {
    world.cfg_primary_env = Some(path);
}

#[given(regex = r#"^the system config default is "(.*)"$"#)]
fn given_system_default(world: &mut DomainWorld, path: String) {
    world.cfg_system_default = Some(path);
}

#[given(regex = r#"^the system config env is "(.*)"$"#)]
fn given_system_env(world: &mut DomainWorld, path: String) {
    world.cfg_system_env = Some(path);
}

#[given(regex = r#"^the file "(.*)" exists$"#)]
fn given_file_exists(world: &mut DomainWorld, path: String) {
    world.cfg_existing.push(path);
}

#[when("I select the global config files")]
fn select_global_config_files(world: &mut DomainWorld) {
    let chain: ConfigChain = ConfigChain::new(
        ConfigSlot::new(
            world.cfg_common_env.clone(),
            world.cfg_common_default.clone(),
        ),
        ConfigSlot::new(
            world.cfg_primary_env.clone(),
            world.cfg_primary_default.clone(),
        ),
        ConfigSlot::new(
            world.cfg_system_env.clone(),
            world.cfg_system_default.clone(),
        ),
    );
    let existing: Vec<String> = world.cfg_existing.clone();
    world.cfg_load_order =
        Some(chain.load_order(|path: &str| existing.iter().any(|p: &String| p == path)));
}

#[then("no config files are loaded")]
fn no_config_files_loaded(world: &mut DomainWorld) {
    let order: &[String] = world
        .cfg_load_order
        .as_ref()
        .expect("select the config files first");
    assert!(order.is_empty(), "expected no config files, got: {order:?}");
}

#[then(regex = r#"^the load order is "(.*)"$"#)]
fn load_order_is(world: &mut DomainWorld, expected: String) {
    let order: &[String] = world
        .cfg_load_order
        .as_ref()
        .expect("select the config files first");
    let actual: String = order.join(", ");
    assert_eq!(actual, expected);
}

// --- Global settings value precedence (Settings) -----------------------------

fn last_layer(world: &mut DomainWorld) -> &mut SettingsLayer {
    world
        .settings_layers
        .last_mut()
        .expect("add a config source first")
}

fn resolved(world: &DomainWorld) -> &Settings {
    world
        .resolved_settings
        .as_ref()
        .expect("resolve the settings first")
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item: &str| item.trim().to_owned())
        .collect()
}

fn named_feature(name: &str) -> FeatureName {
    FeatureName::from_key(name).expect("a known feature key")
}

#[given("no config sources")]
fn given_no_config_sources(world: &mut DomainWorld) {
    world.settings_layers.clear();
}

#[given("a config source")]
fn given_a_config_source(world: &mut DomainWorld) {
    world.settings_layers.push(SettingsLayer::default());
}

#[given(regex = r#"^it sets the projectroot to "(.*)"$"#)]
fn set_projectroot(world: &mut DomainWorld, value: String) {
    last_layer(world).projectroot = Some(value);
}

#[given(regex = r#"^it sets the site name to "(.*)"$"#)]
fn set_site_name(world: &mut DomainWorld, value: String) {
    last_layer(world).site_name = Some(value);
}

#[given(regex = r#"^it sets the logo to "(.*)"$"#)]
fn set_logo(world: &mut DomainWorld, value: String) {
    last_layer(world).logo = Some(value);
}

#[given(regex = r#"^it sets the favicon to "(.*)"$"#)]
fn set_favicon(world: &mut DomainWorld, value: String) {
    last_layer(world).favicon = Some(value);
}

#[given(regex = r#"^it sets the clone base URLs to "(.*)"$"#)]
fn set_clone_urls(world: &mut DomainWorld, value: String) {
    last_layer(world).git_base_url_list = Some(split_list(&value));
}

#[given(regex = r#"^it sets the "(.*)" feature default to "(.*)"$"#)]
fn set_feature_default(world: &mut DomainWorld, name: String, value: String) {
    let feature: FeatureName = named_feature(&name);
    last_layer(world)
        .features
        .entry(feature)
        .or_default()
        .default = Some(split_list(&value));
}

#[given(regex = r#"^it makes the "(.*)" feature overridable$"#)]
fn make_feature_overridable(world: &mut DomainWorld, name: String) {
    let feature: FeatureName = named_feature(&name);
    last_layer(world)
        .features
        .entry(feature)
        .or_default()
        .overridable = Some(true);
}

#[when("I resolve the settings")]
fn resolve_settings(world: &mut DomainWorld) {
    world.resolved_settings = Some(Settings::resolve(&world.settings_layers));
}

#[then(regex = r#"^the projectroot is "(.*)"$"#)]
fn projectroot_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).projectroot(), expected);
}

#[then(regex = r#"^the site name is "(.*)"$"#)]
fn site_name_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).site_name(), expected);
}

#[then(regex = r#"^the logo is "(.*)"$"#)]
fn logo_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).logo(), expected);
}

#[then(regex = r#"^the favicon is "(.*)"$"#)]
fn favicon_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).favicon(), expected);
}

// --- feed: title, window, comment lines --------------------------------------

#[when(regex = r#"^I build the feed title for action "([^"]*)" with no branch and no file$"#)]
fn build_feed_title_plain(world: &mut DomainWorld, action: String) {
    world.feed_title = Some(feed_title("Untitled Git", "repo.git", &action, None, None));
}

#[when(
    regex = r#"^I build the feed title for action "([^"]*)" with branch "([^"]*)" and no file$"#
)]
fn build_feed_title_branch(world: &mut DomainWorld, action: String, branch: String) {
    world.feed_title = Some(feed_title(
        "Untitled Git",
        "repo.git",
        &action,
        Some(&branch),
        None,
    ));
}

#[when(
    regex = r#"^I build the feed title for action "([^"]*)" with no branch and file "([^"]*)"$"#
)]
fn build_feed_title_file(world: &mut DomainWorld, action: String, file: String) {
    world.feed_title = Some(feed_title(
        "Untitled Git",
        "repo.git",
        &action,
        None,
        Some(&file),
    ));
}

#[when(
    regex = r#"^I build the feed title for action "([^"]*)" with branch "([^"]*)" and file "([^"]*)"$"#
)]
fn build_feed_title_branch_file(
    world: &mut DomainWorld,
    action: String,
    branch: String,
    file: String,
) {
    world.feed_title = Some(feed_title(
        "Untitled Git",
        "repo.git",
        &action,
        Some(&branch),
        Some(&file),
    ));
}

#[then(regex = r#"^the feed title is "(.*)"$"#)]
fn feed_title_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.feed_title.as_deref(), Some(expected.as_str()));
}

#[given(regex = r#"^the feed clock is (\d+)$"#)]
fn feed_clock_is(world: &mut DomainWorld, now: i64) {
    world.feed_now = now;
}

#[given(regex = r#"^(\d+) feed commits aged (\d+) hours$"#)]
fn feed_commits_aged(world: &mut DomainWorld, count: usize, hours: i64) {
    let epoch: i64 = world.feed_now - hours * 3600;
    world.feed_epochs.extend(std::iter::repeat_n(epoch, count));
}

#[when("I window the feed")]
fn window_the_feed(world: &mut DomainWorld) {
    world.feed_kept = Some(feed_window(&world.feed_epochs, world.feed_now));
}

#[then(regex = r#"^the feed keeps (\d+) commits$"#)]
fn feed_keeps(world: &mut DomainWorld, expected: usize) {
    assert_eq!(world.feed_kept, Some(expected));
}

#[when(regex = r#"^I extract the feed comment from "(.*)"$"#)]
fn extract_feed_comment(world: &mut DomainWorld, message: String) {
    let decoded: String = message.replace("\\n", "\n");
    world.feed_comment = Some(
        comment_lines(&decoded)
            .into_iter()
            .map(str::to_owned)
            .collect(),
    );
}

#[then(regex = r#"^the feed comment lines are "(.*)"$"#)]
fn feed_comment_lines_are(world: &mut DomainWorld, expected: String) {
    let joined: String = world
        .feed_comment
        .as_ref()
        .expect("extract the feed comment first")
        .join("|");
    assert_eq!(joined, expected);
}

#[then(regex = r#"^the default projects order is "(.*)"$"#)]
fn default_projects_order_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).default_projects_order(), expected);
}

#[then(regex = r#"^the fallback encoding is "(.*)"$"#)]
fn fallback_encoding_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).fallback_encoding(), expected);
}

#[then(regex = r#"^the clone base URLs are "(.*)"$"#)]
fn clone_urls_are(world: &mut DomainWorld, expected: String) {
    assert_eq!(resolved(world).git_base_url_list().join(", "), expected);
}

#[then(regex = r#"^the "(.*)" feature default is "(.*)"$"#)]
fn feature_default_is(world: &mut DomainWorld, name: String, expected: String) {
    let feature: FeatureName = named_feature(&name);
    assert_eq!(
        resolved(world)
            .feature(feature)
            .default_options()
            .join(", "),
        expected
    );
}

#[then(regex = r#"^the "(.*)" feature is overridable$"#)]
fn feature_is_overridable(world: &mut DomainWorld, name: String) {
    let feature: FeatureName = named_feature(&name);
    assert!(resolved(world).feature(feature).is_overridable());
}

#[then(regex = r#"^the "(.*)" feature is not overridable$"#)]
fn feature_is_not_overridable(world: &mut DomainWorld, name: String) {
    let feature: FeatureName = named_feature(&name);
    assert!(!resolved(world).feature(feature).is_overridable());
}

#[then(regex = r#"^the "(.*)" feature is enabled$"#)]
fn feature_is_enabled(world: &mut DomainWorld, name: String) {
    let feature: FeatureName = named_feature(&name);
    assert!(resolved(world).feature(feature).enabled());
}

#[then(regex = r#"^the "(.*)" feature is disabled$"#)]
fn feature_is_disabled(world: &mut DomainWorld, name: String) {
    let feature: FeatureName = named_feature(&name);
    assert!(!resolved(world).feature(feature).enabled());
}

// --- dispatch routing ---------------------------------------------------------

#[when("I route the request")]
fn when_route_request(world: &mut DomainWorld) {
    let request: Request = Request::from_query(&world.request_params)
        .expect("the routing fixtures are valid requests");
    world.routed = Some(route(&request));
}

#[then(regex = r#"^the dispatched action is "([^"]*)"$"#)]
fn then_dispatched_action_is(world: &mut DomainWorld, expected: String) {
    let dispatch: Dispatch = world
        .routed
        .clone()
        .expect("route the request first")
        .expect("routing succeeded");
    let action: Action = Action::parse(&expected).expect("the expected name is a valid action");
    assert_eq!(dispatch, Dispatch::Action(action));
}

#[then("the object kind must be resolved")]
fn then_resolve_object_kind(world: &mut DomainWorld) {
    let dispatch: Dispatch = world
        .routed
        .clone()
        .expect("route the request first")
        .expect("routing succeeded");
    assert_eq!(dispatch, Dispatch::ResolveObjectKind);
}

#[then("routing fails as project needed")]
fn then_routing_fails_project_needed(world: &mut DomainWorld) {
    let error: DomainError = match world.routed.clone().expect("route the request first") {
        Ok(_) => panic!("expected routing to fail"),
        Err(error) => error,
    };
    assert_eq!(error, DomainError::Invalid("Project needed".to_owned()));
}

// --- object action: the two pure rules (git_object) --------------------------

#[given(regex = r#"^the object hash is "([^"]*)"$"#)]
fn given_object_hash(world: &mut DomainWorld, hash: String) {
    world.obj_hash = Some(hash);
}

#[given(regex = r#"^the object base ref is "([^"]*)"$"#)]
fn given_object_base(world: &mut DomainWorld, base: String) {
    world.obj_base = Some(base);
}

#[given(regex = r#"^the object file name is "([^"]*)"$"#)]
fn given_object_file(world: &mut DomainWorld, file: String) {
    world.obj_file = Some(file);
}

#[when("I classify the object request")]
fn when_classify_object_request(world: &mut DomainWorld) {
    world.obj_lookup = Some(resolution(
        world.obj_hash.as_deref(),
        world.obj_base.as_deref(),
        world.obj_file.as_deref(),
    ));
}

#[then(regex = r#"^the lookup is by id "([^"]*)"$"#)]
fn then_lookup_by_id(world: &mut DomainWorld, expected: String) {
    let lookup: Resolution = world
        .obj_lookup
        .clone()
        .expect("classify the object request first")
        .expect("classification succeeded");
    assert_eq!(lookup, Resolution::ById { id: expected });
}

#[then(regex = r#"^the lookup is by base "([^"]*)" and file "([^"]*)"$"#)]
fn then_lookup_by_base_and_file(world: &mut DomainWorld, base: String, file: String) {
    let lookup: Resolution = world
        .obj_lookup
        .clone()
        .expect("classify the object request first")
        .expect("classification succeeded");
    assert_eq!(
        lookup,
        Resolution::ByBasePath {
            base,
            file_name: file,
        }
    );
}

#[then("classification fails as not enough information")]
fn then_classification_not_enough_info(world: &mut DomainWorld) {
    let error: DomainError = match world
        .obj_lookup
        .clone()
        .expect("classify the object request first")
    {
        Ok(_) => panic!("expected classification to fail"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        DomainError::Invalid("Not enough information to find object".to_owned())
    );
}

#[given(regex = r#"^the resolved object kind is "([^"]*)"$"#)]
fn given_resolved_object_kind(world: &mut DomainWorld, kind: String) {
    world.obj_kind_in = Some(ObjectKind::parse(&kind).expect("a valid object kind name"));
}

#[when("I map the object kind to its action")]
fn when_map_object_kind(world: &mut DomainWorld) {
    let kind: ObjectKind = world.obj_kind_in.expect("a resolved object kind first");
    world.obj_action_out = Some(target_action(kind));
}

#[then(regex = r#"^the redirect action is "([^"]*)"$"#)]
fn then_redirect_action_is(world: &mut DomainWorld, expected: String) {
    let action: Action = world.obj_action_out.expect("map the object kind first");
    let want: Action = Action::parse(&expected).expect("a valid action name");
    assert_eq!(action, want);
}

// --- no-action default object lookup (dispatch git_get_type) ------------------

#[when("I classify the dispatch request")]
fn when_classify_dispatch_request(world: &mut DomainWorld) {
    world.dispatch_lookup = Some(dispatch_lookup(
        world.obj_hash.as_deref(),
        world.obj_base.as_deref(),
        world.obj_file.as_deref(),
    ));
}

#[then(regex = r#"^the dispatch lookup is by id "([^"]*)"$"#)]
fn then_dispatch_lookup_by_id(world: &mut DomainWorld, expected: String) {
    let lookup: Option<DispatchLookup> = world
        .dispatch_lookup
        .clone()
        .expect("classify the dispatch request first");
    assert_eq!(lookup, Some(DispatchLookup::ByHash { hash: expected }));
}

#[then(regex = r#"^the dispatch lookup is by base "([^"]*)" and file "([^"]*)"$"#)]
fn then_dispatch_lookup_by_base_and_file(world: &mut DomainWorld, base: String, file: String) {
    let lookup: Option<DispatchLookup> = world
        .dispatch_lookup
        .clone()
        .expect("classify the dispatch request first");
    assert_eq!(
        lookup,
        Some(DispatchLookup::ByBasePath {
            base,
            file_name: file,
        })
    );
}

#[then("the dispatch request names no object")]
fn then_dispatch_names_no_object(world: &mut DomainWorld) {
    let lookup: Option<DispatchLookup> = world
        .dispatch_lookup
        .clone()
        .expect("classify the dispatch request first");
    assert_eq!(lookup, None);
}

// --- section: the summary-section cap rule -----------------------------------

/// Splits a comma-separated list step argument into items, treating the empty
/// string as no items.
fn split_items(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split(", ").map(str::to_owned).collect()
}

#[given(regex = r"^a section cap of (\d+)$")]
fn given_section_cap(world: &mut DomainWorld, cap: usize) {
    world.section_cap = cap;
}

#[given(regex = r#"^the section items "(.*)"$"#)]
fn given_section_items(world: &mut DomainWorld, items: String) {
    world.section_items = split_items(&items);
}

#[when("I limit the section")]
fn when_limit_section(world: &mut DomainWorld) {
    world.section = Some(Section::limited(
        world.section_items.clone(),
        world.section_cap,
    ));
}

#[then(regex = r#"^the shown section items are "(.*)"$"#)]
fn then_shown_section_items(world: &mut DomainWorld, expected: String) {
    let section: &Section<String> = world.section.as_ref().expect("limit the section first");
    assert_eq!(section.shown().join(", "), expected);
}

#[then("the section is truncated")]
fn then_section_truncated(world: &mut DomainWorld) {
    assert!(
        world
            .section
            .as_ref()
            .expect("limit the section first")
            .is_truncated()
    );
}

#[then("the section is not truncated")]
fn then_section_not_truncated(world: &mut DomainWorld) {
    assert!(
        !world
            .section
            .as_ref()
            .expect("limit the section first")
            .is_truncated()
    );
}

// --- remote: a configured remote's URL lines ---------------------------------

/// Serializes one URL line to its `role value` form (just `missing` for the
/// placeholder), so a scenario asserts the whole set with one comma-joined string.
fn url_line_string(line: &RemoteUrl) -> String {
    match line {
        RemoteUrl::Combined(url) => format!("combined {url}"),
        RemoteUrl::Fetch(url) => format!("fetch {url}"),
        RemoteUrl::Push(url) => format!("push {url}"),
        RemoteUrl::Missing => "missing".to_owned(),
    }
}

#[given(regex = r#"^a remote "([^"]*)" fetching from "([^"]*)" pushing to "([^"]*)"$"#)]
fn given_remote_fetch_push(world: &mut DomainWorld, name: String, fetch: String, push: String) {
    world.remote = Some(Remote::new(name, Some(fetch), Some(push)));
}

#[given(regex = r#"^a remote "([^"]*)" fetching from "([^"]*)"$"#)]
fn given_remote_fetch_only(world: &mut DomainWorld, name: String, fetch: String) {
    world.remote = Some(Remote::new(name, Some(fetch), None));
}

#[given(regex = r#"^a remote "([^"]*)" pushing to "([^"]*)"$"#)]
fn given_remote_push_only(world: &mut DomainWorld, name: String, push: String) {
    world.remote = Some(Remote::new(name, None, Some(push)));
}

#[given(regex = r#"^a remote "([^"]*)" with no URLs$"#)]
fn given_remote_no_urls(world: &mut DomainWorld, name: String) {
    world.remote = Some(Remote::new(name, None, None));
}

#[when("I read the remote's URL lines")]
fn when_read_url_lines(world: &mut DomainWorld) {
    let remote: &Remote = world.remote.as_ref().expect("declare a remote first");
    world.remote_url_lines = Some(remote.url_lines());
}

#[then(regex = r#"^the URL lines are "(.*)"$"#)]
fn then_url_lines_are(world: &mut DomainWorld, expected: String) {
    let lines: &[RemoteUrl] = world
        .remote_url_lines
        .as_ref()
        .expect("read the URL lines first");
    let rendered: String = lines
        .iter()
        .map(url_line_string)
        .collect::<Vec<String>>()
        .join(", ");
    assert_eq!(rendered, expected);
}

// --- snapshot: format table, selection cascade, archive naming ----------------

/// Parses a format token (`tgz`/`tbz2`/`txz`/`zip`) into an [`ArchiveFormat`].
fn parse_archive_format(token: &str) -> ArchiveFormat {
    ArchiveFormat::from_key(token)
        .unwrap_or_else(|| panic!("unknown snapshot format token {token:?}"))
}

/// Splits a comma-separated list, trimming each token (empty for "").
fn split_tokens(list: &str) -> Vec<String> {
    if list.trim().is_empty() {
        return Vec::new();
    }
    list.split(',')
        .map(|token: &str| token.trim().to_owned())
        .collect()
}

#[given(regex = r#"^the snapshot format "([^"]*)"$"#)]
fn given_snapshot_format(world: &mut DomainWorld, key: String) {
    world.snapshot_format = Some(parse_archive_format(&key));
}

#[then(regex = r#"^its content type is "([^"]*)"$"#)]
fn then_format_content_type(world: &mut DomainWorld, expected: String) {
    let format: ArchiveFormat = world.snapshot_format.expect("name a snapshot format first");
    assert_eq!(format.content_type(), expected);
}

#[then(regex = r#"^its filename suffix is "([^"]*)"$"#)]
fn then_format_suffix(world: &mut DomainWorld, expected: String) {
    let format: ArchiveFormat = world.snapshot_format.expect("name a snapshot format first");
    assert_eq!(format.suffix(), expected);
}

#[then(regex = r#"^its display name is "([^"]*)"$"#)]
fn then_format_display(world: &mut DomainWorld, expected: String) {
    let format: ArchiveFormat = world.snapshot_format.expect("name a snapshot format first");
    assert_eq!(format.display(), expected);
}

#[then(regex = r#"^the snapshot format "([^"]*)" is disabled$"#)]
fn then_format_disabled(_world: &mut DomainWorld, key: String) {
    assert!(parse_archive_format(&key).is_disabled());
}

#[then(regex = r#"^the snapshot format "([^"]*)" is not disabled$"#)]
fn then_format_not_disabled(_world: &mut DomainWorld, key: String) {
    assert!(!parse_archive_format(&key).is_disabled());
}

#[given(regex = r#"^the configured snapshot formats "([^"]*)"$"#)]
fn given_configured_formats(world: &mut DomainWorld, list: String) {
    world.configured_formats = split_tokens(&list);
}

#[when("I compute the enabled snapshot formats")]
fn when_compute_enabled(world: &mut DomainWorld) {
    world.computed_formats = Some(enabled_formats(&world.configured_formats));
}

#[then(regex = r#"^the enabled formats are "([^"]*)"$"#)]
fn then_enabled_formats_are(world: &mut DomainWorld, expected: String) {
    let computed: &[ArchiveFormat] = world
        .computed_formats
        .as_deref()
        .expect("compute the enabled formats first");
    let rendered: String = computed
        .iter()
        .map(|format: &ArchiveFormat| format.key())
        .collect::<Vec<&str>>()
        .join(", ");
    assert_eq!(rendered, expected);
}

#[then("no formats are enabled")]
fn then_no_formats_enabled(world: &mut DomainWorld) {
    let computed: &[ArchiveFormat] = world
        .computed_formats
        .as_deref()
        .expect("compute the enabled formats first");
    assert!(computed.is_empty());
}

#[given(regex = r#"^the enabled snapshot formats "([^"]*)"$"#)]
fn given_enabled_formats(world: &mut DomainWorld, list: String) {
    world.selection_enabled = split_tokens(&list)
        .iter()
        .map(|token: &String| parse_archive_format(token))
        .collect();
}

#[given("no snapshot formats are enabled")]
fn given_no_enabled_formats(world: &mut DomainWorld) {
    world.selection_enabled = Vec::new();
}

#[when(regex = r#"^I select the snapshot format requested "(.*)"$"#)]
fn when_select_format_requested(world: &mut DomainWorld, requested: String) {
    world.selection_result = Some(select_format(Some(&requested), &world.selection_enabled));
}

#[when("I select the snapshot format with no request")]
fn when_select_format_unset(world: &mut DomainWorld) {
    world.selection_result = Some(select_format(None, &world.selection_enabled));
}

/// The selection outcome, or a panic if the When never ran.
fn selection(world: &DomainWorld) -> &Result<ArchiveFormat, DomainError> {
    world
        .selection_result
        .as_ref()
        .expect("select a snapshot format first")
}

#[then(regex = r#"^the selected snapshot format is "([^"]*)"$"#)]
fn then_selected_format_is(world: &mut DomainWorld, expected: String) {
    let format: &ArchiveFormat = selection(world).as_ref().expect("selection succeeded");
    assert_eq!(format.key(), expected);
}

#[then(regex = r#"^snapshot selection is forbidden as "([^"]*)"$"#)]
fn then_selection_forbidden(world: &mut DomainWorld, message: String) {
    match selection(world) {
        Err(DomainError::Forbidden(actual)) => assert_eq!(actual, &message),
        other => panic!("expected Forbidden({message:?}), got {other:?}"),
    }
}

#[then(regex = r#"^snapshot selection is invalid as "([^"]*)"$"#)]
fn then_selection_invalid(world: &mut DomainWorld, message: String) {
    match selection(world) {
        Err(DomainError::Invalid(actual)) => assert_eq!(actual, &message),
        other => panic!("expected Invalid({message:?}), got {other:?}"),
    }
}

#[given(regex = r#"^the project path "([^"]*)"$"#)]
fn given_snapshot_project_path(world: &mut DomainWorld, project: String) {
    world.snapshot_project = project;
}

#[given(regex = r#"^the snapshot hash "([^"]*)" abbreviating to "([^"]*)"$"#)]
fn given_snapshot_hash(world: &mut DomainWorld, hash: String, short: String) {
    world.snapshot_hash = hash;
    world.snapshot_short = short;
}

#[when("I build the snapshot name")]
fn when_build_snapshot_name(world: &mut DomainWorld) {
    world.snapshot_name_out = Some(snapshot_name(
        &world.snapshot_project,
        &world.snapshot_hash,
        &world.snapshot_short,
        &["heads"],
    ));
}

#[then(regex = r#"^the snapshot name is "([^"]*)"$"#)]
fn then_snapshot_name_is(world: &mut DomainWorld, expected: String) {
    assert_eq!(world.snapshot_name_out.as_deref(), Some(expected.as_str()));
}

fn marker_view_of(name: &str) -> MarkerView {
    match name {
        "shortlog" => MarkerView::Shortlog,
        "log" => MarkerView::Log,
        "history" => MarkerView::History,
        "tag" => MarkerView::Tag,
        _ => MarkerView::Other,
    }
}

fn push_deref_ref(world: &mut DomainWorld, full: String, target: String, indirect: bool) {
    let oid: ObjectId = ObjectId::parse(&target).expect("a 40-character hex object id");
    world
        .deref_refs
        .push(DereferencedRef::new(RefName::new(full), oid, indirect));
}

fn computed_markers(world: &DomainWorld) -> &[RefMarker] {
    world.markers.as_deref().expect("markers must be computed")
}

fn computed_marker(world: &DomainWorld, index: usize) -> &RefMarker {
    &computed_markers(world)[index - 1]
}

#[given(regex = r#"^a ref "(.*)" targeting commit "([0-9a-f]+)"$"#)]
fn given_direct_ref(world: &mut DomainWorld, full: String, target: String) {
    push_deref_ref(world, full, target, false);
}

#[given(regex = r#"^an annotated tag ref "(.*)" targeting commit "([0-9a-f]+)"$"#)]
fn given_annotated_ref(world: &mut DomainWorld, full: String, target: String) {
    push_deref_ref(world, full, target, true);
}

#[when(regex = r#"^I compute ref markers for commit "([0-9a-f]+)" in the "(.*)" view$"#)]
fn when_compute_markers(world: &mut DomainWorld, commit: String, view: String) {
    let oid: ObjectId = ObjectId::parse(&commit).expect("a 40-character hex object id");
    world.markers = Some(markers_for(&world.deref_refs, &oid, marker_view_of(&view)));
}

#[then("there are no markers")]
fn then_no_markers(world: &mut DomainWorld) {
    assert!(computed_markers(world).is_empty());
}

#[then("there is 1 marker")]
fn then_one_marker(world: &mut DomainWorld) {
    assert_eq!(computed_markers(world).len(), 1);
}

#[then(regex = r#"^there are (\d+) markers$"#)]
fn then_n_markers(world: &mut DomainWorld, count: usize) {
    assert_eq!(computed_markers(world).len(), count);
}

#[then(regex = r#"^marker (\d+) has kind "(.*)"$"#)]
fn then_marker_kind(world: &mut DomainWorld, index: usize, kind: String) {
    assert_eq!(computed_marker(world, index).kind().class_token(), kind);
}

#[then(regex = r#"^marker (\d+) shows "(.*)"$"#)]
fn then_marker_name(world: &mut DomainWorld, index: usize, name: String) {
    assert_eq!(computed_marker(world, index).name(), name);
}

#[then(regex = r#"^marker (\d+) is titled "(.*)"$"#)]
fn then_marker_title(world: &mut DomainWorld, index: usize, title: String) {
    assert_eq!(computed_marker(world, index).title(), title);
}

#[then(regex = r#"^marker (\d+) links to the "(.*)" action$"#)]
fn then_marker_action(world: &mut DomainWorld, index: usize, action: String) {
    assert_eq!(computed_marker(world, index).action().as_str(), action);
}

#[then(regex = r#"^marker (\d+) targets ref "(.*)"$"#)]
fn then_marker_dest(world: &mut DomainWorld, index: usize, dest: String) {
    assert_eq!(computed_marker(world, index).dest(), dest);
}

#[then(regex = r#"^marker (\d+) is indirect$"#)]
fn then_marker_indirect(world: &mut DomainWorld, index: usize) {
    assert!(computed_marker(world, index).indirect());
}

#[then(regex = r#"^marker (\d+) is not indirect$"#)]
fn then_marker_direct(world: &mut DomainWorld, index: usize) {
    assert!(!computed_marker(world, index).indirect());
}

// --- conditional GET: If-Modified-Since freshness ----------------------------

#[given(regex = r#"^a resource last modified at epoch (\d+)$"#)]
fn given_resource_epoch(world: &mut DomainWorld, epoch: i64) {
    world.cond_epoch = epoch;
}

#[when("I evaluate freshness with no cached copy")]
fn evaluate_freshness_none(world: &mut DomainWorld) {
    world.cond_result = Some(freshness(world.cond_epoch, None));
}

#[when(regex = r#"^I evaluate freshness against "(.*)"$"#)]
fn evaluate_freshness_against(world: &mut DomainWorld, header: String) {
    world.cond_result = Some(freshness(world.cond_epoch, Some(&header)));
}

#[then("the resource is not modified")]
fn then_not_modified(world: &mut DomainWorld) {
    assert_eq!(world.cond_result, Some(Freshness::NotModified));
}

#[then("the resource is modified")]
fn then_modified(world: &mut DomainWorld) {
    assert_eq!(world.cond_result, Some(Freshness::Modified));
}

// --- by-oid cache freshness: Expires +1d -------------------------------------

#[given(regex = r#"^a view addressed by hash "(.*)"$"#)]
fn given_view_hash(world: &mut DomainWorld, hash: String) {
    world.expiry_hash = Some(hash);
}

#[given("a view addressed by no hash")]
fn given_view_no_hash(world: &mut DomainWorld) {
    world.expiry_hash = None;
}

#[when("I evaluate its cache freshness")]
fn evaluate_expiry(world: &mut DomainWorld) {
    world.expiry_result = Some(Expiry::for_hash(world.expiry_hash.as_deref()));
}

#[given(regex = r#"^a single-file diff with base "(.*)" and parent base "(.*)"$"#)]
fn given_blobdiff_bases(world: &mut DomainWorld, base: String, parent_base: String) {
    world.expiry_base = Some(base);
    world.expiry_parent_base = Some(parent_base);
}

#[given(regex = r#"^a single-file diff with base "(.*)" and no parent base$"#)]
fn given_blobdiff_base_only(world: &mut DomainWorld, base: String) {
    world.expiry_base = Some(base);
    world.expiry_parent_base = None;
}

#[when("I evaluate the single-file diff cache freshness")]
fn evaluate_blobdiff_expiry(world: &mut DomainWorld) {
    world.expiry_result = Some(Expiry::for_hashes(&[
        world.expiry_base.as_deref(),
        world.expiry_parent_base.as_deref(),
    ]));
}

#[then("the freshness window is one day")]
fn then_window_one_day(world: &mut DomainWorld) {
    assert_eq!(world.expiry_result, Some(Expiry::OneDay));
}

#[then(regex = r"^the freshness window is (\d+) seconds$")]
fn then_window_seconds(world: &mut DomainWorld, seconds: i64) {
    let expiry: Expiry = world.expiry_result.expect("freshness must be evaluated");
    assert_eq!(expiry.seconds(), Some(seconds));
}

#[then("there is no freshness window")]
fn then_no_window(world: &mut DomainWorld) {
    assert_eq!(world.expiry_result, Some(Expiry::None));
    let expiry: Expiry = world.expiry_result.expect("freshness must be evaluated");
    assert_eq!(expiry.seconds(), None);
}

// --- feed content-type negotiation: Accept -----------------------------------

#[given("an RSS feed")]
fn given_rss_feed(world: &mut DomainWorld) {
    world.accept_feed_type = "application/rss+xml".to_owned();
}

#[given("an Atom feed")]
fn given_atom_feed(world: &mut DomainWorld) {
    world.accept_feed_type = "application/atom+xml".to_owned();
}

#[when("I negotiate with no Accept header")]
fn negotiate_no_accept(world: &mut DomainWorld) {
    world.accept_result = Some(prefer_text_xml_feed(None, &world.accept_feed_type));
}

#[when(regex = r#"^I negotiate with Accept "(.*)"$"#)]
fn negotiate_with_accept(world: &mut DomainWorld, header: String) {
    world.accept_result = Some(prefer_text_xml_feed(Some(&header), &world.accept_feed_type));
}

#[then("the feed type is kept")]
fn then_feed_kept(world: &mut DomainWorld) {
    assert_eq!(world.accept_result, Some(false));
}

#[then("the feed type is downgraded to text/xml")]
fn then_feed_downgraded(world: &mut DomainWorld) {
    assert_eq!(world.accept_result, Some(true));
}

#[tokio::main]
async fn main() {
    DomainWorld::run("features/domain").await;
}
