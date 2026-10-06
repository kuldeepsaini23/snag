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

function sizeLabel(bytes) {
  if (!bytes) return "";
  const mb = bytes / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb.toFixed(mb >= 10 ? 0 : 1)} MB`;
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
      say(result.ok ? "Sent to Snag ✓" : result.error);
    });
    row.append(tag, name, go);
    box.append(row);
  }
}

async function init() {
  const { token = "", catchDownloads = true } = await chrome.storage.local.get(["token", "catchDownloads"]);
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
    say("Sending…");
    const result = await chrome.runtime.sendMessage({ type: "send", url: tab.url, referrer: tab.url, tabId: tab.id, withFallback: true });
    say(result.ok ? "Sent to Snag ✓" : result.error);
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
}

init();
