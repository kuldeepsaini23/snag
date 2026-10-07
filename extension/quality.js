// The quality menu (in-page panel and popup): pure helpers shared with the tests.

const DEFAULT_ACCENT = "#ff9f0a";

function sizeLabel(bytes) {
  if (!bytes) return "";
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(mb >= 10 ? 0 : 1)} MB`;
}

/** Height of a video option (0 for audio), for sorting and the HD/4K badge. */
function height(format) {
  const inner = format && typeof format === "object" ? format.Video || format.Live : null;
  return (inner && inner.max_height) || 0;
}

/** Snag's options as rows: best video first, audio last. `format` goes back to /add-media. */
function qualityRows(info) {
  return (info.options || [])
    .map((o) => {
      const h = height(o.format);
      const badge = h >= 2160 ? "4K" : h >= 720 ? "HD" : "";
      return { label: o.label, detail: sizeLabel(o.size), badge, format: o.format, height: h };
    })
    .sort((a, b) => b.height - a.height);
}

/** Snag's accent colour, only if it really is a hex colour (it ends up in CSS). */
function safeAccent(value) {
  return typeof value === "string" && /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(value) ? value : DEFAULT_ACCENT;
}

/** Media the page's player already fetched (the sniffer's list) as instant download rows. */
function sniffedRows(items) {
  return items.map((item) => {
    const referrer = item.referrer;
    if (item.kind !== "file") return { label: `Stream (${item.kind.toUpperCase()})`, detail: "", url: item.url, kind: "media", referrer };
    let ext = "";
    try {
      ext = (new URL(item.url).pathname.match(/\.([a-z0-9]{2,4})$/i) || [])[1] || "";
    } catch (_) {
      // Not a URL: no extension.
    }
    const audio = /^(mp3|m4a|aac|ogg|oga|opus|flac|wav)$/i.test(ext);
    const label = ext ? `${ext.toUpperCase()} ${audio ? "audio" : "video"}` : "Media file";
    return { label, detail: sizeLabel(item.size), url: item.url, kind: "file", referrer };
  });
}

/**
 * Snag's reading of a page, started early (prefetch) and shared by whoever asks: one read per
 * page; answers kept for `ttlMs` (the links inside them expire), failures not kept at all.
 */
class ProbeCache {
  constructor(read, clock, ttlMs) {
    this.read = read;
    this.clock = clock;
    this.ttlMs = ttlMs;
    this.entries = new Map();
  }

  get(url) {
    const hit = this.entries.get(url);
    if (hit && this.clock() - hit.at < this.ttlMs) return hit.answer;
    const answer = this.read(url).then(
      (info) => ({ ok: true, info }),
      (e) => {
        this.entries.delete(url);
        return { ok: false, error: e.message };
      },
    );
    this.entries.set(url, { at: this.clock(), answer });
    // Keep the map small: the oldest pages go first.
    if (this.entries.size > 30) this.entries.delete(this.entries.keys().next().value);
    return answer;
  }
}

/** X serves a small preview by default; the same image at full size, with a real extension. */
function fullSizeImage(url) {
  try {
    const u = new URL(url);
    if (u.hostname !== "pbs.twimg.com" || !u.pathname.startsWith("/media/")) return url;
    const ext = u.searchParams.get("format") || (u.pathname.match(/\.(\w+)$/) || [])[1] || "jpg";
    const id = u.pathname.replace(/\.\w+$/, "");
    return `${u.origin}${id}.${ext}?name=orig`;
  } catch (_) {
    return url;
  }
}

/**
 * What a right-click "Download with Snag" sends. An image or a video element's own file goes
 * straight in as a file (instant, like IDM), even when it sits inside a link; a link is left for
 * Snag to sort out; a player with no real file (blob:) sends its page instead.
 */
function contextTarget(info) {
  const page = info.pageUrl;
  if (info.menuItemId === "rdm-page") return { url: page, kind: "media", referrer: undefined };
  if (info.menuItemId === "rdm-save-page") return { url: page, kind: "page", referrer: undefined };
  const media = info.mediaType === "image" || info.mediaType === "video" || info.mediaType === "audio";
  if (media) {
    if (/^https?:/i.test(info.srcUrl || "")) return { url: fullSizeImage(info.srcUrl), kind: "file", referrer: page };
    if (!info.linkUrl) return { url: page, kind: "media", referrer: undefined };
  }
  return { url: info.linkUrl || info.srcUrl || page, kind: undefined, referrer: page };
}

const RELOAD = "Snag's extension was updated: reload this page";

/**
 * `runtime.sendMessage` as a promise that always settles: a page script cut off by an extension
 * reload, a browser error, or no answer within `timeoutMs` all become `{ ok: false, error }`.
 */
function makeAsk(runtime, timeoutMs) {
  return (msg) =>
    new Promise((resolve) => {
      const timer = setTimeout(() => resolve({ ok: false, error: "Snag gave no answer. Is it running?" }), timeoutMs);
      const settle = (r) => {
        clearTimeout(timer);
        resolve(r);
      };
      try {
        runtime.sendMessage(msg, (reply) => {
          if (runtime.lastError || reply === undefined) settle({ ok: false, error: RELOAD });
          else settle(reply);
        });
      } catch (_) {
        settle({ ok: false, error: RELOAD });
      }
    });
}

if (typeof module !== "undefined") module.exports = { sizeLabel, qualityRows, sniffedRows, ProbeCache, safeAccent, makeAsk, contextTarget, fullSizeImage, DEFAULT_ACCENT };
