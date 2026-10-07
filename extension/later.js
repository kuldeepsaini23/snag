// Links sent while Snag is closed wait here (chrome.storage.local) and go to Snag, in order,
// as soon as it answers again. Pure helpers shared by background.js and the tests.

/** At most this many links wait; past it the oldest are dropped. */
const LATER_MAX = 500;

/** Adds a link to the waiting list (once; the first time it was saved is kept). */
function addLater(list, entry, now) {
  if (list.some((e) => e.url === entry.url)) return list;
  return [...list, { ...entry, at: now }].slice(-LATER_MAX);
}

/**
 * Snag answered with an error about the link itself (not a web link, can't be read…): sending
 * it again won't help. Not pairing (401/403), a timeout (408) or too many requests (429).
 */
function isPermanent(status) {
  return status >= 400 && status < 500 && ![401, 403, 408, 429].includes(status);
}

/**
 * Sends waiting links one by one with `send`. A link Snag refuses for good (`send` throws an
 * error with `permanent: true`) is dropped; any other failure stops the run so the order holds.
 * Returns what is still waiting.
 */
async function drainLater(list, send) {
  for (let i = 0; i < list.length; i++) {
    try {
      await send(list[i]);
    } catch (e) {
      if (e && e.permanent) continue;
      return list.slice(i);
    }
  }
  return [];
}

function laterBadge(list) {
  if (!list.length) return "";
  return list.length > 99 ? "99+" : String(list.length);
}

if (typeof module !== "undefined") module.exports = { addLater, drainLater, laterBadge, isPermanent, LATER_MAX };
