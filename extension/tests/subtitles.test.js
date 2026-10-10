const test = require("node:test");
const assert = require("node:assert");
const { classifySubtitle, guessLang, SubtitleList, subtitlesToSend, classifyMedia } = require("../sniffer.js");
const { trackList } = require("../quality.js");

const resp = (url, contentType = "", status = 200) => ({ url, contentType, status });

test("subtitle files are found by extension or type, query strings included", () => {
  assert.deepStrictEqual(classifySubtitle(resp("https://cdn.x/subs/movie.en.vtt")), { url: "https://cdn.x/subs/movie.en.vtt", lang: "en" });
  assert.deepStrictEqual(classifySubtitle(resp("https://cdn.x/s/abc.srt?token=1&exp=2")), { url: "https://cdn.x/s/abc.srt?token=1&exp=2" });
  assert.ok(classifySubtitle(resp("https://cdn.x/s/a.ASS")));
  assert.ok(classifySubtitle(resp("https://cdn.x/s/a.ssa")));
  assert.ok(classifySubtitle(resp("https://api.x/caption?id=9", "text/vtt; charset=utf-8")), "by type");
  assert.ok(classifySubtitle(resp("https://api.x/caption?id=9", "application/x-subrip")));
  assert.strictEqual(classifyMedia({ url: "https://cdn.x/subs/movie.en.vtt", contentType: "text/vtt", size: 900, status: 200 }), null, "never listed as media");
});

test("failed requests, other files and a stream's subtitle pieces are not subtitle files", () => {
  for (const r of [
    resp("https://cdn.x/subs/en.vtt", "text/html", 404),
    resp("https://cdn.x/subs/en.vtt", "", 302),
    resp("https://cdn.x/page.html", "text/html"),
    resp("https://cdn.x/v/master.m3u8", "application/vnd.apple.mpegurl"),
    resp("https://cdn.x/subs/eng/seg-12.vtt"),
    resp("https://cdn.x/subs/eng/fileSequence3.webvtt", "text/vtt"),
    resp("https://cdn.x/subs/eng/17.vtt"),
    resp("blob:https://site.x/1234", "text/vtt"),
  ]) {
    assert.strictEqual(classifySubtitle(r), null, r.url);
  }
});

test("the language is guessed from the file name, a parameter or the folder", () => {
  const cases = {
    "https://cdn.x/Sintel.en.vtt": "en",
    "https://cdn.x/sintel_eng.srt": "en",
    "https://cdn.x/subs/English.vtt": "en",
    "https://cdn.x/subs/sintel-de.vtt": "de",
    "https://cdn.x/subs/Spanish%20(Latin).srt": "es",
    "https://cdn.x/subs/sub_pt-BR.vtt": "pt-BR",
    "https://cdn.x/subs/movie.en.hi.srt": "en",
    "https://cdn.x/subs/movie.hi.srt": "hi",
    "https://cdn.x/subs/track.vtt?lang=fr": "fr",
    "https://cdn.x/subs/track.vtt?language=ja-JP": "ja",
    "https://cdn.x/subs/french/track.vtt": "fr",
    "https://cdn.x/subs/abc123.vtt": "",
    "https://cdn.x/video-id.vtt": "",
    "https://cdn.x/de/track.vtt": "",
  };
  for (const [url, lang] of Object.entries(cases)) assert.strictEqual(guessLang(url), lang, url);
});

test("each subtitle file is listed once, and learns its language", () => {
  const list = new SubtitleList(3);
  assert.strictEqual(list.add({ url: "https://cdn.x/a.vtt?range=0-10" }), true);
  assert.strictEqual(list.add({ url: "https://cdn.x/a.vtt?range=20-30", lang: "en" }), false, "the same file");
  assert.deepStrictEqual(list.items, [{ url: "https://cdn.x/a.vtt?range=0-10", lang: "en" }]);
  for (let i = 0; i < 5; i++) list.add({ url: `https://cdn.x/${i}x.vtt` });
  assert.strictEqual(list.items.length, 3, "capped");
});

test("what goes to Snag: the page's tracks (their language wins) and what the player fetched", () => {
  const sniffed = [
    { url: "https://cdn.x/subs/sintel_eng.vtt", lang: "en" },
    { url: "https://cdn.x/subs/other.srt" },
  ];
  const tracks = [
    { url: "https://cdn.x/subs/sintel_eng.vtt", lang: "en-GB", label: " English " },
    { url: "https://cdn.x/subs/sintel_de.vtt", lang: "", label: "" },
    { url: "javascript:alert(1)", lang: "en" },
    null,
  ];
  assert.deepStrictEqual(subtitlesToSend(sniffed, tracks), [
    { url: "https://cdn.x/subs/sintel_eng.vtt", lang: "en-GB", label: "English" },
    { url: "https://cdn.x/subs/sintel_de.vtt", lang: "de" },
    { url: "https://cdn.x/subs/other.srt" },
  ]);
  assert.deepStrictEqual(subtitlesToSend([], []), []);
  assert.strictEqual(subtitlesToSend(Array.from({ length: 40 }, (_, i) => ({ url: `https://cdn.x/${i}a.vtt` })), []).length, 20, "at most 20");
});

test("the page's <track>s: captions and subtitles with web links only", () => {
  const els = [
    { kind: "subtitles", src: "https://cdn.x/en.vtt", srclang: "en", label: "English" },
    { kind: "captions", src: "http://cdn.x/cc.vtt", srclang: "en", label: "English CC" },
    { kind: "", src: "https://cdn.x/plain.vtt", srclang: "", label: "" },
    { kind: "chapters", src: "https://cdn.x/chapters.vtt", srclang: "en" },
    { kind: "metadata", src: "https://cdn.x/thumbs.vtt" },
    { kind: "subtitles", src: "blob:https://site.x/1", srclang: "fr" },
    { kind: "subtitles", src: "data:text/vtt,WEBVTT" },
  ];
  assert.deepStrictEqual(trackList(els), [
    { url: "https://cdn.x/en.vtt", lang: "en", label: "English" },
    { url: "http://cdn.x/cc.vtt", lang: "en", label: "English CC" },
    { url: "https://cdn.x/plain.vtt", lang: "", label: "" },
  ]);
  assert.deepStrictEqual(trackList(undefined), []);
});
