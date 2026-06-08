// diff-viewer.js — render a commit's diff in the browser with @pierre/diffs.
//
// gitweb's git_commitdiff colourises the diff on the server with
// format_diff_line. We modernize that: the host page (see render's diff_host
// module) ships an empty #diff-root carrying a `data-diff-url` attribute — the
// URL of a *clean* unified diff (git's `diff --git` patch text, what the domain
// Patch::render emits). This module fetches that URL, turns it into structured
// file diffs with pierre's parsePatchFiles, and renders them with CodeView.
//
// The viewer is loaded from the vendored bundle under /static/vendor/pierre/;
// when that bundle is absent (it is git-ignored and built on demand by
// scripts/vendor-pierre.sh) the import fails and the catch() below leaves the
// no-JavaScript fallback's raw-diff link in place. The worker pool is left
// disabled, so syntax highlighting runs on the main thread and no separate
// worker bundle is served.

import { CodeView, DEFAULT_THEMES, parsePatchFiles } from "/static/vendor/pierre/index.js";
// Side-effect import: registers the <diffs-container> custom element CodeView
// creates per file (a shadow root carrying pierre's bundled, theme-aware CSS).
import "/static/vendor/pierre/components/web-components.js";

const root = document.getElementById("diff-root");

/** Replace the container with a single status / error line. */
function message(text) {
  const line = document.createElement("p");
  line.className = "status";
  line.textContent = text;
  root.replaceChildren(line);
}

async function main() {
  const diffUrl = root?.dataset.diffUrl;
  if (!diffUrl) {
    throw new Error("diff-root is missing its data-diff-url");
  }

  const response = await fetch(diffUrl);
  if (!response.ok) {
    throw new Error(`${diffUrl} responded ${response.status}`);
  }

  const patchText = await response.text();
  if (patchText.trim() === "") {
    message("No changes to show.");
    return;
  }

  // One unified diff is a single patch holding many file diffs; flatten every
  // file into a CodeView "diff" item. CodeView dedupes items by `id`, so each
  // needs a distinct one.
  const items = parsePatchFiles(patchText)
    .flatMap((patch) => patch.files ?? [])
    .map((fileDiff, index) => ({ type: "diff", id: `file-${index}`, fileDiff }));
  if (items.length === 0) {
    message("No file changes found in the diff.");
    return;
  }

  root.replaceChildren();
  const view = new CodeView({ theme: DEFAULT_THEMES }, undefined, true);
  view.setup(root);
  view.setItems(items);
  view.render(true);
}

main().catch((error) => {
  message(`Failed to render diff: ${error?.message ?? error}`);
  console.error(error);
});
