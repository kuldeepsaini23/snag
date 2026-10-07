// "Grab all": what a page links to or shows, filtered down to what's worth downloading.
// Pure functions shared by grab-page.js and the tests.

const TYPES = {
  image: ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "avif", "heic", "tif", "tiff"],
  video: ["mp4", "m4v", "webm", "mkv", "mov", "avi", "flv", "wmv", "ts", "3gp"],
  audio: ["mp3", "m4a", "aac", "flac", "wav", "ogg", "opus", "wma"],
  document: ["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "csv", "epub", "odt", "rtf", "md"],
  archive: ["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "tgz", "iso"],
  program: ["exe", "msi", "apk", "dmg", "deb", "rpm", "appimage"],
};

/** Images smaller than this on both sides are icons, avatars or spacers. */
const MIN_IMAGE_PX = 200;
/** Above this many items nothing is selected for you. */
const PRESELECT_MAX = 200;

const PRESETS = {
  images: { types: ["image"], minImagePx: MIN_IMAGE_PX },
  videos: { types: ["video", "audio"] },
  documents: { types: ["document"] },
  archives: { types: ["archive", "program"] },
  everything: { types: ["image", "video", "audio", "document", "archive", "program"], minImagePx: MIN_IMAGE_PX },
};

function extOf(url) {
  try {
    const name = new URL(url).pathname.split("/").pop() || "";
    const dot = name.lastIndexOf(".");
    return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  } catch (_) {
    return "";
  }
}

function typeOf(url) {
  const ext = extOf(url);
  for (const [type, exts] of Object.entries(TYPES)) if (exts.includes(ext)) return type;
  return "page";
}

/** Web links only, without fragments, each once; with type and extension. */
function normalize(raw) {
  const seen = new Set();
  const out = [];
  for (const item of raw) {
    let url;
    try {
      const u = new URL(item.url);
      if (u.protocol !== "http:" && u.protocol !== "https:") continue;
      u.hash = "";
      url = u.toString();
    } catch (_) {
      continue;
    }
    if (seen.has(url)) continue;
    seen.add(url);
    const type = item.tag === "img" && typeOf(url) === "page" ? "image" : typeOf(url);
    out.push({ url, type, ext: extOf(url), text: (item.text || "").trim().replace(/\s+/g, " "), width: item.width || 0, height: item.height || 0 });
  }
  return out;
}

/**
 * @param {Array} items from `normalize`
 * @param {{types: string[], minImagePx?: number, extensions?: string[], text?: string}} f
 */
function filterItems(items, f) {
  const needle = (f.text || "").trim().toLowerCase();
  const exts = (f.extensions || []).map((e) => e.toLowerCase().replace(/^\./, ""));
  return items.filter((i) => {
    if (!f.types.includes(i.type)) return false;
    // Unknown sizes (lazy images, links to pictures) are kept; known small ones are icons.
    if (i.type === "image" && f.minImagePx && i.width && i.height && i.width < f.minImagePx && i.height < f.minImagePx) return false;
    if (exts.length && !exts.includes(i.ext)) return false;
    if (needle && !(i.url.toLowerCase().includes(needle) || i.text.toLowerCase().includes(needle))) return false;
    return true;
  });
}

/** The URLs ticked at first: all of them, unless there are so many that it's surely too much. */
function preselect(items) {
  return new Set(items.length > PRESELECT_MAX ? [] : items.map((i) => i.url));
}

/** Snag takes at most this many links per request. */
const BATCH_MAX = 1000;

/**
 * Sends `urls` with `sendChunk` in batches of at most `size`, one after another. Stops at the
 * first failure: `{ added, skipped }` so far, plus `error`.
 */
async function sendInChunks(urls, sendChunk, size = BATCH_MAX) {
  const total = { added: 0, skipped: 0 };
  for (let i = 0; i < urls.length; i += size) {
    try {
      const r = await sendChunk(urls.slice(i, i + size));
      total.added += r.added || 0;
      total.skipped += r.skipped || 0;
    } catch (e) {
      return { ...total, error: e.message };
    }
  }
  return total;
}

/** What "Download N" reports; `skipped` were downloaded before, so Snag left them out. */
function batchMessage({ added, skipped = 0, error }) {
  const done = skipped > 0 ? `${added} added, ${skipped} already downloaded` : `Sent ${added} to Snag`;
  if (!error) return `${done} ✓`;
  return added || skipped ? `${done}, then: ${error}` : error;
}

if (typeof module !== "undefined") module.exports = { typeOf, normalize, filterItems, preselect, sendInChunks, batchMessage, PRESETS, PRESELECT_MAX, BATCH_MAX };
