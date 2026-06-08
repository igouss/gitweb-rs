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

# Runtime config: point gitweb at the throwaway corpus.
cat >"$work/gitweb.conf" <<EOF
our \$projectroot = "$project_root";
our \$projects_list = "";
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

echo ">> done. goldens under $crate_dir/goldens" >&2
