#!/usr/bin/env bash
# Regenerate the golden delta bytes pinned in features/domain/delta.feature.
#
# Each delta is captured from real git's diff-delta.c via its `test-tool delta`
# helper, so the .feature scenarios assert byte-for-byte what git's create_delta()
# emits — no git binary is needed at test time, only when re-blessing goldens here.
#
# Prerequisite: build git's test-tool from a git checkout:
#     make -C /path/to/git t/helper/test-tool
# Then run (path via $1 or $GIT_TEST_TOOL):
#     ./delta_goldens_capture.sh /path/to/git/t/helper/test-tool
#
# The procedural inputs (ramp / valued / appended) mirror the Gherkin step
# vocabulary exactly: "ramp N" == bytes(i & 0xff for i in range(N)); "N valued V"
# == N copies of byte V. Keep this script and the steps in lockstep.
set -euo pipefail
TT="${1:-${GIT_TEST_TOOL:-$HOME/IdeaProjects/git/t/helper/test-tool}}"
[ -x "$TT" ] || { echo "error: test-tool not found/executable at: $TT" >&2; exit 2; }
D="$(mktemp -d)"; trap 'rm -rf "$D"' EXIT
hex() { od -An -v -tx1 "$1" | tr -s ' ' | sed 's/^ //;s/ $//' | tr '\n' ' ' | tr -s ' '; }
cap() { # name src trg
  if "$TT" delta -d "$2" "$3" "$D/out" 2>/dev/null; then
    printf '%-26s %s\n' "$1" "$(hex "$D/out")"
  else
    printf '%-26s (no delta — NULL)\n' "$1"
  fi
}

printf 'aaaaaaaaaaaaaaaa' > "$D/a"; printf 'bbbb' > "$D/b"
cap "insert-short" "$D/a" "$D/b"

printf 'aaaaaaaaaaaaaaaa' > "$D/a"
python3 -c 'import sys;sys.stdout.buffer.write(bytes(i&0xff for i in range(200)))' > "$D/b"
cap "insert-flush-0x7f" "$D/a" "$D/b"

printf 'The quick brown fox jumps over!!' > "$D/a"; cp "$D/a" "$D/b"
cap "whole-copy" "$D/a" "$D/b"

printf 'The quick brown fox jumps over the lazy dog.'  > "$D/a"
printf 'The quick brown fox JUMPED over the lazy dog.' > "$D/b"
cap "copy-then-insert" "$D/a" "$D/b"

printf 'PREFIX-the common suffix block long enough to match' > "$D/a"
printf 'X-the common suffix block long enough to match'      > "$D/b"
cap "nonzero-offset-backext" "$D/a" "$D/b"

printf 'pqthe_quick_brown_fox_jumped_over'    > "$D/a"
printf 'ABCpqthe_quick_brown_fox_jumped_over' > "$D/b"
cap "partial-backext" "$D/a" "$D/b"

# two distinct 40-byte common runs separated by an inserted gap: forces
# copy / insert / copy, where the second copy depends on the rolling fingerprint
# being re-primed after the first copy (and back-extends one byte over the gap).
A='abcdefghijklmnopqrstuvwxyz0123456789ABCD'
B='zyxwvutsrqponmlkjihgfedcba9876543210WXYZ'
printf '%s%s'   "$A"       "$B" > "$D/a"
printf '%s%s%s' "$A" 'MID' "$B" > "$D/b"
cap "two-copies-gap" "$D/a" "$D/b"

python3 -c 'import sys;sys.stdout.buffer.write(bytes(i&0xff for i in range(65541)))' > "$D/a"
cp "$D/a" "$D/b"
cap "copy-split-0x10000" "$D/a" "$D/b"

python3 -c 'import sys;sys.stdout.buffer.write(b"\xff"*320 + bytes(i&0xff for i in range(400)))' > "$D/a"
python3 -c 'import sys;sys.stdout.buffer.write(bytes(i&0xff for i in range(336)))' > "$D/b"
cap "two-byte-off-and-size" "$D/a" "$D/b"

: > "$D/a"; printf 'abc' > "$D/b"; cap "empty-source" "$D/a" "$D/b"
printf 'abc' > "$D/a"; : > "$D/b"; cap "empty-target" "$D/a" "$D/b"
