// Floating "Download" pill on video pages, with a quality menu above it. Lives in a shadow root
// so page styles can't touch it, and follows single-page-app navigation.

(() => {
  const HOST_ID = "rdm-download-pill";

  function build() {
    const host = document.createElement("div");
    host.id = HOST_ID;
    const root = host.attachShadow({ mode: "closed" });
    root.innerHTML = `
      <style>
        :host { --accent: #ff9f0a; }
        * { box-sizing: border-box; font-family: "Inter", "Segoe UI", system-ui, sans-serif; }
        .pill {
          position: fixed; bottom: 24px; right: 24px; z-index: 2147483647;
          display: flex; align-items: center; gap: 7px;
          padding: 9px 14px 9px 12px; border: 0; border-radius: 999px;
          background: var(--accent); color: #1a1816; cursor: pointer;
          font-size: 12.5px; font-weight: 650; line-height: 1;
          box-shadow: 0 8px 22px rgba(0, 0, 0, .35);
          opacity: .94; transition: opacity .15s, transform .18s cubic-bezier(.2,.8,.2,1);
        }
        .pill:hover { opacity: 1; transform: translateY(-2px); }
        .pill[disabled] { opacity: .75; cursor: default; transform: none; }
        .panel {
          position: fixed; bottom: 70px; right: 24px; z-index: 2147483647; width: 292px;
          background: #1f1d1b; color: #ffffffe5; border: 1px solid #ffffff17; border-radius: 14px;
          box-shadow: 0 18px 50px rgba(0, 0, 0, .5); padding: 12px; font-size: 13px;
          opacity: 0; transform: translateY(8px) scale(.98); transform-origin: bottom right;
          transition: opacity .16s ease-out, transform .18s cubic-bezier(.2,.8,.2,1); pointer-events: none;
        }
        .panel.open { opacity: 1; transform: none; pointer-events: auto; }
        .head { display: flex; gap: 10px; align-items: center; margin-bottom: 10px; }
        .thumb { width: 64px; height: 36px; border-radius: 6px; object-fit: cover; background: #ffffff10; flex: none; }
        .title { font-weight: 600; font-size: 12.5px; line-height: 1.3; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
        .close { margin-left: auto; background: none; border: 0; color: #ffffff80; cursor: pointer; font-size: 16px; padding: 2px 4px; }
        .row {
          display: flex; align-items: center; gap: 8px; width: 100%; border: 0; text-align: left;
          background: #ffffff08; color: inherit; border-radius: 9px; padding: 9px 10px; margin-top: 4px;
          cursor: pointer; font-size: 13px; transition: background .12s;
        }
        .row:hover { background: color-mix(in srgb, var(--accent) 22%, transparent); }
        .row .label { font-weight: 600; }
        .row .detail { margin-left: auto; color: #ffffff80; font-size: 12px; }
        .badge { font-size: 9.5px; font-weight: 700; color: #1a1816; background: var(--accent); border-radius: 4px; padding: 1px 4px; }
        .note { color: #ffffff91; font-size: 12px; padding: 6px 2px; }
        .spinner { width: 14px; height: 14px; border: 2px solid #ffffff30; border-top-color: var(--accent); border-radius: 50%; animation: spin .8s linear infinite; display: inline-block; vertical-align: -2px; margin-right: 6px; }
        @keyframes spin { to { transform: rotate(360deg); } }
        @media (prefers-reduced-motion: reduce) { .pill, .panel { transition: none; } .spinner { animation: none; } }
      </style>
      <div class="panel" role="dialog" aria-label="Choose a quality"></div>
      <button class="pill" title="Download with Snag"><svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v11M6.5 10.5 12 16l5.5-5.5M5 20h14"/></svg><span class="label">Download</span></button>`;
    const pill = root.querySelector(".pill");
    const panel = root.querySelector(".panel");

    chrome.storage.local.get(["accent"]).then(({ accent }) => host.style.setProperty("--accent", safeAccent(accent)));

    const el = (tag, cls, text) => {
      const e = document.createElement(tag);
      if (cls) e.className = cls;
      if (text !== undefined) e.textContent = text;
      return e;
    };
    const close = () => panel.classList.remove("open");
    const note = (content) => {
      panel.textContent = "";
      const n = el("div", "note");
      if (typeof content === "string") n.textContent = content;
      else n.append(...content);
      panel.append(n);
      panel.classList.add("open");
    };
    const done = (text) => {
      note(text);
      setTimeout(close, 1800);
    };

    // Snag couldn't list qualities: hand it the page (and the stream it played) as before.
    const sendPage = () => {
      note([el("span", "spinner"), "Sending to Snag…"]);
      chrome.runtime.sendMessage({ type: "send", url: location.href, kind: "media", referrer: location.href, withFallback: true }, (r) => {
        done(r && r.ok ? "Sent to Snag ✓" : (r && r.error) || "Snag isn't running");
      });
    };

    const showChoices = (info) => {
      panel.textContent = "";
      const head = el("div", "head");
      if (info.thumbnail) {
        const img = el("img", "thumb");
        img.src = info.thumbnail;
        img.alt = "";
        head.append(img);
      }
      head.append(el("div", "title", info.title || document.title));
      const x = el("button", "close", "×");
      x.title = "Close";
      x.addEventListener("click", close);
      head.append(x);
      panel.append(head);
      if (info.playlist > 0) {
        const all = el("button", "row");
        all.append(el("span", "label", `Whole playlist (${info.playlist})`), el("span", "detail", "choose in Snag"));
        all.addEventListener("click", sendPage);
        panel.append(all);
      }
      for (const q of qualityRows(info)) {
        const row = el("button", "row");
        row.append(el("span", "label", q.label));
        if (q.badge) row.append(el("span", "badge", q.badge));
        row.append(el("span", "detail", q.detail));
        row.addEventListener("click", () => {
          note([el("span", "spinner"), `Adding ${q.label}…`]);
          const choice = { url: location.href, title: info.title || document.title, format: q.format, thumbnail: info.thumbnail || undefined, duration: info.duration || undefined };
          chrome.runtime.sendMessage({ type: "add-media", choice }, (r) => done(r && r.ok ? `Downloading ${q.label} ✓` : (r && r.error) || "Snag isn't running"));
        });
        panel.append(row);
      }
      panel.classList.add("open");
    };

    pill.addEventListener("click", () => {
      if (panel.classList.contains("open")) return close();
      note([el("span", "spinner"), "Reading the video…"]);
      pill.disabled = true;
      chrome.runtime.sendMessage({ type: "probe", url: location.href, referrer: location.href }, (r) => {
        pill.disabled = false;
        if (r && r.ok && r.info.options && r.info.options.length) showChoices(r.info);
        else if (r && r.ok === false && /isn't running|not allowed|connect/i.test(r.error || "")) done(r.error);
        else sendPage();
      });
    });
    document.addEventListener("keydown", (e) => e.key === "Escape" && close());
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
