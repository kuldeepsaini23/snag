const test = require("node:test");
const assert = require("node:assert");
const { firefoxManifest } = require("../build.js");
const base = require("../manifest.json");

test("firefox manifest runs the background as scripts, catch rules first", () => {
  const ff = firefoxManifest(base);
  assert.deepStrictEqual(ff.background, { scripts: ["catch-rules.js", "sniffer.js", "quality.js", "later.js", "connect.js", "background.js"] });
  assert.strictEqual(ff.background.service_worker, undefined);
});

test("firefox manifest has an add-on id and declares no data collection", () => {
  const ff = firefoxManifest(base);
  assert.match(ff.browser_specific_settings.gecko.id, /^[^@\s]+@[^@\s]+$/);
  assert.deepStrictEqual(ff.browser_specific_settings.gecko.data_collection_permissions, { required: ["none"] });
});

test("everything else is the same as the chrome manifest", () => {
  const ff = firefoxManifest(base);
  for (const key of ["name", "version", "permissions", "host_permissions", "action", "content_scripts"]) {
    assert.deepStrictEqual(ff[key], base[key], key);
  }
  assert.deepStrictEqual(base.background, { service_worker: "background.js" }, "the chrome manifest is untouched");
});

test("background.js only calls importScripts where it exists (Firefox has none)", () => {
  const src = require("node:fs").readFileSync(require("node:path").join(__dirname, "..", "background.js"), "utf8");
  const calls = src.split("\n").filter((l) => l.includes("importScripts(") && !l.trim().startsWith("//"));
  assert.ok(calls.length > 0);
  for (const line of calls) assert.match(line, /typeof importScripts === "function"/, line);
});

test("build writes both browser folders", () => {
  const os = require("node:os");
  const fs = require("node:fs");
  const path = require("node:path");
  const out = fs.mkdtempSync(path.join(os.tmpdir(), "rdm-ext-"));
  require("../build.js").build(out);
  for (const target of ["chrome", "firefox"]) {
    for (const f of ["manifest.json", "background.js", "catch-rules.js", "sniffer.js", "connect.js", "content.js", "popup.html", "popup.js"]) {
      assert.ok(fs.existsSync(path.join(out, target, f)), `${target}/${f}`);
    }
  }
  assert.ok(JSON.parse(fs.readFileSync(path.join(out, "firefox", "manifest.json"))).browser_specific_settings);
  fs.rmSync(out, { recursive: true, force: true });
});
