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

test("what the player is already playing: one instant row each", () => {
  const { sniffedRows } = require("../quality.js");
  const rows = sniffedRows([
    { kind: "hls", url: "https://cdn.x/hls/master.m3u8?sig=1" },
    { kind: "file", url: "https://cdn.x/v/movie_720.mp4?t=9", size: 120 * 1024 * 1024 },
    { kind: "file", url: "https://cdn.x/a/track.m4a" },
  ]);
  assert.deepStrictEqual(rows.map((r) => r.label), ["Stream (HLS)", "MP4 video", "M4A audio"]);
  assert.deepStrictEqual(rows.map((r) => r.kind), ["media", "file", "file"]);
  assert.strictEqual(rows[1].detail, "120 MB");
  assert.strictEqual(rows[1].url, "https://cdn.x/v/movie_720.mp4?t=9");
  // The embedded player's page is what the video host expects as Referer.
  const embedded = sniffedRows([{ kind: "file", url: "https://cdn.x/v.mp4", referrer: "https://player.embed.tv/e/1" }]);
  assert.strictEqual(embedded[0].referrer, "https://player.embed.tv/e/1");
  assert.strictEqual(rows[1].referrer, undefined);
  assert.deepStrictEqual(sniffedRows([]), []);
});

test("prefetch: a page is read once, then the answer is ready (and expires)", async () => {
  const { ProbeCache } = require("../quality.js");
  let now = 0;
  let calls = 0;
  const cache = new ProbeCache(async (url) => {
    calls++;
    if (url.includes("bad")) throw new Error("Unsupported URL");
    return { title: url };
  }, () => now, 60_000);
  const a = cache.get("https://v.x/1");
  const b = cache.get("https://v.x/1"); // asked again while still reading: same read
  assert.deepStrictEqual(await a, { ok: true, info: { title: "https://v.x/1" } });
  assert.deepStrictEqual(await b, await a);
  assert.strictEqual(calls, 1);
  assert.deepStrictEqual(await cache.get("https://v.x/bad"), { ok: false, error: "Unsupported URL" });
  await cache.get("https://v.x/bad");
  assert.strictEqual(calls, 3, "failures are not kept: the next click tries again");
  now = 61_000;
  await cache.get("https://v.x/1");
  assert.strictEqual(calls, 4, "old answers expire (links in them stop working)");
});

test("right-click: an image is the image itself (instant), not the post it links to", () => {
  const { contextTarget } = require("../quality.js");
  const page = "https://x.com/someone/status/1";
  // A photo on X sits inside a link to its post.
  const x = contextTarget({ mediaType: "image", srcUrl: "https://pbs.twimg.com/media/GxYz?format=jpg&name=small", linkUrl: "https://x.com/someone/status/1/photo/1", pageUrl: page });
  assert.deepStrictEqual(x, { url: "https://pbs.twimg.com/media/GxYz.jpg?name=orig", kind: "file", referrer: page });
  const png = contextTarget({ mediaType: "image", srcUrl: "https://pbs.twimg.com/media/Ab?format=png&name=900x900", pageUrl: page });
  assert.strictEqual(png.url, "https://pbs.twimg.com/media/Ab.png?name=orig");
  const plain = contextTarget({ mediaType: "image", srcUrl: "https://site.com/a.webp", linkUrl: "https://site.com/post/2", pageUrl: "https://site.com/" });
  assert.deepStrictEqual(plain, { url: "https://site.com/a.webp", kind: "file", referrer: "https://site.com/" });
  // A video element's own file is a file too; a blob: player isn't, so the page goes instead.
  assert.strictEqual(contextTarget({ mediaType: "video", srcUrl: "https://cdn.x/v.mp4", pageUrl: page }).kind, "file");
  assert.deepStrictEqual(contextTarget({ mediaType: "video", srcUrl: "blob:https://x.com/1", pageUrl: page }), { url: page, kind: "media", referrer: undefined });
  // A plain link: Snag decides (video page, file, torrent…).
  assert.deepStrictEqual(contextTarget({ linkUrl: "https://site.com/file.zip", pageUrl: page }), { url: "https://site.com/file.zip", kind: undefined, referrer: page });
  // "Download this page".
  assert.deepStrictEqual(contextTarget({ menuItemId: "rdm-page", pageUrl: page }), { url: page, kind: "media", referrer: undefined });
});

test("right-click 'Save page' sends the page to be saved as one file", () => {
  const { contextTarget } = require("../quality.js");
  assert.deepStrictEqual(contextTarget({ menuItemId: "rdm-save-page", pageUrl: "https://blog.x/post" }), { url: "https://blog.x/post", kind: "page", referrer: undefined });
});

test("prefetch: only known video pages, once per page (hash ignored), after the address settles", () => {
  const { PrefetchPlan } = require("../quality.js");
  const { isVideoPage } = require("../catch-rules.js");
  const plan = new PrefetchPlan(1500, isVideoPage);
  // An ordinary page is never read in the background, even while it plays something.
  assert.strictEqual(plan.next("https://blog.example/post", 0), null);
  assert.strictEqual(plan.next("https://blog.example/post", 5000), null);
  // A video page: only once its address has held still for 1.5 s.
  const yt = "https://www.youtube.com/watch?v=abc";
  assert.strictEqual(plan.next(yt, 10_000), null);
  assert.strictEqual(plan.next(yt, 11_000), null);
  assert.strictEqual(plan.next(yt, 11_500), yt);
  assert.strictEqual(plan.next(yt, 13_000), null, "once");
  assert.strictEqual(plan.next(`${yt}#t=30`, 14_000), null, "the same page with a #fragment");
  // Clicking quickly through videos reads none of the pages skipped past.
  const a = "https://www.youtube.com/watch?v=a";
  const b = "https://www.youtube.com/watch?v=b#comments";
  assert.strictEqual(plan.next(a, 20_000), null);
  assert.strictEqual(plan.next(b, 21_000), null);
  assert.strictEqual(plan.next(b, 22_000), null);
  assert.strictEqual(plan.next(b, 22_500), "https://www.youtube.com/watch?v=b", "keyed without the hash");
  // Hidden on this site (the content script passes no page): nothing.
  assert.strictEqual(plan.next("", 30_000), null);
  assert.strictEqual(plan.next("not a url", 40_000), null);
});

test("a page read is shared whatever its #fragment", () => {
  const { pageKey } = require("../quality.js");
  assert.strictEqual(pageKey("https://v.x/watch?v=1#t=30"), "https://v.x/watch?v=1");
  assert.strictEqual(pageKey("https://v.x/watch?v=1"), "https://v.x/watch?v=1");
});

test("prefetch reads pass their options through (no cookies), clicks theirs", async () => {
  const { ProbeCache } = require("../quality.js");
  const seen = [];
  const cache = new ProbeCache(async (url, opts) => {
    seen.push([url, opts]);
    return {};
  }, () => 0, 60_000);
  await cache.get("https://v.x/1", { cookies: false });
  await cache.get("https://v.x/2", { cookies: true });
  assert.deepStrictEqual(seen, [["https://v.x/1", { cookies: false }], ["https://v.x/2", { cookies: true }]]);
});
