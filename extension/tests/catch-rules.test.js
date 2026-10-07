const test = require("node:test");
const assert = require("node:assert");
const { shouldCatch } = require("../catch-rules.js");

const NOW = Date.parse("2026-10-06T12:00:00Z");
const on = { token: "t", catchDownloads: true, minSizeMB: 1 };
const fresh = (extra = {}) => ({
  url: "https://example.com/big.zip",
  state: "in_progress",
  startTime: new Date(NOW - 1000).toISOString(),
  totalBytes: 50 * 1024 * 1024,
  fileSize: -1,
  mime: "application/zip",
  ...extra,
});

test("catches a new download that just started", () => {
  assert.strictEqual(shouldCatch(fresh(), on, NOW), true);
});

test("ignores finished downloads replayed from history at browser start", () => {
  assert.strictEqual(shouldCatch(fresh({ state: "complete", startTime: "2026-10-01T09:00:00Z" }), on, NOW), false);
});

test("ignores old downloads even if Chrome reports them in progress", () => {
  assert.strictEqual(shouldCatch(fresh({ startTime: new Date(NOW - 5 * 60 * 1000).toISOString() }), on, NOW), false);
});

test("ignores interrupted and cancelled downloads", () => {
  assert.strictEqual(shouldCatch(fresh({ state: "interrupted" }), on, NOW), false);
});

test("respects the switch, pairing, scheme, images, extensions and size", () => {
  assert.strictEqual(shouldCatch(fresh(), { ...on, catchDownloads: false }, NOW), false);
  assert.strictEqual(shouldCatch(fresh(), { ...on, token: "" }, NOW), false);
  assert.strictEqual(shouldCatch(fresh({ url: "blob:https://x/y" }), on, NOW), false);
  assert.strictEqual(shouldCatch(fresh({ mime: "image/png" }), on, NOW), false);
  assert.strictEqual(shouldCatch(fresh({ byExtensionId: "abc" }), on, NOW), false);
  assert.strictEqual(shouldCatch(fresh({ totalBytes: 200 * 1024 }), on, NOW), false);
  assert.strictEqual(shouldCatch(fresh({ totalBytes: 0, fileSize: -1 }), on, NOW), true, "unknown size is caught");
});

test("a download Snag can't take goes back to the browser under its own name, folders left out", () => {
  const { handBackDownload } = require("../catch-rules.js");
  const url = "https://cdn.x/dl?id=7";
  assert.deepStrictEqual(handBackDownload({ url, filename: "C:\\Users\\me\\Downloads\\Report 2026.pdf" }), { url, filename: "Report 2026.pdf" });
  assert.deepStrictEqual(handBackDownload({ url: "https://a/x", finalUrl: url, filename: "/home/me/Downloads/a.zip" }), { url, filename: "a.zip" });
  assert.deepStrictEqual(handBackDownload({ url, filename: "" }), { url }, "no name yet: the browser picks one");
  assert.deepStrictEqual(handBackDownload({ url, filename: "C:\\dir\\.." }), { url });
});
