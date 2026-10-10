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

// ---------- subtitles: the caption files a player loads next to its stream ----------

const SUB_EXT = /\.(vtt|srt|ass|ssa)$/i;
const SUB_TYPES = ["text/vtt", "application/x-subrip", "text/srt", "application/srt", "text/x-ssa", "text/x-ass"];
// A stream's own subtitle track comes in numbered pieces (seg-3.vtt, fileSequence12.vtt, 7.vtt):
// none of them is the subtitle file.
const SUB_PIECE = /(^|\/)((seg|segment|chunk|frag|fragment|fileSequence)[-_]?\d+|\d+)\.(vtt|webvtt)$/i;

/** Language names and codes seen in subtitle file names, by language. */
const LANGS = {
  en: ["en", "eng", "english"],
  es: ["es", "spa", "spanish", "espanol"],
  fr: ["fr", "fre", "fra", "french", "francais"],
  de: ["de", "ger", "deu", "german", "deutsch"],
  it: ["it", "ita", "italian", "italiano"],
  pt: ["pt", "por", "portuguese", "portugues"],
  ru: ["ru", "rus", "russian"],
  ja: ["ja", "jpn", "jp", "japanese"],
  ko: ["ko", "kor", "korean"],
  zh: ["zh", "chi", "zho", "chinese"],
  ar: ["ar", "ara", "arabic"],
  hi: ["hi", "hin", "hindi"],
  tr: ["tr", "tur", "turkish"],
  nl: ["nl", "nld", "dut", "dutch"],
  pl: ["pl", "pol", "polish"],
  // Not "id": far more often an identifier than Indonesian.
  id: ["ind", "indonesian"],
  vi: ["vi", "vie", "vietnamese"],
  th: ["th", "tha", "thai"],
  sv: ["sv", "swe", "swedish"],
  uk: ["uk", "ukr", "ukrainian"],
};
const LANG_OF = new Map(Object.entries(LANGS).flatMap(([code, names]) => names.map((n) => [n, code])));
const LANG_PARAMS = ["lang", "language", "srclang", "locale", "sub_lang", "subtitle_lang", "l"];

/**
 * The language a subtitle link names, if any: `en`, `pt-BR`… from its file name (`movie.en.vtt`,
 * `English.srt`, `sub_pt-BR.vtt`), a `?lang=` parameter, or a folder named after a language
 * (`/subs/english/1.vtt`). Two-letter codes count only in the file name, where they mean that.
 */
function guessLang(url) {
  const u = parse(url);
  if (!u) return "";
  let segments;
  try {
    segments = u.pathname.split("/").filter(Boolean).map((s) => decodeURIComponent(s));
  } catch (_) {
    segments = u.pathname.split("/").filter(Boolean);
  }
  const file = (segments.pop() || "").replace(/\.[a-z0-9]+$/i, "");
  // `pt-BR`, `en_US`: a language and a region.
  const region = /(?:^|[._\s-])([a-z]{2})[-_]([A-Z]{2})(?=$|[._\s-])/.exec(file);
  if (region && LANG_OF.get(region[1]) === region[1]) return `${region[1]}-${region[2]}`;
  const found = file.toLowerCase().split(/[._\s\-[\]()]+/).filter((t) => LANG_OF.has(t));
  // `movie.en.hi.srt`: English for the hard of hearing, not Hindi.
  if (found.length > 1 && found[found.length - 1] === "hi") found.pop();
  if (found.length) return LANG_OF.get(found[found.length - 1]);
  for (const p of LANG_PARAMS) {
    const v = (u.searchParams.get(p) || "").toLowerCase().split(/[-_]/)[0];
    if (LANG_OF.has(v)) return LANG_OF.get(v);
  }
  for (const s of segments.reverse()) {
    const t = s.toLowerCase();
    if (t.length > 2 && LANG_OF.has(t)) return LANG_OF.get(t);
  }
  return "";
}

/**
 * A network response that is a subtitle file: by its extension (query strings allowed) or its
 * type. Failed requests and a stream's numbered subtitle pieces aren't.
 * @param {{url: string, contentType?: string, status?: number}} r
 * @returns {null | {url: string, lang?: string}}
 */
function classifySubtitle(r) {
  const u = parse(r.url);
  if (!u) return null;
  if (r.status && (r.status < 200 || r.status >= 300)) return null;
  const type = (r.contentType || "").toLowerCase().split(";")[0].trim();
  if (!SUB_EXT.test(u.pathname) && !SUB_TYPES.includes(type)) return null;
  if (SUB_PIECE.test(u.pathname)) return null;
  const lang = guessLang(r.url);
  return lang ? { url: r.url, lang } : { url: r.url };
}

/** One tab's subtitle files, each once (by address), capped. */
class SubtitleList {
  constructor(cap = 30) {
    this.cap = cap;
    this.items = [];
  }

  add(sub) {
    const key = mediaKey(sub.url);
    const known = this.items.find((s) => mediaKey(s.url) === key);
    if (known) {
      // Seen again with more to say (a language): keep that.
      if (!known.lang && sub.lang) known.lang = sub.lang;
      return false;
    }
    if (this.items.length >= this.cap) return false;
    this.items.push({ ...sub });
    return true;
  }
}

/**
 * What goes to Snag with a stream: the page's own `<track>`s (all of them, though a player
 * fetches only the one shown) and the subtitle files the player fetched. The page's language
 * and label win over a guess from the file name.
 * @param {{url: string, lang?: string}[]} sniffed
 * @param {{url: string, lang?: string, label?: string}[]} tracks
 */
function subtitlesToSend(sniffed, tracks, cap = 20) {
  const list = new SubtitleList(cap);
  for (const t of tracks || []) {
    if (!t || !parse(t.url)) continue;
    const sub = { url: t.url };
    const lang = (t.lang || "").trim() || guessLang(t.url);
    if (lang) sub.lang = lang;
    if (t.label && t.label.trim()) sub.label = t.label.trim();
    list.add(sub);
  }
  for (const s of sniffed || []) list.add(s);
  return list.items;
}

if (typeof module !== "undefined") module.exports = { classifyMedia, mediaKey, MediaList, bestMedia, classifySubtitle, guessLang, SubtitleList, subtitlesToSend };
