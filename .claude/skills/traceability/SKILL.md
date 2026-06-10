---
name: traceability
description: Use when auditing the link between requirements and tests, before changing or deleting a requirement, when deciding whether a test is allowed to exist, or when asked "what verifies X" / "why does this test exist". Convention-based; the audit script is Babashka, ad-hoc searches use ripgrep.
---

# Traceability

The invariant: **every requirement is verified by at least one test, and every
test is justified by at least one requirement.** Both directions, always.
This is what lets a future session change a requirement and know exactly what
else must change — instead of archaeology.

## Conventions (all enforcement is grep)

- Requirement IDs appear in exactly two kinds of places:
  1. The spec file `specs/REQ-<AREA>-<NNN>.md` (its home).
  2. A comment line directly above a test attribute:
     `// REQ-SALE-003` (multiple IDs comma-separated on one line if one test
     genuinely verifies several requirements — rare; prefer one).
- Never put REQ IDs in implementation code. The implementation satisfies the
  spec; it does not cite it. (ADRs cite specs; code does not.)

## The audit

Run `templates/trace-audit.bb` (copy it to `scripts/` in the project).
It checks both directions:

- **Orphan requirement**: a `specs/REQ-*.md` whose ID appears in no test.
  → Incomplete work. Either write the test (verification-ratchet) or, if the
  requirement is dead, mark it `superseded` — never silently delete.
- **Orphan test**: a `// REQ-` comment whose ID has no spec file, or a test
  function with no REQ comment at all.
  → The test is pinning behavior nobody asked for. Either write the missing
  requirement (spec-authoring) or report the test to the human as a deletion
  candidate. Do not delete it yourself — deletion weakens checks (CLAUDE.md).

Run the audit: before declaring any feature done, after any change to `specs/`,
and when starting work in an unfamiliar area of the repo.

## Changing a requirement

1. `rg -n 'REQ-SALE-003' specs/ src/ tests/` — full blast radius first.
2. Spec + all referencing tests change in ONE commit. A commit that changes
   the spec but not the tests (or vice versa) is the rot this system exists
   to prevent.
3. If a test must be deleted because its requirement is superseded: that is a
   check-weakening change → separate commit, human-approved, commit message
   cites the superseding REQ ID.

## Oracle-mode traceability (ports/rewrites)

Three field-proven conventions, all naming-based rather than annotation-based:

- **Commit subject carries the reference symbol in parens**:
  `search: grep tree content (git_search_files)`. Every commit maps back
  to the reference function it ports — commit ↔ spec-source, greppable
  from either end.
- **Structure IS the trace**: feature-file path mirrors the capability
  mirrors the commit scope mirrors the module. When the naming convention
  holds, you don't annotate — a 29k-LOC port needed exactly 5 work-item
  IDs in source code.
- **Work-item IDs appear in code ONLY to mark known divergences** — a doc
  comment on the code site saying "diverges from reference here, tracked
  by <bead>". Sprinkling tracker IDs through implementation code is noise;
  marking the spots where behavior deliberately differs is signal.

## Answering "why does this code exist"

Path: code → test that forces it (delete-and-see or cargo-mutants tells you)
→ REQ comment on the test → spec file → ADR links in the spec. If the chain
breaks anywhere, you have found either dead code or an orphan — report it.
