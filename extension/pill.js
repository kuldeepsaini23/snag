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

if (typeof module !== "undefined") module.exports = { CORNERS, siteKey, isHiddenOn, hideSite, showSite, cornerPosition, nearestCorner };
