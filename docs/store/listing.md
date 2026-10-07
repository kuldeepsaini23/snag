# Snag extension — store listing kit (1.0.0)

Copy-paste material for the Chrome Web Store, Microsoft Edge Add-ons and Firefox Add-ons.

- **Packages:** `snag-chrome-1.0.0.zip` (Chrome, Edge) and `snag_download_manager-1.0.0.zip` (Firefox), from `target/store/` after `bash installer/build.sh`, or attached to the GitHub release.
- **Privacy policy URL:** https://snag.kuldeepsaini.dev/privacy
- **Homepage:** https://snag.kuldeepsaini.dev
- **Support:** https://github.com/kuldeepsaini23/snag/issues
- **Category:** Chrome: *Productivity → Tools*, Firefox: *Download Management*, Edge: *Productivity*
- **License:** MIT. **Source:** https://github.com/kuldeepsaini23/snag (plain JavaScript, no bundler or minifier: answer **No** to "does your add-on use minified/obfuscated code").

## Name

Snag — Download Manager

## Short description (Chrome ≤ 132 characters, Firefox summary ≤ 250)

Send downloads, videos and pictures to the Snag app: pick a quality, catch what a page plays, download 8× faster.

## Full description

Snag is a free, open-source download manager for Windows. This extension connects your browser to it.

What it does
• A Download button on video pages: pick 1080p, 720p or MP3, and Snag downloads it.
• Catches what the page's player is playing and starts it at once, even on sites other tools can't read.
• Hands your browser's downloads to Snag, which downloads big files over up to 8 connections and resumes after a pause or restart.
• Right-click a link, picture or page → Download with Snag. Save a whole page as one offline file.
• "Grab all": every link and picture on a page, filtered, in one go.
• Links sent while Snag is closed wait and go through when it opens.

Private by design
• It talks only to the Snag app on your own computer (127.0.0.1), and only after you click Connect and allow it in Snag.
• No account, no analytics, no ads, and nothing sent to the developer or anyone else.

Needs the Snag app for Windows (free): https://snag.kuldeepsaini.dev

## Single purpose (Chrome)

Send the user's downloads — files, videos, audio and pictures from the pages they visit — to the Snag download manager app running on the same computer.

## Permission justifications (Chrome "Privacy practices" tab; also useful for Firefox/Edge reviewers)

| Permission | Why |
|---|---|
| `downloads` | When "Catch downloads" is on, a download the browser starts is cancelled in the browser and handed to Snag instead (and given back to the browser if Snag isn't running). |
| `cookies` | When the user sends a download, the cookies of that one site go with it to Snag on the same computer, so files behind a login (or age checks) download as they do in the browser. Never read in the background, never sent anywhere else. |
| `webRequest` | Reads the address, type and size of media a tab's video player loads, to offer those streams in the Download menu ("what's playing"). Requests are only observed, never changed or blocked. |
| `scripting` | "Grab all from this page": runs a small script in the current tab, after the user clicks it, to list the page's links and pictures. |
| `contextMenus` | The right-click items "Download with Snag" and "Save this page with Snag". |
| `storage` | Remembers the connection code, the user's choices (button position, hidden sites, catch downloads) and links waiting for Snag to open. |
| `alarms` | Once a minute, hands links saved while Snag was closed to Snag when it's running again. |
| Host access `<all_urls>` | The Download button and stream detection work on any video site, and the site's cookies are needed when sending a download. |
| Host `http://127.0.0.1/*` | The only place the extension sends data: the Snag app on the same computer. |

**Remote code:** No. All code is in the package; nothing is downloaded or evaluated at run time.

## Data use disclosure (Chrome form)

- **Data handled:** *Web history* (addresses of the pages and media the user sends to Snag) and *Authentication information* (the site's cookies, only when sending a download).
- **Use:** only for the single purpose above, and only on the user's own computer (sent to 127.0.0.1).
- Check all three certifications: not sold to third parties; not used or transferred for purposes unrelated to the single purpose; not used for creditworthiness or lending.

## Firefox notes

- The manifest declares `data_collection_permissions: { required: ["none"] }`: nothing leaves the device.
- Firefox asks users to allow "Access your data for all websites" (Manifest V3 makes host access optional). Until they do, the Download button and stream detection stay off; the popup and right-click menu still work.
- Reviewer note to paste: "Needs the Snag desktop app (Windows, free, open source: https://github.com/kuldeepsaini23/snag/releases/latest) running on the same computer; the extension talks only to http://127.0.0.1:47321–47326. To test without the app, the popup shows 'Snag isn't running'."
