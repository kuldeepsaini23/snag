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

if (typeof module !== "undefined") module.exports = { sizeLabel, qualityRows, safeAccent, DEFAULT_ACCENT };
