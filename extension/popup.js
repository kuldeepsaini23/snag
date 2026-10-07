const $ = (id) => document.getElementById(id);

function say(text) {
  $("message").textContent = text;
}

async function refreshStatus() {
  const { app } = await chrome.runtime.sendMessage({ type: "status" });
  const dot = $("dot");
  dot.className = "dot";
  if (!app) {
    $("status").textContent = "Snag isn't running";
    dot.classList.add("err");
  } else if (!app.paired) {
    $("status").textContent = "Found Snag: connect to start";
    dot.classList.add("err");
    $("connect").style.display = "block";
  } else {
    $("connect").style.display = "none";
    $("status").textContent = `Connected (port ${app.port})`;
    dot.classList.add("ok");
  }
}

function mediaName(item) {
  try {
    const name = decodeURIComponent(new URL(item.url).pathname.split("/").filter(Boolean).pop() || "");
    if (item.kind === "hls") return name.replace(/\.m3u8$/i, "") || "Video stream";
    if (item.kind === "dash") return name.replace(/\.mpd$/i, "") || "Video stream";
    return name || "Media file";
  } catch (_) {
    return "Media";
  }
}

/** Where the page button sits, and the sites it stays off. */
async function buttonSettings(tab) {
  let host = "";
  try {
    host = tab && /^https?:/.test(tab.url) ? new URL(tab.url).hostname : "";
  } catch (_) {
    // Not a web page.
  }
  const render = async () => {
    const { hiddenSites = [], buttonCorner = "bottom-right" } = await chrome.storage.local.get(["hiddenSites", "buttonCorner"]);
    $("corner").value = buttonCorner;
    $("site-label").textContent = host ? `Show on ${siteKey(host)}` : "Show on this site";
    $("site-on").disabled = !host;
    $("site-on").checked = !host || !isHiddenOn(host, hiddenSites);
    const list = $("hidden-list");
    list.textContent = "";
    if (!hiddenSites.length) return;
    list.append("Hidden on: ");
    for (const site of hiddenSites) {
      const chip = document.createElement("span");
      chip.className = "chip";
      const undo = document.createElement("button");
      undo.textContent = "×";
      undo.title = `Show on ${site} again`;
      undo.addEventListener("click", async () => {
        await chrome.storage.local.set({ hiddenSites: hiddenSites.filter((s) => s !== site) });
        render();
      });
      chip.append(site, undo);
      list.append(chip);
    }
  };
  $("site-on").addEventListener("change", async (e) => {
    const { hiddenSites = [] } = await chrome.storage.local.get(["hiddenSites"]);
    await chrome.storage.local.set({ hiddenSites: e.target.checked ? showSite(hiddenSites, host) : hideSite(hiddenSites, host) });
    render();
  });
  $("corner").addEventListener("change", (e) => chrome.storage.local.set({ buttonCorner: e.target.value }));
  render();
}

/** The page's qualities: pick one and Snag starts it. */
function showQualities(tab, info, sendPage) {
  $("q-title").textContent = info.title || tab.title || "";
  const list = $("q-list");
  list.textContent = "";
  if (info.playlist > 0) {
    const all = document.createElement("button");
    all.className = "q-row";
    all.textContent = `Whole playlist (${info.playlist}) — choose in Snag`;
    all.addEventListener("click", sendPage);
    list.append(all);
  }
  for (const q of qualityRows(info)) {
    const row = document.createElement("button");
    row.className = "q-row";
    const label = document.createElement("span");
    label.textContent = q.label;
    row.append(label);
    if (q.badge) {
      const badge = document.createElement("span");
      badge.className = "tag";
      badge.textContent = q.badge;
      row.append(badge);
    }
    const detail = document.createElement("span");
    detail.className = "detail";
    detail.textContent = q.detail;
    row.append(detail);
    row.addEventListener("click", async () => {
      say(`Adding ${q.label}…`);
      const choice = { url: tab.url, title: info.title || tab.title, format: q.format, thumbnail: info.thumbnail || undefined, duration: info.duration || undefined };
      const r = await chrome.runtime.sendMessage({ type: "add-media", choice });
      say(r.ok ? `Downloading ${q.label} ✓` : r.error);
    });
    list.append(row);
  }
  $("quality").classList.add("has");
}

/** What the page has played or loaded: streams first, then files. */
async function showMedia(tab) {
  const { items } = await chrome.runtime.sendMessage({ type: "media-list", tabId: tab.id });
  const box = $("media-list");
  box.textContent = "";
  $("media").classList.toggle("has", items.length > 0);
  for (const item of items) {
    const row = document.createElement("div");
    row.className = "media-item";
    const tag = document.createElement("span");
    tag.className = "tag";
    tag.textContent = item.kind === "file" ? "FILE" : item.kind.toUpperCase();
    const name = document.createElement("span");
    name.className = "media-name";
    name.title = item.url;
    name.textContent = [mediaName(item), sizeLabel(item.size)].filter(Boolean).join(" · ");
    const go = document.createElement("button");
    go.textContent = "Download";
    go.addEventListener("click", async () => {
      say("Sending…");
      const kind = item.kind === "file" ? "file" : "media";
      const result = await chrome.runtime.sendMessage({ type: "send", url: item.url, kind, referrer: tab.url });
      say(result.ok ? (result.later ? "Saved: goes to Snag when it opens" : "Sent to Snag ✓") : result.error);
    });
    row.append(tag, name, go);
    box.append(row);
  }
}

async function init() {
  const { token = "", catchDownloads = true, accent } = await chrome.storage.local.get(["token", "catchDownloads", "accent"]);
  document.documentElement.style.setProperty("--accent", safeAccent(accent));
  $("token").value = token;
  $("catch").checked = catchDownloads;

  $("connect").addEventListener("click", async () => {
    say("Allow it in the Snag window…");
    const result = await chrome.runtime.sendMessage({ type: "pair" });
    say(result.ok ? "Connected ✓" : result.error);
    if (result.ok) {
      const { token = "" } = await chrome.storage.local.get(["token"]);
      $("token").value = token;
    }
    refreshStatus();
  });

  $("save").addEventListener("click", async () => {
    await chrome.storage.local.set({ token: $("token").value.trim() });
    say("Saved.");
    refreshStatus();
  });

  $("catch").addEventListener("change", async (e) => {
    await chrome.storage.local.set({ catchDownloads: e.target.checked });
    say(e.target.checked ? "New downloads go to Snag." : "Chrome keeps its downloads.");
  });

  $("send").addEventListener("click", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab || !tab.url) return say("No page to send.");
    const sendPage = async () => {
      say("Sending…");
      const result = await chrome.runtime.sendMessage({ type: "send", url: tab.url, referrer: tab.url, tabId: tab.id, withFallback: true });
      say(result.ok ? (result.later ? "Saved: goes to Snag when it opens" : "Sent to Snag ✓") : result.error);
    };
    say("Reading the page…");
    const probe = await chrome.runtime.sendMessage({ type: "probe", url: tab.url });
    if (!probe.ok || !probe.info.options || !probe.info.options.length) return sendPage();
    say("");
    showQualities(tab, probe.info, sendPage);
  });

  $("save-page").addEventListener("click", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab || !/^https?:/.test(tab.url || "")) return say("Only web pages can be saved.");
    say("Saving…");
    const result = await chrome.runtime.sendMessage({ type: "send", url: tab.url, kind: "page", tabId: tab.id });
    say(result.ok ? (result.later ? "Saved: goes to Snag when it opens" : "Saving in Snag (Pages folder) ✓") : result.error);
  });

  $("grab").addEventListener("click", async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab) return;
    chrome.tabs.create({ url: chrome.runtime.getURL(`grab.html?tab=${tab.id}`) });
    window.close();
  });

  refreshStatus();
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab) showMedia(tab);
  buttonSettings(tab);
}

init();
