// Links sent while Snag is closed wait here (chrome.storage.local) and go to Snag, in order,
// as soon as it answers again. Pure helpers shared by background.js and the tests.

/** Adds a link to the waiting list (once; the first time it was saved is kept). */
function addLater(list, entry, now) {
  if (list.some((e) => e.url === entry.url)) return list;
  return [...list, { ...entry, at: now }];
}

/**
 * Sends waiting links one by one with `send`. Stops at the first failure so the order holds;
 * returns what is still waiting.
 */
async function drainLater(list, send) {
  for (let i = 0; i < list.length; i++) {
    try {
      await send(list[i]);
    } catch (_) {
      return list.slice(i);
    }
  }
  return [];
}

function laterBadge(list) {
  if (!list.length) return "";
  return list.length > 99 ? "99+" : String(list.length);
}

if (typeof module !== "undefined") module.exports = { addLater, drainLater, laterBadge };
