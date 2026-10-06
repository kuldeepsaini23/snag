# Plan 9 — Download anything (extension), torrents, channels, phone, after-download

> **User's picks, 2026-10-07:** all four groups ("I want this extension to download anything"), plus "Safety check + later links". No AI features: the user didn't pick transcripts, a command bar or smart rename.
> Compact format, executed natively with TDD. **Branch:** `feat/anything`, started from master after plan 8 merges.

## Global constraints
- Plan 7/8 rules apply:
  - Logic goes in pure, tested functions; `ui/` only draws.
  - New persisted fields are `#[serde(default)]`.
  - No dead controls. MIT/Apache/BSD/OFL dependencies only.
- **Extension:**
  - One source for Chrome and Firefox (`build.js`). New permissions are listed in the extension description and are only what each feature needs.
  - The node tests cover every classifier and filter.
- **Network:** nothing listens beyond 127.0.0.1 unless the user switches phone sharing on. That listener is token-protected.

## Review focus
1. **Sniffer noise:** HLS/DASH segment requests (`.ts`, `.m4s`, byte ranges) and ad/analytics beacons must not flood the "media found" list. A page shows its streams (master playlists and progressive files), not hundreds of chunks.
2. **Grab-all scale:** a page with 5,000 links must not freeze the browser or send 5,000 downloads by accident. Nothing is selected by default above 200, and a confirm appears above 100.
3. **Torrent safety:** a torrent writes only inside its own download folder (no `..` or absolute paths from the torrent's file list).
4. **Phone sharing exposure:** off by default. When on, it needs the token, binds only while on, and shows on screen that it's on.
5. **After-download rules:** never delete the original unless the rule says so. A failed conversion leaves the original untouched.

---

### Task 1: media sniffer (any site)
- `extension/sniffer.js` (pure): `classifyMedia({url, contentType, size, initiator}) -> null | {kind: "hls"|"dash"|"file", url}`.
  - **Kept:** `.m3u8` master/media playlists, `.mpd`, and video/audio files of 1 MB or more.
  - **Skipped:** segments (`.ts`/`.m4s`/`.aac` chunks, `Range` sub-requests) and known ad/analytics hosts.
- Background:
  - `webRequest.onResponseStarted` (observational, `webRequest` permission) keeps a per-tab, deduplicated list.
  - The action badge shows the count; the list is cleared on navigation.
  - The popup lists the media with "Download"; it sends `kind: "media"` plus the referrer.
- App/bridge: HLS/DASH URLs go to yt-dlp (its generic extractor); direct files go to the engine.
- Tests: `classify_*` (node).

### Task 2: grab all / image grabber
- `extension/grab.js` (pure): `filterItems(items, {types, minSize, extensions, text}) -> items`.
- A content-script collector gathers `a[href]`, `img` (largest `srcset` candidate, natural size), `video/audio/source`, and CSS `background-image` from visible elements.
- `grab.html` (extension page):
  - A table with checkboxes and type/extension/size filters.
  - Presets: "Images (≥ 200 px)", "Videos", "Documents", "Archives", "Everything".
  - "Download N", plus a confirm above 100 items.
- Bridge: `POST /add-batch {urls[], referrer, cookies}`, added as files (or gallery/media by the usual routing).
- Tests:
  - `filterItems` presets
  - dedupe
  - nothing pre-selected above 200 items
  - bridge `add_batch_adds_each`

### Task 3: live stream recording
- A live video (yt-dlp `is_live`) gets "Record" in the picker. Arguments: `--hls-use-mpegts --no-part`, so the file stays playable if stopped.
- "Stop recording" stops the process and marks the item Done with the file as recorded.
- Tests: probe parses `is_live`; the args; the fake live run is stopped and becomes Done with its file.

### Task 4: torrents and magnet links
- `crates/torrent` wraps `librqbit` (Apache-2.0); `Kind::Torrent`.
- Magnet links come from the URL bar, the clipboard and the extension (catching `magnet:` link clicks); `.torrent` files come from downloads.
- Files are saved to `<download>/Torrents/<name>`. Paths are sanitised: never outside the folder.
- Progress, speed, peers, pause/resume and the speed limit are shared with the rest of the app.
- Seeding stops when the download finishes (setting: "Keep sharing after download", default off).
- Tests:
  - magnet parse
  - path sanitising
  - a local two-peer test (seed from a temp dir to a leech in-process) downloads a small file

### Task 5: watch channels
- `Watch {url, name, audio_only, max_minutes, last_ids, every_hours}` in state.
- Checking runs `yt-dlp --flat-playlist`; new entries are added in the chosen format. The first check only records what's there.
- A sidebar "Watching" section; add a watch from the picker ("Watch this channel/playlist").
- Tests: new-entry diff, the first check adds nothing, max_minutes filter, check timing.

### Task 6: send from your phone
- An opt-in "Phone sharing" setting:
  - The bridge also listens on the LAN with the token.
  - It serves a small installable page (PWA with `share_target`) where links are shared or pasted.
- Settings shows a QR code (the iced `qr_code` widget) with `http://<lan-ip>:<port>/m?t=<token>`.
- Tests: off means no LAN listener; a wrong token gives 401; the share POST adds the link; the QR payload is correct.

### Task 7: after-download rules
- Rules: `{match: ext|site|category, action: ToMp3 | SmallerMp4(crf) | Extract | MoveTo(dir), keep_original}`.
  - ffmpeg is used for conversions.
  - `zip` (MIT/Apache) and `sevenz-rust` (Apache) are used for extracting.
- A rules editor in Settings → General.
- Tests:
  - matching
  - zip extract into a folder of the archive's name
  - a failed conversion keeps the original
  - "keep original" is honoured

### Task 8: save a web page
- "Save page" (extension context menu and popup) sends the URL.
- Snag saves a single offline HTML file via the `monolith` library (CC0/MIT), with the browser's cookies, in `<download>/Pages/`.
- Test: the local test server page with CSS and an image becomes one file with the assets inlined.

### Task 9: duplicate guard
- Adding a URL that is already Done, or a file with the same final name and size, shows a toast: "Already downloaded <date>: Open / Download again / Skip".
- Tests: matching by normalised URL (ignoring tracking params) and by name+size; skip adds nothing.

### Task 10: safety check for programs (user pick)
- When an `.exe`, `.msi`, `.bat`, `.ps1`, `.scr` or `.zip` finishes, Snag computes its SHA-256 and looks it up on VirusTotal. It uses the user's free API key from Settings → Tools; the hash only, never uploading the file.
- The right-hand panel shows "Clean (0/72)", "Flagged by N scanners" in red, or "Not known to VirusTotal". Programs that are flagged get a warning before "Show in folder".
- Tests: hashing; response parsing (clean, flagged, unknown, quota); no key means no request.

### Task 11: "download for later" while Snag is closed (user pick)
- With the app closed, the extension stores links in its own queue, shows a badge count, and sends them when Snag answers again.
- The phone page (Task 6) does the same on its side.
- Tests: the queue persists in `storage.local`, drains in order, nothing is lost on a failed send, no duplicates.

### Task 12: verify
- Run all tests, clippy and web-ext lint, plus snapshots of the new screens.
- Review the whole branch with a fresh opus reviewer and fix its findings.
- Merge, rebuild the release and relaunch.
