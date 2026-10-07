const test = require("node:test");
const assert = require("node:assert");
const { addLater, drainLater, laterBadge } = require("../later.js");

test("links saved for later keep their order and are not doubled", () => {
  let list = [];
  list = addLater(list, { url: "https://a.com/1.zip", kind: "file" }, 1000);
  list = addLater(list, { url: "https://a.com/2.zip" }, 2000);
  list = addLater(list, { url: "https://a.com/1.zip", kind: "file" }, 3000);
  assert.deepStrictEqual(list.map((e) => e.url), ["https://a.com/1.zip", "https://a.com/2.zip"]);
  assert.strictEqual(list[0].at, 1000, "the first time it was saved");
});

test("draining sends in order and keeps whatever failed (nothing is lost)", async () => {
  const list = [{ url: "u1" }, { url: "u2" }, { url: "u3" }];
  const sent = [];
  const left = await drainLater(list, async (e) => {
    if (e.url === "u2") throw new Error("Snag isn't running");
    sent.push(e.url);
  });
  assert.deepStrictEqual(sent, ["u1"], "stops at the first failure, so the order holds");
  assert.deepStrictEqual(left.map((e) => e.url), ["u2", "u3"]);
  assert.deepStrictEqual(await drainLater(left, async () => {}), []);
});

test("the badge shows how many wait", () => {
  assert.strictEqual(laterBadge([]), "");
  assert.strictEqual(laterBadge([{}, {}]), "2");
  assert.strictEqual(laterBadge(new Array(120).fill({})), "99+");
});
