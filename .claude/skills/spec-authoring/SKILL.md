---
name: spec-authoring
description: Use when writing a new requirement, defining a new feature, or when asked to implement behavior that has no requirement in specs/. Produces an EARS requirement with an FDD-style feature name, anchor Gherkin scenarios, and a stable REQ ID. Also use when an existing requirement must change.
---

# Spec Authoring

One requirement = one file in `specs/`, named after its ID: `specs/REQ-SALE-003.md`.
Use the template at `templates/requirement.md`.

## 1. ID assignment

Format: `REQ-<AREA>-<NNN>`.
- `<AREA>`: short uppercase domain area (SALE, AUTH, SCRAPE...). Reuse existing
  areas: `ls specs/ | cut -d- -f2 | sort -u`. Invent a new one only for a
  genuinely new bounded context.
- `<NNN>`: next free number in that area: `ls specs/REQ-<AREA>-*`.
- IDs are permanent. Never renumber, never reuse a retired ID.
  Retired requirements keep their file with status `superseded` and a pointer.

## 2. Feature name (FDD form)

`<action> the <result> <by|of|for|to> a(n) <object>`

Examples: "Calculate the total of a sale", "Reject the login of a locked
account". This is the vocabulary lock: the nouns and verbs here are the only
ones to use in code, tests, and conversation for this feature. Do not invent
synonyms later (no `applyRebate` when the spec says discount).

If the feature can't be named in this form in one line, it is too big — split it.

## 2a. Design heuristics (apply HERE, not in implementation)

Three questions to ask of every requirement before accepting its shape:

- **Is this one thing actually two things?** If the EARS statement needs an
  "and", or the anchor scenarios cluster into two unrelated groups, split
  into two REQ IDs. Most bad features are two good features stapled together.
- **Is there a broader problem whose solution makes this one trivial?**
  "Parse the vendor-A price page" might really be "normalize any vendor page
  into a PriceQuote" with vendor-A as the first adapter. If yes, write the
  broader requirement and make this a case of it.
- **Is there a thing which, built first, makes this thing easy?** If the
  honest implementation plan starts with "first I'd need...", that
  prerequisite is its own requirement with its own ID, sequenced first.

These heuristics are LEGAL at spec level and ILLEGAL at code level. Deciding
to spec the general problem is design; writing generalized code no test
demands is a Three Laws violation (tdd-cycle, Law 3). Generalize the
requirement, then let the tests force the generality into the code.

Also: the `<AREA>` prefix is a business capability and should match the
module directory (screaming architecture — `REQ-PRICING-*` ↔ `src/pricing/`).
If you can't pick the AREA, you haven't found the bounded context yet;
that's a design conversation with the human, not a naming problem.

## 3. The requirement (EARS)

Pick the matching template. One requirement statement per file.

| Pattern | Form |
|---|---|
| Ubiquitous | The system shall `<response>` |
| Event-driven | When `<trigger>`, the system shall `<response>` |
| State-driven | While `<state>`, the system shall `<response>` |
| Unwanted behavior | If `<condition>`, then the system shall `<response>` |
| Optional feature | Where `<feature is present>`, the system shall `<response>` |

The EARS statement is universally quantified — it is the law. Be precise about
units, ranges, and boundary inclusivity ("0–100 % inclusive", not "a valid
percentage").

## 4. Anchor scenarios (Gherkin)

1–3 scenarios maximum. They are conversation artifacts and canonical examples,
not exhaustive test scripts — properties and the type system carry exhaustiveness.

Two supported spec modes — pick per project, record the choice in an ADR:

**Mode A — markdown anchors (default for greenfield):** scenarios embedded
in this spec file, translated by hand into plain `#[test]` functions.
Lightweight; the scenario is a conversation artifact.

**Mode B — executable `.feature` files (default for ports/parity projects,
or whenever spec↔test drift is the dominant risk):** scenarios live in
`features/` as real Gherkin run by cucumber-rs; the spec file links to them
instead of embedding. The `.feature` IS the test — spec↔test drift becomes
mechanically impossible, which matters most with agents, who are precisely
the authors that let markdown specs and test code diverge. Side benefit:
fn-hash hashes each Scenario block, so scenario-level change detection
works on the spec itself. Cost: you own a step-definitions layer — keep
steps thin (parse, call the port, assert); business logic in step defs is
Zone-1 leakage. In Mode B the slice WIP convention uses a
`@wip(REQ-<AREA>-<NNN>)` tag excluded from the default cucumber run instead
of `#[ignore]` — same rules, same staleness audit.

## 4a. Oracle mode (ports, rewrites, legacy)

When a reference implementation exists, it — not your intuition — is the
spec source. Before writing any requirement or scenario for behavior being
ported: READ the corresponding source in the reference implementation. Do
not guess what the original does; do not infer it from its output alone.
The EARS statement describes the original's observable behavior; the spec
file records WHERE in the reference it lives — file + function/sub name
**+ line range** — so the next session can re-check the oracle instead of
trusting your reading. Name the reference function in the commit subject
too (`search: grep tree content (git_search_files)`): commit ↔ reference
traceability for free.
Deliberate divergences from the reference (modernized markup, dropped
misfeatures) are not silent: each one is named in the spec under
`## Divergences`, with the reason — otherwise a future conformance test
failure is undiagnosable as bug-vs-decision.

Field-proven additions to the mode:

- **The reference's error strings, status codes, and gate ordering are
  ABI.** Three distinct not-found messages are three behaviors, not one;
  the order in which the reference checks its gates is observable and
  therefore spec.
- **"One thing is two things" — verify against the reference before
  unifying.** Two call sites that look like the same feature routinely
  hide two rules with different bases, case-sensitivity, or error strings
  (the explicit action redirects; the implicit default serves inline, with
  different 404 text). Unifying them is how ports drift.
- **Define the conformance contract on day one**: which outputs are
  byte-exact vs behavioral, and the edge-case matrix promoted to
  acceptance criteria — with frequency evidence from the reference source
  ("~118 merge sites") so edges can't be waved off as exotic. See the
  golden-parity skill.
- **Record ruled-out approaches with verification dates.** "Verified
  2026-06: no existing crate reproduces this byte-exact; library X has no
  encoder." Negative results are the costliest knowledge to regenerate
  and the first thing a fresh session re-attempts.
- **Specs are falsifiable claims.** When implementation disproves a spec's
  or work item's assertion, correct it in writing with the evidence
  (reference line, golden bytes) — never silently implement the truth
  while the document still states the falsehood.

Rules:
- Domain language only. No UI steps, no HTTP verbs, no struct names.
- Concrete values, exact expected outcomes.
- Always include at least one boundary or unwanted-behavior scenario if the
  EARS statement has an If/unwanted form.
- If the behavior is tabular (rate tables, brackets, matrices), use a single
  `Scenario Outline` with an `Examples:` table instead of prose scenarios.
  The table IS the spec.

## 5. Candidate laws

Before finishing, list 1–3 candidate properties (invariants, round-trips,
identities) implied by the requirement, in plain language, under
`## Candidate laws` in the spec file. The verification-ratchet skill turns
these into property tests. If you cannot state a single law, the requirement
is probably an example in disguise — reconsider its EARS form.

## 6. Changing an existing requirement

1. `rg 'REQ-<ID>' --type rust specs/ src/ tests/` — list every artifact bound to it.
2. Update the spec file AND every referencing test in the same commit.
3. If the change invalidates an anchor scenario, rewrite it — do not delete
   it without replacement.
4. Bump the `revision:` field in the spec front-matter and add one line to
   its `## History`.

## Done means

- Spec file exists with ID, FDD name, EARS statement, anchor scenarios,
  candidate laws.
- The ID does not collide: `ls specs/REQ-<AREA>-<NNN>*` returns only this file.
- The human has seen the EARS statement and scenarios before implementation
  starts (paste them in your reply; do not silently proceed for non-trivial
  requirements).
