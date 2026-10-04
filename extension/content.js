// Floating "Download with RDM" pill on video pages. Lives in a shadow root so page
// styles can't touch it, and follows single-page-app navigation.

(() => {
  const HOST_ID = "rdm-download-pill";

  function isVideoPage() {
    const { hostname, pathname } = location;
    if (hostname.endsWith("youtube.com")) return pathname === "/watch" || pathname.startsWith("/shorts/");
    if (hostname.endsWith("x.com") || hostname.endsWith("twitter.com")) return pathname.includes("/status/");
    if (hostname.endsWith("instagram.com")) return /^\/(p|reel|reels|tv)\//.test(pathname);
    if (hostname.endsWith("tiktok.com")) return pathname.includes("/video/");
    return pathname.length > 1;
  }

  function build() {
    const host = document.createElement("div");
    host.id = HOST_ID;
    const root = host.attachShadow({ mode: "closed" });
    root.innerHTML = `
      <style>
        button {
          position: fixed; top: 80px; right: 20px; z-index: 2147483647;
          display: flex; align-items: center; gap: 7px;
          padding: 9px 14px 9px 12px; border: 0; border-radius: 999px;
          background: #ff9f0a; color: #1a1816; cursor: pointer;
          font: 600 13px/1 "Inter", "Segoe UI", system-ui, sans-serif;
          box-shadow: 0 6px 16px rgba(0, 0, 0, .4);
        }
        button:hover { filter: brightness(1.08); }
        button[disabled] { opacity: .7; cursor: default; }
      </style>
      <button title="Send this video to RDM"><span>↓</span><span class="label">Download with RDM</span></button>`;
    const button = root.querySelector("button");
    const label = root.querySelector(".label");
    button.addEventListener("click", () => {
      button.disabled = true;
      label.textContent = "Sending…";
      chrome.runtime.sendMessage({ type: "send", url: location.href, kind: "media" }, (result) => {
        label.textContent = result && result.ok ? "Sent to RDM ✓" : (result && result.error) || "RDM not reachable";
        setTimeout(() => {
          label.textContent = "Download with RDM";
          button.disabled = false;
        }, 2500);
      });
    });
    return host;
  }

  let lastUrl = "";
  function sync() {
    if (location.href === lastUrl) return;
    lastUrl = location.href;
    const existing = document.getElementById(HOST_ID);
    if (isVideoPage()) {
      if (!existing) document.documentElement.appendChild(build());
    } else if (existing) {
      existing.remove();
    }
  }

  sync();
  setInterval(sync, 1000);
})();
