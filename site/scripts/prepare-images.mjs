// Turns the repo's screenshots and logo into the web-sized images in public/.
// Run with `bun run images` after a screenshot changes; the output is committed.
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const site = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repo = path.resolve(site, "..");
const shots = path.join(repo, "docs", "images");
const store = path.join(repo, "docs", "store", "screenshots");
const engraving = path.join(repo, "docs", "logo", "engraving", "c-gpt.png");
const logo = path.join(repo, "crates", "app", "assets", "logo", "snag-256.png");
const out = path.join(site, "public", "images");
const app = path.join(site, "src", "app");

await mkdir(out, { recursive: true });

async function webp(input, name, widths) {
  for (const width of widths) {
    await sharp(input)
      .resize({ width, withoutEnlargement: true })
      .webp({ quality: 82, effort: 6 })
      .toFile(path.join(out, `${name}-${width}.webp`));
  }
}

// Screenshots with the new logo, from the store listing (1280x800).
await webp(path.join(store, "3-snag-app.png"), "app", [1280, 720]);
// Phones get the left of the window (library and downloads) at its own size, in 4:3.
await webp(
  await sharp(path.join(store, "3-snag-app.png")).extract({ left: 0, top: 0, width: 732, height: 549 }).toBuffer(),
  "app-phone",
  [732],
);
// The extension: the video page with its Download button, whole, and the popup on its own. They
// are 1x captures, so they are served at their own size at most, never scaled up.
await webp(path.join(store, "1-download-menu.png"), "ext-download-menu", [1280, 720]);
await webp(
  await sharp(path.join(store, "2-popup.png")).extract({ left: 918, top: 46, width: 350, height: 640 }).toBuffer(),
  "ext-popup-panel",
  [350],
);

// The older screenshots still carry the old logo in their title bar, so only the part
// without it is used: the stats panel, and the first-run tour's permissions card.
await webp(
  await sharp(path.join(shots, "stats.png")).extract({ left: 243, top: 57, width: 1129, height: 778 }).toBuffer(),
  "stats",
  [1129, 720],
);
await webp(
  await sharp(path.join(shots, "permissions.png")).extract({ left: 401, top: 104, width: 580, height: 656 }).toBuffer(),
  "permissions",
  [580],
);

// The engraving: the hook and arrow cut out of their orange, so they hang on the dark page.
// Each pixel's distance from the orange sets its opacity, and edge pixels are un-mixed from
// the orange so no fringe is left (the same method as docs/logo/make_logo.py).
async function cutOut() {
  const { data, info } = await sharp(engraving).removeAlpha().raw().toBuffer({ resolveWithObject: true });
  const { width, height } = info;
  const corner = [];
  for (const [x0, y0] of [
    [0, 0],
    [0, height - 40],
    [width - 40, height - 40],
  ]) {
    for (let y = y0; y < y0 + 40; y++) {
      for (let x = x0; x < x0 + 40; x++) corner.push((y * width + x) * 3);
    }
  }
  const bg = [0, 1, 2].map((c) => {
    const values = corner.map((i) => data[i + c]).sort((a, b) => a - b);
    return values[values.length >> 1];
  });
  const rgba = Buffer.alloc(width * height * 4);
  for (let p = 0; p < width * height; p++) {
    const px = [data[p * 3], data[p * 3 + 1], data[p * 3 + 2]];
    const dist = Math.hypot(px[0] - bg[0], px[1] - bg[1], px[2] - bg[2]);
    const a = Math.min(1, Math.max(0, (dist - 60) / 70));
    for (let c = 0; c < 3; c++) {
      const v = a > 0.01 ? (px[c] - (1 - a) * bg[c]) / Math.max(a, 0.01) : 0;
      rgba[p * 4 + c] = Math.min(255, Math.max(0, Math.round(v)));
    }
    rgba[p * 4 + 3] = Math.round(a * 255);
  }
  return sharp(rgba, { raw: { width, height, channels: 4 } }).png().toBuffer();
}

const subject = await sharp(await cutOut()).trim({ threshold: 1 }).png().toBuffer();
for (const height of [972, 560]) {
  await sharp(subject)
    .resize({ height })
    .webp({ quality: 86, alphaQuality: 90, effort: 6 })
    .toFile(path.join(out, `hook-${height}.webp`));
}
const hookMeta = await sharp(subject).metadata();
console.log("hook art", hookMeta.width, "x", hookMeta.height);

// Logo: the mark used in the header, plus the icons Next.js picks up from src/app.
await sharp(logo).resize(96).png().toFile(path.join(out, "logo-96.png"));
await sharp(logo).resize(256).png().toFile(path.join(app, "icon.png"));
await sharp(logo).resize(180).png().toFile(path.join(app, "apple-icon.png"));

// Guilloche, the turned line-work of banknotes: strands of the same wave, each shifted a
// little, so they cross and form a rope. Written as SVG files the page uses as CSS masks, so
// the lines take whatever colour the page gives them (and follow the accent picker).
function polyline(points) {
  return `M${points.map(([x, y]) => `${Math.round(x)} ${Math.round(y)}`).join("L")}Z`;
}
// A ring of `strands` closed curves around (cx, cy), each a circle of radius r waved `lobes` times.
function rosetteRing(cx, cy, r, amp, lobes, strands, steps) {
  const paths = [];
  for (let s = 0; s < strands; s++) {
    const phase = (s / strands) * Math.PI * 2;
    const pts = [];
    for (let i = 0; i < steps; i++) {
      const t = (i / steps) * Math.PI * 2;
      const rad = r + amp * Math.sin(lobes * t + phase);
      pts.push([cx + rad * Math.cos(t), cy + rad * Math.sin(t)]);
    }
    paths.push(polyline(pts));
  }
  return paths.join("");
}
const rosette = (size) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}"><g fill="none" stroke="#000" stroke-width="1.1">` +
  `<path d="${rosetteRing(size / 2, size / 2, size * 0.34, size * 0.085, 12, 16, 288)}"/>` +
  `<path d="${rosetteRing(size / 2, size / 2, size * 0.19, size * 0.05, 9, 12, 216)}"/>` +
  `<circle cx="${size / 2}" cy="${size / 2}" r="${size * 0.465}"/><circle cx="${size / 2}" cy="${size / 2}" r="${size * 0.475}"/>` +
  `<circle cx="${size / 2}" cy="${size / 2}" r="${size * 0.1}"/>` +
  `</g></svg>`;
await writeFile(path.join(out, "rosette.svg"), rosette(1000));

// A horizontal band that tiles: eight strands of one wave, between two rules.
function band(width, height, strands) {
  const mid = height / 2;
  const amp = height * 0.34;
  let d = "";
  for (let s = 0; s < strands; s++) {
    const phase = (s / strands) * Math.PI;
    const pts = [];
    for (let x = 0; x <= width; x += 2) {
      const t = (x / width) * Math.PI * 4;
      pts.push([x, mid + amp * Math.sin(t + phase) * Math.cos(t / 2 - phase / 2)]);
    }
    d += `M${pts.map(([x, y]) => `${x} ${y.toFixed(1)}`).join("L")}`;
  }
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" preserveAspectRatio="none">` +
    `<g fill="none" stroke="#000" stroke-width="0.9"><path d="${d}"/>` +
    `<path d="M0 0.5H${width}M0 ${height - 0.5}H${width}"/></g></svg>`
  );
}
await writeFile(path.join(out, "band.svg"), band(240, 36, 8));

// Open Graph card, 1200x630: the headline on the left, the engraving hanging on the right,
// over the same rosette as the page.
const W = 1200;
const H = 630;
const hook = await sharp(subject).resize({ height: 560 }).png().toBuffer();
const hookWidth = (await sharp(hook).metadata()).width;
const ogRosette = (stroke, opacity, scale) =>
  sharp(Buffer.from(rosette(1000).replace('stroke="#000"', `stroke="${stroke}" stroke-opacity="${opacity}"`)))
    .resize(scale)
    .png()
    .toBuffer();
const svg = `
<svg width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="#0b0b0c"/>
  <rect x="24" y="24" width="${W - 48}" height="${H - 48}" fill="none" stroke="#efe9dc" stroke-opacity="0.22"/>
  <rect x="30" y="30" width="${W - 60}" height="${H - 60}" fill="none" stroke="#efe9dc" stroke-opacity="0.12"/>
  <text x="72" y="104" font-family="Consolas, monospace" font-size="20" letter-spacing="3" fill="#d9682b">SNAG · FOR WINDOWS</text>
  <text x="72" y="236" font-family="Segoe UI, Arial, sans-serif" font-size="92" font-weight="600" letter-spacing="-3" fill="#efe9dc">Download</text>
  <text x="72" y="336" font-family="Georgia, serif" font-style="italic" font-size="100" fill="#efe9dc">anything.</text>
  <text font-family="Segoe UI, Arial, sans-serif" font-size="27" fill="#b5afa3">
    <tspan x="72" y="420">The fast, free download manager.</tspan>
    <tspan x="72" y="458">Catches any video like IDM,</tspan>
    <tspan x="72" y="496">downloads files 8× in parallel.</tspan>
  </text>
  <text x="72" y="568" font-family="Consolas, monospace" font-size="20" letter-spacing="1" fill="#efe9dc" fill-opacity="0.7">snag.kuldeepsaini.dev</text>
</svg>`;
await sharp(Buffer.from(svg))
  .composite([
    { input: await ogRosette("#efe9dc", 0.16, 560), left: 905 - 280, top: 35 },
    { input: hook, left: Math.round(905 - hookWidth / 2), top: 36 },
  ])
  .png({ compressionLevel: 9, palette: true, quality: 92 })
  .toFile(path.join(site, "public", "og.png"));

console.log("images written to", path.relative(repo, out));
