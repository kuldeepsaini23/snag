// RDM extension service worker: finds the desktop app on 127.0.0.1 and hands it links.

importScripts("catch-rules.js");

const PORTS = [47321, 47322, 47323, 47324, 47325, 47326];
const DEFAULTS = { token: "", catchDownloads: true, minSizeMB: 1 };
// Downloads we gave back to Chrome because RDM couldn't take them: don't catch them again.
const handBack = new Set();

async function settings() {
  return { ...DEFAULTS, ...(await chrome.storage.local.get(Object.keys(DEFAULTS))) };
}

/** First port that answers as RDM: { port, paired } or null. */
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

async function sendToApp(url, kind) {
  const { token } = await settings();
  const app = await findApp(token);
  if (!app) throw new Error("RDM isn't running");
  if (!app.paired) throw new Error("Not paired: paste the pairing code from RDM → Settings");
  const resp = await fetch(`http://127.0.0.1:${app.port}/add`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-RDM-Token": token },
    body: JSON.stringify(kind ? { url, kind } : { url }),
  });
  const body = await resp.json().catch(() => ({}));
  if (!resp.ok) throw new Error(body.error || `RDM answered ${resp.status}`);
  return body;
}

/** Brief ✓ / ! on the toolbar icon. */
function flash(ok) {
  chrome.action.setBadgeBackgroundColor({ color: ok ? "#32d74b" : "#ff453a" });
  chrome.action.setBadgeText({ text: ok ? "✓" : "!" });
  setTimeout(() => chrome.action.setBadgeText({ text: "" }), 2500);
}

// Catch new Chrome downloads and move them to RDM.
chrome.downloads.onCreated.addListener(async (item) => {
  const url = item.finalUrl || item.url;
  if (handBack.has(url)) {
    handBack.delete(url);
    return;
  }
  if (!shouldCatch(item, await settings(), Date.now())) return;

  try {
    await chrome.downloads.cancel(item.id);
    await chrome.downloads.erase({ id: item.id });
  } catch (_) {
    // Already finished or gone: nothing to cancel.
  }
  try {
    await sendToApp(url, "file");
    flash(true);
  } catch (e) {
    // RDM unavailable: let Chrome download it after all.
    console.warn("RDM:", e.message);
    handBack.add(url);
    chrome.downloads.download({ url });
    flash(false);
  }
});

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: "rdm-link", title: "Download with RDM", contexts: ["link", "video", "audio", "image"] });
  chrome.contextMenus.create({ id: "rdm-page", title: "Download this page's video with RDM", contexts: ["page"] });
});

chrome.contextMenus.onClicked.addListener((info) => {
  const page = info.menuItemId === "rdm-page";
  const url = page ? info.pageUrl : info.linkUrl || info.srcUrl;
  sendToApp(url, page ? "media" : undefined)
    .then(() => flash(true))
    .catch((e) => {
      console.warn("RDM:", e.message);
      flash(false);
    });
});

// Popup and in-page button.
chrome.runtime.onMessage.addListener((msg, _sender, reply) => {
  if (msg.type === "send") {
    sendToApp(msg.url, msg.kind)
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
  if (msg.type === "status") {
    settings()
      .then((s) => findApp(s.token))
      .then((app) => reply({ app }));
    return true;
  }
  return false;
});
