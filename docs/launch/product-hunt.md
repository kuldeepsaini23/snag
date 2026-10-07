# Snag on Product Hunt: launch kit

Submit at https://www.producthunt.com/posts/new (you, as the maker). Launch on a **Tuesday, Wednesday or Thursday, at 00:01 Pacific** (12:31 IST), when the daily ranking resets.

## Do this first
- [ ] Landing page live at https://snag.kuldeepsaini.dev with the Download button.
- [ ] The extension is in at least one store (Firefox is the fastest), or the page explains "load unpacked".
- [ ] Ideally the installer is code-signed, so first impressions don't start with a SmartScreen warning.
- [ ] Ask 10–20 people you know to try Snag the week before, so the first comments are real users.

## Name
Snag

## Tagline (max 60 characters)
Pick one:
- **The free IDM alternative that catches any video**
- Download any video, file or picture — 8× faster
- A fast, open-source download manager for Windows

## Description (max 260 characters)
Snag is a free, open-source download manager for Windows. Files come down 8× in parallel and resume after a restart; its browser extension catches the video a page is playing and gives you 1080p, 4K or MP3 in one click. No account, no tracking.

## Topics
Productivity · Open Source · Windows · Chrome Extensions · Developer Tools

## Links
- Website: https://snag.kuldeepsaini.dev
- GitHub: https://github.com/kuldeepsaini23/snag
- Download: https://github.com/kuldeepsaini23/snag/releases/latest

## Gallery (1270×760, the first image is the thumbnail on the feed)
1. The app downloading with the speed graph: `docs/images/main.png`, cropped/padded to 1270×760, caption "Up to 8 connections per file, resume anytime".
2. The in-page quality menu: `docs/store/screenshots/1-download-menu.png`, caption "One click on any video: 4K, 1080p or MP3".
3. Stats: `docs/images/stats.png`, caption "See what you download, by day, type and site".
4. Privacy step: `docs/images/permissions.png`, caption "You decide what Snag may use".
5. A 30–45 s video (see below).

## Video (30–45 s, no voice needed, captions on screen)
1. (0–5 s) A video page → click **Download** → quality menu → **1080p**.
2. (5–12 s) Snag opens: the bar fills, segments light up, the speed graph rises.
3. (12–20 s) Right-click a picture → Download with Snag; drop a link list on the window.
4. (20–30 s) Torrents, saved pages, stats, accent colours change live.
5. (30–40 s) "Free · open source · no account". Logo + snag.kuldeepsaini.dev.

## Maker's first comment (post it right after launch)
> Hi Product Hunt 👋 I'm Kuldeep.
>
> I used IDM for years and wanted the same speed without the nag screens, closed source or a tool that phones home. So I built **Snag** in Rust:
>
> • Files download over up to 8 connections and resume after a crash or restart
> • The browser extension catches whatever a page's player is playing, the IDM trick, and also reads 1,800+ sites through yt-dlp for 4K/1080p/MP3
> • Images, torrents, whole web pages, GitHub repos as ZIP, rules after download (convert, unpack, move)
> • About 35 MB of memory, no account, no analytics; the extension only talks to the app on your own PC
>
> It's free and MIT-licensed, and Windows comes first, with Mac and Linux next. I'd love to hear what you download most and what's missing. I'm here all day to answer. 🙏

## Replies to have ready
- **"Is it safe? Windows warned me."** The installer isn't code-signed yet (signing is coming). The source and SHA-256 checksums are on GitHub.
- **"Netflix/Spotify?"** No. Those use DRM, and Snag doesn't break DRM. Please download only what you're allowed to.
- **"Mac/Linux?"** Next on the list. Follow the GitHub repo for releases.
- **"Why not just yt-dlp?"** Snag uses it, and adds the browser button, the quality menu, parallel file downloads, queues and a UI.

## Launch day
- [ ] Post at 00:01 PT; share the link with your network: X/LinkedIn post, communities you're part of. Don't ask for upvotes directly (against PH rules); ask people to "check it out and share feedback".
- [ ] Answer every comment within the hour.
- [ ] Watch GitHub issues and snag.log reports, and ship a 1.0.1 quickly if something breaks.
