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

if (typeof module !== "undefined") module.exports = { sizeLabel, qualityRows, safeAccent, makeAsk, DEFAULT_ACCENT };
