const test = require("node:test");
const assert = require("node:assert");
const { classifyMedia, mediaKey, MediaList } = require("../sniffer.js");

const MB = 1024 * 1024;
const resp = (url, contentType = "", size = 0, extra = {}) => ({ url, contentType, size, status: 200, contentRange: "", ...extra });

test("HLS and DASH playlists are found on any site", () => {
  assert.deepStrictEqual(classifyMedia(resp("https://cdn.site.tv/live/master.m3u8?token=1")), { kind: "hls", url: "https://cdn.site.tv/live/master.m3u8?token=1" });
  assert.strictEqual(classifyMedia(resp("https://cdn.x/v/index", "application/vnd.apple.mpegurl")).kind, "hls");
  assert.strictEqual(classifyMedia(resp("https://cdn.x/v/manifest.mpd")).kind, "dash");
  assert.strictEqual(classifyMedia(resp("https://cdn.x/v/m", "application/dash+xml")).kind, "dash");
  assert.strictEqual(classifyMedia(resp("https://player.x/load?src=https%3A%2F%2Fcdn.x%2Fhls%2Fmaster.m3u8", "text/plain")).kind, "hls", "named in the query");
  assert.strictEqual(classifyMedia(resp("https://cdn.x/hls/720p/index-v1-a1.m3u8?sig=1", "application/octet-stream")).kind, "hls");
});

test("the best media to fall back on: a playlist, else the biggest file", () => {
  const { bestMedia } = require("../sniffer.js");
  assert.strictEqual(bestMedia([]), null);
  assert.strictEqual(bestMedia([{ kind: "file", url: "a", size: 5 }, { kind: "file", url: "b", size: 50 }]).url, "b");
  assert.strictEqual(bestMedia([{ kind: "file", url: "a", size: 500 }, { kind: "hls", url: "m" }]).url, "m");
});

test("progressive video and audio files of 1 MB or more are found", () => {
  assert.strictEqual(classifyMedia(resp("https://files.x/movie.mp4", "video/mp4", 80 * MB)).kind, "file");
  assert.strictEqual(classifyMedia(resp("https://files.x/stream", "audio/mpeg", 5 * MB)).kind, "file");
  assert.strictEqual(classifyMedia(resp("https://files.x/clip.webm", "", 0)).kind, "file", "unknown size (chunked) still counts");
});

test("segments, partial ranges, small files and ad beacons are noise", () => {
  for (const r of [
    resp("https://cdn.x/v/seg-17.ts", "video/mp2t", 2 * MB),
    resp("https://cdn.x/v/chunk-00042.m4s", "video/iso.segment", 1 * MB),
    resp("https://cdn.x/v/movie.mp4", "video/mp4", 30 * MB, { status: 206, contentRange: "bytes 1048576-2097151/31457280" }),
    resp("https://cdn.x/v/preview.mp4", "video/mp4", 200 * 1024),
    resp("https://securepubads.g.doubleclick.net/ad.mp4", "video/mp4", 5 * MB),
    resp("https://rr3---sn.googlevideo.com/videoplayback?itag=22", "video/mp4", 9 * MB),
    resp("https://site.x/page.html", "text/html", 10 * MB),
    resp("blob:https://site.x/1234", "video/mp4", 9 * MB),
  ]) {
    assert.strictEqual(classifyMedia(r), null, r.url);
  }
});

test("a range request from the start reports the whole file", () => {
  const r = resp("https://cdn.x/v/movie.mp4", "video/mp4", 1 * MB, { status: 206, contentRange: "bytes 0-1048575/31457280" });
  assert.deepStrictEqual(classifyMedia(r), { kind: "file", url: "https://cdn.x/v/movie.mp4", size: 31457280 });
});

test("the same media seen twice is listed once (range params ignored)", () => {
  assert.strictEqual(mediaKey("https://cdn.x/a.mp4?range=0-100&sig=1"), mediaKey("https://cdn.x/a.mp4?sig=1&range=200-300"));
  assert.notStrictEqual(mediaKey("https://cdn.x/a.mp4"), mediaKey("https://cdn.x/b.mp4"));
  const list = new MediaList(3);
  list.add({ kind: "file", url: "https://cdn.x/a.mp4?range=0-1" });
  list.add({ kind: "file", url: "https://cdn.x/a.mp4?range=5-9" });
  list.add({ kind: "hls", url: "https://cdn.x/m.m3u8" });
  assert.strictEqual(list.items.length, 2);
  for (let i = 0; i < 10; i++) list.add({ kind: "file", url: `https://cdn.x/${i}.mp4` });
  assert.strictEqual(list.items.length, 3, "capped");
  assert.strictEqual(list.items[0].kind, "hls", "playlists stay at the top (they're what people want)");
});
