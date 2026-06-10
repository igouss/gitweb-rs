---
name: slice-workflow
description: Use when a requirement spans a full vertical capability slice across multiple hexagonal zones — domain entity, port, adapter(s), use case, render/web — plus port conformance. Defines the double-loop TDD protocol, the sanctioned WIP-acceptance-test mechanism, the layer implementation order, and the slice definition of done.
---

# Vertical Slice Workflow (double-loop TDD)

A slice is ONE behavioral requirement whose implementation crosses all four
zones. It is not five requirements. REQ IDs name behavior the user of the
system can observe; "implement the repository adapter" is not a requirement,
it's a layer of one. The exception: a port guarantee that is domain-
meaningful in its own right ("save then load returns an equal sale") gets
its own REQ ID, because the contract suite that verifies it outlives any
single slice.

## The double loop

Two loops, two clocks:

- **Outer loop (acceptance)**: ONE end-to-end test per in-flight slice,
  derived from the REQ's primary anchor scenario, exercising the real wiring
  (web/render boundary → use case → domain, with real or containerized
  adapters). It goes red on day one and goes green when the slice is done.
  Days may pass in between. That is its job.
- **Inner loop (tdd-cycle)**: the normal Three-Laws red/green/refactor,
  zone by zone. The "at most ONE failing test" rule and "never end mid-red"
  apply to THIS loop only.

The reconciliation with the rest of the kit is the WIP convention below:
the outer test exists, is committed, guides the work — but is excluded from
the green bar in a form that is mechanically tracked and cannot be silently
left behind.

## The WIP convention (the only sanctioned `#[ignore]`)

```rust
// REQ-SALE-007
#[ignore = "WIP(REQ-SALE-007)"]
#[test]
fn customer_sees_discounted_total_on_checkout() { ... }
```

Rules, all mechanically enforced:
- Exactly this string form: `WIP(REQ-<AREA>-<NNN>)`. check-guard permits
  adding `#[ignore]` ONLY in this form; any other ignore is still a
  weakening violation.
- The referenced spec must exist and have `status: in-progress`.
- trace-audit fails on STALE WIP: a WIP marker whose spec has moved to
  `status: accepted`. You cannot declare a slice done and leave the
  acceptance test parked — removing the marker and seeing the test green
  IS the definition of done.
- One WIP acceptance test per in-flight REQ. A pile of WIP markers means
  too many slices in flight; finish one.
- Spec Mode B (executable .feature files): the marker is a
  `@wip(REQ-<AREA>-<NNN>)` tag on the scenario, with the wip tag excluded
  from the default cucumber run — identical rules, identical staleness
  semantics; the audit greps the tag instead of the attribute.

## Layer order (default for a slice)

Inside-out with the outer rail in place. Each step is a normal tdd-cycle
loop in its zone, with the test types testing-strategy assigns to that zone:

1. **Spec** (spec-authoring): EARS + anchor scenarios + candidate laws.
   Spec status: `in-progress`.
2. **Acceptance test** from the primary scenario, committed WIP-ignored.
   This is the slice's contract with itself; write it before any layer so
   the wiring shape is decided by the behavior, not by whichever adapter
   you happened to build first.
3. **Domain** (Zone 1 — entities, pure logic): types first, then example +
   property tests. Most of the slice's actual thinking happens here.
4. **Port + conformance** (Zone 2): trait definition, contract test suite
   as a generic fn, in-memory fake passing it. The fake is now certified
   for use upstream.
5. **Use case / control** (Zone 4): scenario tests against the certified
   fake. No real I/O yet.
6. **Adapters** (Zone 3): each real adapter (DB, HTTP, scraper) passes the
   SAME contract suite as the fake, plus thin translation tests.
7. **Render/web boundary** (Zone 3): thin — request decoding, response
   encoding, status mapping. No business rules; if you're tempted to test
   discount math here, you've leaked domain into the boundary.
8. **Wire-up**: the composition root (hex-lint role `composition-root` —
   the only crate permitted to depend on everything) connects the pieces.
   Remove the WIP marker. Acceptance test green. Spec status → `accepted`.
   `hex-lint` clean — any new crate created during the slice was tagged
   with its `hex-arch.role` at creation (untagged members are a hard
   error, so this cannot be deferred).
   **DON'T FORGET: register in the composition root and prove it from the
   real binary.** The test harness wires its own world — a handler can
   pass every web feature while the real binary still 404s the route
   because nothing registered it. Field incident: the action took the
   "unknown action" error path even though the handler existed and its
   tests were green. "Served by the real binary" is part of the slice's
   definition of done, not an ops detail.
9. **Gates**: mutants + CRAP scoped to the changed zones (mutation triage
   weights by zone per testing-strategy), trace-audit, check-guard.

Steps 3–7 each end in green commits with layer-scoped subjects
(`pricing/domain:`, `pricing/port:`, `pricing/http:`), so the history reads
as the slice assembling.

## First slice in a capability: walking skeleton

If the capability has no slices yet, do NOT pick a meaty behavior first.
Pick the thinnest behavior that still touches every layer (a tracer bullet:
"GET returns the stored entity's name") and drive it through steps 1–9.
This forces every wiring, serialization, and composition decision while the
stakes are one field, not a business rule. Slice two onward is then mostly
Zone 1/4 work hanging off proven plumbing — which is also where the
walking skeleton pays off for sessions: later slices fit in one session;
the skeleton is the one that may not.

## Port design heuristics (step 4, before you write the trait)

- **A port method earns its existence by result shape, not by topic.** Two
  searches that return different shapes (`Vec<Commit>` vs commit-plus-files)
  are two methods, not one method with a kind enum. And when the real
  method lands, DELETE the placeholder enum arm — no second way to do it.
- **Cross-cutting concerns go on the port method all consumers share.** A
  filter that applies to every listing belongs on the one `list` method —
  one port change, every consumer inherits — never copied into each
  caller.
- **Don't widen a port when a pure function over existing output
  suffices.** Need a single-file view of a whole-tree result? A pure
  `select`/`narrow` function in the use case beats a new port method.
- **Two views over the same data with different filters share a core.**
  Extract the shared assemble function; each action becomes a thin wrapper
  supplying filter + error message. If you're copying an assemble body,
  you missed the seam.
- **Adapter-internal seams are not domain ports.** A non-deterministic OS
  lookup consumed by exactly one adapter gets a trait *inside that
  adapter*, with the real impl installed by default and a builder-style
  injection for tests — domain stays clean, existing call sites stay
  untouched.
- **A capability only some implementations support is a supertrait with
  NO default methods.** Default impls let every backend type-check into
  the capability path and degrade silently at runtime; the no-default
  split makes a wrong backend a compile error at the wiring site.
- **Keep ports narrow; fake size is the metric.** If a use-case test
  fake stubs 15 methods to exercise one, the port is fat — split it into
  capability traits (a field ISP split shrank fakes from ~15 stubbed
  methods to 1–5).
- **Placement follows signature types.** A port whose arguments carry an
  outer-layer type lives in the outer layer; a port carrying only
  primitives/handles stays inward. Decided by what rides the signature,
  not by topic — and verified against the dep matrix BEFORE the bead is
  written (tracker-discipline).

## Seam memos (closing a slice)

A slice isn't closed until its reusable seams are written down where the
next session will find them (memory-discipline, seam-catalog genre): each
new port method, shared mapper, or extracted core, by path and signature,
with an imperative consumer note — "future file browsing MUST drive
through this, not a new method." The next slice is built by a session that
wasn't there; the seam memo is the difference between reuse and
reinvention. Shared-code extraction discovered mid-slice (two handlers
growing the same row logic) happens in the REFACTOR step and goes in the
memo too.

## Migration slices (existing code, not greenfield)

When the slice replaces something rather than adding it, the grammar
changes: land the new code consumer-free, swap consumers one commit at a
time, delete duplicates with the last swap; or expand–migrate–contract
for data-shape changes. The full sequencing patterns, legacy-bridge
labeling, and fractional-slice convention are in refactoring-campaign —
use that skill the moment a slice's diff is mostly deletions of an old
shape.

## Decomposing a fat capability

A capability covering N distinct behaviors/endpoints is N slices. Split at
pickup time, not in upfront planning — by then the seams are visible. One
behavior = one child work item = full test layers. Mid-flight rescoping is
normal: when a slice turns out to be two things, split again and ship the
shippable half (see tracker-discipline). The parent stays open until the
last child closes, and tracks which child blocks it.

## Slices vs sessions

A slice frequently outlives a session. The handoff (session-lifecycle)
carries a layer map for the in-flight slice in the bead/HANDOFF.md:

```
REQ-SALE-007  [spec ✓] [acceptance WIP ✓] [domain ✓] [port+fake ✓]
              [usecase ▶ next: scenario 'discount on empty cart']
              [adapters –] [web –] [wire-up –]
```

The inner loop's "never end mid-red" holds per session because the WIP test
is ignored: the suite is green at every handoff even though the slice isn't
done. That's the whole point of the convention.

## Anti-patterns

- Building all five layers speculatively, then writing tests. That's
  waterfall in a trench coat and Law 1 dies first.
- Per-layer REQ IDs ("REQ for the adapter"). Layers are not behavior.
- A second WIP acceptance test before the first slice closes.
- The fake drifting from the real adapter — impossible if both run the
  contract suite; a fake with its own bespoke tests is a lie incubator.
- Driving domain-rule tests through the web layer "because the acceptance
  test already goes through it". The acceptance test proves WIRING, once,
  per slice. The rules are proven in Zone 1 where mutants are cheap.
