// Turns the repo's screenshots and logo into the web-sized images in public/.
// Run with `bun run images` after a screenshot changes; the output is committed.
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const site = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repo = path.resolve(site, "..");
const shots = path.join(repo, "docs", "images");
const logo = path.join(repo, "crates", "app", "assets", "logo", "snag-256.png");
const out = path.join(site, "public", "images");
const app = path.join(site, "src", "app");

await mkdir(out, { recursive: true });

// Screenshots: a full-size and a phone-size WebP of each.
for (const name of ["main", "stats", "permissions"]) {
  const src = path.join(shots, `${name}.png`);
  for (const width of [1382, 760]) {
    await sharp(src)
      .resize({ width, withoutEnlargement: true })
      .webp({ quality: 82, effort: 6 })
      .toFile(path.join(out, `${name}-${width}.webp`));
  }
}

// Browser extension screenshots from the store listing (1280x800).
for (const name of ["1-download-menu", "2-popup"]) {
  const src = path.join(repo, "docs", "store", "screenshots", `${name}.png`);
  for (const width of [1280, 720]) {
    await sharp(src)
      .resize({ width, withoutEnlargement: true })
      .webp({ quality: 82, effort: 6 })
      .toFile(path.join(out, `ext-${name.slice(2)}-${width}.webp`));
  }
}

// Logo: the mark used in the header, plus the icons Next.js picks up from src/app.
await sharp(logo).resize(96).png().toFile(path.join(out, "logo-96.png"));
await sharp(logo).resize(256).png().toFile(path.join(app, "icon.png"));
await sharp(logo).resize(180).png().toFile(path.join(app, "apple-icon.png"));

// Open Graph card: the pitch on the left, the app on the right, 1200x630.
const W = 1200;
const H = 630;
const shot = await sharp(path.join(shots, "main.png"))
  .resize({ width: 760 })
  .png()
  .toBuffer();
const shotMeta = await sharp(shot).metadata();
const rounded = await sharp(shot)
  .composite([
    {
      input: Buffer.from(
        `<svg width="${shotMeta.width}" height="${shotMeta.height}"><rect width="100%" height="100%" rx="14" ry="14"/></svg>`,
      ),
      blend: "dest-in",
    },
  ])
  .png()
  .toBuffer();
const mark = await sharp(logo).resize(88).png().toBuffer();
const text = `
<svg width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <radialGradient id="glow" cx="78%" cy="62%" r="60%">
      <stop offset="0" stop-color="#ff9f0a" stop-opacity="0.28"/>
      <stop offset="1" stop-color="#ff9f0a" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="100%" height="100%" fill="#1c1a18"/>
  <rect width="100%" height="100%" fill="url(#glow)"/>
  <text x="64" y="248" font-family="Segoe UI, Arial, sans-serif" font-size="96" font-weight="800" fill="#f4efe8">Snag</text>
  <text font-family="Segoe UI, Arial, sans-serif" font-size="34" font-weight="600" fill="#f4efe8">
    <tspan x="64" y="318">The fast, free download</tspan>
    <tspan x="64" y="362">manager for Windows</tspan>
  </text>
  <text font-family="Segoe UI, Arial, sans-serif" font-size="24" fill="#b9afa3">
    <tspan x="64" y="430">Catches any video like IDM.</tspan>
    <tspan x="64" y="464">Downloads files 8× in parallel.</tspan>
  </text>
  <text x="64" y="566" font-family="Segoe UI, Arial, sans-serif" font-size="22" font-weight="600" fill="#ff9f0a">snag.kuldeepsaini.dev</text>
</svg>`;
await sharp(Buffer.from(text))
  .composite([
    { input: mark, left: 64, top: 70 },
    { input: rounded, left: 520, top: 150 },
  ])
  .png({ compressionLevel: 9 })
  .toFile(path.join(site, "public", "og.png"));

console.log("images written to", path.relative(repo, out));
