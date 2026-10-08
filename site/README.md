# snag.kuldeepsaini.dev

The website for Snag: the landing page (`/`) and the privacy policy for the app and the browser extension (`/privacy/`). Next.js App Router with a static export and Tailwind CSS. The idea: an engraved catalogue for a modern download manager, in the colours of Snag's logo (ink, warm paper, one orange). The page is in eight parts and the sticky top bar is both its download and its index: one segment per part fills as you read it, the label names the part you are in and opens the contents. Features are numbered catalogue entries, screenshots are plates with captions, and the line-work (guilloche and hatching) is drawn by SVG masks and CSS gradients. All motion is CSS plus two small IntersectionObserver/scroll scripts, and it all stops for reduced motion.

## Run it

```bash
bun install
bun run dev        # http://localhost:3000
```

## Build

```bash
bun run build      # writes the whole site to out/
```

`out/` is plain HTML, CSS, JS and images, so any static host can serve it. To preview the build locally: `bunx serve out`.

`bun run lint` checks the code.

### Images

The screenshots in `public/images/`, the Open Graph card `public/og.png` and the icons in `src/app/` (`icon.png`, `apple-icon.png`) are made from `../docs/store/screenshots/`, `../docs/images/*.png` (cropped clear of the old logo), the engraving `../docs/logo/engraving/c-gpt.png` (cut out of its orange) and `../crates/app/assets/logo/` by the script below. It also draws the guilloche masks `public/images/rosette.svg` and `band.svg`:

```bash
bun run images
```

Run it again whenever a screenshot or the logo changes, and commit the results. `src/app/favicon.ico` is a copy of `crates/app/assets/logo/snag.ico`.

### Where things are

- `src/app/page.tsx`: the landing page, one component per part in `src/components/`
- `src/lib/parts.ts`: the eight parts in order, with the names the top bar shows
- `src/components/download-bar.tsx`: the sticky bar that fills part by part and lists the contents
- `src/components/catalogue.tsx`: the shared pieces (part headings, numbered entries, plates)
- `src/components/features.tsx`: the catalogue entries No. 01–12 (edit the entry lists)
- `src/app/privacy/page.tsx`: the privacy policy (update the effective date when it changes)
- `src/lib/site.ts`: the site address, GitHub links, the installer link (the stable `Snag-Setup.exe` asset of the latest release) and the version, read at build time from `../Cargo.toml`. Nothing to change for a new release; just rebuild.
- `src/app/sitemap.ts`, `src/app/robots.ts`: `sitemap.xml` and `robots.txt`

## Deploy

The site lives at **https://snag.kuldeepsaini.dev**. Either host works; pick one.

### Cloudflare Pages

1. Cloudflare dashboard → **Workers & Pages** → **Create** → **Pages** → **Connect to Git**, and pick the `kuldeepsaini23/snag` repository.
2. Build settings:
   - Production branch: `master`
   - Framework preset: **None**
   - Build command: `bun install && bun run build`
   - Build output directory: `out`
   - Root directory (advanced): `site`
3. **Save and Deploy**. The site is then at `<project>.pages.dev`.
4. Project → **Custom domains** → **Set up a custom domain** → `snag.kuldeepsaini.dev`.
   - If `kuldeepsaini.dev` is on Cloudflare DNS, Cloudflare adds the record for you.
   - Otherwise, at your DNS provider add a **CNAME** record: name `snag`, target `<project>.pages.dev`.

Or upload a local build without Git: `bun run build`, then `bunx wrangler pages deploy out --project-name snag-site`.

### Vercel

1. vercel.com → **Add New** → **Project** → import `kuldeepsaini23/snag`.
2. **Root Directory**: `site`. Framework preset: **Next.js** (it detects the static export). Install command: `bun install`, build command: `bun run build`.
3. **Deploy**.
4. Project → **Settings** → **Domains** → add `snag.kuldeepsaini.dev`.
5. At your DNS provider add a **CNAME** record: name `snag`, target `cname.vercel-dns.com` (Vercel shows the exact value on the Domains page).

HTTPS certificates are issued automatically by both hosts once the CNAME resolves.

### After the first deploy

- Check `https://snag.kuldeepsaini.dev/privacy/` loads; that is the privacy policy URL for the Chrome Web Store, Edge Add-ons and Firefox Add-ons listings.
- Paste the home page into a link preview (Slack, X, Discord) to check the Open Graph card.
- Submit `https://snag.kuldeepsaini.dev/sitemap.xml` in Google Search Console if you want it indexed sooner.
