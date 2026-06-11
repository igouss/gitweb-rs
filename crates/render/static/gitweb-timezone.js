// gitweb-rs client module: render timestamps in the viewer's chosen timezone.
//
// Ports the BEHAVIOUR of git's gitweb static/js/adjust-timezone.js +
// lib/datetime.js + lib/cookies.js to a modern, framework-free ES module (no
// jQuery, no build step). gitweb's server emits each adjustable date as an
// RFC-2822 string inside <span class="datetime">…</span>; this module reads the
// viewer's preferred timezone (a cookie, default from the data attributes the
// boundary stamps), rewrites every such span in place, and offers a small
// click-to-change popup that persists the choice in the cookie.
//
// MANUAL CHECK (cannot be asserted headless): load any page carrying
// <span class="datetime"> dates with the javascript-timezone feature on; the
// dates render in the local timezone (or the cookie's), clicking one opens a
// timezone menu, and selecting an entry re-renders every date and persists the
// choice across reloads. Selecting "UTC" leaves the server's UTC text unchanged.

const TZ_CLASS = "datetime";
const COOKIE_NAME = "gitweb_tz";
const COOKIE_DAYS = 14;
const COOKIE_PATH = "/";
// gitweb's server-side default timezone for this feature ('local' | 'utc' | '+HHMM').
const TZ_DEFAULT = "local";

const TZ_RE = /^([+-])(\d\d)(\d\d)$/;
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Two-digit zero-padded string for a non-negative integer. */
function pad2(value) {
  return String(value).padStart(2, "0");
}

/** Read a cookie value by name, or null when absent. */
function getCookie(name) {
  const match = document.cookie.match(new RegExp("(?:^|; )" + name + "=([^;]*)"));
  return match ? decodeURIComponent(match[1]) : null;
}

/** Store a cookie that expires `days` from now, scoped to `path`. */
function setCookie(name, value, days, path) {
  const expires = new Date(Date.now() + days * 24 * 60 * 60 * 1000).toUTCString();
  document.cookie =
    name + "=" + encodeURIComponent(value) + "; expires=" + expires + "; path=" + path;
}

/** The browser's current offset as '(+|-)HHMM'. */
function localTimezoneInfo() {
  const minutes = -new Date().getTimezoneOffset();
  const sign = minutes >= 0 ? "+" : "-";
  const abs = Math.abs(minutes);
  return sign + pad2(Math.floor(abs / 60)) + pad2(abs % 60);
}

/** Translate 'utc'/'local' to a numeric '(+|-)HHMM' offset; pass others through. */
function normalizeTimezone(tz) {
  if (tz === "utc") return "+0000";
  if (tz === "local") return localTimezoneInfo();
  return tz;
}

/** Offset of a numeric '(+|-)HHMM' timezone from UTC, in seconds. */
function timezoneOffsetSeconds(tz) {
  const match = TZ_RE.exec(tz);
  if (!match) return 0;
  const sign = match[1] === "-" ? -1 : 1;
  return sign * (parseInt(match[2], 10) * 60 + parseInt(match[3], 10)) * 60;
}

/** Format an epoch (seconds) as an RFC-2822 date in the given '(+|-)HHMM' tz. */
function formatRFC2822(epochSeconds, tz) {
  const date = new Date(1000 * (epochSeconds + timezoneOffsetSeconds(tz)));
  const datePart =
    DAYS[date.getUTCDay()] + ", " + date.getUTCDate() + " " +
    MONTHS[date.getUTCMonth()] + " " + date.getUTCFullYear();
  const timePart =
    pad2(date.getUTCHours()) + ":" + pad2(date.getUTCMinutes()) + ":" + pad2(date.getUTCSeconds());
  return datePart + " " + timePart + " " + tz;
}

/** Rewrite every <span class="datetime"> to the chosen tz (unless it is UTC). */
function fixDatetimes(tz) {
  const numeric = normalizeTimezone(tz);
  const noChange = tz === "utc";
  for (const span of document.getElementsByClassName(TZ_CLASS)) {
    span.title = "Click to change timezone";
    if (noChange) continue;
    const text = span.firstChild ? span.firstChild.data : span.textContent;
    const epoch = Date.parse(text) / 1000;
    if (!Number.isNaN(epoch)) {
      const adjusted = formatRFC2822(epoch, numeric);
      if (span.firstChild) span.firstChild.data = adjusted;
      else span.textContent = adjusted;
    }
  }
}

/** The selectable timezones: UTC, local, then every whole hour from -12 to +14. */
function timezoneOptions() {
  const list = [
    { value: "utc", label: "UTC/GMT" },
    { value: "local", label: "Local (per browser)" },
  ];
  for (let hour = -12; hour <= 14; hour++) {
    const sign = hour >= 0 ? "+" : "-";
    const hh = pad2(Math.abs(hour));
    list.push({
      value: sign + hh + "00",
      label: hour === 0 ? "UTC±00:00" : "UTC" + sign + hh + ":00",
    });
  }
  return list;
}

/** Build the timezone-selection popup, wiring its onchange to apply + persist. */
function buildPopup(selected) {
  const popup = document.createElement("div");
  popup.className = "tz-popup";

  const close = document.createElement("button");
  close.type = "button";
  close.className = "tz-close";
  close.title = "(click to close)";
  close.textContent = "X";
  close.addEventListener("click", () => popup.remove());
  popup.append(close, "Select timezone: ");

  const select = document.createElement("select");
  select.name = "tzoffset";
  for (const option of timezoneOptions()) {
    const element = document.createElement("option");
    element.value = option.value;
    element.textContent = option.label;
    if (option.value === selected) element.selected = true;
    select.append(element);
  }
  select.addEventListener("change", () => {
    const value = select.value;
    setCookie(COOKIE_NAME, value, COOKIE_DAYS, COOKIE_PATH);
    fixDatetimes(value);
    popup.remove();
  });
  popup.append(select);
  return popup;
}

/** Open the popup inside the clicked datetime span (one popup at a time). */
function onDatetimeClick(event, selected) {
  const target = event.target.closest("." + TZ_CLASS);
  if (!target || target.querySelector(".tz-popup")) return;
  target.style.position = "relative";
  target.append(buildPopup(selected));
}

function init() {
  let tz = TZ_DEFAULT;
  const saved = getCookie(COOKIE_NAME);
  if (saved) {
    tz = saved;
    // Refresh the cookie so its expiry counts from this visit.
    setCookie(COOKIE_NAME, saved, COOKIE_DAYS, COOKIE_PATH);
  }
  document.addEventListener("click", (event) => onDatetimeClick(event, tz));
  fixDatetimes(tz);
}

// type="module" defers until the DOM is parsed, so the spans already exist.
init();
