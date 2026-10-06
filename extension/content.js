// Floating "Download with Snag" pill on video pages. Lives in a shadow root so page
// styles can't touch it, and follows single-page-app navigation.

(() => {
  const HOST_ID = "rdm-download-pill";

  function build() {
    const host = document.createElement("div");
    host.id = HOST_ID;
    const root = host.attachShadow({ mode: "closed" });
    root.innerHTML = `
      <style>
        button {
          position: fixed; bottom: 24px; right: 24px; z-index: 2147483647;
          display: flex; align-items: center; gap: 7px;
          padding: 8px 13px 8px 11px; border: 0; border-radius: 999px;
          background: #ff9f0a; color: #1a1816; cursor: pointer;
          font: 600 12.5px/1 "Inter", "Segoe UI", system-ui, sans-serif;
          box-shadow: 0 6px 18px rgba(0, 0, 0, .35);
          opacity: .92; transition: opacity .15s, transform .15s;
        }
        button:hover { opacity: 1; transform: translateY(-1px); }
        button[disabled] { opacity: .7; cursor: default; }
      </style>
      <button title="Send this video to Snag"><span>↓</span><span class="label">Download with Snag</span></button>`;
    const button = root.querySelector("button");
    const label = root.querySelector(".label");
    button.addEventListener("click", () => {
      button.disabled = true;
      label.textContent = "Sending…";
      chrome.runtime.sendMessage({ type: "send", url: location.href, kind: "media", referrer: location.href, withFallback: true }, (result) => {
        // Not connected yet: the background connects first (Snag asks "Allow?") and then sends.
        label.textContent = result && result.ok ? "Sent to Snag ✓" : (result && result.error) || "Snag not reachable";
        setTimeout(() => {
          label.textContent = "Download with Snag";
          button.disabled = false;
        }, 2500);
      });
    });
    return host;
  }

  // Shown on known video pages, and on any page once it plays a video or audio stream.
  let lastUrl = "";
  let mediaSeen = 0;
  function show(on) {
    const existing = document.getElementById(HOST_ID);
    if (on && !existing) document.documentElement.appendChild(build());
    if (!on && existing) existing.remove();
  }
  function sync() {
    if (location.href !== lastUrl) {
      lastUrl = location.href;
      mediaSeen = 0;
    }
    show(isVideoPage(location.hostname, location.pathname) || mediaSeen > 0);
  }
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === "media-count") {
      mediaSeen = msg.n;
      sync();
    }
  });
  chrome.runtime.sendMessage({ type: "media-list" }, (r) => {
    mediaSeen = (r && r.items && r.items.length) || 0;
    sync();
  });
  sync();
  setInterval(sync, 1000);
})();
