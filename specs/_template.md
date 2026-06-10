---
id: REQ-AREA-NNN
status: draft          # draft | in-progress | accepted | superseded by REQ-...
                       # in-progress = slice under construction; requires a
                       # WIP(this-id) acceptance test (slice-workflow skill)
revision: 1
---

# REQ-AREA-NNN — <FDD feature name: action the result of an object>

## Requirement (EARS)

When <trigger>, the system shall <response>.

<!-- One statement. Exact units, ranges, boundary inclusivity. -->

## Anchor scenarios

```gherkin
Feature: <FDD feature name>

  Scenario: <canonical happy path>
    Given <concrete precondition>
    When <trigger with concrete values>
    Then <exact expected outcome>

  Scenario: <boundary or unwanted behavior>
    Given <...>
    When <...>
    Then <...>
```

<!-- Scenarios. Tabular behavior → one Scenario Outline + Examples table. -->

## Candidate laws

- <invariant, e.g. "the total is never greater than the subtotal">
- <identity/round-trip, e.g. "a 0% discount leaves the total unchanged">

## Related ADRs

- <docs/adr/NNNN-... or "none">

## History

- rev 1: created
