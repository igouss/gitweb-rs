#!/bin/sh
#
# capture-goldens.sh — freeze reference output from the original gitweb.perl.
#
# Run ONCE (and again only to refresh) to capture the byte-stable responses our
# format-stable endpoints must match. The committed goldens under ../goldens are
# what the `golden` conformance test diffs against; neither perl nor git runs at
# test time.
#
# What it does, all in a throwaway temp dir:
#   1. builds the deterministic corpus with `build-corpus` (the same gix builder
#      the test rebuilds, so object ids match),
#   2. assembles a real `gitweb.cgi` from the pinned `gitweb.perl` via git's own
#      `generate-gitweb-cgi.sh` and a synthesized build-options file,
#   3. drives that CGI over the corpus once per golden, saving the raw response.
#
# Requirements (install with Homebrew):
#   - a perl with the CGI module:   brew install perl cpanminus && cpanm CGI
#   - the git source tree checked out (for gitweb.perl + generate-gitweb-cgi.sh)
#
# Overridable via the environment:
#   GIT_SRC   git source tree            (default: ~/IdeaProjects/git)
#   PERL      perl that has CGI.pm       (default: brew perl, else /usr/bin/perl)
#   GIT       git binary                 (default: first git on PATH)

set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
crate_dir=$(CDPATH= cd -- "$here/.." && pwd)
repo_root=$(CDPATH= cd -- "$crate_dir/../.." && pwd)

GIT_SRC=${GIT_SRC:-"$HOME/IdeaProjects/git"}
gitweb_perl="$GIT_SRC/gitweb/gitweb.perl"
generate="$GIT_SRC/gitweb/generate-gitweb-cgi.sh"

# Prefer a perl that actually has CGI; the system perl 5.22+ no longer bundles it.
if [ -z "${PERL:-}" ]; then
	if [ -x /home/linuxbrew/.linuxbrew/bin/perl ]; then
		PERL=/home/linuxbrew/.linuxbrew/bin/perl
	else
		PERL=/usr/bin/perl
	fi
fi
GIT=${GIT:-$(command -v git)}
git_bindir=$(dirname -- "$GIT")

[ -f "$gitweb_perl" ] || { echo "no gitweb.perl at $gitweb_perl (set GIT_SRC)" >&2; exit 1; }
[ -f "$generate" ]    || { echo "no generate-gitweb-cgi.sh at $generate" >&2; exit 1; }
"$PERL" -MCGI -e1 2>/dev/null || { echo "$PERL lacks CGI.pm (cpanm CGI)" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
project_root="$work/projectroot"
mkdir -p "$project_root"

echo ">> building corpus" >&2
manifest="$work/manifest.txt"
( cd "$repo_root" && cargo run -q -p gitweb-parity --bin build-corpus -- "$project_root" ) >"$manifest"

echo ">> assembling gitweb.cgi from $gitweb_perl" >&2
cat >"$work/BUILD-OPTIONS" <<EOF
PERL_PATH='$PERL'
JSMIN=
CSSMIN=
GIT_BINDIR='$git_bindir'
GITWEB_CONFIG='gitweb_config.perl'
GITWEB_CONFIG_SYSTEM='/etc/gitweb.conf'
GITWEB_CONFIG_COMMON='/etc/gitweb-common.conf'
GITWEB_HOME_LINK_STR='projects'
GITWEB_SITENAME=
GITWEB_PROJECTROOT='/pub/git'
GITWEB_PROJECT_MAXDEPTH=2007
GITWEB_EXPORT_OK=
GITWEB_STRICT_EXPORT=
GITWEB_BASE_URL=
GITWEB_LIST=
GITWEB_HOMETEXT='indextext.html'
GITWEB_CSS='static/gitweb.css'
GITWEB_LOGO='static/git-logo.png'
GITWEB_FAVICON='static/git-favicon.png'
GITWEB_JS='static/gitweb.js'
GITWEB_SITE_HTML_HEAD_STRING=
GITWEB_SITE_HEADER=
GITWEB_SITE_FOOTER=
HIGHLIGHT_BIN='highlight'
EOF
echo "GIT_VERSION=gitweb-parity-corpus" >"$work/VERSION-FILE"
sh "$generate" "$work/BUILD-OPTIONS" "$work/VERSION-FILE" "$gitweb_perl" "$work/gitweb.cgi"

# Runtime config: point gitweb at the throwaway corpus. The snapshot feature is
# off by default; enable tgz + zip so the snapshot goldens can be captured (the
# two formats gix-archive produces natively; tbz2/txz wrap the same tar).
cat >"$work/gitweb.conf" <<EOF
our \$projectroot = "$project_root";
our \$projects_list = "";
\$feature{'snapshot'}{'default'} = ['tgz', 'zip'];
1;
EOF

# Drives one gitweb action and saves the raw response (headers + body) verbatim.
capture() {
	out="$crate_dir/goldens/$1"
	query="$2"
	mkdir -p "$(dirname -- "$out")"
	GITWEB_CONFIG="$work/gitweb.conf" \
	REQUEST_METHOD=GET GATEWAY_INTERFACE=CGI/1.1 QUERY_STRING="$query" \
		"$PERL" "$work/gitweb.cgi" >"$out"
	echo "   captured $1" >&2
}

# blob_plain is addressed by tree path (f=;hb=HEAD), not by raw hash: the
# by-hash path makes gitweb stamp volatile Expires/Date cache headers, which
# would make the committed golden non-reproducible. By-path serves the identical
# body with stable headers.
echo ">> capturing blob_plain goldens" >&2
while read -r name oid file; do
	[ -n "$name" ] || continue
	capture "blob_plain/$name" "p=repo.git;a=blob_plain;f=$file;hb=HEAD"
done <"$manifest"

# The RSS/Atom feeds are format-stable: capture both over the whole-branch log.
echo ">> capturing feed goldens" >&2
capture "feed/rss" "p=repo.git;a=rss"
capture "feed/atom" "p=repo.git;a=atom"

# The machine-readable project listings are format-stable. project_index is the
# text/plain "index.aux" (one "path owner" line per project, CGI-quoted); opml is
# the OPML XML outline. Both list every project under the root, so they are
# captured with no per-project query.
echo ">> capturing project listing goldens" >&2
capture "project_index/index" "a=project_index"
capture "opml/opml" "a=opml"

# commitdiff_plain is format-stable (git am parses it): a mailbox header, the
# comment, then the raw `git diff-tree -p` patch. It is addressed by hash (no
# by-path form), and is captured over the corpus root commit (a --root diff of
# every blob: creation headers, abbreviated index lines, the binary notice, the
# non-UTF-8 line, the missing-newline marker). The hash is the deterministic
# corpus HEAD, so the X-Git-Url self-link in the BODY and the filename in the
# Content-Disposition are reproducible; only the by-hash Expires/Date cache
# HEADERS are volatile, and the golden test compares the body, not those.
echo ">> capturing commitdiff_plain golden" >&2
head_hash=$("$GIT" --git-dir="$project_root/repo.git" rev-parse HEAD)
capture "commitdiff_plain/root" "p=repo.git;a=commitdiff_plain;h=$head_hash"

# The ancestor-named commitdiff_plain: the `anchored` branch's middle commit (a1)
# carries no tag of its own and is named only as an ancestor of the annotated
# v2.0 tag at the tip, so gitweb stamps `X-Git-Tag: v2.0~1` — the ~N
# ancestor-distance form the old distance-zero subset omitted (the annotated ^0
# stripped before the suffix). By-hash like the root, so only the cache headers
# are volatile and the golden test compares the body.
anchored_hash=$("$GIT" --git-dir="$project_root/repo.git" rev-parse anchored~1)
capture "commitdiff_plain/anchored" "p=repo.git;a=commitdiff_plain;h=$anchored_hash"

# blobdiff_plain is format-stable: the bare `git diff-tree -r -p` of ONE path
# (abbreviated index, no mailbox header), prefixed by an X-Git-Url self-link
# line. It is captured over the two-commit `diffs` branch (NOT HEAD, so the
# single-commit goldens above are untouched), one golden per single-file diff
# surface: a text modification, a pure mode change, a binary modification, and an
# exact rename. The query params are in @cgi_param_mapping order (p, a, f, fp, hb,
# hpb) so the X-Git-Url our endpoint emits matches byte for byte. The file paths
# mirror Corpus::DIFF_* in ../src/corpus.rs. Like commitdiff_plain it is by-hash,
# so only the Expires/Date cache headers are volatile; the golden compares body.
echo ">> capturing blobdiff_plain goldens" >&2
diff_head=$("$GIT" --git-dir="$project_root/repo.git" rev-parse diffs)
diff_parent=$("$GIT" --git-dir="$project_root/repo.git" rev-parse diffs^)
bases="hb=$diff_head;hpb=$diff_parent"
capture "blobdiff_plain/text"   "p=repo.git;a=blobdiff_plain;f=a.txt;$bases"
capture "blobdiff_plain/mode"   "p=repo.git;a=blobdiff_plain;f=mode.sh;$bases"
capture "blobdiff_plain/binary" "p=repo.git;a=blobdiff_plain;f=bin.dat;$bases"
capture "blobdiff_plain/rename" "p=repo.git;a=blobdiff_plain;f=new.txt;fp=old.txt;$bases"

# patch is format-stable: gitweb streams `git format-patch --stdout` verbatim, a
# `git am`-able mailbox (mail header + diffstat + diff + `-- `/git-version
# signature). git's binary patch body is its own zlib output, which the gix-only
# no-unsafe port cannot reproduce, so it is captured over the TEXT-ONLY `texts`
# branch root commit (a --root create of two text files: the mailbox header, the
# multi-file diffstat with its column alignment and `create mode` lines, the
# create diff, the signature). By-hash like commitdiff_plain, so only the cache
# headers are volatile; the golden compares the body.
# patches is the range form: gitweb streams `git format-patch -n --root <tip>`,
# the most recent commits oldest-first, each Subject numbered `[PATCH i/N]`. It is
# captured over the whole two-commit `texts` branch (tip), so the stream is the
# `[PATCH 1/2]` root create plus the `[PATCH 2/2]` modify-and-add. The single
# `patch` form is captured over the branch ROOT (texts~1) — the same root commit,
# so its golden is byte-identical to before the second commit was stacked on.
echo ">> capturing patch / patches goldens" >&2
texts_head=$("$GIT" --git-dir="$project_root/repo.git" rev-parse texts)
texts_root=$("$GIT" --git-dir="$project_root/repo.git" rev-parse texts~1)
capture "patch/single"   "p=repo.git;a=patch;h=$texts_root"
capture "patches/range"  "p=repo.git;a=patches;h=$texts_head"

# The binary `patch` golden is captured over the `binmix` branch head (a text +
# binary modify, plus a binary delta-wins modify and a binary create). gitweb
# streams `git format-patch` verbatim, which embeds each binary file as a base85
# `GIT binary patch` (git's own zlib output plus a literal-vs-delta heuristic) with
# a full `index` — all reproduced byte-for-byte by model::binary_patch (af6), so
# the golden is asserted as full equality. By-hash, so only cache headers vary.
echo ">> capturing binary patch golden" >&2
binmix_head=$("$GIT" --git-dir="$project_root/repo.git" rev-parse binmix)
capture "patch/binary"   "p=repo.git;a=patch;h=$binmix_head"

# The feed body carries a <generator> version composite ($version/$git_version).
# $version is pinned via VERSION-FILE above; $git_version is the capturing git's
# version, which is not byte-stable across machines and is not part of the feed
# FORMAT. Record the exact composite so the golden test injects the same value
# and the rest of the body is still compared to the byte.
git_version=$("$GIT" --version | sed -e 's/^git version //')
mkdir -p "$crate_dir/goldens/feed"
printf '%s' "gitweb-parity-corpus/$git_version" >"$crate_dir/goldens/feed/generator"

# The patch mail's `-- ` signature carries the capturing git's version, which is
# not byte-stable across machines and is not part of the format. Record it the
# way the feed records its generator version, so the golden test stamps the same
# value on its signature and the rest of the mail is compared to the byte.
mkdir -p "$crate_dir/goldens/patch"
printf '%s' "$git_version" >"$crate_dir/goldens/patch/version"

# snapshot streams `git archive --prefix=<name>/ <hash>` (tar piped through
# `gzip -n`) as a downloadable archive. It is captured over HEAD for the two
# formats gix-archive produces natively, tgz and zip. ONLY the headers are
# byte-reproducible — the archive BODY cannot be byte-identical to `git archive`
# (gix-archive emits no pax global header / no directory entries / mode 0644 /
# different padding for tar, bakes the mtime into the gzip header and uses a
# different deflate for tgz, and omits the EOCD commit-id comment while adding a
# per-entry UT field for zip). So the snapshot golden test asserts the headers
# only, not the body; these goldens record gitweb's real bytes for the divergence
# on the record. snapshot adds no volatile cache headers, so the captured
# Content-Type / Content-Disposition / Last-Modified are all stable.
echo ">> capturing snapshot goldens" >&2
snap_hash=$("$GIT" --git-dir="$project_root/repo.git" rev-parse HEAD)
capture "snapshot/tgz" "p=repo.git;a=snapshot;h=$snap_hash;sf=tgz"
capture "snapshot/zip" "p=repo.git;a=snapshot;h=$snap_hash;sf=zip"

echo ">> done. goldens under $crate_dir/goldens" >&2
