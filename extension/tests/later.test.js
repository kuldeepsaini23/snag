const test = require("node:test");
const assert = require("node:assert");
const { addLater, drainLater, laterBadge, isPermanent, LATER_MAX } = require("../later.js");

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

test("the waiting list holds at most 500 links: the oldest go first", () => {
  assert.strictEqual(LATER_MAX, 500);
  let list = [];
  for (let i = 0; i < 502; i++) list = addLater(list, { url: `u${i}` }, i);
  assert.strictEqual(list.length, 500);
  assert.strictEqual(list[0].url, "u2", "the two oldest were dropped");
  assert.strictEqual(list[499].url, "u501");
});

test("a link Snag refuses for good is dropped, so it never blocks the ones behind it", async () => {
  const list = [{ url: "ftp://x" }, { url: "u2" }, { url: "u3" }];
  const sent = [];
  const left = await drainLater(list, async (e) => {
    if (e.url === "ftp://x") throw Object.assign(new Error("only web links"), { permanent: true });
    sent.push(e.url);
  });
  assert.deepStrictEqual(sent, ["u2", "u3"]);
  assert.deepStrictEqual(left, []);
});

test("only a 4xx answer about the link itself is permanent (not pairing, timeouts or rate limits)", () => {
  assert.ok(isPermanent(400));
  assert.ok(isPermanent(404));
  assert.ok(isPermanent(422));
  for (const status of [401, 403, 408, 429, 500, 503, 200]) assert.ok(!isPermanent(status), String(status));
});
