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
    $("status").textContent = "Found Snag. Paste the pairing code";
    dot.classList.add("err");
  } else {
    $("status").textContent = `Connected (port ${app.port})`;
    dot.classList.add("ok");
  }
}

async function init() {
  const { token = "", catchDownloads = true } = await chrome.storage.local.get(["token", "catchDownloads"]);
  $("token").value = token;
  $("catch").checked = catchDownloads;

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
    const result = await chrome.runtime.sendMessage({ type: "send", url: tab.url });
    say(result.ok ? "Sent to Snag ✓" : result.error);
  });

  refreshStatus();
}

init();
