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

test("asking the extension never hangs: cut-off page script, errors and silence all answer", async () => {
  const { makeAsk } = require("../quality.js");
  const ok = makeAsk({ sendMessage: (m, cb) => cb({ ok: true, echo: m.type }), lastError: undefined }, 1000);
  assert.deepStrictEqual(await ok({ type: "probe" }), { ok: true, echo: "probe" });
  // The extension was reloaded: this page's script is cut off and sendMessage throws.
  const cut = makeAsk({ sendMessage: () => { throw new Error("Extension context invalidated."); } }, 1000);
  const r1 = await cut({ type: "probe" });
  assert.strictEqual(r1.ok, false);
  assert.match(r1.error, /reload this page/i);
  // The browser reports an error instead of an answer.
  const rt = { sendMessage: (m, cb) => { rt.lastError = { message: "Could not establish connection" }; cb(undefined); } };
  const r2 = await makeAsk(rt, 1000)({ type: "probe" });
  assert.strictEqual(r2.ok, false);
  assert.match(r2.error, /reload this page/i);
  // No answer at all.
  const r3 = await makeAsk({ sendMessage: () => {} }, 30)({ type: "probe" });
  assert.strictEqual(r3.ok, false);
  assert.match(r3.error, /no answer/i);
});
