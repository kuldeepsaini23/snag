const test = require("node:test");
const assert = require("node:assert");
const { sizeLabel, qualityRows, safeAccent } = require("../quality.js");

test("sizes read like people say them", () => {
  assert.strictEqual(sizeLabel(0), "");
  assert.strictEqual(sizeLabel(null), "");
  assert.strictEqual(sizeLabel(3.4 * 1024 * 1024), "3.4 MB");
  assert.strictEqual(sizeLabel(250 * 1024 * 1024), "250 MB");
  assert.strictEqual(sizeLabel(2.5 * 1024 * 1024 * 1024), "2.5 GB");
});

test("qualities: best video first, audio last, each with what to send back", () => {
  const info = {
    options: [
      { label: "360p", size: 10 * 1024 * 1024, format: { Video: { max_height: 360 } } },
      { label: "Audio only (MP3)", size: null, format: "AudioMp3" },
      { label: "1080p", size: 90 * 1024 * 1024, format: { Video: { max_height: 1080 } } },
    ],
  };
  const rows = qualityRows(info);
  assert.deepStrictEqual(rows.map((r) => r.label), ["1080p", "360p", "Audio only (MP3)"]);
  assert.strictEqual(rows[0].detail, "90 MB");
  assert.strictEqual(rows[0].badge, "HD");
  assert.strictEqual(rows[2].detail, "");
  assert.deepStrictEqual(rows[2].format, "AudioMp3");
  assert.deepStrictEqual(qualityRows({}), []);
});

test("the accent from Snag is used only when it is a real colour", () => {
  assert.strictEqual(safeAccent("#0a84ff"), "#0a84ff");
  assert.strictEqual(safeAccent("#FFF"), "#FFF");
  assert.strictEqual(safeAccent("red; background:url(x)"), "#ff9f0a");
  assert.strictEqual(safeAccent(undefined), "#ff9f0a");
});
