# Handoff

## Last session: gitweb_in_rust-d7s DONE (extra-branch-refs threaded into every consumer)

**What shipped** — `get_branch_refs` (the `heads` + validated extra-branch-refs
directory set, from cp4) was only honoured by the snapshot handler. d7s threaded
it into every remaining gitweb consumer. Built red→green, one consumer at a time;
each behavioural change was seen RED on an assertion first.

- **heads listing** (`usecase::heads::assemble_heads`): now takes
  `branch_refs: &[&str]` and lists each directory through the references port via
  a new `branch_references` helper (gitweb's `map { "refs/$_" } get_branch_refs()`,
  concatenated; the existing `assemble_head_rows` sort restores gitweb's global
  `--sort=-HEAD --sort=-committerdate`). The ` (dir)` name-suffix for non-heads/
  remotes dirs was ALREADY done by `RefName::short()` (covered by the `ref_name`
  domain feature), so only listing *breadth* was missing. Heads handler resolves
  `get_branch_refs(settings…)` and passes it (lazy 500 on a malformed entry).
- **summary heads section** (`usecase::summary::assemble_summary`): already took
  `&Settings`, so it resolves `get_branch_refs` internally and feeds
  `assemble_heads`; the summary *handler* is unchanged.
- **last activity** (`crates/git project_store`): `GixProjectStore` gained
  `extra_branch_refs: Vec<String>` + a `with_extra_branch_refs` wither (mirrors the
  o09 `with_user_directory` injection). `build_router` reads
  `settings.feature(ExtraBranchRefs).default_options()` and injects it (infallible).
  `info()` resolves `get_branch_refs(&self.extra_branch_refs)?` (LAZY 500 kept,
  consistent with heads/summary/snapshot) and feeds a new `is_branch_ref(full, dirs)`
  membership test in `most_recent_branch_time(repo, branch_refs)`.
- **feed branch-title = N/A** (confirmed, not skipped): gitweb's `git_feed`
  `<title>` (gitweb.perl 8230-8247) uses the RAW `$hash` — our
  `model::feed::feed_title` already matches it. The `get_branch_refs` branch
  classification lives only in `get_feed_info`/`print_feed_meta` (the HTML `<head>`
  feed AUTO-DISCOVERY `<link rel="alternate">` tags), which are UNIMPLEMENTED here
  (every web handler passes `feeds: Vec::new()`). Filed as **gitweb_in_rust-38y**
  (whole print_feed_meta feature; it carries the get_branch_refs requirement).

**Commits** (interleaved on a SHARED tree with a concurrent agent — my commits use
targeted `git add <paths>`, never `-A`; each was diff-stat-verified to touch only
my files): `ffa9dc4` (usecase heads breadth) → `933b693` (heads handler + web e2e)
→ `fefea96` (summary heads section) → `6fcdd57` (last-activity adapter + composition
wiring) → `4236f27` (project-list age column web e2e).

**Gates**: full `cargo test --workspace --no-fail-fast` green (exit 0, 0 failure
markers); `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets`
clean. cargo-mutants (TMPDIR=$HOME/.cache/cargo-mutants, scoped by `-F` to the new
functions — there is still no `.fn-hashes.jsonl` baseline, so `--file`/`-F` is the
scoping): domain `branch_references`/`assemble_heads`/`assemble_summary` = 1 caught
(branch_references empty-Vec, killed by the "listed with dir" usecase scenario), 3
unviable (`Ok(Default::default())` on non-Default `HeadsView`/`SummaryView`/`Reference`),
**0 survivors**. git `is_branch_ref`/`most_recent_branch_time`/`with_extra_branch_refs`
= 9 caught, 1 unviable, **0 survivors** (the conformance scenarios kill `is_branch_ref`'s
true/false mutants and the `most_recent_branch_time` variants). Both runs reported 0
missed.

### THREE THINGS THE NEXT SESSION MUST KNOW

1. **The tree is SHARED with a concurrent agent** committing as the same git author.
   This session it landed `67514c6` (hex-arch role tags — "hex-lint green"), `6b2022e`
   (hooks: PreToolUse guard + Stop check-guard), `35909a8` (check-change scripts),
   `49e405d` (the CLAUDE.md "Running tests" doc), all interleaved between my commits.
   It is ALSO running cargo-mutants in this workspace, so `mutants.out/` is polluted
   with its scope — trust each run's OWN stdout summary, not the shared `mutants.out/`.
   Use targeted `git add <paths>`; never `git add -A`.

2. **cargo-mutants MUST run with `TMPDIR=$HOME/.cache/cargo-mutants`** (the /tmp
   tmpfs trap → every mutant falsely "unviable"). See memory `cargo-mutants-tmpfs-trap`.

3. **The branch_refs lazy-500 contract**: `get_branch_refs` is resolved lazily at
   each consumer (heads/summary handlers, the store's `info()`), surfacing the same
   `die_error(500)` per-request rather than at config load. Keep new consumers on
   that pattern. See memory `branch-refs-seam` (updated this session).

## Next session

Baseline is green. Ready beads (`br ready --json`), concrete picks:
- **gitweb_in_rust-38y** (P3, just filed): feed auto-discovery `<link rel=alternate>`
  meta (gitweb's `print_feed_meta` + `get_feed_info`), populating the always-empty
  `DocumentHead.feeds`; honours extra-branch-refs via the now-established seam.
- **gitweb_in_rust-2os.13** (P3): real combined `--cc` merge diff — the big one.
- **gitweb_in_rust-h2t** (P2): client-side JavaScript (blame_incremental, tz, actions);
  sprawling, would want decomposing into per-feature children first.
- **e5l / him** (P4): golden-parity beads (need brew perl+CGI to regenerate refs).
- Infra (likely the concurrent agent's lane now): CI bead `9uc`; `fn-hash` still has
  no `.fn-hashes.jsonl` baseline; spec/REQ traceability machinery still unbuilt
  (zero REQ refs repo-wide).
