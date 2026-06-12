---
name: rust-determinism-engineering
description: Use when writing deterministic tests, golden/snapshot suites, or any Rust whose output must be byte-stable across runs, machines, and parallel test execution — or when a test is flaky. In greenfield there is no reference oracle, so your own goldens ARE the spec and a flaky golden is a spec that lies. Covers the single-entropy-fixture funnel, stable hashing/ordering, golden checksum-gating, the four flaky-test causes, seeded randomness, and cross-architecture drift. testing-strategy says WHERE determinism is injected; this is the Rust-deep HOW.
---

# Rust Determinism Engineering

In a port you have an oracle: the reference implementation regenerates the
truth, so a flaky test is an annoyance. In greenfield there is no oracle —
your golden and snapshot tests ARE the spec, derived from nothing but your
own runs. A golden that changes between runs is not flaky; it is a spec
that lies, and every test that trusts it inherits the lie. So determinism
here is not test hygiene, it is spec integrity.

The rule, same as the testing-strategy skill's determinism-injection list
but taken all the way down: **non-determinism is hunted at the source,
never tolerated in an assertion.** No `retry`, no `eventually`, no
"sort-of-equal". This skill is the Rust-deep companion to that list, and
the greenfield analog of golden-parity (which is its oracle-mode form).

Field origin: a six-project Rust fleet (a database engine, a TUI kernel,
two ML-library ports, a terminal emulator, an ASR stack) where determinism
work recurs in every one — 330+ beads in the TUI kernel alone mention
snapshot/flaky/determinism. Every rule below was paid for.

## 1. Funnel ALL entropy through one fixture

Entropy enters through five doors: the clock, randomness, the environment,
hashing, and identifiers. Production reads them at the boundary
(testing-strategy: "the clock is a parameter"); tests read them from a
single deterministic fixture, never ad hoc.

- **Clock**: a fixed time-step counter, not wall time. The TUI harness
  stamps `T000000`, `T000001`, … from a counter and feeds a fixed `now_ms`
  step — never `Instant::now()` (ftui-harness `determinism.rs`).
- **Randomness**: a seeded PRNG (xoshiro256++ / fixed seed), never
  `thread_rng()` in a tested path (frankentorch, commit `0dcce83`).
- **Environment**: capture it into a `BTreeMap` so field order is stable;
  a `HashMap` snapshot reorders per run (ftui `EnvSnapshot`).
- **Identity**: derive `run_id` from the seed, so the same seed reproduces
  the same ids; do not mint ids from time or a process-global counter that
  survives across tests.

One fixture owns all five. If a test reaches around it for "just the
clock", that is the door the next flake walks through.

## 2. Stable hashing and stable ordering — the two silent sources

These two produce goldens that pass locally and fail in CI, the worst
failure shape because it looks like someone else's bug.

- **Never `DefaultHasher`/`RandomState` for anything that reaches a golden,
  cache key, or fingerprint.** It is seeded randomly *per process* — your
  checksum differs every run by design. Use BLAKE3, SHA-256, or a
  fixed-seed hasher (TUI bead: "deterministic checksum, avoid DefaultHasher").
- **Hash through explicit `to_le_bytes()`**, not native-endian — native
  endianness makes the same input hash differently on aarch64 vs x86_64
  (see §6).
- **Floats cannot derive `Eq`/`Hash`** (NaN). When a type must be hashable
  or comparable for an IR node or a cache key, store the bit pattern:
  `enum Literal { F64Bits(u64), .. }` from `f64::to_bits()` (frankenjax
  `fj-core/src/lib.rs:293`). Round-trip with `from_bits`.
- **`BTreeMap` (or `IndexMap`) over `HashMap` wherever iteration feeds
  output** — serialization, a golden, a report, a cache key. `HashMap`
  iteration order is randomized; output built from it is nondeterministic.
  This is the single most-recurring determinism bug across the fleet
  (confirmed independently in the terminal emulator and the TUI kernel).
  Sort anything read from a directory scan or a `HashSet` for the same
  reason.
- **Counterpoint — don't over-sort.** A fixed 1–3 element collection (a
  transform stack, a few effect tokens) keeps declaration order in an
  insertion-ordered `Vec`; a `BTreeMap` there just adds a sort and loses
  the meaningful order (frankenjax `fj-dispatch`). Order by intent, not
  reflex.

## 3. Golden comparison mechanics

- **Checksum-gate before you enumerate.** Compare two artifacts by one
  content hash first (BLAKE3 over dimensions + every field via
  `to_le_bytes()`); only walk per-element diffs when the hashes differ.
  Fast, exact, and it never blesses partial corruption the way a
  `contains`/substring check does (ftui `golden.rs`; golden-parity has the
  full argument for why byte-exact beats `contains`).
- **Make the artifact human-diffable.** A raw hash diff diagnoses nothing.
  Render a canonical text projection alongside it, and for any
  variable-width or unresolved element emit a *width-correct* placeholder —
  `'?'` repeated to the display width, never a single char, or every
  downstream column drifts and the diff lies about where the change is
  (ftui `lib.rs`, a paid-for bug).
- **A golden change is a proof obligation, not a reflex `--accept`.**
  Re-blessing a golden redefines correct — it is a check-change (CLAUDE.md
  boundary; verification-ratchet). Record an isomorphism note: what
  changed, old→new checksum, which invariants still hold, why the drift is
  legitimate. Hook-protect the goldens directory the same as
  `proptest-regressions/` (quality-gates).

## 4. The four causes of flaky tests (test-process determinism)

Distinct from production determinism: these are why a *correct* system has a
red test. In order of field frequency:

1. **Shared mutable global state under parallelism — the #1 cause.** A test
   that calls `reset_metrics()` / `GLOBAL.reset()` clobbers state a
   *parallel* test is mid-read on. Fix: **delta-based assertions** —
   snapshot the counter before, run, assert the *difference* — never
   reset-then-assert-absolute. One project converted 12 tests across 10
   files this way and went from 5 failing to 0 (frankensqlite `bd-2n04d`).
   Recycled slots/pools are the same bug: clear every field before
   publishing a reused slot, or a stale field leaks into the next test.
2. **Timing budgets shrink under `cargo test -j`.** A test that times out
   only under parallelism is not broken logic — the wall-clock budget was
   sized for a quiet machine. Widen it for the parallel case, or better,
   assert against the fixed clock (§1), not elapsed time.
3. **Steady-state asserted before warmup.** Allocator, cache, and
   first-call effects skew the first N iterations. Throw them away, then
   measure (the TUI render budget warms up 30 frames before asserting).
4. **Exact cross-thread event order asserted.** Tracing callsite caches and
   channel coalescing legitimately reorder events. Relax to set-membership
   or causal ordering; an exact-sequence assert over concurrent producers
   is testing the scheduler, not your code.

For "is this growing unboundedly" tests, prefer a statistical detector
(CUSUM / e-process with a bounded false-positive rate) over a brittle
`assert!(x < N)` — the threshold version is itself a flake (ftui
`alloc_budget`).

## 5. Seeded randomness is logged, not just fixed

Every property or randomized test seeds explicitly AND logs the seed +
config + `run_id`, so a CI failure reproduces on the first try instead of
never (frankensqlite verification contract `bd-1dp9.7.1`). This is the
upstream of verification-ratchet's committed `proptest-regressions/`: the
seed log is how a counterexample becomes a regression. A randomized test
whose seed you cannot recover is not a test, it is a slot machine.

## 6. Cross-architecture determinism (the CI-only trap)

"Deterministic" almost always means "deterministic on my machine." Replay
hashes and goldens must be byte-identical across architectures
(aarch64/x86_64, linux/darwin) or the claim is false — one project shipped
a README promising cross-fleet deterministic replay while only same-host
determinism was tested (frankenterm `ft-5cl1b`). The drift sources, all
fixable: native-endian hashing (use `to_le_bytes`, §2), `HashMap`
`RandomState` (§2), `usize`-width-dependent serialization, and locale/float
formatting. Verify it the only way that works: a golden matrix and a
multi-seed sweep in a **nightly CI lane**, not the inner loop (frankenterm
`ft-qrjvh`). Ship the fixture corpus as one canonical schema, not
regenerated per test (frankenterm `ft-tf6g3`).

## 7. Concurrency: prove it, and use replay as a tripwire

- **The cheapest mechanism that fits the question.** Loom for lock-free /
  atomic *ordering* proofs — gate behind `#[cfg(loom)]`, run with
  `RUSTFLAGS="--cfg loom"`, and keep models ≤3 threads because the
  interleaving search is exponential; proptest state-machine for stateful
  sequencing invariants (testing-strategy Zone 4); a TLA+/Stateright model
  for protocol-level questions. Loom is **not** for Mutex-serialized logic
  — that is an ordinary unit test (frankenterm `proof-techniques.md`).
- **Replay determinism is a free nondeterminism tripwire.** Hash the output,
  run the same input N times, assert one hash. Any divergence is an
  unfunneled entropy source from §1 — the test tells you a door is open
  before a user does (franken_whisper `replay_pack.rs`; frankentorch
  `prop_scheduler_replay_is_deterministic`). `#![forbid(unsafe_code)]`
  removes a whole class of nondeterministic UB from this surface for free.

## Smells that mean you are faking determinism, not engineering it

- A `sleep` before an assertion (timing, not synchronization).
- `--accept` / re-bless run reflexively until the test goes green.
- A test that passes alone and fails under `-j` (shared state, §4.1).
- Comparing with `contains` / normalized equality where byte-exact was
  possible (golden-parity).
- "Works on my machine" offered as evidence a replay is deterministic (§6).
