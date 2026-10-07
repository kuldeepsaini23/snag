// The "Grab from page" tab: lists what the page links to or shows, lets you pick, sends the picks to Snag.

const $ = (id) => document.getElementById(id);
const tabId = Number(new URLSearchParams(location.search).get("tab"));
let all = [];
let shown = [];
let picked = new Set();
let preset = "everything";
let pageUrl = "";

/** Runs inside the page: every link, image and media source (no network requests). */
function collectPage() {
  const out = [];
  const largest = (srcset) =>
    (srcset || "")
      .split(",")
      .map((s) => s.trim().split(/\s+/))
      .filter((p) => p[0])
      .map(([u, w]) => [u, parseFloat(w) || 1])
      .sort((a, b) => b[1] - a[1])[0]?.[0];
  for (const a of document.querySelectorAll("a[href]")) out.push({ url: a.href, tag: "a", text: a.textContent || a.title || "" });
  for (const img of document.images) {
    const src = largest(img.srcset) ? new URL(largest(img.srcset), location.href).href : img.currentSrc || img.src;
    out.push({ url: src, tag: "img", text: img.alt || "", width: img.naturalWidth, height: img.naturalHeight });
  }
  for (const m of document.querySelectorAll("video[src], audio[src], video source[src], audio source[src]")) out.push({ url: m.src, tag: "media", text: "" });
  for (const s of document.querySelectorAll("picture source[srcset]")) {
    const u = largest(s.srcset);
    if (u) out.push({ url: new URL(u, location.href).href, tag: "img", text: "" });
  }
  return out;
}

function nameOf(item) {
  try {
    const last = decodeURIComponent(new URL(item.url).pathname.split("/").filter(Boolean).pop() || "");
    return last || new URL(item.url).hostname;
  } catch (_) {
    return item.url;
  }
}

function render() {
  const exts = $("exts").value.split(/[\s,]+/).filter(Boolean);
  shown = filterItems(all, { ...PRESETS[preset], extensions: exts, text: $("text").value });
  const rows = $("rows");
  rows.textContent = "";
  if (!shown.length) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 5;
    td.className = "empty";
    td.textContent = "Nothing of this kind on the page.";
    tr.append(td);
    rows.append(tr);
  }
  for (const item of shown) {
    const tr = document.createElement("tr");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = picked.has(item.url);
    box.addEventListener("change", () => {
      box.checked ? picked.add(item.url) : picked.delete(item.url);
      count();
    });
    const thumb = document.createElement("td");
    thumb.className = "thumb";
    if (item.type === "image") {
      const img = document.createElement("img");
      img.loading = "lazy";
      img.src = item.url;
      thumb.append(img);
    }
    const name = document.createElement("td");
    name.className = "name";
    name.textContent = item.text || nameOf(item);
    const small = document.createElement("small");
    small.textContent = item.url;
    name.append(small);
    const type = document.createElement("td");
    const tag = document.createElement("span");
    tag.className = "type";
    tag.textContent = item.ext || item.type; // from the page's URLs: text only, never markup
    type.append(tag);
    const size = document.createElement("td");
    size.textContent = item.width && item.height ? `${item.width}×${item.height}` : "";
    const c0 = document.createElement("td");
    c0.append(box);
    tr.append(c0, thumb, name, type, size);
    rows.append(tr);
  }
  count();
}

function count() {
  const n = shown.filter((i) => picked.has(i.url)).length;
  $("go").textContent = `Download ${n}`;
  $("go").disabled = n === 0;
  $("all").checked = n > 0 && n === shown.length;
}

function setPreset(name) {
  preset = name;
  for (const b of document.querySelectorAll(".preset")) b.classList.toggle("on", b.dataset.preset === name);
  render();
}

async function init() {
  const tab = await chrome.tabs.get(tabId);
  pageUrl = tab.url;
  $("page").textContent = tab.title ? `${tab.title} — ${tab.url}` : tab.url;
  let raw = [];
  try {
    const [result] = await chrome.scripting.executeScript({ target: { tabId }, func: collectPage });
    raw = result?.result || [];
  } catch (e) {
    $("message").textContent = `Can't read this page: ${e.message}`;
  }
  all = normalize(raw);
  picked = preselect(filterItems(all, PRESETS.everything));
  if (all.length > PRESELECT_MAX) $("message").textContent = `${all.length} items: pick what you want (nothing is ticked for you).`;
  for (const b of document.querySelectorAll(".preset")) b.addEventListener("click", () => setPreset(b.dataset.preset));
  $("exts").addEventListener("input", render);
  $("text").addEventListener("input", render);
  $("all").addEventListener("change", (e) => {
    for (const i of shown) e.target.checked ? picked.add(i.url) : picked.delete(i.url);
    render();
  });
  $("go").addEventListener("click", async () => {
    const urls = shown.filter((i) => picked.has(i.url)).map((i) => i.url);
    if (urls.length > 100 && !confirm(`Download ${urls.length} files?`)) return;
    $("message").textContent = "Sending…";
    const result = await sendInChunks(urls, async (chunk) => {
      const r = await chrome.runtime.sendMessage({ type: "send-batch", urls: chunk, referrer: pageUrl });
      if (!r.ok) throw new Error(r.error);
      return r;
    });
    $("message").textContent = batchMessage(result);
  });
  setPreset(all.some((i) => i.type !== "image" && i.type !== "page") ? "everything" : "images");
}

init();
