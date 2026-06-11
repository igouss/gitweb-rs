// gitweb-rs client module: route links through their JavaScript-driven actions.
//
// Ports the BEHAVIOUR of git's gitweb static/js/javascript-detection.js to a
// modern, framework-free ES module. gitweb's server cannot know whether the
// client runs JavaScript, so a link to a heavy server action (e.g. blame) points
// at the plain action by default; when JS is present this module rewrites every
// in-page link to carry `js=1`, which the server honours by serving the
// JavaScript-driven variant of the action (currently blame_incremental for
// blame). Links that already carry `js=0`/`js=1` are left alone.
//
// MANUAL CHECK (cannot be asserted headless): load any page with the
// javascript-actions feature on; every link gains a `js=1` query parameter, and
// following e.g. a blame link lands on the incremental blame view rather than the
// server-rendered one. A link already carrying `js=…` is unchanged.

// Matches a link that already declares its js preference (with optional anchor).
const JS_PARAM_RE = /[;?]js=[01](#.*)?$/;

/** Append `js=1` to a link's href, before any `#anchor`, unless already set. */
function addJsParam(link) {
  const href = link.getAttribute("href");
  if (href === null || JS_PARAM_RE.test(href)) return;
  const separator = href.includes("?") ? ";" : "?";
  link.setAttribute("href", href.replace(/(#|$)/, separator + "js=1$1"));
}

function init() {
  for (const link of document.getElementsByTagName("a")) {
    addJsParam(link);
  }
}

// type="module" defers until the DOM is parsed, so all links already exist.
init();
