// content.js runs in a page: here it runs in a bare VM context with just enough of a fake DOM and
// `chrome` to build the in-page button, take it down and build it again (single-page apps).
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

class FakeNode {
  constructor(tag) {
    this.tag = tag;
    this.id = "";
    this.children = [];
    this.listeners = {};
    this.found = {};
    this.parent = null;
    this.isConnected = true;
    this.textContent = "";
    this.innerHTML = "";
    this.style = { setProperty(k, v) { this[k] = v; } };
    const classes = new Set();
    this.classList = {
      add: (c) => classes.add(c),
      remove: (c) => classes.delete(c),
      contains: (c) => classes.has(c),
      toggle: (c, on) => (on ? classes.add(c) : classes.delete(c)),
    };
  }
  addEventListener(type, fn) {
    (this.listeners[type] ||= []).push(fn);
  }
  dispatch(type, e) {
    for (const fn of this.listeners[type] || []) fn(e);
  }
  attachShadow() {
    this.shadow = new FakeNode("#shadow-root");
    return this.shadow;
  }
  querySelector(sel) {
    return (this.found[sel] ||= new FakeNode(sel));
  }
  append(...nodes) {
    this.children.push(...nodes);
  }
  appendChild(node) {
    this.children.push(node);
    node.parent = this;
    return node;
  }
  remove() {
    if (this.parent) this.parent.children = this.parent.children.filter((c) => c !== this);
    this.parent = null;
  }
  setPointerCapture() {}
  getBoundingClientRect() {
    return { width: 0, height: 0 };
  }
}

function page(href) {
  const sent = [];
  const storageListeners = [];
  let tick = null;
  let shadows = 0;
  const html = new FakeNode("html");
  const document = {
    title: "",
    documentElement: html,
    listeners: {},
    addEventListener(type, fn) {
      (this.listeners[type] ||= []).push(fn);
    },
    getElementById: (id) => html.children.find((c) => c.id === id) || null,
    createElement: (tag) => {
      const node = new FakeNode(tag);
      const attach = node.attachShadow.bind(node);
      node.attachShadow = () => (shadows++, attach());
      return node;
    },
  };
  const location = {};
  const go = (to) => {
    const u = new URL(to);
    Object.assign(location, { href: u.href, hostname: u.hostname, pathname: u.pathname });
  };
  go(href);
  const answers = { status: { app: null }, "media-list": { items: [] }, prefetch: { ok: true }, send: { ok: true, later: true } };
  const chrome = {
    runtime: {
      id: "snag",
      lastError: undefined,
      sendMessage(msg, cb) {
        sent.push(msg.type);
        cb(answers[msg.type] || { ok: true });
      },
      onMessage: { addListener() {} },
    },
    storage: {
      local: { get: async () => ({}), set: async () => {} },
      onChanged: { addListener: (fn) => storageListeners.push(fn) },
    },
  };
  const context = vm.createContext({
    chrome,
    document,
    location,
    window: { innerWidth: 1000, innerHeight: 800 },
    URL,
    setTimeout: () => 0,
    clearTimeout: () => {},
    setInterval: (fn) => ((tick = fn), 1),
    clearInterval: () => {},
  });
  for (const file of ["catch-rules.js", "quality.js", "pill.js", "content.js"]) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, "..", file), "utf8"), context, { filename: file });
  }
  const flush = () => new Promise((r) => setImmediate(r));
  return {
    sent,
    storageListeners,
    document,
    go,
    flush,
    tick: () => tick(),
    shadows: () => shadows,
    host: () => document.getElementById("rdm-download-pill"),
  };
}

test("the button's listeners are registered once, however often a single-page app rebuilds it", async () => {
  const p = page("https://www.youtube.com/watch?v=1");
  await p.flush();
  assert.ok(p.host(), "the button is on a video page");
  const storage = p.storageListeners.length;
  const keys = (p.document.listeners.keydown || []).length;
  for (let i = 2; i <= 4; i++) {
    p.go("https://www.youtube.com/");
    p.tick();
    assert.strictEqual(p.host(), null, "gone off the video page");
    p.go(`https://www.youtube.com/watch?v=${i}`);
    p.tick();
    await p.flush();
  }
  assert.strictEqual(p.shadows(), 4, "the button was built four times");
  assert.strictEqual(p.storageListeners.length, storage, "storage listeners");
  assert.strictEqual((p.document.listeners.keydown || []).length, keys, "keydown listeners");
  assert.ok(keys <= 1);

  // The one listener moves the button on the page now.
  for (const fn of p.storageListeners) fn({ buttonCorner: { newValue: "top-left" } });
  assert.strictEqual(p.host().shadow.querySelector(".pill").style.top, "24px");
});

test("the button's host can't be restyled by the page and ignores clicks a script fakes", async () => {
  const p = page("https://www.youtube.com/watch?v=1");
  await p.flush();
  const css = p.host().style.cssText || "";
  assert.match(css, /all: initial !important/);
  assert.match(css, /position: fixed !important/);
  assert.match(css, /z-index: 2147483647 !important/);

  const pill = p.host().shadow.querySelector(".pill");
  p.sent.length = 0;
  pill.dispatch("click", { isTrusted: false });
  await p.flush();
  assert.deepStrictEqual(p.sent, [], "a script's click does nothing");
  pill.dispatch("click", { isTrusted: true });
  await p.flush();
  assert.ok(p.sent.includes("status"), "the user's click opens the menu");
});
