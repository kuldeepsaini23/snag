// Which Chrome download events RDM should take over. Shared by background.js and the tests.

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

if (typeof module !== "undefined") module.exports = { shouldCatch };
