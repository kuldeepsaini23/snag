const test = require("node:test");
const assert = require("node:assert");
const { siteKey, isHiddenOn, hideSite, showSite, cornerPosition, nearestCorner, CORNERS } = require("../pill.js");

test("a site is its host without www", () => {
  assert.strictEqual(siteKey("www.youtube.com"), "youtube.com");
  assert.strictEqual(siteKey("M.YouTube.com"), "m.youtube.com");
  assert.strictEqual(siteKey(""), "");
});

test("hiding on a site covers its subdomains, nothing else", () => {
  const hidden = ["youtube.com", "example.org"];
  assert.ok(isHiddenOn("www.youtube.com", hidden));
  assert.ok(isHiddenOn("m.youtube.com", hidden));
  assert.ok(isHiddenOn("example.org", hidden));
  assert.ok(!isHiddenOn("notyoutube.com", hidden), "a lookalike name isn't the same site");
  assert.ok(!isHiddenOn("vimeo.com", hidden));
  assert.ok(!isHiddenOn("vimeo.com", undefined));
});

test("hide/show keep the list tidy", () => {
  assert.deepStrictEqual(hideSite([], "www.youtube.com"), ["youtube.com"]);
  assert.deepStrictEqual(hideSite(["youtube.com"], "youtube.com"), ["youtube.com"], "no duplicates");
  assert.deepStrictEqual(hideSite(["b.com"], "a.com"), ["a.com", "b.com"], "sorted");
  assert.deepStrictEqual(showSite(["a.com", "youtube.com"], "www.youtube.com"), ["a.com"]);
});

test("each corner places the button 24 px from its edges; unknown means bottom-right", () => {
  assert.deepStrictEqual(CORNERS, ["bottom-right", "bottom-left", "top-right", "top-left"]);
  assert.deepStrictEqual(cornerPosition("bottom-right"), { bottom: "24px", right: "24px", top: "auto", left: "auto" });
  assert.deepStrictEqual(cornerPosition("top-left"), { top: "24px", left: "24px", bottom: "auto", right: "auto" });
  assert.deepStrictEqual(cornerPosition("nonsense"), cornerPosition("bottom-right"));
});

test("dropping the button snaps it to the nearest corner", () => {
  assert.strictEqual(nearestCorner(10, 10, 1000, 800), "top-left");
  assert.strictEqual(nearestCorner(990, 790, 1000, 800), "bottom-right");
  assert.strictEqual(nearestCorner(900, 50, 1000, 800), "top-right");
  assert.strictEqual(nearestCorner(100, 700, 1000, 800), "bottom-left");
});
