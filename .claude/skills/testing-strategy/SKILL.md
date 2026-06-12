---
name: testing-strategy
description: Use when deciding WHAT kind of test a piece of code needs — before writing tests for a new module, when unsure whether something deserves a property test, an example test, a contract test, or an integration test, or when scoping where cargo-mutants should run. Maps test types onto hexagonal zones. The verification-ratchet skill defines HOW to write each kind; this skill defines WHERE each kind belongs.
---

# Testing Strategy — what goes where

The architecture decides the test type. In a hexagonal system there are four
zones, and putting the wrong test type in a zone wastes effort or, worse,
produces theater. In ECB terms: Zone 1 = Entities, Zone 2 = the ports
between Controls and Boundaries, Zone 3 = Boundaries, Zone 4 = Controls.
Zones live in capability-named modules (screaming architecture), so the test
file for `src/pricing/` carries `REQ-PRICING-*` IDs — the AREA, the module,
and the bounded context are the same word.

Zones map onto hex-lint roles, which makes the zone boundaries mechanically
enforced at the crate-dependency level (the gate fails on forbidden edges):

| Zone | hex-lint role(s) |
|---|---|
| 1 — Domain core | `domain` |
| 2 — Ports | `port-and-adapter` |
| 3 — Adapters | `driven-adapter` (DB, clients), `driving-adapter` (HTTP, CLI), `infra` (plumbing) |
| 4 — Use cases | `usecase` |
| wiring (slice step 8) | `composition-root` — the only crate allowed to see everything |

A test that needs a dependency its zone's role forbids is not a test
problem — it's a structure problem, and hex-lint will say so before you do.

## Zone 1 — Domain core (pure logic inside the hexagon)

Money math, discount rules, withdrawal logic, parsers, planners, validators.
Pure functions and types: no I/O, no clock, no randomness (inject those
through ports). This is where ~80% of the verification budget goes, because
it is where the requirements live.

- **Example tests** — one per anchor scenario, exact values.
- **Property tests** — every candidate law in the spec. This zone is where
  properties shine: inputs are cheap to generate, oracles and invariants
  exist, no setup.
- **Mutation testing** — full coverage; every surviving mutant here matters.
- **NO mocks.** The core is pure; if you feel the need to mock inside the
  domain, a port boundary is in the wrong place — report it.

## Zone 2 — Ports (the trait definitions)

The contracts the domain depends on (`SaleRepository`, `PriceSource`, ...).

- **Contract test suite** — written ONCE per port as a generic function over
  `T: PortTrait`, asserting the semantic guarantees the domain relies on
  (e.g. "save then load returns an equal value", "fetch of unknown id is
  NotFound, not panic"). Run it against EVERY implementation: the real
  adapter AND the in-memory fake. This is what keeps fakes honest — a fake
  that passes a different test suite than the real adapter is a lie the
  domain tests inherit.
- Round-trip properties fit naturally here (save/load, serialize/parse).
- **A port trait gets NO scenario/feature spec of its own.** A trait is a
  contract declaration — there is nothing to assert about a declaration.
  Its guarantees are verified downstream: the real adapter via the
  contract/conformance suite over deterministic fixtures, the use cases
  via the certified fake. Writing a fake in the domain just to "BDD the
  trait" front-runs the use-case work and only tests the fake.
- The same scoping rule one level down: **record/data types get no
  behavior specs** — constructors and accessors are not behavior. Only a
  real derivation over the data earns a red→green scenario. (The
  wrong-but-valid stub trick in tdd-cycle confirms this mechanically:
  stub everything, and the types with zero assertion failures are the
  ones with no behavior worth specifying.)

## Zone 3 — Adapters (outside the hexagon: DB, HTTP, scrapers, files)

- **Thin integration tests** — against the real technology (testcontainers
  for Postgres, wiremock for HTTP, fixture files for parsers of scraped
  pages). They verify TRANSLATION only: SQL is valid, JSON maps to the
  domain type, the HTML selector still finds the price. Business rules are
  NOT re-tested here — that duplication couples tests to implementation.
- **Property tests: rarely.** Generating arbitrary inputs against real I/O
  is slow and flaky. The exception is pure translation logic inside an
  adapter (a response parser) — extract it into a pure function and treat it
  as Zone 1.
- **Mutation testing: expect noise.** Mutants in error-handling and retry
  paths are expensive to kill and low-value. Scope mutants to domain + app
  by default; adapter mutants are triaged with a lighter hand, but
  exclusions still go through the human (CLAUDE.md).

## Zone 4 — Application services / orchestration

Use-case functions wiring domain + ports; sagas, workflows, schedulers.

- **Scenario tests** — anchor scenarios that span steps, run with in-memory
  fakes (which the Zone 2 contract suite has certified).
- **State-machine properties** — if the orchestration is stateful, generate
  random valid command sequences and assert invariants hold after every
  step ("a cancelled order is never shipped"). This is the highest-value
  advanced use of proptest; reach for it when sequencing bugs are the risk.
- Clock/randomness enter only via ports, or none of this zone is testable
  deterministically.

## Determinism injection (all zones — the flaky-test prevention list)

Non-determinism is hunted at the source, never tolerated in assertions:

- The clock is a parameter (`now: i64` into the use case); only the
  boundary reads a real clock. Tests pass fixed values; ages and
  expirations become pure rules.
- Any OS-dependent lookup (user names, file ownership, locale) gets a
  fake injected in the test — and build the system-under-test at the
  point of use so the fake CAN be injected. Field incident: a "no
  configured owner" test passed only because the real uid hadn't been
  consulted yet; adding the OS fallback would have silently broken it.
- Fixture timestamps are pinned and strictly increasing where ordering
  depends on time; archive/file mtimes are pinned or every generated
  artifact differs.
- Sort anything read from a directory scan — read order is unspecified.

## Choosing example vs property for a given requirement

- Spec states an EXACT mapping ("100 at 10% → 90") → example test.
- Spec states a LAW ("total never exceeds subtotal") → property test.
- Spec is a TABLE (brackets, rate matrices) → table-driven test straight
  from the spec's Examples block, plus one property over the table's
  monotonicity/continuity if it has one.
- You can state no law at all → suspicious; either the requirement is
  under-specified (spec-authoring) or the code is glue that belongs in
  Zone 3/4 with a scenario test.

## Golden-master / conformance tests (the oracle pattern)

For ports, rewrites, and legacy capture, a fifth test type joins the four
zones: run the reference implementation, capture its output as committed
golden files, and diff your implementation against them. This is the
mechanical form of "the original is the spec." The full harness
discipline — two-tier oracle, determinism engineering at capture time,
fixture evolution, divergence fencing — is the **golden-parity** skill;
what follows is the placement summary. Two field results worth knowing
before you skimp: a byte-exact golden surfaced two latent bugs that a
`contains`-only suite had already blessed, and a missing golden must
panic, never skip.

- **Tier first, then compare.** Format-stable machine-parsed outputs are
  compared byte-for-byte — substring/normalized checks pass over
  systematic whole-output corruption. Deliberately-modernized presentation
  is compared as a semantic projection (extracted fields, parsed
  structure), never as markup. Normalization is reserved for the
  reference's transport accidents (legacy header casing, charset quirks),
  each one a recorded decision — otherwise the golden ossifies accidents
  and fails on cosmetics you already ADR'd.
- Golden files are committed and append-only in spirit: regenerating them
  is a check-change (it redefines correct), hook-protect the directory the
  same way as proptest-regressions.
- Each golden test carries the REQ ID of the parity requirement it
  verifies; an unexplained golden diff is triaged as bug-vs-divergence
  against the spec's `## Divergences` list, never resolved by silently
  re-blessing the output.
- Granularity: prefer many small golden cases (one behavior each) over one
  giant page dump — a monolithic golden file fails for every reason at
  once and diagnoses nothing.
- Where they live: the conformance harness sits with acceptance tests
  (outer loop), driven through the same boundary the reference was —
  they prove parity, not internal design; zones 1–4 still carry the
  unit/property/contract load.

## No reference? Manufacture the oracle (greenfield)

The golden-master pattern above needs an external reference to capture.
Most greenfield work has none — there is no "original," and your own
golden files are only as correct as the run that produced them, so a
captured golden cannot be the source of truth the way a port's can. The
oracle is *manufactured*, not captured:

- **Behavioral correctness comes from Zone-1 properties**
  (verification-ratchet Layer 2): metamorphic relations and round-trips
  need no known answer, and a self-written slow-obviously-correct model is
  your reference when a requirement pins an exact one — the substitute for
  the reference implementation a port would diff against.
- **The agent's *structural* reference is the walking-skeleton template
  slice + the type system** (slice-workflow, session-lifecycle), not an
  external truth. A new slice is pattern-matched against the worked example
  and constrained by the types; "what does correct look like" is answered
  by the canonical slice, not a reference binary.

Reach for golden-parity only when a real external reference exists;
otherwise correctness is carried by types, properties, and the template
slice. A golden captured from your own first run is a regression pin
(it catches *change*), never a correctness oracle (it cannot catch a
first-run bug) — do not confuse the two.

## Vacuous green — tests that pass without running

A suite can be green because nothing ran. Field incident: 11 smoke tests
guarded by a skip-if-dependency-missing probe that checked a filesystem
path; the dependency lived elsewhere, every test silently skipped for
weeks, and the adapter was broken in two ways against the real binary
the whole time. Rules:

- **A skip-guard is test logic.** Verify it fires: probe for the
  dependency the way the dependency actually presents (ask the tool
  where it lives), never via a path assumption.
- **Skips are loud or fatal.** Count and report skips in normal runs;
  in gated contexts (CI, the canonical verify recipe), fail on skip.
  In executable-Gherkin suites this is `fail_on_skipped`: an undefined
  step — i.e., spec wording that drifted from the step definitions —
  is a red build, which is the entire anti-drift point of Mode B.
- "Tests pass" is not evidence the adapter works if the tests can
  self-skip; once doubted, verify with one real invocation against the
  real dependency.

## Extracting testability: functional core, imperative shell

When a decision lives inside an IO loop (async watch loops, retry
drivers, schedulers), don't test it through the loop — extract the
decision table into a pure function (`next_step(&Event) -> Step`) and
the loop becomes a thin shell that calls it. The full termination/
decision table is then Zone-1 testable synchronously: no runtime, no
fake channels, no sleeps. If a Zone-3/4 test needs a fake clock AND a
fake channel AND a timeout to assert one decision, the decision is in
the wrong zone — extract it.

## Anti-patterns (each is a report-worthy smell)

- Property test whose body re-implements the algorithm under test
  (a mirror, not an oracle — it proves nothing and ossifies the code).
- `prop_assume!` discarding most generated inputs — the strategy is wrong;
  build validity into the strategy (`0u8..=100`), don't filter it in.
- Mocking the unit under test, or mocking inside Zone 1.
- Re-testing domain rules through an adapter ("integration test" that
  asserts discount math through HTTP) — slow, redundant, and couples the
  business rule to transport.
- Test-per-branch parity: one example test added per `if` added. That's the
  tdd-cycle "specific tests, generic code" smell — the property is missing.
