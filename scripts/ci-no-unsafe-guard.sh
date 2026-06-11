#!/usr/bin/env bash
#
# CI guard: prove the workspace's no-unsafe policy stays intact.
#
# The policy is enforced at compile time by `[lints.rust] unsafe_code = "forbid"`
# in every workspace member's Cargo.toml. `cargo build`/`cargo test` already fail
# if any crate carrying that lint contains `unsafe`. The ONE thing the compiler
# can't catch is a NEW crate being added to the workspace WITHOUT the forbid line
# — that crate would silently be allowed to use unsafe. This guard closes that
# gap: it asserts that every workspace member declares the forbid lint.
#
# We deliberately do NOT grep source for the `unsafe` keyword: that produces
# false positives on the word in comments/strings (this repo has ~10 such benign
# hits) and false negatives via `r#unsafe`/macros. Asserting the declarative
# lint is present, and letting rustc enforce it, is both stronger and exact.
#
# Exit 0 = every member forbids unsafe. Non-zero = at least one member is missing
# the lint (printed), or the workspace manifest couldn't be parsed.
set -euo pipefail

cd "$(dirname "$0")/.."

# Authoritative list of workspace members, straight from cargo (handles globs,
# default-members, exclude — no hand-maintained list to drift out of sync).
mapfile -t manifests < <(
  cargo metadata --no-deps --format-version 1 \
    | python3 -c '
import json, sys
md = json.load(sys.stdin)
ws = set(md["workspace_members"])
for pkg in md["packages"]:
    if pkg["id"] in ws:
        print(pkg["manifest_path"])
'
)

if [ "${#manifests[@]}" -eq 0 ]; then
  echo "no-unsafe guard: cargo metadata returned zero workspace members — refusing to pass" >&2
  exit 2
fi

missing=0
for manifest in "${manifests[@]}"; do
  # The lint must be the forbid level. `allow`/`warn`/`deny` are all weaker and
  # must fail the guard (deny can be downgraded per-call; forbid cannot).
  if ! grep -Eq '^[[:space:]]*unsafe_code[[:space:]]*=[[:space:]]*"forbid"' "$manifest"; then
    echo "no-unsafe guard: MISSING 'unsafe_code = \"forbid\"' in $manifest" >&2
    missing=1
  fi
done

if [ "$missing" -ne 0 ]; then
  echo "no-unsafe guard: FAILED — every workspace crate must declare [lints.rust] unsafe_code = \"forbid\"" >&2
  exit 1
fi

echo "no-unsafe guard: OK — all ${#manifests[@]} workspace crates forbid unsafe."
