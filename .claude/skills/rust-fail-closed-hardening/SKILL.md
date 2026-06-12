---
name: rust-fail-closed-hardening
description: Use when constructing a domain type with invariants, validating untrusted or numeric input, building subprocess argv, sharing state behind a lock, writing a predicate that gates a release/admission decision, or auditing safe Rust for the silent ways it produces and propagates a wrong value. Greenfield has no reference oracle to catch a wrong-but-non-crashing result — so the wrong value must be made un-constructable, rejected at the boundary, and never allowed to poison state. Fail closed.
---

# Rust Fail-Closed Hardening

In a port you have an oracle: a wrong result diverges from the reference and a
golden catches it. **Greenfield has no oracle.** A value that is wrong but does
not crash will ship, and stay wrong, indefinitely — there is nothing to compare
against. So correctness here is not "diff the output"; it is making the wrong
value impossible: un-constructable, rejected at the boundary, unable to poison
shared state, and — when something must give — failing toward the safe state,
not the open one.

Field origin: a 9.8k-commit `#![forbid(unsafe_code)]` Rust workspace whose
maintainers ran a defensive audit file-by-file — **24 passes, 56 bugs, a
self-reported 100% hit rate on previously-unreviewed modules** (`docs/audit-
checklist.md`). Every pattern below is safe Rust silently doing the wrong
thing. None of them are caught by `cargo build`, clippy, or "the tests pass."
This skill is the checklist; verification-ratchet Layer 0 is the type-level
half of the same fight (make the illegal state uncompilable).

## 1. The constructor is not a gate if the fields are `pub`

A `new()` that clamps and validates is worthless the moment the fields are
`pub` — sibling code in the same process writes past it, no compile error, no
panic:

```rust
pub struct Breaker { pub failure_threshold: u32 }   // new() clamps to [1, MAX]...
let mut b = Breaker::new(5);
b.failure_threshold = u32::MAX;                      // ...and is routed around. Breaker disabled.
```

Rust developers trust that `new()` enforces the invariant and forget that a
`pub` field is a second, unvalidated constructor. The fix gives the invariant
**exactly one enforcement point**:

```rust
pub struct Breaker { failure_threshold: u32 }        // private
impl Breaker {
    pub fn new(t: u32) -> Self { Self { failure_threshold: t.clamp(1, MAX_THRESHOLD) } }
    pub fn with_threshold(mut self, t: u32) -> Self {
        self.failure_threshold = t.clamp(1, MAX_THRESHOLD); self                 // re-applies on every write
    }
    pub fn failure_threshold(&self) -> u32 { self.failure_threshold }            // read accessor, no write path
}
```

Audit smell: a `pub`-field struct whose `new()` does any validation. (Field:
a capability passport shipped 28 `pub` fields past its constructor — anchors
`ft-l5z7z`, `ft-mnua0`, `ft-n9btw`, `ft-t9ydu`.) This is the value-level form
of the predicate-on-the-entity rule (refactoring-campaign): one place may set
the invariant, and the type makes that the only place.

## 2. Non-finite floats poison stateful accumulators forever

`NaN`/`Inf` is the quietest wrong value in the language. One bad sample pins a
running accumulator permanently — and `NaN`'s comparison semantics walk it
straight past the guard that should have caught it:

```rust
self.sum += sample;                          // sample is NaN once -> self.sum is NaN forever;
                                             // every clean sample after it leaves sum == NaN
if self.mean() > self.threshold { alert() }  // NaN > x == false -> the alert silently never fires again
```

An EWMA, a Bayesian posterior, a histogram mean, a centroid — all latch. The
detector you built is now permanently off and nothing says so. **Reject
non-finite at the entry point, before mutating state**, and fail closed:

```rust
fn record(&mut self, sample: f64) -> Result<(), Invalid> {
    if !sample.is_finite() { return Err(Invalid::NonFinite); }   // state untouched
    self.sum += sample; self.n += 1; Ok(())
}
```

And sanitize where the float **enters** the system (the JSON/extraction
boundary), not deep in the pipeline where it has already spread —
`sanitize(t) = Some(t).filter(|v| v.is_finite() && *v >= 0.0)`. Constructors
that take a rate/alpha/weight reject non-finite there too. (Field: 43 beads of
this class — `ft-b4l62`, `ft-n4fdx`, `ft-c17iw`, `ft-fiuu6`; whisper
`sanitize_timestamp`, `bd99c6c`.)

## 3. A clamp after the overflow is no clamp at all

Saturating/checked math is not optional decoration; the order matters as much
as the operation:

```rust
let refill: u64 = rate * elapsed;                 // overflows to a tiny wrapped value BEFORE the clamp
self.tokens = (self.tokens + refill).min(cap);    // the clamp now faithfully clamps garbage
```

```rust
let refill = rate.saturating_mul(elapsed);
self.tokens = self.tokens.saturating_add(refill).min(cap);   // saturate, THEN clamp
```

Two traps inside the fix: `saturating_*` on `u*::MAX` produces silent identity
collisions (two distinct sequence numbers both saturate to the same value —
`ft-g8nbu`); and `saturating_sub` that bottoms out at `0` is itself the bug
when `0` is your "closed/empty/denied" state — a small-capacity queue saturates
into a permanent closed reading (`ft-cuzeu`). Pick the saturating *direction*
for the safe state deliberately, and use `saturating_mul` for every unit
conversion on a config-supplied duration/timeout (whisper `9f94946`). (Field:
30 overflow beads; one filed twice — `ft-b5rbd`/`ft-htf0i` — because the first
fix clamped without reordering.) For shape/index/`numel` math, this is
existential: checked arithmetic, and replace panicking `assert!` with typed
errors (torch `3ecc6d8`, `75c48db`, `8f2af1c` — 21 hardening commits).

## 4. Choose fail-open vs fail-closed on purpose, and test the failure

The default behavior when a gate's own inputs are degenerate (a `NaN`
threshold, misordered bounds, an empty config) is a decision, and almost
everyone makes it by accident — toward open. A canary gate that "fails open"
on a `NaN` error-rate ships the bad build to everyone; a capacity governor that
fails open removes the backpressure exactly when load is pathological. **A
safety/admission gate fails CLOSED** (deny, degrade, refuse). State the
direction in the code and write the test that feeds it the degenerate input —
the failure path is the one that matters and the one nobody exercises. (Field:
`ft-43wis`, `ft-n0i68`, `ft-394fc`; retry that panicked on `NaN` backoff with
no fail-closed default — `ft-s4c6y`.) The mirror image for hostile input is
torch's "fail closed on non-grad roots" (`c9b04a9`): a corrupt graph refuses
rather than fabricates.

## 5. A predicate that passes on empty state is a vacuous gate

The release/done/clean predicate that looks at *absence of failure* says
"green" on a cold default that ran nothing:

```rust
fn release_ready(&self) -> bool { self.failures == 0 }   // true before any check ran
```

Pair the predicate with **evidence of work**, never absence alone, and test
that the zero-value default returns `false`:

```rust
fn release_ready(&self) -> bool { self.checks_run > 0 && self.failures == 0 }
```

This is the value-level form of quality-gates' vacuous-green: a check is only a
check if its *firing* is observable. The same applies to any "attested",
"verified", "meets_bar" flag — require ≥1 counter non-zero, and a regression
asserting the empty case fails. (Field: `ft-vqohn`, `ft-vesy5`, `ft-yxrez`.)
When you write the regression, also prove the gate *fails* when it should:
inject a violation and assert it's caught — a gate that has only ever been
seen to pass is untested.

## 6. Panics are for invariant violations, not for input or for locks

`.unwrap()`/`.expect()` on anything input-derived converts malformed input into
a crash — use a typed error (the domain owns the noun, the use case owns the
sentence; refactoring-campaign §5). On a *shared lock* it is worse: one
panicking thread poisons the mutex and `.lock().expect("poisoned")` then bricks
the subsystem for every later caller — a single-thread bug becomes a
total outage cascade. Recover instead of cascading:

```rust
let guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
```

But a *silent* recovery hides the degradation — count it (a per-subsystem
poison counter, or a `record_poison_and_recover<T>` helper), or operators stay
blind to a lock that is quietly failing under them. (Field: this split into
three classes — panic-cascade `ft-zvhav` (13 sites), `ft-h2vyr` (8 sites);
recover-without-a-counter `ft-ky7nf` (32), `ft-rln0q` (34); whisper `b1682e8`.
Hot-path `.expect()` → typed errors, `0c4d557`.)

## 7. Unbounded growth is a correctness bug, not just a performance one

Three shapes, all of which produce a wrong/dead system rather than merely a
slow one:

- **Content-validating constructors with no count/length cap** OOM on a hostile
  input that passes every *content* check. `MAX_*` consts at module top, wired
  into `validate()`, tested at `MAX + 1` (`ft-a2bt5`).
- **A map keyed by a resource id** (session, connection, pane, txn) that
  inserts but never evicts leaks forever — and *adding the eviction method is
  not the fix; wiring it into the destroy/teardown hook is.* The field repo
  filed this exact leak **three times** because the method existed and the
  `*Destroyed` handler never called it (`ft-yjt9e`). Worse, a late event
  arriving *after* teardown re-inserts the entry — lifecycle events race their
  own ordering, so the destroy path must be idempotent and last-wins.
- **Tasks/handles/FDs orphaned on an early return.** Every `?` between "spawn /
  open" and "join / close" is a potential leak — audit the error paths, not the
  happy path.

Prove it with a deterministic leak-oracle regression (churn → reconnect →
shutdown, then assert the registries are empty), not ad hoc — it is the only
test that catches "wired the eviction but missed one event."

## 8. Subprocess argv and child output are an injection/deadlock surface

User-controlled positionals handed to `Command` without a `--` sentinel are
reinterpreted as flags — `grep --exec`, `git --use-mailmap` (`ft-vesy5`). Emit
your flags, then a literal `"--"`, then the user values, and regression-test
that a leading `-x` lands as a positional:

```rust
cmd.args(["--color=never", "--"]);   // sentinel
cmd.args(user_positionals);          // everything past -- is data, never a flag
```

Same boundary, different failure: draining a child's stdout then stderr
sequentially **deadlocks** once output passes the OS pipe buffer (~64KB) — the
child blocks writing stderr while you block reading stdout. Read each pipe on
its own thread (or async task) before `wait()` (whisper `dd6cbb2`). These are
Zone-3 boundary concerns (testing-strategy) — the boundary will hand you
hostile shapes, and the domain must never see them.

## The audit discipline

Run this checklist file-by-file on any non-trivial type, not as a one-time
sweep. **File a finding before fixing it** (one bead per finding), then fix and
**pin a regression that encodes the new contract** — the cap at `MAX+1`, the
`-x`-as-positional case, the `NaN`-is-rejected case, the cold-default-is-false
case. The bug is cheap; the regression is what stops it recurring under the
next amnesiac session. Greenfield's whole bet is that the type, the boundary
check, and the regression together do the job the missing oracle would have
done — that substitution (no reference to diff against, so make the wrong value
un-constructable instead) is the reason this skill exists.
