// Snag extension background: finds the desktop app on 127.0.0.1 and hands it links.
// Chrome runs it as a service worker; Firefox loads catch-rules.js before it (see build.js).

if (typeof importScripts === "function") importScripts("catch-rules.js", "sniffer.js", "quality.js", "later.js", "connect.js");

const DEFAULTS = { token: "", catchDownloads: true, minSizeMB: 1 };
// Downloads we gave back to Chrome because Snag couldn't take them: don't catch them again.
const handBack = new Set();

async function settings() {
  return { ...DEFAULTS, ...(await chrome.storage.local.get(Object.keys(DEFAULTS))) };
}

const { findApp, pair, pairedApp } = makeConnection((url, opts) => fetch(url, opts), {
  token: async () => (await settings()).token,
  saveToken: (token) => chrome.storage.local.set({ token }),
  saveAccent: (accent) => chrome.storage.local.set({ accent }).catch(() => {}),
});

/** The browser's cookies for `url`, so logged-in and age-restricted downloads work in Snag. */
async function cookiesFor(url) {
  try {
    return await chrome.cookies.getAll({ url });
  } catch (_) {
    return [];
  }
}

// ---------- media sniffer: media playing on any page ----------

/** tabId -> MediaList of what the page fetched (cleared when the tab navigates). Also kept in
 * session storage: the browser stops this worker when idle and the list must survive that. */
const tabMedia = new Map();

async function mediaOf(tabId) {
  let list = tabMedia.get(tabId);
  if (!list) {
    list = new MediaList(50);
    try {
      const saved = (await chrome.storage.session.get(`media:${tabId}`))[`media:${tabId}`] || [];
      for (const item of saved) list.add(item);
    } catch (_) {
      // No session storage: memory only.
    }
    tabMedia.set(tabId, list);
  }
  return list;
}

function forgetMedia(tabId) {
  tabMedia.delete(tabId);
  chrome.storage.session.remove(`media:${tabId}`).catch(() => {});
}

function header(headers, name) {
  const h = (headers || []).find((x) => x.name.toLowerCase() === name);
  return h ? h.value : "";
}

function showCount(tabId) {
  const n = tabMedia.get(tabId)?.items.length || 0;
  chrome.action.setBadgeBackgroundColor({ tabId, color: "#ff9f0a" });
  chrome.action.setBadgeText({ tabId, text: n ? String(n) : "" });
}

chrome.webRequest.onResponseStarted.addListener(
  (d) => {
    if (d.tabId < 0) return;
    const size = Number(header(d.responseHeaders, "content-length")) || 0;
    const found = classifyMedia({
      url: d.url,
      contentType: header(d.responseHeaders, "content-type"),
      size,
      status: d.statusCode,
      contentRange: header(d.responseHeaders, "content-range"),
    });
    if (!found) return;
    mediaOf(d.tabId).then((list) => {
      // The page (or embedded player) that loaded it: video hosts check it as Referer.
      // Firefox gives the full address; Chrome only the origin.
      const referrer = d.documentUrl || (d.initiator && d.initiator !== "null" ? `${d.initiator}/` : undefined);
      if (!list.add({ ...found, size: found.size || size, referrer })) return;
      chrome.storage.session.set({ [`media:${d.tabId}`]: list.items }).catch(() => {});
      showCount(d.tabId);
      // The page shows its "Download with Snag" button once it plays something.
      chrome.tabs.sendMessage(d.tabId, { type: "media-count", n: list.items.length }).catch(() => {});
    });
  },
  { urls: ["<all_urls>"] },
  ["responseHeaders"],
);

chrome.tabs.onUpdated.addListener((tabId, change) => {
  if (change.status === "loading" && change.url) {
    forgetMedia(tabId);
    showCount(tabId);
  }
});
chrome.tabs.onRemoved.addListener((tabId) => forgetMedia(tabId));

/** `mayPair`: the user just clicked, so Snag may ask "Allow?" first (see `pairedApp`). */
async function sendToApp(url, kind, referrer, fallback, { mayPair = false } = {}) {
  const { app, token } = await pairedApp({ mayPair });
  const resp = await fetch(`http://127.0.0.1:${app.port}/add`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify({ url, kind, referrer: referrer || undefined, fallback: fallback || undefined, cookies: await cookiesFor(url) }),
  });
  const body = await resp.json().catch(() => ({}));
  // `permanent`: Snag will never take this link, so a waiting one is dropped (see drainLater).
  if (!resp.ok) throw Object.assign(new Error(body.error || `Snag answered ${resp.status}`), { permanent: isPermanent(resp.status) });
  return body;
}

/** "Grab all": many links from one page in one request. */
async function sendBatch(urls, referrer) {
  const { app, token } = await pairedApp({ mayPair: true });
  const resp = await fetch(`http://127.0.0.1:${app.port}/add-batch`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify({ urls, referrer, cookies: await cookiesFor(referrer) }),
  });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) throw new Error(body.error || `Snag answered ${resp.status}`);
  return body;
}

/**
 * Snag reads a video page and lists its qualities (for the menu in the page / popup). A click
 * sends the page's cookies and may connect first; a prefetch does neither.
 */
async function probeInApp(url, { click = false } = {}) {
  const { app, token } = await pairedApp({ mayPair: click });
  const resp = await fetch(`http://127.0.0.1:${app.port}/probe`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify({ url, cookies: click ? await cookiesFor(url) : [] }),
  });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) throw new Error(body.error || `Snag answered ${resp.status}`);
  return body;
}

// Pages Snag has read (or is reading): the menu opens with them ready. Ten minutes, then the
// links inside go stale.
const probes = new ProbeCache((url, opts) => probeInApp(url, opts), () => Date.now(), 10 * 60 * 1000);

/** The quality picked in the menu: Snag adds it straight away. */
async function addMediaInApp(choice) {
  const { app, token } = await pairedApp({ mayPair: true });
  const resp = await fetch(`http://127.0.0.1:${app.port}/add-media`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify({ ...choice, cookies: await cookiesFor(choice.url) }),
  });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) throw new Error(body.error || `Snag answered ${resp.status}`);
  return body;
}

/** Brief ✓ / ! on the toolbar icon, then back to the count of links waiting for Snag. */
function flash(ok) {
  chrome.action.setBadgeBackgroundColor({ color: ok ? "#32d74b" : "#ff453a" });
  chrome.action.setBadgeText({ text: ok ? "✓" : "!" });
  setTimeout(showLater, 2500);
}

// ---------- links saved while Snag is closed ----------

async function savedLinks() {
  return (await chrome.storage.local.get(["later"])).later || [];
}

/** The toolbar badge shows how many links wait for Snag (a tab's media count shows over it). */
async function showLater() {
  const list = await savedLinks();
  chrome.action.setBadgeBackgroundColor({ color: "#8e8e93" });
  chrome.action.setBadgeText({ text: laterBadge(list) });
}

/** A link the user sent: now, or kept for when Snag runs again. Returns true if it was kept. */
async function sendOrSave(url, kind, referrer, fallback) {
  try {
    await sendToApp(url, kind, referrer, fallback, { mayPair: true });
    return false;
  } catch (e) {
    if (e.message !== "Snag isn't running") throw e;
    const list = addLater(await savedLinks(), { url, kind, referrer, fallback }, Date.now());
    await chrome.storage.local.set({ later: list });
    showLater();
    return true;
  }
}

let draining = null;
/**
 * Hands the waiting links to Snag, in order; whatever fails keeps waiting, except links Snag
 * refuses for good, which are dropped. One run at a time.
 * It runs on its own (every minute), so it never connects: until the user does, links wait.
 */
function sendSaved() {
  draining ??= (async () => {
    const list = await savedLinks();
    if (!list.length) return;
    const left = await drainLater(list, (e) => sendToApp(e.url, e.kind, e.referrer, e.fallback, { mayPair: false }));
    // Links saved while this ran are kept too.
    const now = await savedLinks();
    const sent = new Set(list.slice(0, list.length - left.length).map((e) => e.url));
    await chrome.storage.local.set({ later: now.filter((e) => !sent.has(e.url)) });
    showLater();
  })().finally(() => {
    draining = null;
  });
  return draining;
}

chrome.alarms.create("send-saved", { periodInMinutes: 1 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "send-saved") sendSaved();
});
chrome.runtime.onStartup.addListener(() => sendSaved());
sendSaved();

// Catch new Chrome downloads and move them to Snag.
chrome.downloads.onCreated.addListener(async (item) => {
  const url = item.finalUrl || item.url;
  // One we gave back ourselves, by its first or its final address.
  if (handBack.has(item.url) || handBack.has(url)) {
    handBack.delete(item.url);
    handBack.delete(url);
    return;
  }
  const s = await settings();
  if (!shouldCatch(item, s, Date.now())) return;
  // Only take the download away from the browser when Snag is running and paired.
  const app = await findApp(s.token);
  if (!app || !app.paired) return;

  try {
    await chrome.downloads.cancel(item.id);
    await chrome.downloads.erase({ id: item.id });
  } catch (_) {
    // Already finished or gone: nothing to cancel.
  }
  try {
    await sendToApp(url, "file", item.referrer);
    flash(true);
  } catch (e) {
    // Snag unavailable: let Chrome download it after all, under the name it had.
    console.warn("Snag:", e.message);
    const again = handBackDownload(item);
    handBack.add(again.url);
    chrome.downloads.download(again).catch(() => chrome.downloads.download({ url: again.url }));
    flash(false);
  }
});

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: "rdm-link", title: "Download with Snag", contexts: ["link", "video", "audio", "image"] });
  chrome.contextMenus.create({ id: "rdm-page", title: "Download this page's video with Snag", contexts: ["page"] });
  chrome.contextMenus.create({ id: "rdm-save-page", title: "Save this page with Snag (one offline file)", contexts: ["page"] });
});

chrome.contextMenus.onClicked.addListener((info) => {
  const { url, kind, referrer } = contextTarget(info);
  sendOrSave(url, kind, referrer)
    .then((later) => !later && flash(true))
    .catch((e) => {
      console.warn("Snag:", e.message);
      flash(false);
    });
});

// Popup and in-page button.
chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  if (msg.type === "send-batch") {
    sendBatch(msg.urls, msg.referrer)
      .then((body) => reply({ ok: true, added: body.added, skipped: body.skipped || 0 }))
      .catch((e) => reply({ ok: false, error: e.message }));
    return true;
  }
  if (msg.type === "media-list") {
    const tabId = tabOf(msg, sender);
    mediaOf(tabId).then((list) => reply({ items: list.items }));
    return true;
  }
  if (msg.type === "send") {
    // Sending a page: the stream it played is the fallback if Snag can't read the page itself.
    const tabId = tabOf(msg, sender);
    const best = msg.withFallback && tabId !== undefined ? mediaOf(tabId).then((l) => bestMedia(l.items)) : Promise.resolve(null);
    best
      .then((b) => sendOrSave(msg.url, msg.kind, msg.referrer, b && b.url !== msg.url ? b.url : undefined))
      .then((later) => {
        if (!later) flash(true);
        reply({ ok: true, later });
      })
      .catch((e) => {
        flash(false);
        reply({ ok: false, error: e.message });
      });
    return true;
  }
  if (msg.type === "probe") {
    probes.get(pageKey(msg.url), { click: true }).then(reply);
    return true;
  }
  if (msg.type === "prefetch") {
    // Only when already connected (a prefetch never pops up "Allow?" in Snag), and without the
    // page's cookies: nobody has clicked anything yet.
    probes.get(pageKey(msg.url), { click: false });
    reply({ ok: true });
    return false;
  }
  if (msg.type === "add-media") {
    addMediaInApp(msg.choice)
      .then(() => reply({ ok: true }))
      .catch((e) => reply({ ok: false, error: e.message }));
    return true;
  }
  if (msg.type === "pair") {
    pair()
      .then(() => reply({ ok: true }))
      .catch((e) => reply({ ok: false, error: e.message }));
    return true;
  }
  if (msg.type === "status") {
    settings()
      .then((s) => findApp(s.token))
      .then((app) => reply({ app }));
    return true;
  }
  return false;
});
