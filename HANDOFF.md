# Handoff

## Last session: gitweb_in_rust-12k DONE (forks empty-container state)

**What shipped** — gitweb's `forks` field is a tri-state (undef / `[]` / `[N>0]`)
and the listing collapsed it to a fork count, so a fork-capable project with a
real-but-fork-less sibling directory (`repo.git` next to an empty `repo/`) was
indistinguishable from one that can't parent forks. Now all three states render
faithfully. Built red→green through every layer (each layer's new behavior was
seen RED first, then made green):

- **domain** `model::forks::ForkState` = NotForkable / EmptyContainer /
  Forked(NonZeroUsize), with `fork_count()` + `is_forkable()` (gitweb's
  `scalar @{forks}` and truthy `$pr->{forks}`). `fork_container(name)->Option<&str>`
  = the name-side `-d` guard (strip one `.git`, reject non-bare `repo/.git` and
  the bare `.git`); `partition_forks` reuses it (old `container_path` deleted).
- **port** `ProjectStore::container_exists(subdir)->bool` = gitweb's infallible
  `-d`; gix adapter = `SafePath::parse` + `root.join().is_dir()`.
- **usecase** resolves each surviving project's state: Forked from the fold
  count, else EmptyContainer iff `fork_container` is Some AND `container_exists`,
  else NotForkable. The `-d` is consulted only for the 0-fork case (a forked
  project always owns its container). `fork_count()` kept as a derived row
  accessor so existing count assertions are untouched.
- **render** `fork_cell` matches the tri-state (linked `+` / unlinked
  `<span title="0 forks">+</span>` / empty); `| forks` quick link gated on
  `is_forkable()`.
- **web** maps `row.fork_state()`; handler wiring was already in place (6i5).
- **fixture** `ProjectRoot::add_dir` lays the empty sibling container; web e2e
  proves state-2 over the real gix store.

**Commits**: 3789098 (fork_container) - db4e374 (container_exists port+adapters)
- e855f87 (usecase) - 01a1f58 (render+web) - b164c2a (web e2e) - a3dc7d2
(is_forkable domain coverage, mutation strengthen).

**Gates**: fmt --check clean; clippy --workspace --all-targets clean; full
workspace suite green (1574 cucumber scenarios, exit 0). cargo-mutants scoped to
the 6 changed prod fns: 14 caught + 2 unviable (`Default::default()` on ForkState,
which has no Default) + 3 is_forkable survivors that were then KILLED by adding
domain coverage (re-run: 3 caught). CRAP: every changed fn is CC<=4 and fully
covered → far under 30 (not mechanically run; whole-workspace llvm-cov).

### TWO THINGS THE NEXT SESSION MUST KNOW

1. **cargo-mutants MUST run with `TMPDIR=$HOME/.cache/cargo-mutants`.** The
   default `/tmp` is a 16 GB tmpfs; cargo-mutants copies the whole `target/` per
   mutant and overflows it → `Disk quota exceeded` → EVERY mutant falsely
   "unviable" (a false green). Now documented in CLAUDE.md step 4 +
   verification-ratchet skill (commit 28a097e). See memory
   `cargo-mutants-tmpfs-trap`.

2. **`db4e374` is contaminated (NOT cleaned up).** A concurrent session
   (committing as the same git author) was editing `.claude/skills/*` in the
   working tree; my `git add -A` for db4e374 swept NINE skill files
   (quality-gates, refactoring-campaign, verification-ratchet, slice-workflow,
   …) into my forks commit. Content is preserved in git, just misattributed and
   bundled. I did NOT rewrite history — another agent was actively committing
   (interleaved commits 0acbf03, 57cb6cf) and a rebase would clobber its work.
   Lesson applied: use targeted `git add <paths>`, never `git add -A`, on this
   shared tree. If the human wants db4e374 split, that's a deliberate,
   coordinated history edit.

## Next session

Baseline is green. Suggested ready beads (handoff carried over from last
session, still valid):
- **d7s** (P3): thread `get_branch_refs` into the heads listing (' (dir)'
  suffix) + last-activity. The meatiest; needs `references` to take multiple
  prefixes. See memory `branch-refs-seam`.
- **e5l** / **him**: golden-parity beads (need brew perl+CGI to regenerate
  gitweb refs).
- Pre-existing, unenforced: hex-lint reports every package missing
  `package.metadata.hex-arch.role`; `fn-hash` has no `.fn-hashes.jsonl`
  baseline; and the spec/REQ traceability machinery (`specs/` + `REQ-AREA-NNN`
  comments) is unbuilt repo-wide (zero REQ refs exist). Likely belongs with the
  CI bead 9uc.

### Exact next test if picking d7s
Add to `crates/domain/features/domain/` a heads-name scenario: a ref under a
non-`heads`/`remotes` branch dir (e.g. `refs/sandbox/wip`) renders as
`wip (sandbox)` (the ` ($ref_dir)` suffix), watched red against the current
heads use case (which calls `references("refs/heads/")`, a single prefix), then
thread `branch_refs` + multi-prefix `references` through `assemble_heads`.
