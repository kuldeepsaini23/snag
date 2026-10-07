const test = require("node:test");
const assert = require("node:assert");
const { typeOf, normalize, filterItems, preselect, PRESETS } = require("../grab.js");

const img = (url, width = 800, height = 600) => ({ url, tag: "img", width, height, text: "" });
const link = (url, text = "") => ({ url, tag: "a", text });

test("types come from the file extension", () => {
  assert.strictEqual(typeOf("https://x/a/photo.JPG?w=1"), "image");
  assert.strictEqual(typeOf("https://x/movie.mkv"), "video");
  assert.strictEqual(typeOf("https://x/song.flac"), "audio");
  assert.strictEqual(typeOf("https://x/report.pdf"), "document");
  assert.strictEqual(typeOf("https://x/tools.7z"), "archive");
  assert.strictEqual(typeOf("https://x/setup.exe"), "program");
  assert.strictEqual(typeOf("https://x/about"), "page");
});

test("normalize drops junk, duplicates and non-web links", () => {
  const items = normalize([
    link("https://x/a.zip"),
    link("https://x/a.zip#top"),
    link("javascript:void(0)"),
    link("mailto:me@x.com"),
    img("data:image/png;base64,AAAA"),
    link("https://x/b.pdf", "  Report  "),
  ]);
  assert.deepStrictEqual(items.map((i) => i.url), ["https://x/a.zip", "https://x/b.pdf"]);
  assert.strictEqual(items[1].text, "Report");
  assert.strictEqual(items[1].type, "document");
});

test("the Images preset keeps real pictures, not icons", () => {
  const items = normalize([img("https://x/big.jpg"), img("https://x/icon.png", 32, 32), img("https://x/unknown-size.webp", 0, 0), link("https://x/a.zip")]);
  const out = filterItems(items, PRESETS.images);
  assert.deepStrictEqual(out.map((i) => i.url), ["https://x/big.jpg", "https://x/unknown-size.webp"]);
});

test("filters combine: preset, extensions and text", () => {
  const items = normalize([link("https://x/a.zip", "Source code"), link("https://x/b.rar", "Data"), link("https://x/c.pdf", "Manual")]);
  assert.strictEqual(filterItems(items, PRESETS.everything).length, 3);
  assert.deepStrictEqual(filterItems(items, { ...PRESETS.archives, extensions: ["zip"] }).map((i) => i.url), ["https://x/a.zip"]);
  assert.deepStrictEqual(filterItems(items, { ...PRESETS.everything, text: "manual" }).map((i) => i.url), ["https://x/c.pdf"]);
  assert.strictEqual(filterItems(items, PRESETS.everything).filter((i) => i.type === "page").length, 0, "plain page links aren't downloads");
});

test("nothing is pre-selected when there are more than 200 items", () => {
  const few = normalize(Array.from({ length: 5 }, (_, i) => link(`https://x/${i}.zip`)));
  assert.strictEqual(preselect(few).size, 5);
  const many = normalize(Array.from({ length: 201 }, (_, i) => link(`https://x/${i}.zip`)));
  assert.strictEqual(preselect(many).size, 0);
});

test("more than 1000 links go to Snag in batches of at most 1000, totals added up", async () => {
  const { sendInChunks, BATCH_MAX } = require("../grab.js");
  assert.strictEqual(BATCH_MAX, 1000, "Snag takes at most 1000 links per request");
  const urls = Array.from({ length: 2500 }, (_, i) => `https://x/${i}.zip`);
  const sizes = [];
  const result = await sendInChunks(urls, async (chunk) => {
    sizes.push(chunk.length);
    return { added: chunk.length - 1, skipped: 1 };
  });
  assert.deepStrictEqual(sizes, [1000, 1000, 500]);
  assert.deepStrictEqual(result, { added: 2497, skipped: 3 });
});

test("a failed batch stops the rest and keeps what was already added", async () => {
  const { sendInChunks } = require("../grab.js");
  const urls = Array.from({ length: 1500 }, (_, i) => `https://x/${i}.zip`);
  let calls = 0;
  const result = await sendInChunks(urls, async (chunk) => {
    if (++calls === 2) throw new Error("Snag isn't running");
    return { added: chunk.length };
  });
  assert.deepStrictEqual(result, { added: 1000, skipped: 0, error: "Snag isn't running" });
});

test("the message says how many were already downloaded", () => {
  const { batchMessage } = require("../grab.js");
  assert.strictEqual(batchMessage({ added: 5, skipped: 0 }), "Sent 5 to Snag ✓");
  assert.strictEqual(batchMessage({ added: 5, skipped: 2 }), "5 added, 2 already downloaded ✓");
  assert.strictEqual(batchMessage({ added: 0, skipped: 0, error: "Snag isn't running" }), "Snag isn't running");
  assert.strictEqual(batchMessage({ added: 1000, skipped: 0, error: "Snag isn't running" }), "Sent 1000 to Snag, then: Snag isn't running");
});
