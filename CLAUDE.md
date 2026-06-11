# Spec & Verification Policy (merge into project CLAUDE.md)

**Mode B project:** the `.feature` files ARE the spec. No `specs/REQ-*.md`, no
REQ IDs, no `// REQ-` comments. Read "requirement"/"REQ ID"/"spec" below as the
`.feature` scenario. `trace-audit`'s REQ checks are off by default
(`REQ_ID_CHECKS=1` re-enables).

## The check-modification boundary (non-negotiable)

You may make a failing check pass ONLY by changing the implementation.
You may NEVER make a check pass by changing the check.

"Weakening a check" includes ALL of the following:
- Deleting or commenting out a test, scenario, or assertion
- Adding `#[ignore]`, `#[cfg(never)]`, or skipping a test in any way —
  with exactly ONE sanctioned exception: a slice's acceptance test marked
  `#[ignore = "WIP(REQ-<AREA>-<NNN>)"]` per the slice-workflow skill (the token
  is just a slice label in Mode B; check-guard requires this exact form)
- Loosening an assertion (`assert_eq!` → `assert!`, exact value → range, etc.)
- Narrowing a property test's input strategy to dodge a failing case
- Adding a failing input to a "known exceptions" list
- Lowering the mutation-testing threshold or excluding files from `cargo-mutants`
- Raising the CRAP threshold, or padding coverage with weak/assertion-free
  tests to shrink a function's CRAP score
- Editing tests during a refactor step (refactoring changes structure, never
  behavior — if tests must change, the tests are coupled to implementation;
  stop and report)
- Replacing the unit under test with a mock/stub of itself
- Deleting or editing entries in `proptest-regressions/` (these are
  append-only archives of known counterexamples)
- Adding entries to `hex-lint-exceptions.toml` (the architectural-debt
  ledger) to dodge a role-matrix violation — every entry is a check-change,
  and stale entries fail the lint on their own
- Regenerating or editing golden/conformance files to make a failing parity
  test pass — re-blessing output redefines "correct" and is always a
  check-change (testing-strategy: golden-master rules)
- Deleting and regenerating a module, test file, or function to escape a
  failing check instead of fixing forward — rewriting is not fixing, and it
  silently discards behavior other tests don't pin
- Relaxing a type (newtype → primitive, `NonZeroU32` → `u32`, removing a validation)

If you believe a check is genuinely wrong: STOP. Do not touch it. Report to the
human: which check, why you believe it is wrong, what you propose instead.
A check change is always a separate, human-approved commit — never bundled
into a feature or fix commit.

## Traceability (every test has a reason to exist)

- Every behavior is pinned by a `.feature` scenario; the feature + scenario name
  IS the requirement.
- No test for behavior no scenario describes: write the scenario first (skill:
  spec-authoring), watch it fail, then the code.
- No scenario nothing executes. Plain `#[test]` helpers trace via their module.

## Order of work for any new behavior

1. Spec the behavior as a `.feature` scenario (skill: spec-authoring).
2. Types first: make illegal states unrepresentable before writing logic.
3. Implement via red/green/refactor (skill: tdd-cycle), under the Three Laws:
   - No production code except to pass a failing test; at most ONE failing
     test in existence at a time; no more code than the test forces.
   - RED: see the new test fail, for the expected reason, before any
     production code.
   - GREEN: minimum code to pass; all tests green.
   - REFACTOR: mandatory every cycle, on green only, tests untouched.
     Commit on green.
4. **Mutation testing — SUSPENDED as of 2026-06-10 (human-authorized).**
   Do not run `cargo mutants` and do not block work on a mutation pass for
   now. Steps 1–3 (spec → types → red/green/refactor) and steps 5–6 (quality
   gate, ADR) remain in force; the test suite is still the source of truth.
   When the suspension is lifted, restore the obligation below verbatim:
   > `cargo mutants` scoped to your diff — fast and false-survivor-free:
   > `cargo mutants --in-diff <(git diff) --test-workspace=true --profile mutants`.
   > `--in-diff` scopes the MUTANTS to your change; `--test-workspace=true` runs
   > the whole suite so a fn covered only by a downstream crate still gets killed
   > (scope the mutants, never the tests); `--profile mutants` (debuginfo off) +
   > mold build each mutant cheaply. Triage every surviving mutant (skill:
   > verification-ratchet). Surviving mutants are reported, never hidden. Diff
   > scoping is an approximation — full runs in CI remain the proof. ALWAYS run
   > cargo-mutants with `TMPDIR=$HOME/.cache/cargo-mutants` (the default `/tmp` is
   > a 16 GB tmpfs; cargo-mutants copies the whole `target/` per mutant and
   > overflows it, reporting every mutant "unviable" — a false green). See
   > verification-ratchet.
5. Quality gate: hex-lint clean (role matrix), no changed function over
   CRAP 30. Over-threshold functions are refactored or reported — never
   accepted silently and never gamed via coverage padding
   (skill: verification-ratchet, Layer 4).
6. ADR if a structural decision was made (skill: adr-discipline).

## Vertical slices (the double loop)

When one requirement spans multiple zones (domain, port, adapter, use case,
web/render), follow the slice-workflow skill:
- ONE behavior (one `.feature` scenario) owns the slice; layers are not it.
- ONE acceptance test per in-flight slice, committed `#[ignore = "WIP(REQ-…)"]`
  on day one, un-ignored as the final wire-up step. Removing the marker and
  seeing it green IS the slice's definition of done.
- The one-failing-test rule and "never end mid-red" apply to the INNER
  (unit) loop; the WIP-ignored acceptance test is the outer loop and is
  allowed to stay red — visibly, trackably — for the slice's lifetime.
- Layer order default: spec → WIP acceptance → domain → port + contract
  suite + fake → use case (against fake) → adapters (passing the same
  contract suite) → web/render → wire-up → gates.
- First slice in a new capability is a walking skeleton: thinnest behavior
  through all layers first.

## Getting stuck

If the next test cannot pass without a large rewrite, do not brute-force it
and do not weaken the test. Revert to the last green commit, report the
blockage and a proposed re-sequencing or design change, and wait
(skill: tdd-cycle, backtracking protocol).

## Baseline rule

The suite must be green before you write anything. If orientation finds a
red baseline, that is inherited breakage: stop, report it, and wait — never
build on red, or your failures and the previous session's become
indistinguishable. Never run interactive/TUI commands or watch modes; use
non-interactive flags (`--json`, `--no-pager`) or report the tool as
unusable.

## Running tests (how to read the result)

`cargo test` is the source of truth. **Do NOT use `cargo nextest` here.** Every
crate carries at least one cucumber `harness = false` target, and nextest cannot
introspect a custom-harness binary — it hard-errors trying to list it
(`config-… --list --format terse` → `error: unexpected argument '--list' found`)
and aborts the run. A "green" nextest run in this workspace is a **false green**:
it never executed the conformance/feature suites. There is also no
machine-readable output — `--format json` is nightly-only (`-Z unstable-options`)
and this toolchain is stable, so you parse the text below.

### The exit code is the first and most reliable signal
`cargo test` exits `0` iff everything passed; any failure — libtest assertion or
cucumber step — yields non-zero (`101`). Check it; never trust a summary that
scrolled past. A green run prints a *lot* of noise (a `✔` per cucumber step,
gherkin echoes, `Compiling`/`Running` chatter, a `… ok` per libtest case), so
filter it. Define the noise pattern once:

    # passing cucumber steps, gherkin echoes, cargo build + libtest chatter:
    NOISE='✔|^\s*(Feature|Rule|Scenario|Background):|^\s+(Compiling|Finished|Running|Doc-tests) |^running [0-9]+ test|^test .* \.\.\. ok$'

Then run, filtering noise but keeping the full log on disk and cargo's real exit
code (the pipe would otherwise hand you grep's code, not cargo's):

    cargo test --workspace --no-fail-fast 2>&1 | tee /tmp/cargo-test.log | grep -vE "$NOISE"
    echo "exit=${PIPESTATUS[0]}"   # cargo's status, NOT grep's — read it on the line after the pipe

`--no-fail-fast` is mandatory for a full picture. Without it, cargo stops after
the FIRST failing test binary, so you see one crate's failures and silently miss
every later crate. With it, all binaries run and you get every failure in one
pass. (`RUST_BACKTRACE=1 cargo test …` adds backtraces to panic messages.) On a
failure, `/tmp/cargo-test.log` holds the unfiltered output to drill into.

### Two harnesses → two output shapes to grep for
**libtest** (`#[test]` unit/doc tests) — load-bearing lines:
- per-binary summary: `test result: FAILED. 12 passed; 1 failed; 0 ignored; …`
  (success looks like `test result: ok. 8 passed; 0 failed; …`).
- a `failures:` block listing each failed test by name, and for each a
  `---- <test> stdout ----` section with the `panicked at <file>:<line>` message.

**cucumber** (`harness = false` `.feature` suites) — load-bearing lines:
- a `[Summary]` block: `8 scenarios (7 passed, 1 failed)` and
  `35 steps (34 passed, 1 failed)` (all-green omits the `, N failed`).
- the failing step is marked `✘`, printed with its `<name>.feature:<line>`
  location and the assertion/panic message inline.
- on any failure the binary panics (`N step(s) failed`) → non-zero exit, and
  cargo prints `error: test failed, to rerun pass '-p <crate> --test <name>'`.

### Extract just the failure data
    # counts + which crates/suites failed, nothing else:
    cargo test --workspace --no-fail-fast 2>&1 \
      | grep -E 'test result: FAILED|[0-9]+ failed|scenarios \(|steps \(|^error'

    # failed libtest names + their panic messages:
    cargo test --workspace --no-fail-fast 2>&1 \
      | grep -E '^---- .* stdout ----|panicked at|^    [a-zA-Z0-9_:]+$'

### Scope while iterating (the TDD inner loop)
    cargo test -p gitweb-git                 # one crate, all its targets
    cargo test -p gitweb-git --test refs     # one cucumber suite (its [[test]] name)
    cargo test -p gitweb-domain --lib        # only that crate's unit/doc tests
    cargo test -p gitweb-git some_fn_name    # libtest name filter (does NOT filter cucumber)

Cucumber suites load their `.feature` files by a path relative to the crate root
(`SomeWorld::run("features/<x>")`); `cargo test` sets CWD to the package dir, so
always invoke them through cargo — never the bare binary from another directory.

## Evidence discipline (claims require receipts)

- Never state that tests pass, a mutant is killed, a gate is green, or a
  build succeeds unless you ran the command IN THIS SESSION and saw the
  output. "Should pass" is not "passes".
- When declaring a step done, show the evidence: the command and the
  relevant tail of its output (test count, mutant summary, gate result).
- A RED step requires showing the failure output and confirming it failed
  for the expected reason — a test you never saw fail is not evidence.
- If a command was not run (timeout, environment problem), say exactly that.
  An honest "could not verify" is acceptable; a fabricated "verified" is the
  single worst thing you can do in this codebase.

## Scope discipline

- Touch only files the current requirement forces you to touch. No drive-by
  refactors, renames, dependency bumps, or "while I'm here" cleanups outside
  the REFACTOR step of the code you are working on — file them instead
  (see: Structural improvements, below). Discovery is encouraged; silent
  scope creep is not.
- If implementing REQ-X reveals a bug or gap elsewhere: report it, propose a
  requirement, do not silently fix it in the same change.
- Diff size is a review burden you impose on the human. Prefer several small
  green commits over one large one; the commit history should replay the
  red/green/refactor sequence.

## Session lifecycle (first and last steps are mandatory)

- START of session and after any compaction: run the orientation protocol
  (skill: session-lifecycle). Read this file, check git state, read
  HANDOFF.md and the assigned bead, state which `.feature` you're working on.
  Do not rely on a summary's paraphrase of these rules.
- END of session: handoff protocol (skill: session-lifecycle). Never end
  mid-red; gates green; bead updated; HANDOFF.md rewritten with the exact
  next test. Anything not committed to the repo does not exist tomorrow.
- Beads specify WHAT, never HOW. A bead references the `.feature` it delivers
  and gives context; implementation decisions belong to the session doing the
  work, inside the constraints of the feature specs and ADRs.

## Project conventions (mechanically enforced where possible)

- Hexagonal architecture, ECB flavor: Entities hold business logic,
  Controls implement use cases via ports, Boundaries are adapters
  (API, CLI, DB). All dependencies point inward. No framework types in
  the domain. This is MECHANICALLY enforced by hex-lint: every workspace
  member carries `[package.metadata.hex-arch] role = "..."` (domain,
  usecase, port-and-adapter, driven-adapter, driving-adapter, infra,
  composition-root) and the gate fails on any forbidden cross-role edge.
  A missing role tag is a hard error. Adding an entry to
  `hex-lint-exceptions.toml` is a check-change: standalone commit, human
  approval — and stale exceptions fail the lint on their own, so debt
  cannot be papered over and forgotten.
- Screaming architecture: organize modules by business capability, not by
  technical layer. The capability, bounded context, and module directory share
  a name — `pricing` features and code live under a `pricing` module. If you
  can't tell what the system does from `ls`, the structure is wrong.
- `#![forbid(unsafe_code)]` in every domain and application crate. If a
  boundary crate genuinely needs `unsafe` (embedded HAL, FFI), that is an
  ADR with human approval — never a quiet `#[allow]`.
- Tests have cyclomatic complexity 1: straight-line bodies, no loops, no
  branches. Case coverage follows zero/one/many — and two counts as many.
  A loop over a table in a test is either separate test functions, rstest
  cases, or a property. A branch in a test means you don't know what the
  test asserts.
- Explicit type annotations on variables and closure parameters. The type
  system is a spec; make it visible at the point of use.
- Commits: scoped style, `<scope>: <description>` — scope is the
  capability/module touched, not a generic type. When a tracker is in use, the
  message ends with the bead/work-item id — commit ↔ bead ↔ `.feature` is the
  traceability triangle. `check-change:` is the reserved scope for
  human-approved check modifications (check-guard enforces it).

## Structural improvements (the senior-dev override, bounded)

When you find flawed architecture, duplicated state, or inconsistent
patterns, you do not ignore it and you do not silently fix it. The rule:

- Inside the code you are already changing for the current REQ: fix it in
  the REFACTOR step, as its own scoped commit. That's what the step is for.
- Outside the current REQ's blast radius: file it — a bead (and a spec
  revision or ADR proposal if it's structural). Filing aggressively is
  mandatory; fixing opportunistically is forbidden. A "senior dev" who
  bundles an unrelated restructuring into a feature diff gets the whole
  diff rejected, and so do you.
- Step 0 rule: before any structural refactor of a file >300 LOC, first
  strip dead code — unused imports/exports, dead props, debug logs — as a
  separate commit. Dead code poisons context compaction and inflates every
  diff; clean the lens before grinding it.

## Mutation testing outcomes (the only allowed responses)

> **SUSPENDED as of 2026-06-10 (human-authorized).** Mutation testing is
> turned off for now — do not run `cargo mutants`, so there are no surviving
> mutants to triage. The rules below apply unchanged once the suspension in
> "Order of work" step 4 is lifted. The check-modification boundary still
> stands: do NOT lower a threshold, exclude a file, or edit `mutants.toml` on
> your own authority — suspending mutation testing was a human decision, and
> re-enabling or further weakening it is another one.

For each surviving mutant, exactly one of:
- **Strengthen**: add/extend a test that kills it.
- **Report**: tell the human "this mutant survives because the spec does not
  constrain this behavior — should it?" and wait.
Never: exclude the file, lower the threshold, or declare it unimportant
on your own authority.

## ADRs are settled law

Before proposing any architectural or modelling change, search `docs/adr/`.
If an accepted ADR already covers it, do not relitigate — surface the ADR to
the human and proceed within it unless the human supersedes it.

<!-- bv-agent-instructions-v2 -->

---

## Beads Workflow Integration

This project uses [beads_rust](https://github.com/Dicklesworthstone/beads_rust) (`br`) for issue tracking and [beads_viewer](https://github.com/Dicklesworthstone/beads_viewer) (`bv`) for graph-aware triage. Issues are stored in `.beads/` and tracked in git.

### Using bv as an AI sidecar

bv is a graph-aware triage engine for Beads projects (.beads/beads.jsonl). Instead of parsing JSONL or hallucinating graph traversal, use robot flags for deterministic, dependency-aware outputs with precomputed metrics (PageRank, betweenness, critical path, cycles, HITS, eigenvector, k-core).

**Scope boundary:** bv handles *what to work on* (triage, priority, planning). `br` handles creating, modifying, and closing beads.

**CRITICAL: Use ONLY --robot-* flags. Bare bv launches an interactive TUI that blocks your session.**

#### The Workflow: Start With Triage

**`bv --robot-triage` is your single entry point.** It returns everything you need in one call:
- `quick_ref`: at-a-glance counts + top 3 picks
- `recommendations`: ranked actionable items with scores, reasons, unblock info
- `quick_wins`: low-effort high-impact items
- `blockers_to_clear`: items that unblock the most downstream work
- `project_health`: status/type/priority distributions, graph metrics
- `commands`: copy-paste shell commands for next steps

```bash
bv --robot-triage        # THE MEGA-COMMAND: start here
bv --robot-next          # Minimal: just the single top pick + claim command

# Token-optimized output (TOON) for lower LLM context usage:
bv --robot-triage --format toon
```

Before claiming, verify current state with `br show <id> --json` or `br ready --json`. `recommendations` can include graph-important blocked or assigned work; only `quick_ref.top_picks` and non-empty `claim_command` fields represent claimable work.

#### Other bv Commands

| Command | Returns |
|---------|---------|
| `--robot-plan` | Parallel execution tracks with unblocks lists |
| `--robot-priority` | Priority misalignment detection with confidence |
| `--robot-insights` | Full metrics: PageRank, betweenness, HITS, eigenvector, critical path, cycles, k-core |
| `--robot-alerts` | Stale issues, blocking cascades, priority mismatches |
| `--robot-suggest` | Hygiene: duplicates, missing deps, label suggestions, cycle breaks |
| `--robot-diff --diff-since <ref>` | Changes since ref: new/closed/modified issues |
| `--robot-graph [--graph-format=json\|dot\|mermaid]` | Dependency graph export |

#### Scoping & Filtering

```bash
bv --robot-plan --label backend              # Scope to label's subgraph
bv --robot-insights --as-of HEAD~30          # Historical point-in-time
bv --recipe actionable --robot-plan          # Pre-filter: ready to work (no blockers)
bv --recipe high-impact --robot-triage       # Pre-filter: top PageRank scores
```

### br Commands for Issue Management

```bash
br ready              # Show issues ready to work (no blockers)
br list --status=open # All open issues
br show <id>          # Full issue details with dependencies
br create --title="..." --type=task --priority=2
br update <id> --status=in_progress
br close <id> --reason="Completed"
br close <id1> <id2>  # Close multiple issues at once
br sync --flush-only  # Export DB to JSONL
```

### Workflow Pattern

1. **Triage**: Run `bv --robot-triage` to find the highest-impact actionable work
2. **Claim**: Use `br update <id> --status=in_progress`
3. **Work**: Implement the task
4. **Complete**: Use `br close <id>`
5. **Sync**: Always run `br sync --flush-only` at session end

### Key Concepts

- **Dependencies**: Issues can block other issues. `br ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers 0-4, not words)
- **Types**: task, bug, feature, epic, chore, docs, question
- **Blocking**: `br dep add <issue> <depends-on>` to add dependencies

### Session Protocol

```bash
git status              # Check what changed
git add <files>         # Stage code changes
br sync --flush-only    # Export beads changes to JSONL
git commit -m "..."     # Commit everything
git push                # Push to remote
```

<!-- end-bv-agent-instructions -->
