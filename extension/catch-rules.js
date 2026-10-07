// Which Chrome download events Snag should take over. Shared by background.js and the tests.

// Chrome replays its whole download history through downloads.onCreated when the
// browser starts. Only downloads that are running and began moments ago are new.
const FRESH_MS = 30 * 1000;

/**
 * @param {object} item  chrome.downloads.DownloadItem
 * @param {{token: string, catchDownloads: boolean, minSizeMB: number}} settings
 * @param {number} now  Date.now()
 */
function shouldCatch(item, settings, now) {
  const url = item.finalUrl || item.url || "";
  if (!settings.catchDownloads || !settings.token) return false;
  if (item.state !== "in_progress") return false;
  const started = Date.parse(item.startTime);
  if (!(now - started >= 0 && now - started < FRESH_MS)) return false;
  if (!/^https?:\/\//.test(url) || item.byExtensionId) return false;
  if ((item.mime || "").startsWith("image/")) return false;
  const size = item.fileSize > 0 ? item.fileSize : item.totalBytes;
  if (size > 0 && size < settings.minSizeMB * 1024 * 1024) return false;
  return true;
}

/**
 * What to download again when Snag couldn't take a caught download: the same link under the
 * name the browser gave it (the file name only, never its folders).
 */
function handBackDownload(item) {
  const url = item.finalUrl || item.url;
  const name = (item.filename || "").split(/[\\/]/).pop();
  return name && name !== "." && name !== ".." ? { url, filename: name } : { url };
}

/** Pages that show a single video/track, where the "Download with Snag" button makes sense. */
function isVideoPage(hostname, pathname) {
  const on = (domain) => hostname === domain || hostname.endsWith("." + domain);
  const parts = pathname.split("/").filter(Boolean);
  if (on("youtube.com")) return pathname === "/watch" || pathname.startsWith("/shorts/");
  if (on("x.com") || on("twitter.com")) return pathname.includes("/status/");
  if (on("instagram.com")) return /^\/(p|reel|reels|tv)\//.test(pathname);
  if (on("tiktok.com")) return pathname.includes("/video/");
  if (on("vimeo.com")) return /^\/\d+/.test(pathname);
  if (on("reddit.com")) return pathname.includes("/comments/");
  if (on("facebook.com")) return pathname === "/watch" || /^\/(watch|reel|videos)\//.test(pathname) || pathname.includes("/videos/");
  if (on("twitch.tv")) return pathname.startsWith("/videos/") || pathname.includes("/clip/");
  if (on("soundcloud.com")) return parts.length >= 2;
  if (on("dailymotion.com")) return pathname.startsWith("/video/");
  return false;
}

if (typeof module !== "undefined") module.exports = { shouldCatch, handBackDownload, isVideoPage };
