// Snag extension background: finds the desktop app on 127.0.0.1 and hands it links.
// Chrome runs it as a service worker; Firefox loads catch-rules.js before it (see build.js).

if (typeof importScripts === "function") importScripts("catch-rules.js", "sniffer.js");

const PORTS = [47321, 47322, 47323, 47324, 47325, 47326];
const DEFAULTS = { token: "", catchDownloads: true, minSizeMB: 1 };
// Downloads we gave back to Chrome because Snag couldn't take them: don't catch them again.
const handBack = new Set();

async function settings() {
  return { ...DEFAULTS, ...(await chrome.storage.local.get(Object.keys(DEFAULTS))) };
}

/** First port that answers as Snag: { port, paired } or null. */
async function findApp(token) {
  for (const port of PORTS) {
    try {
      const resp = await fetch(`http://127.0.0.1:${port}/ping`, {
        headers: token ? { "X-RDM-Token": token } : {},
        signal: AbortSignal.timeout(800),
      });
      if (!resp.ok) continue;
      const body = await resp.json();
      if (body.app === "rdm") return { port, paired: Boolean(body.paired) };
    } catch (_) {
      // Nothing on this port: try the next one.
    }
  }
  return null;
}

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
      if (!list.add({ ...found, size: found.size || size })) return;
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

/** One-click pairing: Snag asks the user "Allow?", then hands over the code. */
async function pair() {
  const app = await findApp("");
  if (!app) throw new Error("Snag isn't running");
  const resp = await fetch(`http://127.0.0.1:${app.port}/pair`, { method: "POST" });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok || !body.token) throw new Error(body.error || "Not allowed in Snag");
  await chrome.storage.local.set({ token: body.token });
  return body.token;
}

/** The pairing code, connecting first if there is none (Snag asks the user once). */
async function pairedApp() {
  const { token } = await settings();
  let app = await findApp(token);
  if (!app) throw new Error("Snag isn't running");
  if (!app.paired) {
    const fresh = await pair();
    app = await findApp(fresh);
    if (!app || !app.paired) throw new Error("Couldn't connect to Snag");
    return { app, token: fresh };
  }
  return { app, token };
}

async function sendToApp(url, kind, referrer, fallback) {
  const { app, token } = await pairedApp();
  const resp = await fetch(`http://127.0.0.1:${app.port}/add`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify({ url, kind, referrer: referrer || undefined, fallback: fallback || undefined, cookies: await cookiesFor(url) }),
  });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) throw new Error(body.error || `Snag answered ${resp.status}`);
  return body;
}

/** "Grab all": many links from one page in one request. */
async function sendBatch(urls, referrer) {
  const { app, token } = await pairedApp();
  const resp = await fetch(`http://127.0.0.1:${app.port}/add-batch`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify({ urls, referrer, cookies: await cookiesFor(referrer) }),
  });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) throw new Error(body.error || `Snag answered ${resp.status}`);
  return body;
}

/** Brief ✓ / ! on the toolbar icon. */
function flash(ok) {
  chrome.action.setBadgeBackgroundColor({ color: ok ? "#32d74b" : "#ff453a" });
  chrome.action.setBadgeText({ text: ok ? "✓" : "!" });
  setTimeout(() => chrome.action.setBadgeText({ text: "" }), 2500);
}

// Catch new Chrome downloads and move them to Snag.
chrome.downloads.onCreated.addListener(async (item) => {
  const url = item.finalUrl || item.url;
  if (handBack.has(url)) {
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
    // Snag unavailable: let Chrome download it after all.
    console.warn("Snag:", e.message);
    handBack.add(url);
    chrome.downloads.download({ url });
    flash(false);
  }
});

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: "rdm-link", title: "Download with Snag", contexts: ["link", "video", "audio", "image"] });
  chrome.contextMenus.create({ id: "rdm-page", title: "Download this page's video with Snag", contexts: ["page"] });
});

chrome.contextMenus.onClicked.addListener((info) => {
  const page = info.menuItemId === "rdm-page";
  const url = page ? info.pageUrl : info.linkUrl || info.srcUrl;
  sendToApp(url, page ? "media" : undefined, page ? undefined : info.pageUrl)
    .then(() => flash(true))
    .catch((e) => {
      console.warn("Snag:", e.message);
      flash(false);
    });
});

// Popup and in-page button.
chrome.runtime.onMessage.addListener((msg, _sender, reply) => {
  if (msg.type === "send-batch") {
    sendBatch(msg.urls, msg.referrer)
      .then((body) => reply({ ok: true, added: body.added }))
      .catch((e) => reply({ ok: false, error: e.message }));
    return true;
  }
  if (msg.type === "media-list") {
    const tabId = msg.tabId ?? _sender.tab?.id;
    mediaOf(tabId).then((list) => reply({ items: list.items }));
    return true;
  }
  if (msg.type === "send") {
    // Sending a page: the stream it played is the fallback if Snag can't read the page itself.
    const tabId = msg.tabId ?? _sender.tab?.id;
    const best = msg.withFallback && tabId !== undefined ? mediaOf(tabId).then((l) => bestMedia(l.items)) : Promise.resolve(null);
    best
      .then((b) => sendToApp(msg.url, msg.kind, msg.referrer, b && b.url !== msg.url ? b.url : undefined))
      .then((result) => {
        flash(true);
        reply({ ok: true, result });
      })
      .catch((e) => {
        flash(false);
        reply({ ok: false, error: e.message });
      });
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
