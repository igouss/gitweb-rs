---
name: refactoring-campaign
description: Use when a codebase needs systematic structural repair — not a one-function refactor (tdd-cycle handles that) but a campaign: an audit of accumulated smells, an epic of ordered slices, strangler-style migrations across crates. Typical trigger: an agent-built codebase whose structure lagged its growth. Defines the audit report format, the audit→epic→slices pipeline, migration sequencing grammars, and the recurring refactor species.
---

# Refactoring Campaign (audit → epic → ordered slices)

Field origin: a workspace whose history runs 117 refactor commits to 86
feat — rebuilt in place more than extended, driven by one structured
audit, and ending with every recovered rule mechanically gated. A
refactor-heavy phase is healthy in agent-built code; an *unstructured*
one is churn.

## Phase 1 — the audit (a findings document, not vibes)

The audit is a committed report with a fixed shape:

- **A smell taxonomy with counts.** Name each smell class up front
  (adapter-holds-orchestration, composition-root-business-logic,
  domain-does-io, missing-use-case, duplicated-orchestration,
  god-struct) and tally findings per class. The discipline rule:
  **"If you can't name the smell, you don't have a finding."**
- **Per finding**: Smell class / Location with file:line / Current shape /
  Why it's a smell / a verbatim code quote / Proposed home / Ports
  needed / Risk / Couples-with (which other findings move together).
- **A canonical anchor**: point at the best in-tree exemplar of the
  target shape, by path and symbol, plus the commit that established the
  precedent. Agents pattern-match an anchor far more reliably than they
  follow prose criteria.
- **Zero-findings sections are recorded**, with the search that produced
  them ("every domain crate grepped for IO imports — zero hits; that's
  worth keeping"). Absence of findings is a result, not an omission.

Finding IDs (F001…) are a second coordinate system alongside work-item
IDs: commit subjects carry both (`refactor(x): ... (br-ejs1.14 slice 2,
F014)`), so `git log` is greppable by work unit *and* by why.

## Phase 2 — the epic

One epic per audit. Children cluster findings (one bead per root cause,
not per symptom), **ordered by dependency, not severity**. The epic body
carries the goal, non-goals, the target layout, and the ordered child
list with their finding IDs.

**Step 0, always**: if the canonical gate fails at HEAD (silently
accumulated debt — see quality-gates), green it in a dedicated `chore:`
commit titled "Step 0 for <epic>" before any child starts. The field
incident: clippy green "for the first time in the epic" only after a 20+
error sweep that pre-existing slices had quietly walked past.

## Phase 3 — slice grammar (strangler sequencing)

Three field-proven sequencing patterns for migrations that cross crates;
each slice is independently green with receipts (per-crate test run
named in the commit body):

- **(a) Land consumer-free → swap one consumer per commit → delete with
  the last swap.** Slice 1 adds the new code with zero call sites
  ("cargo grep shows no consumers yet" goes in the commit body). Each
  subsequent slice swaps exactly one consumer and deletes its inline
  duplicate. Nothing is ever half-swapped within a commit.
- **(b) Extract verbs, then the dispatcher, keep a legacy bridge.** N
  use-cases extracted one commit each; the old god-module survives as a
  thin bridge whose **file header names the bead that retires it** — a
  transitional state is fine, an unlabeled one is rot.
- **(c) Expand–migrate–contract** for data-shape changes: add the new
  write path → dual-write → switch readers → delete the old port. Each
  arrow is its own commit.

Course corrections mid-epic get **fractional slice numbers** (slice 1.5)
rather than a renumbered plan — and the correcting commit says plainly
what the plan got wrong ("the bead author didn't account for the dep
matrix; this slice walks the placement back BEFORE either adapter
consumes it, so no downstream churn").

**Mechanize every rule the campaign establishes, or it's decoration.**
Each recovered invariant ends the campaign as a gate (dep-matrix lint,
complexity gate, generated-architecture build) — otherwise the next
growth phase re-accumulates the same debt. The one convention in the
field repo with no gate (a glossary file) is the one already rotting.

## The refactor species (what actually recurs)

1. **Extract pure decision core from an IO loop** (functional core /
   imperative shell): the async driver becomes a thin shell around a
   pure `next_step(&Event) -> Step`; the full decision table is then
   tested synchronously — no runtime, no fake channels.
2. **Port splitting (ISP).** A 15-method port becomes 4 capability
   traits; consumers depend on the slice they exercise. The measurable
   outcome: **test fakes shrink from ~15 stubbed methods to 1–5** — fake
   size is the port-design metric.
3. **Capability supertraits with NO default methods.** When only some
   implementations support a capability, a default impl lets every
   backend type-check into the path and degrade silently at runtime; a
   no-default supertrait makes the wrong backend a compile error at the
   wiring site.
4. **God-struct → one-verb use cases**: one verb-shaped name, one narrow
   deps struct of `Arc<dyn Port>`, exactly one public method, no
   framework imports.
5. **Typed errors over strings**: kill `Result<_, String>` and
   Debug-formatted text in user-facing output — Debug is not a stable
   contract; the domain owns the noun, the use case owns the sentence.
6. **Dedupe test fakes and duplicated orchestration**; **sharpen weak
   assertions** (a `starts_with` becomes an exact pin).

Test deletion inside a campaign always states its coverage justification
in the commit ("the 735 LOC of old tests are deleted — the verb structs'
per-file unit tests cover the same surface"). Hunting test theater is
toolable: classify tests by tier and flag the ones that pass whether the
system under test works or not.

## Scope discipline inside a campaign

Small warts found while reading are fixed inline *if they sit at the
edges and touch no orchestration* — and the commit says which warts and
why inline was safe. Cross-cutting discoveries are **filed, not fixed**
("the fat-port wart is cross-cutting; tracked as <bead>"). Same
discovery-mandatory / opportunistic-fixing-forbidden line the rest of
the kit draws, applied at campaign speed.
