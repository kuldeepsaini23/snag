# Plan 5 — Clipboard watch, local bridge, Chrome extension

> Compact format, executed inline. **Spec:** §2.3 (clipboard, bridge), §2.4 (extension). **Branch:** `feat/bridge`.

## core
- `Settings.extension_token: String` (serde default). `Manager::start` fills it with `new_token()` when it's empty.
- `pub fn new_token() -> String`: 32 hex characters, random per call.
- Tests: `tokens_are_32_hex_and_differ`, `fresh_state_gets_extension_token`.

## `rdm-bridge` (axum on 127.0.0.1 only)
```rust
pub const PORTS: RangeInclusive<u16> = 47321..=47326;
pub struct Bridge { pub port: u16 }
pub async fn start(manager: Manager, ports: RangeInclusive<u16>) -> Result<Bridge, String>; // first free port
```
- `GET /ping` → `{"app":"rdm","version":…,"paired":bool}`. `paired` means the `X-RDM-Token` header matches the token.
- `POST /add` with JSON body `{url, kind?: "file"|"media"}` → 401 without a valid token, 400 for a non-http(s) URL.
  - File: `{"id":n}`.
  - Media (or `is_media_url`): returns 202 at once. In the background it probes and adds the best quality ≤1080p, or MP3 for audio-only links.
- Tests: `ping_reports_pairing`, `add_requires_valid_token`, `add_file_link_creates_item`, `rejects_non_http_url`, `falls_back_to_next_port`.

## App
- Starts the bridge on the manager runtime. The Settings screen shows the bridge status, the pairing code with Copy and New code buttons, and a "Watch clipboard" checkbox.
- **Clipboard watch:** every 700 ms it reads the clipboard. A new http(s) link that isn't the last one seen fills the URL box and shows "Link detected — press Add". It's only a suggestion and never auto-starts.
- Pure, tested helper: `clipboard_link(text, last_seen) -> Option<String>`. Tests: `clipboard_accepts_new_links_only`.

## Extension (`rdm/extension`, MV3, loaded unpacked)
- `manifest.json`: downloads, storage, contextMenus. Host permission for `http://127.0.0.1/*`. Content script on video sites.
- `background.js`:
  - Finds the app on ports 47321–47326 and catches new Chrome downloads (cancel, then send to RDM; Chrome keeps the download if RDM can't be reached).
  - Context menu "Download with RDM".
  - Handles messages from the popup and the content script.
- `popup.html/js`: connection status, pairing-code field, Catch downloads toggle, "Send this page to RDM".
- `content.js`: a floating "Download with RDM" pill on video pages.
- Checks: `node --check` on every JS file. The bridge is exercised with curl the same way the extension calls it. **Loading it in Chrome is done by the user** (developer mode, Load unpacked).
