# Vendored browser assets

This directory holds large, third-party browser bundles that the gitweb-rs
binary serves at runtime but does **not** compile in. They are built on demand
and **git-ignored** — only this README is committed.

## `pierre/` — the `@pierre/diffs` client viewer

The diff host page (gitweb's modernized `commitdiff` HTML view) renders diffs in
the browser with [`@pierre/diffs`](https://github.com/pierrecomputer/pierre), a
Shiki-based web component that parses a standard unified diff client-side and
renders a themed, syntax-highlighted, light/dark diff view (~280 languages). This
replaces gitweb's server-side `format_diff_line` colouriser — zero
reimplementation — and also covers the diff-body half of the syntax-highlighting
feature.

The bundle is ~11 MB (mostly Shiki grammars), so it is not committed. Build it:

```sh
scripts/vendor-pierre.sh        # requires bun + git; writes static/vendor/pierre/
```

The binary serves whatever is under `static/vendor/pierre/` at
`/static/vendor/pierre/`. The static directory defaults to `./static` relative to
the working directory; set `GITWEB_STATIC_DIR` to point elsewhere.

### When the bundle is absent

Nothing breaks. The boot module's import 404s, its `catch` leaves the diff host
page's no-JavaScript fallback (a link straight to the raw unified diff) in place,
and every other route serves normally. You only need the bundle to see diffs
rendered *inline*; the raw diff is always reachable. The bundle is therefore a
deployment-time asset, never a build- or test-time dependency.

### Refreshing / bumping the pinned version

`scripts/vendor-pierre.sh` pins an exact upstream commit for reproducibility.
Edit `PIERRE_COMMIT` in that script and re-run to bump it.

## Manual checks (client-side rendering)

The host page markup, the boot module asset, and the graceful-absent behaviour
are verified automatically (render `diff_host.feature`, web `diff_viewer.feature`,
app `server.feature`). The *visual* rendering of a diff in the browser is the
viewer's job and is verified by hand, since the bundle is not present at test
time. With the bundle built and the server running, open a diff host page and
confirm each parity edge case renders (not just the happy path):

- [ ] a plain single-file modification — additions / deletions / context coloured
- [ ] a multi-file diff — each file in its own `<diffs-container>`
- [ ] a merge / combined (`--cc`) diff, including an octopus merge
- [ ] a rename and a copy (with similarity index)
- [ ] a file-mode change (e.g. `100644` → `100755`), and a symlink (mode `120000`)
- [ ] a binary file — shown as a notice, not garbage
- [ ] an empty diff — the "No changes to show." state, not a blank page
- [ ] a non-UTF-8 / fallback-encoded file — text legible, no mojibake
- [ ] light vs. dark — toggle the OS appearance; the diff follows it
- [ ] JavaScript disabled, or the bundle absent — the no-JS fallback link to the
      raw unified diff is present and works
