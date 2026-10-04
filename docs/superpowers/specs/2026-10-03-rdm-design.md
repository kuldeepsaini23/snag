# RDM — Rust Download Manager · Design Spec

**Date:** 2026-10-03 · **Status:** Draft for review · **Working name:** RDM (rename any time)

## 1. Goal

A personal IDM-style download manager for Windows 11, written in pure Rust, with the
Cutroom (Figma) visual language and a user-chosen accent colour.

**Success criteria**
- Large files download faster than a browser by using up to 8 parallel segments, and they resume after pause, crash or reboot.
- A video link from YouTube or any other yt-dlp-supported site becomes an MP4 or MP3 at the chosen quality in two clicks.
- Downloads arrive from three places: paste/Add URL, clipboard watch and the Chrome extension.
- The UI matches the approved mockup (`.superpowers/brainstorm/.../accent-picker.html`).

**What the user asked for:** Rust, iced, a Mac look based on their Figma file, a Chrome extension and
clipboard watch from day one, a quality picker, audio-only MP3, playlists/channels, a speed limit,
a scheduler and a custom accent colour.

**Assumptions (correct if wrong):** Windows only for v1 (the code stays cross-platform where
it costs nothing). The app is for personal use, not distribution, so there is no installer or auto-update in v1.

**Out of scope:** DRM-protected streams (Netflix, Prime, Hotstar…), torrents, a macOS/Linux build,
an installer, app self-update, multiple users and a cloud sync.

## 2. Architecture

Cargo workspace at `C:\Users\kulde\rdm`:

```
rdm/
├─ crates/
│  ├─ engine/     # HTTP segmented downloader (no UI, no yt-dlp)
│  ├─ media/      # yt-dlp + ffmpeg wrapper: probe, formats, playlists, download
│  ├─ core/       # app state: downloads, queues, scheduler, settings, persistence, bridge
│  └─ app/        # iced UI (binary: rdm.exe)
├─ extension/     # Chrome MV3 extension (JS)
└─ assets/        # Inter font, Phosphor icon font (MIT)
```

Dependency direction: `app → core → {engine, media}`. `engine` and `media` never depend on each other or on `core`.
`core` exposes a `Manager` handle and a stream of `Event`s that the UI subscribes to.

### 2.1 engine — segmented HTTP downloads
- **Probe:** a `HEAD` request (falling back to `GET` with `Range: bytes=0-0`) reads the size, `Accept-Ranges`, filename (`Content-Disposition` or URL) and ETag/Last-Modified.
- **Segments:** the file is split into N segments (default 8, configurable 1–16). Each segment is a tokio task doing a ranged GET and writing at its offset into `name.ext.rdmpart` (preallocated).
- **Dynamic splitting (the IDM trick):** when a connection finishes, the largest remaining segment is split in half and the free connection takes the second half.
- **No range support:** falls back to a single connection, and resume is disabled for that item.
- **Resume:** a `name.ext.rdmstate` JSON sidecar stores the URL, size, validators and each segment's `{start, end, written}`. It is flushed every 1 s and on pause. On resume the validators are checked; if the file changed, it restarts from zero.
- **Speed limit:** a shared token bucket, global plus optional per-download.
- **Errors:** each segment retries 5 times with exponential backoff. HTTP 403/410 means the link expired, so the item goes to `Error(LinkExpired)` and the UI offers **Refresh link** (paste a new URL and keep the bytes already downloaded).
- **Finish:** the `.rdmpart` file is renamed to its final name, the sidecar is deleted, and an existing file gets ` (1)` appended.
- **Library:** `reqwest` (rustls) and `tokio`.

### 2.2 media — yt-dlp wrapper
- **Binary management:** `yt-dlp.exe` lives in `%APPDATA%\rdm\bin\`. If it's missing on first run, the app downloads it from GitHub releases. A **Settings → Update yt-dlp** button runs `yt-dlp -U`. ffmpeg comes from the PATH (`C:\ffmpeg\bin` is already there) or a path set in Settings.
- **Probe:** `yt-dlp -J --flat-playlist <url>` is parsed with serde into `MediaInfo { title, thumbnail, duration, formats[], is_playlist, entries[] }`.
- **Quality picker data:** video heights (2160/1440/1080/720/480/360), each with an estimated size, plus audio-only options (MP3 320k, M4A best).
- **Download:** `yt-dlp -f "<sel>" --merge-output-format mp4 -N 8 [-r <limit>] [-x --audio-format mp3] --newline --progress-template ...`. Progress lines are parsed from stdout into the same `Progress` type the engine emits.
- **Pause/resume:** kill the process; on resume yt-dlp continues from its `.part` files.
- **Playlists/channels:** the probe lists the entries, and the user picks all of them or a subset at one quality. Each entry becomes its own download item in a group.

### 2.3 core — state, queues, scheduler, settings, bridge
- **Download item:** `id, kind (Http|Media), url, title, dest, category, status (Queued|Running|Paused|Done|Error), progress, speed, eta, queue_id, created_at`.
- **Categories:** Videos, Music, Archives, Documents, Programs and Other, chosen by file extension or media type. Each category can have its own folder; the defaults are subfolders of `Downloads\RDM\`.
- **Queues:** "Main" (always on) plus user queues. Each queue has a max-concurrent setting (default 3) and an optional schedule (start time, optional stop time, days of the week).
- **Scheduler:** a tokio interval checks the schedules every 30 s and starts or stops queues.
- **Persistence:** a single `%APPDATA%\rdm\state.json` (items, queues, settings), written atomically (temp file + rename), debounced to 1 s. Running items come back as Paused after a restart.
- **Settings:** download folders, connections per download, max concurrent, global speed limit, accent colour (hex), clipboard watch on/off, extension token, ffmpeg path.
- **Clipboard watch:** `arboard` polls every 700 ms. A new http(s) URL that is not the last one seen fills the URL bar and shows a small "Link detected" toast. Video sites are detected via a cheap host allowlist, and anything else gets a lazy `yt-dlp` probe only when the user acts on it.
- **Extension bridge:** a localhost HTTP server (`axum`) on `127.0.0.1:47321`.
  - `POST /add {url, referrer, cookies?, filename?, kind}` requires the header `X-RDM-Token` (random 32-byte token, shown in Settings, pasted into the extension once).
  - It binds to loopback only and rejects requests without the token.
  - `GET /ping` lets the extension show a connected/disconnected state.

### 2.4 extension — Chrome MV3
- **Catch downloads:** `chrome.downloads.onDeterminingFilename` checks the type/size rule (default: all files over 1 MB, excluding images). If it matches, the extension cancels the Chrome download and sends `{url, referrer, cookies for that host, filename}` to the app. If the app isn't reachable, Chrome keeps the download.
- **Video button:** a content script on video pages adds a small floating "Download video" pill in the accent style; clicking it sends the page URL with `kind: media`. A context menu offers "Download with RDM" for links, videos and the page.
- **Popup:** connection status, an on/off toggle and the token field.
- **Load:** loaded unpacked from `rdm/extension/` (no Web Store).

### 2.5 app — iced UI
- **Theme:** the Figma tokens map to an `iced::Theme` palette plus custom styles. `canvas #1a1816, panel #211f1c, sidebar #26221f, surface #2a2724, raised #36322e, hover #433e39, line #ffffff17, line-strong #ffffff26, text #ffffffe5/#ffffff91/#ffffff4f, success #32d74b`. The accent comes from settings (default `#ff9f0a`). `accent-soft` is the accent at 20% alpha. Text on the accent is chosen by luminance (above 0.6 → dark `#1a1816`, otherwise white).
- **Fonts:** Inter and the Phosphor icon font (MIT), bundled via `include_bytes!`.
- **Window:** no native decorations. A custom 52 px toolbar is the drag region, with min/max/close controls on the right in Windows style. The Mac look comes from the styling, not from traffic-light buttons. 1280×800 default, 960×600 minimum.
- **Screens and components** (per the approved mockup):
  - **Toolbar:** logo, sidebar toggle, title + live speed, filter pill (All/Active/Done/Scheduled), search (Ctrl K), speed-limit popover (gauge icon), settings, white **Add URL** button.
  - **Sidebar:** Library categories with counts, Queues, and Sources (extension and clipboard status dots).
  - **Center:** URL bar with accent outline and detection tag, then the "Downloading" group, then the "Recent" group. Rows show a thumbnail (video) or file icon, name, meta, a 3 px progress bar, speed/ETA and an action button.
  - **Inspector:** preview, a details table (source, quality, format, size, speed, save-to), an 8-segment bar for HTTP items, and Pause + **Show in folder** buttons.
  - **Add / Quality dialog (sheet):** thumbnail, title, a Video ⇄ Audio switch, quality list with sizes, folder, queue, and **Download now** / **Add to queue**. For playlists it shows a list of checkboxes with select-all.
  - **Settings sheet:** General, Appearance (accent swatches + custom hex), Connections, Speed & Schedule, Extension (token, status), Tools (yt-dlp / ffmpeg).
- **Glass blur:** try Windows acrylic via `window-vibrancy` on the raw window handle. If it misbehaves under iced/wgpu, ship the solid Figma colours (they're already the approved look).

## 3. Data flow

```
Add URL / Clipboard / Extension ──► core::Manager::add(url, opts)
   └─ media host? ──► media::probe ──► UI Quality sheet ──► Manager::enqueue(Media item)
   └─ else        ──► engine::probe ──► (filename/size) ──► Manager::enqueue(Http item)
Queue runner (respects max-concurrent + schedule + speed limit)
   └─ engine::download / media::download  ── Progress events ──► Manager ──► Event stream ──► iced Subscription ──► UI
Manager ── debounced ──► state.json
```

## 4. Error handling
- Network drops: per-segment retries with backoff, then the item becomes `Error(Network)` with **Retry**.
- An expired link (403/410) shows the **Refresh link** flow.
- A server without range support gets a single connection and a "Can't resume" badge.
- A yt-dlp failure shows the last stderr line in the inspector with **Retry**, and suggests **Update yt-dlp** when the error looks like an extractor failure.
- A missing ffmpeg disables 4K/merged formats and MP3 in the picker, with a hint.
- Disk full or no permission becomes `Error(Disk)` with a clear message.
- If port 47321 is busy, the app tries the next 5 ports. The extension tries the same range.

## 5. Testing
- **engine:** integration tests against a local axum test server that supports ranges, throttling, mid-stream disconnects and no-range mode. They check SHA-256 equality, resume after cancel, dynamic split and the speed-limit tolerance (±15%).
- **media:** parser unit tests on recorded `yt-dlp -J` and progress-line fixtures (no network in CI).
- **core:** queue/scheduler tests with tokio paused time, and persistence round-trip tests.
- **bridge:** token rejection and the add flow over HTTP.
- **app:** manual checklist (screens match the mockup, accent switch, dialogs, keyboard shortcuts).

## 6. Build order (for the implementation plan)
1. `engine` + a tiny CLI (`rdm-cli <url>`) proving fast segmented downloads and resume.
2. `core` (items, queues, persistence) + the iced shell with theme, toolbar, sidebar, list and inspector, wired to real HTTP downloads.
3. `media`: yt-dlp bootstrap, probe, quality sheet, MP4/MP3, playlists.
4. Speed limit, scheduler, settings sheet, accent picker.
5. Clipboard watch, localhost bridge and the Chrome extension.

## 7. Addendum (2026-10-04): backend-first delivery

Agreed with the user to reduce token usage:

1. Engine fix pass: review findings C1, C2, I1–I6 (see `.superpowers/sdd/2026-10-03-plan-1-engine/progress.md`).
2. `core`: Manager, queues, scheduler, speed limit, settings, `state.json` persistence (headless, tested).
3. `media`: yt-dlp bootstrap, probe, quality list, MP4/MP3, playlists.
4. Bridge: clipboard watch, localhost server, basic Chrome MV3 extension (plain popup + catch toggle).
5. **Basic test UI** with default iced widgets, so that every feature works end to end.

The app crate is split into `state.rs` / `update.rs` (logic, kept) and `ui/` (views, replaced later).
The Figma styling pass (https://www.figma.com/design/oe5r4M2xEWvmWfUCmT8Bsr, 14 screens) comes afterwards and replaces only `ui/` and the theme.

**Reordered (2026-10-04, user):** the basic test UI moves up to step 3, right after `core`. Media (yt-dlp) is now step 4 and the bridge/extension step 5; both are wired into the existing basic UI as they land.
