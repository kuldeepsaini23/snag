// Where the in-page Download button sits and the sites it stays off. Pure helpers shared by
// content.js, the popup and the tests.

const CORNERS = ["bottom-right", "bottom-left", "top-right", "top-left"];
const EDGE = "24px";

/** A site as the user thinks of it: the host without "www.". */
function siteKey(hostname) {
  return (hostname || "").toLowerCase().replace(/^www\./, "");
}

/** Hidden on `hostname` if it is a hidden site or one of its subdomains. */
function isHiddenOn(hostname, hidden) {
  const host = siteKey(hostname);
  return (hidden || []).some((site) => host === site || host.endsWith(`.${site}`));
}

function hideSite(hidden, hostname) {
  const site = siteKey(hostname);
  return [...new Set([...(hidden || []), site])].filter(Boolean).sort();
}

function showSite(hidden, hostname) {
  const site = siteKey(hostname);
  return (hidden || []).filter((s) => s !== site);
}

/** CSS offsets for a corner (unknown values fall back to bottom-right). */
function cornerPosition(corner) {
  const c = CORNERS.includes(corner) ? corner : CORNERS[0];
  const [v, h] = c.split("-");
  const pos = { top: "auto", bottom: "auto", left: "auto", right: "auto" };
  pos[v] = EDGE;
  pos[h] = EDGE;
  return pos;
}

/** The corner closest to where the button was dropped. */
function nearestCorner(x, y, width, height) {
  return `${y < height / 2 ? "top" : "bottom"}-${x < width / 2 ? "left" : "right"}`;
}

/**
 * The button's host element on the page. Inline `!important` beats any page rule, and
 * `all: initial` drops whatever the page sets on divs (a transform or filter would pin the
 * fixed button to the page instead of the window).
 */
const HOST_STYLE = [
  "all: initial !important",
  "position: fixed !important",
  "top: 0 !important",
  "left: 0 !important",
  "width: 0 !important",
  "height: 0 !important",
  "z-index: 2147483647 !important",
].join("; ");

/** `fn` for events the user made; ones a page script made up (`isTrusted` false) are ignored. */
function trusted(fn) {
  return (e) => {
    if (e.isTrusted) return fn(e);
  };
}

if (typeof module !== "undefined") module.exports = { CORNERS, HOST_STYLE, siteKey, isHiddenOn, hideSite, showSite, cornerPosition, nearestCorner, trusted };
