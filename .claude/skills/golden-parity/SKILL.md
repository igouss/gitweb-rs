---
name: golden-parity
description: Use when building or extending a golden-master/differential conformance harness against a reference implementation — ports, rewrites, legacy capture. Defines the two-tier oracle, determinism engineering at capture time, the divergence-fencing pattern, and the golden ratchet. testing-strategy says WHERE goldens belong; this skill says HOW to build a harness that never flickers and never lies.
---

# Golden Parity Harness

The mechanical form of "the original is the spec." Field-proven shape: a
gitweb port verified 9 endpoint families byte-for-byte against gitweb.perl
with neither perl nor git running at test time, and the byte-exact goldens
surfaced two latent bugs that `contains`-style conformance checks had
already passed over. Every rule below was paid for.

## The two-tier oracle — decide it on day one

Before building features, split every output of the system into two tiers
and record the split in the parity spec (it is the contract every capability
references):

- **Byte-exact tier**: format-stable, machine-parsed outputs — plain-text
  endpoints, patch/mail formats, feeds, indexes, machine-readable listings.
  Verified by golden diff, byte-for-byte.
- **Behavioral tier**: presentation you are deliberately modernizing (HTML,
  TUI layout). Verified by scenario tests on extracted behavior, never by
  markup diff.

A capability that doesn't know which tier its output is in cannot be
specified. The same day-one spec promotes the **edge-case matrix** to
acceptance criteria — merge/multi-parent, rename/copy, mode changes,
symlinks, submodules, binary content, empty/unborn state, non-UTF8,
pagination boundaries, invalid object — with frequency evidence from the
reference source ("~118 merge sites, ~120 rename sites") so nobody can
argue they're exotic.

## Harness mechanics (non-negotiable)

- **One deterministic corpus builder, shared.** The same fixture-builder
  code runs at capture time (driving the reference) and at test time
  (driving your implementation). Pinned identities, pinned timestamps →
  identical object ids on every machine → goldens never flicker.
- **Capture once, commit, never run the reference at test time.** The
  capture script assembles the reference tool, drives it over the corpus,
  and commits raw responses under `goldens/` with `.gitattributes
  goldens/** binary` so git never munges line endings.
- **Missing golden = panic, not skip.** A test naming a golden that was
  never captured is a broken test. Fail loudly; a skip is a silent hole.
- **Parse binary-safe.** Split header block from body at the first
  CRLF-CRLF; keep the body as raw bytes. Prove it over a NUL blob and a
  non-UTF8 blob before trusting it.
- **Wiring a new endpoint is a 3-step recipe**, documented in the harness
  README: extend corpus → add capture line + re-capture → add the feature
  asserting body (+ headers where stable).

## Determinism engineering (hunt every volatility source)

Each class of volatility has a named technique; pick by class, don't improvise:

| Volatility | Technique |
|---|---|
| Volatile cache headers on some capture paths | Choose the capture path that doesn't emit them (capture by filename, not by hash) |
| Environment-dependent metadata (user names, host config) | Pin it in the corpus so both capture and test read the same value back |
| Genuinely volatile tokens (tool version strings) | **Sidecar file**: capture writes the token next to the golden; the test reads and injects it, the rest stays byte-exact. Re-capture regenerates both together |
| Clock-relative values (Expires, ages) | Exclude from the golden compare; assert presence/format behaviorally at the boundary |
| Unordered output (directory scans) | Sort before asserting — read order is unspecified |
| Order-dependent fixtures | Strictly-increasing pinned timestamps |

## Fixture evolution without breaking existing goldens

- New multi-object fixtures go on a **branch that is not HEAD**, so every
  prior golden stays byte-identical.
- Capture scripts pin to **stable revisions, never moving symbolic names** —
  a capture of `branchname` silently re-points when the branch grows; pin
  the root (`branch~N`) explicitly.
- After ANY re-capture: `git diff goldens/` and account for every changed
  byte. Unexplained churn means a volatility source you haven't found yet.
- Re-run the WHOLE golden suite after fixture changes, not just the new
  endpoint — fixture growth can shift derived values (abbreviation lengths,
  pagination) in goldens you didn't touch.

## Byte-exact beats contains — and semantics beat cruft

- At least one **byte-exact pin per output format** is mandatory. Substring
  assertions pass over systematic whole-output corruption (doubled
  newlines on every line survived a `contains`-only suite).
- But compare the SEMANTIC value, not the reference's transport accidents:
  strip a legacy CGI library's header-casing and charset quirks before
  comparing rather than cargo-culting them into a modern server. The
  decision of what counts as cruft is recorded (ADR or parity spec), so a
  diff there is diagnosable as decision, not bug.
- When output must embed reference-faithful serializations (legacy URL
  formats, parameter ordering, legacy escaping), isolate the legacy
  serializer from the modern one. Two builders, two fidelity regimes —
  never contaminate the modern path to satisfy a golden.

## Documented-divergence goldens (fencing the impossible)

When byte-exactness is impossible under your constraints (e.g. the
reference's output embeds a compression artifact you cannot legally
reproduce), the divergence is never silent. The pattern:

1. Capture TWO references: the real one, and the reference forced into
   your target form (a flag, a config).
2. Assert your output == the forced reference **byte-for-byte**.
3. Assert the byte-exact frame *around* the divergent region matches the
   real reference, plus marker assertions on both sides of the difference
   (theirs contains X, yours contains the documented notice).
4. File the bead that would remove the divergence, link it from the code
   site, and frame the divergence as **tracked/removable, not impossible** —
   "impossible" claims rot (one such claim blamed the wrong component and
   was overturned by a later session that reversed the reference's actual
   bytes).

## The golden ratchet

- Goldens accumulate; they are never deleted and never weakened.
  Strengthening is encouraged and normal: a body-only golden later grows
  differential header verification.
- Re-blessing a golden (regenerating to make a failing test pass) redefines
  correct — it is a check-change: separate human-approved commit,
  hook-protected directory, same regime as proptest-regressions.
- A failing golden has exactly three diagnoses, in order: (1) your bug,
  (2) a volatility source — fix the capture, not the assertion,
  (3) a deliberate divergence missing from the divergence list — file it
  properly. "The golden is too strict" is not a diagnosis.
