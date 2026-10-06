// Media sniffer: which network responses on a page are downloadable media.
// Pure functions shared by background.js and the tests.

const MIN_FILE_BYTES = 1024 * 1024;
// Ads, analytics, and YouTube's own chunk server (YouTube has its own "Download with Snag" button).
const NOISE_HOSTS = ["doubleclick.net", "googlesyndication.com", "google-analytics.com", "googletagmanager.com", "adnxs.com", "moatads.com", "googlevideo.com"];
const FILE_EXT = /\.(mp4|m4v|webm|mkv|mov|avi|flv|mp3|m4a|aac|ogg|oga|opus|flac|wav)$/i;
const SEGMENT_EXT = /\.(ts|m4s|cmfv|cmfa)$/i;
const SEGMENT_NAME = /(^|[/_-])(seg|segment|chunk|frag|fragment)[-_]?\d+/i;
const RANGE_PARAMS = ["range", "bytestart", "byteend", "start", "end"];

function parse(url) {
  try {
    const u = new URL(url);
    return u.protocol === "http:" || u.protocol === "https:" ? u : null;
  } catch (_) {
    return null;
  }
}

/**
 * @param {{url: string, contentType?: string, size?: number, status?: number, contentRange?: string}} r
 * @returns {null | {kind: "hls"|"dash"|"file", url: string, size?: number}}
 */
function classifyMedia(r) {
  const u = parse(r.url);
  if (!u) return null;
  const host = u.hostname.toLowerCase();
  if (NOISE_HOSTS.some((d) => host === d || host.endsWith("." + d))) return null;
  const type = (r.contentType || "").toLowerCase().split(";")[0].trim();
  const path = u.pathname;

  if (/\.m3u8$/i.test(path) || type.includes("mpegurl")) return { kind: "hls", url: r.url };
  // A player that names its playlist in the query (?src=…/master.m3u8): the playlist is the media.
  for (const value of u.searchParams.values()) {
    if (/^https?:\/\/.+\.m3u8(\?|$)/i.test(value)) return { kind: "hls", url: value };
  }
  if (/\.mpd$/i.test(path) || type === "application/dash+xml") return { kind: "dash", url: r.url };
  if (SEGMENT_EXT.test(path) || SEGMENT_NAME.test(path) || type === "video/mp2t" || type.includes("segment")) return null;

  const media = type.startsWith("video/") || type.startsWith("audio/") || FILE_EXT.test(path);
  if (!media) return null;

  let size = r.size || 0;
  if (r.status === 206) {
    // Only the request that starts at byte 0 stands for the file; later ranges are its pieces.
    const m = /bytes\s+(\d+)-\d+\/(\d+|\*)/i.exec(r.contentRange || "");
    if (!m || m[1] !== "0") return null;
    if (m[2] !== "*") size = Number(m[2]);
  }
  if (size > 0 && size < MIN_FILE_BYTES) return null;
  return size > 0 && r.status === 206 ? { kind: "file", url: r.url, size } : { kind: "file", url: r.url };
}

/** What to download if the page itself can't be read: a stream playlist, else the biggest file. */
function bestMedia(items) {
  if (!items.length) return null;
  const playlist = items.find((i) => i.kind !== "file");
  if (playlist) return playlist;
  return items.reduce((a, b) => ((b.size || 0) > (a.size || 0) ? b : a));
}

/** The same media fetched in pieces has range parameters in its query: ignore them. */
function mediaKey(url) {
  const u = parse(url);
  if (!u) return url;
  for (const p of RANGE_PARAMS) u.searchParams.delete(p);
  u.searchParams.sort();
  return u.toString();
}

/** One tab's media, deduplicated and capped; playlists first. */
class MediaList {
  constructor(cap = 50) {
    this.cap = cap;
    this.items = [];
  }

  add(item) {
    const key = mediaKey(item.url);
    if (this.items.some((i) => mediaKey(i.url) === key)) return false;
    const playlist = item.kind !== "file";
    if (this.items.length >= this.cap) {
      // Full: a playlist may push out the last plain file; anything else waits.
      const last = this.items.map((i) => i.kind).lastIndexOf("file");
      if (!playlist || last < 0) return false;
      this.items.splice(last, 1);
    }
    this.items.push(item);
    this.items.sort((a, b) => (a.kind === "file") - (b.kind === "file"));
    return true;
  }
}

if (typeof module !== "undefined") module.exports = { classifyMedia, mediaKey, MediaList, bestMedia };
