const test = require("node:test");
const assert = require("node:assert");
const { makeConnection, NOT_CONNECTED } = require("../connect.js");

/** A fake Snag on the first port: `paired` once the extension holds `code`; /pair counted. */
function fakeSnag({ allow = true } = {}) {
  const snag = { code: "c0de", pairs: 0, saved: "" };
  snag.fetch = async (url, opts = {}) => {
    const path = new URL(url).pathname;
    const answer = (status, body) => ({ ok: status === 200, status, json: async () => body });
    if (path === "/ping") return answer(200, { app: "rdm", paired: (opts.headers || {})["X-RDM-Token"] === snag.code });
    if (path === "/pair") {
      snag.pairs++;
      return allow ? answer(200, { token: snag.code }) : answer(403, { error: "not allowed in Snag" });
    }
    return answer(404, {});
  };
  snag.store = {
    token: async () => snag.saved,
    saveToken: async (t) => {
      snag.saved = t;
    },
    saveAccent: () => {},
  };
  return snag;
}

test("background work never asks Snag to connect", async () => {
  const snag = fakeSnag();
  const { pairedApp } = makeConnection(snag.fetch, snag.store);
  await assert.rejects(pairedApp(), { message: NOT_CONNECTED });
  await assert.rejects(pairedApp({ mayPair: false }), { message: NOT_CONNECTED });
  assert.strictEqual(snag.pairs, 0, "no Allow? question in Snag");
});

test("a click may connect: Snag asks once, then the code is kept", async () => {
  const snag = fakeSnag();
  const { pairedApp } = makeConnection(snag.fetch, snag.store);
  const { app, token } = await pairedApp({ mayPair: true });
  assert.deepStrictEqual(app, { port: 47321, paired: true });
  assert.strictEqual(token, "c0de");
  assert.strictEqual(snag.saved, "c0de");
  // Connected: neither kind of caller asks again.
  await pairedApp();
  await pairedApp({ mayPair: true });
  assert.strictEqual(snag.pairs, 1);
});

test("a refused click says so", async () => {
  const snag = fakeSnag({ allow: false });
  const { pairedApp } = makeConnection(snag.fetch, snag.store);
  await assert.rejects(pairedApp({ mayPair: true }), { message: "not allowed in Snag" });
  assert.strictEqual(snag.saved, "");
});

test("Snag closed: links wait (the same error the queue keeps them for)", async () => {
  const { pairedApp } = makeConnection(async () => {
    throw new Error("connection refused");
  }, fakeSnag().store);
  await assert.rejects(pairedApp({ mayPair: true }), { message: "Snag isn't running" });
});
