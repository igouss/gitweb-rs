// gitweb-rs client module: progressively fill a blame view from blame_data.
//
// Ports the BEHAVIOUR of git's gitweb static/js/blame_incremental.js to a
// modern, framework-free ES module. The blame_incremental page first serves the
// file's lines as a table of empty blame rows, then this module streams
// `git blame --incremental` output from the blame_data endpoint and fills in each
// line's commit, author, date and links as the data arrives — updating a progress
// bar — and finally re-colours the rows into commit groups (zebra striping).
//
// CONSUMER NOTE: the blame_data endpoint is bead 950 (downstream of this asset).
// This module is the static asset 950 wires in via `startBlame(dataUrl, baseUrl)`;
// it does NOT assume the endpoint exists. The boundary serves this file at a
// stable URL the same way as the other client modules.
//
// MANUAL CHECK (cannot be asserted headless): with blame_data available, open a
// blame_incremental view; rows fill in progressively, the progress bar advances
// to 100%, adjacent lines from the same commit merge into rowspan groups with
// alternating light/dark colouring, and each sha1 / line-number link resolves.

const SHA1_RE = /^([0-9a-f]{40}) (\d+) (\d+) (\d+)/;
const INFO_RE = /^([a-z-]+) ?(.*)/;
const END_RE = /^END ?([^ ]*) ?(.*)/;
const COLOR_RE = /\bcolor(\d+)\b/;

/** Unquote a git C-quoted path ("…\t…") to its literal form. */
function unquote(value) {
  if (value.length < 2 || value[0] !== '"') return value;
  return value
    .slice(1, -1)
    .replace(/\\([0-7]{1,3})/g, (_, oct) => String.fromCharCode(parseInt(oct, 8)))
    .replace(/\\n/g, "\n")
    .replace(/\\t/g, "\t")
    .replace(/\\(.)/g, "$1");
}

/** Two-digit zero-padded string. */
function pad2(value) {
  return String(value).padStart(2, "0");
}

/** ISO-like local date for an epoch + numeric '(+|-)HHMM' tz, e.g. for tooltips. */
function formatDateISO(epochSeconds, tz) {
  const match = /^([+-])(\d\d)(\d\d)$/.exec(tz) || ["", "+", "00", "00"];
  const sign = match[1] === "-" ? -1 : 1;
  const offset = sign * (parseInt(match[2], 10) * 60 + parseInt(match[3], 10)) * 60;
  const date = new Date(1000 * (epochSeconds + offset));
  return (
    date.getUTCFullYear() + "-" + pad2(date.getUTCMonth() + 1) + "-" + pad2(date.getUTCDate()) +
    " " + pad2(date.getUTCHours()) + ":" + pad2(date.getUTCMinutes()) + ":" + pad2(date.getUTCSeconds()) +
    " " + tz
  );
}

/** Per-blame state: progress, the partial-line buffer, and the 3-colour chooser. */
class BlameState {
  constructor(projectUrl) {
    this.projectUrl = projectUrl;
    this.commits = new Map();
    this.current = { sha1: null };
    this.group = {};
    this.blamedLines = 0;
    this.totalLines = this.countLines();
    this.colorFreq = [0, 0, 0];
    this.startedAt = Date.now();
  }

  countLines() {
    const table = document.getElementById("blame_table") || document.getElementsByTagName("table")[0];
    return table ? table.getElementsByTagName("tr").length - 1 : 0;
  }

  /** The N from a row's `colorN` class, or null. */
  static colorOf(tr) {
    const match = tr && tr.className ? COLOR_RE.exec(tr.className) : null;
    return match ? parseInt(match[1], 10) : null;
  }

  /** Pick the least-used colour from the candidates and record the choice. */
  leastUsed(...candidates) {
    let chosen = candidates[0];
    for (const candidate of candidates) {
      if (this.colorFreq[candidate - 1] < this.colorFreq[chosen - 1]) chosen = candidate;
    }
    this.colorFreq[chosen - 1]++;
    return chosen;
  }

  /** A colour for a group differing from both neighbouring rows' colours. */
  pickColor(prev, next) {
    const prevColor = BlameState.colorOf(prev);
    const nextColor = BlameState.colorOf(next);
    if (!prevColor && !nextColor) return this.leastUsed(1, 2, 3);
    if (prevColor && nextColor && prevColor !== nextColor) {
      return 6 - prevColor - nextColor;
    }
    const taken = prevColor || nextColor;
    return this.leastUsed((taken % 3) + 1, ((taken + 1) % 3) + 1);
  }
}

/** Update the progress bar and "N / total (P%)" readout. */
function updateProgress(state) {
  const info = document.getElementById("progress_info");
  const bar = document.getElementById("progress_bar");
  if (!info && !bar) return;
  const percent = state.totalLines ? Math.floor((100 * state.blamedLines) / state.totalLines) : 0;
  if (info && info.firstChild) {
    info.firstChild.data = state.blamedLines + " / " + state.totalLines + " (" + percent + "%)";
  }
  if (bar) bar.style.width = percent + "%";
}

/** Fill the rows of one completed blame group with its commit's data. */
function fillGroup(state, commit, group) {
  if (!commit.info) {
    commit.info = commit.author + ", " + formatDateISO(commit.authorTime, commit.authorTimezone);
  }
  const colorNo = state.pickColor(
    document.getElementById("l" + (group.resline - 1)),
    document.getElementById("l" + (group.resline + group.numlines)),
  );
  for (let i = 0; i < group.numlines; i++) {
    const tr = document.getElementById("l" + (group.resline + i));
    if (!tr) break;
    let className = colorNo !== null ? "color" + colorNo : "";
    if (commit.boundary) className += " boundary";
    if (commit.nprevious === 0) className += " no-previous";
    else if (commit.nprevious > 1) className += " multiple-previous";
    tr.className = className;

    const sha1Cell = tr.firstChild;
    if (i === 0) {
      sha1Cell.title = commit.info;
      sha1Cell.rowSpan = group.numlines;
      const anchor = sha1Cell.firstChild;
      anchor.href = state.projectUrl + "a=commit;h=" + commit.sha1;
      anchor.textContent = commit.sha1.slice(0, 8);
    } else {
      tr.deleteCell(0);
    }

    const linenrCommit = "previous" in commit ? commit.previous : commit.sha1;
    const linenrFile = "file_parent" in commit ? commit.file_parent : commit.filename;
    const linenr = sha1Cell.nextSibling
      ? sha1Cell.nextSibling.firstChild
      : tr.cells[0].firstChild;
    if (linenr) {
      linenr.href =
        state.projectUrl + "a=blame_incremental;hb=" + linenrCommit +
        ";f=" + encodeURIComponent(linenrFile) + "#l" + (group.srcline + i);
    }
    state.blamedLines++;
  }
}

/** Parse a batch of complete blame_data lines, filling groups as they finish. */
function processLines(state, lines) {
  for (const line of lines) {
    let match = SHA1_RE.exec(line);
    if (match) {
      const sha1 = match[1];
      let commit = state.commits.get(sha1);
      if (!commit) {
        commit = { sha1, nprevious: 0 };
        state.commits.set(sha1, commit);
      }
      state.current = commit;
      state.group = {
        srcline: parseInt(match[2], 10),
        resline: parseInt(match[3], 10),
        numlines: parseInt(match[4], 10),
      };
      continue;
    }
    match = INFO_RE.exec(line);
    if (match) {
      const [key, data] = [match[1], match[2]];
      const commit = state.current;
      if (key === "filename") {
        commit.filename = unquote(data);
        fillGroup(state, commit, state.group);
        updateProgress(state);
      } else if (key === "author") {
        commit.author = data;
      } else if (key === "author-time") {
        commit.authorTime = parseInt(data, 10);
      } else if (key === "author-tz") {
        commit.authorTimezone = data;
      } else if (key === "previous") {
        commit.nprevious++;
        if (!("previous" in commit)) {
          const parts = data.split(" ");
          commit.previous = parts[0];
          commit.file_parent = unquote(parts.slice(1).join(" "));
        }
      } else if (key === "boundary") {
        commit.boundary = true;
      }
      continue;
    }
    END_RE.exec(line); // server timing footer; ignored in the modern view.
  }
}

/** After streaming finishes, collapse 3-colour groups to zebra light/dark. */
function recolorGroups() {
  const colors = ["light", "dark"];
  let colorIndex = 0;
  let prevGroup = null;
  let lineNo = 1;
  let tr;
  while ((tr = document.getElementById("l" + lineNo))) {
    if (tr.firstChild && tr.firstChild.className === "sha1") {
      const sameCommit =
        prevGroup &&
        prevGroup.firstChild.firstChild.href === tr.firstChild.firstChild.href;
      if (sameCommit) {
        prevGroup.firstChild.rowSpan =
          (prevGroup.firstChild.rowSpan || 1) + (tr.firstChild.rowSpan || 1);
        tr.deleteCell(0);
      } else {
        colorIndex = (colorIndex + 1) % 2;
        prevGroup = tr;
      }
    }
    tr.className = tr.className.replace(COLOR_RE, colors[colorIndex]);
    lineNo++;
  }
}

/**
 * Stream blame data from `dataUrl` into the page, updating links as it arrives.
 *
 * @param {string} dataUrl  - the blame_data endpoint (bead 950) URL.
 * @param {string} baseUrl  - the partial project URL used to build commit links.
 */
export async function startBlame(dataUrl, baseUrl) {
  const projectUrl = baseUrl + (baseUrl.includes("?") ? ";" : "?");
  const state = new BlameState(projectUrl);
  const bar = document.getElementById("progress_bar");
  if (bar) bar.style.width = "100%";
  updateProgress(state);

  const info = document.getElementById("progress_info");
  try {
    const response = await fetch(dataUrl, { headers: { Accept: "text/plain" } });
    if (!response.ok) throw new Error("Server error: " + response.status);
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lastNewline = buffer.lastIndexOf("\n");
      if (lastNewline !== -1) {
        processLines(state, buffer.slice(0, lastNewline).split("\n"));
        buffer = buffer.slice(lastNewline + 1);
      }
    }
    if (buffer) processLines(state, buffer.split("\n"));
    recolorGroups();
  } catch (error) {
    if (info) {
      info.className = "error";
      if (info.firstChild) info.firstChild.data = String(error.message || error);
      else info.textContent = String(error.message || error);
    }
  }
}
