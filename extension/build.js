// Builds the browser-specific extension folders from this one source:
//   node build.js   ->  dist/chrome  (Chrome, Edge, Brave, Opera, Vivaldi…)
//                       dist/firefox (Firefox, Zen, LibreWolf, Waterfox, Floorp…)
// Load dist/firefox in Firefox via about:debugging -> "Load Temporary Add-on" -> manifest.json.

const fs = require("node:fs");
const path = require("node:path");

const FILES = ["background.js", "catch-rules.js", "sniffer.js", "content.js", "popup.html", "popup.js", "grab.html", "grab.js", "grab-page.js"];

/** Firefox runs MV3 backgrounds as plain scripts (no service worker, no importScripts). */
function firefoxManifest(base) {
  const ff = structuredClone(base);
  ff.background = { scripts: ["catch-rules.js", "sniffer.js", "background.js"] };
  ff.browser_specific_settings = {
    gecko: {
      id: "snag@download-manager",
      strict_min_version: "142.0",
      data_collection_permissions: { required: ["none"] },
    },
  };
  return ff;
}

function build(outRoot) {
  const base = JSON.parse(fs.readFileSync(path.join(__dirname, "manifest.json"), "utf8"));
  for (const [target, manifest] of [["chrome", base], ["firefox", firefoxManifest(base)]]) {
    const dir = path.join(outRoot, target);
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir, { recursive: true });
    for (const file of FILES) fs.copyFileSync(path.join(__dirname, file), path.join(dir, file));
    for (const icon of fs.readdirSync(__dirname).filter((f) => /^icon-\d+\.png$/.test(f))) {
      fs.copyFileSync(path.join(__dirname, icon), path.join(dir, icon));
    }
    fs.writeFileSync(path.join(dir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  }
}

if (require.main === module) {
  build(path.join(__dirname, "dist"));
  console.log("built dist/chrome and dist/firefox");
}

module.exports = { firefoxManifest, build };
