#!/usr/bin/env bash
# Build @pierre/diffs from source and vendor its dist/ bundle into
# static/vendor/pierre/, where the gitweb-rs binary serves it (the diff host
# page's boot module imports it from /static/vendor/pierre/).
#
# This is a one-time / refresh step — NOT part of `cargo build` or `cargo test`.
# The committed code never depends on the bundle: when it is absent the binary
# still serves every route and the diff view degrades to its no-JavaScript
# raw-diff link. End users only need the bundle to see diffs rendered inline,
# and never need `bun` installed — only whoever refreshes the vendored bundle.
#
# Requires: bun, git. Re-run to bump the pinned pierre revision.
set -euo pipefail

PIERRE_REPO="https://github.com/pierrecomputer/pierre.git"
# Pinned for reproducibility. @pierre/diffs 1.2.1. Bump deliberately.
PIERRE_COMMIT="96e4623c6471cef003c69fb5b290a308e3f21432"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
VENDOR_DIR="$REPO_DIR/static/vendor/pierre"

command -v bun >/dev/null 2>&1 || {
  echo "vendor-pierre: 'bun' not found on PATH — install from https://bun.sh" >&2
  exit 1
}

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
SRC="$WORK_DIR/pierre"

echo "vendor-pierre: cloning pierre @ ${PIERRE_COMMIT:0:12} ..."
git clone --quiet "$PIERRE_REPO" "$SRC"
git -C "$SRC" checkout --quiet "$PIERRE_COMMIT"

echo "vendor-pierre: bun install ..."
(cd "$SRC" && bun install)

echo "vendor-pierre: building @pierre/diffs (tsdown) ..."
(cd "$SRC/packages/diffs" && bun run build)

DIST="$SRC/packages/diffs/dist"
[ -f "$DIST/index.js" ] || {
  echo "vendor-pierre: build produced no dist/index.js" >&2
  exit 1
}

# tsdown builds a library: it leaves npm deps (shiki, diff, @shikijs/*, ...) as
# bare import specifiers, which a browser's ES module loader cannot resolve.
# Re-bundle the built entry points into self-contained ESM with every dependency
# inlined. `--splitting` dedupes the shared Shiki code across the two entries
# into chunk-*.js files.
echo "vendor-pierre: bundling self-contained browser ESM ..."
rm -rf "$VENDOR_DIR"
mkdir -p "$VENDOR_DIR"
(cd "$DIST" && bun build index.js components/web-components.js \
  --target=browser --format=esm --splitting --minify --outdir="$VENDOR_DIR")

[ -f "$VENDOR_DIR/index.js" ] || {
  echo "vendor-pierre: bundling produced no index.js" >&2
  exit 1
}
cp "$SRC/packages/diffs/LICENSE.md" "$VENDOR_DIR/LICENSE.md" 2>/dev/null \
  || cp "$SRC/LICENSE"* "$VENDOR_DIR/" 2>/dev/null || true

echo "vendor-pierre: done — pinned commit $PIERRE_COMMIT"
echo "vendor-pierre: bundle size: $(du -sh "$VENDOR_DIR" | cut -f1)"
echo "vendor-pierre: served at /static/vendor/pierre/ (set GITWEB_STATIC_DIR if"
echo "               the binary's working directory is not the repo root)"
