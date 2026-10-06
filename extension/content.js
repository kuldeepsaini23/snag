// Floating "Download" pill on video pages, with a quality menu above it. Lives in a shadow root
// so page styles can't touch it, and follows single-page-app navigation.

(() => {
  const HOST_ID = "rdm-download-pill";
  // Reading a page can wait for "Allow" in Snag (up to two minutes) and then yt-dlp.
  const ask = makeAsk(chrome.runtime, 150000);
  const askQuick = makeAsk(chrome.runtime, 5000);
  // A button left by an older copy of this script (the extension was updated) is dead: replace it.
  document.getElementById(HOST_ID)?.remove();

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
      ask({ type: "send", url: location.href, kind: "media", referrer: location.href, withFallback: true }).then((r) => done(r.ok ? "Sent to Snag ✓" : r.error));
    };

    const header = (info) => {
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
      return head;
    };

    // Snag's qualities for the page (ready at once when the prefetch finished).
    const qualityList = (info) => {
      const box = el("div");
      if (info.playlist > 0) {
        const all = el("button", "row");
        all.append(el("span", "label", `Whole playlist (${info.playlist})`), el("span", "detail", "choose in Snag"));
        all.addEventListener("click", sendPage);
        box.append(all);
      }
      for (const q of qualityRows(info)) {
        const row = el("button", "row");
        row.append(el("span", "label", q.label));
        if (q.badge) row.append(el("span", "badge", q.badge));
        row.append(el("span", "detail", q.detail));
        row.addEventListener("click", () => {
          note([el("span", "spinner"), `Adding ${q.label}…`]);
          const choice = { url: location.href, title: info.title || document.title, format: q.format, thumbnail: info.thumbnail || undefined, duration: info.duration || undefined };
          ask({ type: "add-media", choice }).then((r) => done(r.ok ? `Downloading ${q.label} ✓` : r.error));
        });
        box.append(row);
      }
      return box;
    };

    // What the player already fetched: starts downloading at once, like IDM's button.
    const sniffedList = (items) => {
      const box = el("div");
      for (const s of sniffedRows(items)) {
        const row = el("button", "row");
        row.append(el("span", "label", s.label), el("span", "badge", "NOW"), el("span", "detail", s.detail));
        row.title = s.url;
        row.addEventListener("click", () => {
          note([el("span", "spinner"), "Starting…"]);
          const referrer = s.referrer || location.href;
          // A stream goes straight in at the best quality (no reading first, no picker), like IDM.
          const msg =
            s.kind === "media"
              ? { type: "add-media", choice: { url: s.url, title: document.title || "Video", format: { Video: { max_height: 4320 } }, referrer } }
              : { type: "send", url: s.url, kind: s.kind, referrer };
          ask(msg).then((r) => done(r.ok ? "Downloading in Snag ✓" : r.error));
        });
        box.append(row);
      }
      return box;
    };

    pill.addEventListener("click", async () => {
      if (panel.classList.contains("open")) return close();
      const status = await askQuick({ type: "status" });
      if (status.ok === false) return done(status.error);
      if (!status.app) return done("Snag isn't running: start it and try again");

      // Open at once with what's known; Snag's full quality list fills in when ready.
      const media = await askQuick({ type: "media-list" });
      const items = (media.items || []).slice(0, 6);
      panel.textContent = "";
      panel.append(header({}));
      if (items.length) panel.append(sniffedList(items));
      const pending = el("div", "note");
      pending.append(el("span", "spinner"), status.app.paired ? "Finding all qualities…" : "Click Allow in the Snag window…");
      panel.append(pending);
      panel.classList.add("open");

      const r = await ask({ type: "probe", url: location.href });
      if (!panel.isConnected || !panel.classList.contains("open")) return;
      if (r.ok && r.info.options && r.info.options.length) {
        panel.replaceChild(header(r.info), panel.firstChild);
        pending.replaceWith(qualityList(r.info));
      } else if (items.length) {
        pending.textContent = "Pick one above: Snag can't list qualities for this page.";
      } else if (!r.ok && /No video found/.test(r.error || "")) {
        pending.textContent = "Play the video first, then click Download: Snag picks up what the player loads.";
      } else if (!r.ok && /reload this page|no answer|isn't running|not allowed|connect/i.test(r.error || "")) {
        done(r.error);
      } else {
        sendPage();
      }
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
  let prefetched = "";
  function sync() {
    // Cut off by an extension update: leave the page to the new copy of this script.
    if (!chrome.runtime?.id) return clearInterval(timer);
    if (location.href !== lastUrl) {
      lastUrl = location.href;
      mediaSeen = 0;
    }
    const on = isVideoPage(location.hostname, location.pathname) || mediaSeen > 0;
    show(on);
    // Snag starts reading the page now, so the qualities are ready by the time you click.
    if (on && prefetched !== location.href) {
      prefetched = location.href;
      askQuick({ type: "prefetch", url: location.href });
    }
  }
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === "media-count") {
      mediaSeen = msg.n;
      sync();
    }
  });
  askQuick({ type: "media-list" }).then((r) => {
    mediaSeen = (r.items && r.items.length) || 0;
    sync();
  });
  const timer = setInterval(sync, 1000);
  sync();
})();
