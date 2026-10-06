# Plan 8 — Tray and more features (between roadmap phases 3 and 4)

> Compact format, executed natively with TDD. **Asked by the user (2026-10-07):**
> - Closing the window hides RDM to the tray.
> - Quit from the tray, with a warning popup when something is still downloading.
> - "Add as many features as we can".
> - Image downloads.
>
> **Branch:** `feat/tray-features`. **Spec:** §2.5 / §8; this plan extends it.

## Global constraints
- The same rules as plan 7 apply:
  - Logic goes in `state.rs` / `view.rs` / `update.rs` and is unit-tested; `ui/` only draws.
  - New persisted fields are `#[serde(default)]`.
  - No control ships without behaviour behind it.
- Windows only. New crates must be MIT/Apache/OFL licensed.

## Review focus
1. **Quit while downloading:** quitting from the tray with running or queued items must ask first. "Keep downloading" must not lose anything. "Quit" must pause and save exactly like closing used to.
2. **Hidden window and a second launch:** starting RDM again while it's hidden must show the window, not open a second copy.
3. **Tray menu while a sheet is open:** Quit must show the confirm on top of Settings or the picker and must not drop their state silently.
4. **Notifications flood:** a playlist of 50 finishing videos must not show 50 toasts at once.

---

### Task 1: tray, close to tray, quit confirm, pause/resume all
- **Close behaviour:** `exit_on_close_request(false)`. Closing the window (the X button or Alt+F4) hides it (`window::set_mode(Hidden)`); downloads continue.
- **Tray** (`tray-icon`, MIT/Apache):
  - Icon: the RDM logo as RGBA, generated into `assets/icon-32.rgba`. The window gets the same icon.
  - Menu: Open RDM / Pause all / Resume all / — / Quit RDM.
  - A left click or double-click on the icon opens RDM.
  - Events are forwarded by a subscription.
- **Quitting:** `Model::request_quit() -> bool`:
  - true → quit now.
  - false → the window shows the "N downloads are still running" sheet with "Keep downloading" (hides again) and "Quit anyway".
  - The count is Running + Queued.
- `Event::Focus` (a second launch) also un-hides the window.
- `Model::pause_all_ids()` / `resume_all_ids()`.
- Tests:
  - `quit_asks_only_when_something_runs`
  - `keep_downloading_cancels_quit`
  - `pause_and_resume_all_pick_the_right_items`

### Task 2: Windows notifications
- On `Updated` to Done or Failed, show a Windows toast with the name and "Downloaded" or "Failed: …".
- Setting: `Settings.notify: bool` (default true), shown in General.
- Bursts are batched: more than 3 finishing within 2 s gives a single "N downloads finished".
- Tests:
  - `notifications_batch_bursts`
  - `notification_only_on_transition` (an `Updated` event that was already Done doesn't notify again)

### Task 3: images
- `Category::Image`:
  - Extensions jpg, jpeg, png, gif, webp, bmp, svg, avif, heic, tif and tiff.
  - Folder "Images".
  - Sidebar entry and tile icon.
  - Old state files still load.
- **gallery-dl** for image pages (Instagram, Pinterest, Reddit, X, Tumblr, Imgur, DeviantArt, ArtStation, Flickr…):
  - Fetched like yt-dlp (`bin/gallery-dl.exe`, from GitHub releases).
  - A gallery item downloads all of its images into `Images/<title>`.
  - Progress is counted as files done.
- Tests:
  - `image_extensions_categorised`
  - `gallery_urls_detected`
  - The fake gallery-dl runs end to end in core.

### Task 4: video thumbnails and durations
- `MediaInfo.thumbnail: Option<String>`; `Item.thumbnail` and `Item.duration` are persisted.
- The app downloads thumbnails into `%APPDATA%\rdm\thumbs\<hash>.jpg` once and shows them in rows, the inspector and the picker. The gradient tile is the fallback.

### Task 5: refresh link
- For a Failed or Paused item: "Refresh link" in the inspector takes a new URL and keeps the partial data (`Manager::refresh_url(id, url)`). For HTTP, the size must match, else the file restarts.

### Task 6: subtitles
- A checkbox in the picker, plus a setting: `--write-subs --write-auto-subs --sub-langs en.* --embed-subs`.

### Task 7: auto-retry
- Failed with a network/timeout/5xx error → retry up to 3 times at 10 s, 30 s and 90 s (`Settings.auto_retry`, default on).

### Task 8: plan-7 review minors (#8–#20 in that ledger)

### Task 9: Firefox extension
- One source tree with two manifests:
  - `extension/` builds `dist/chrome` (MV3 `service_worker`) and `dist/firefox` (MV3 `background.scripts` plus `browser_specific_settings.gecko.id`).
  - A small build script copies the shared files.
- API differences to cover: `chrome.downloads` cancel/erase timing and `cookies.getAll` with `partitionKey`. Firefox has no `downloads.onDeterminingFilename`.
- The node tests run against both manifests.
- Safari is out of scope: RDM is Windows-only, and Safari extensions only run on macOS/iOS.

### Task 10: name and logo (needs the user's choice)
- The user picks a name from a shortlist.
- The logo is drawn in Figma on the Components page.
- Exported to `.ico` (window, tray, installer) and the extension icons (16/32/48/128).
- "RDM" is replaced in the title, tray, notifications, extension and data folder. The data folder needs a migration from `%APPDATA%
dm`.

### Task 11: verify
- Run the tests, clippy and the snapshots.
- Review the whole branch with a fresh opus reviewer and fix its findings.
- Merge, rebuild the release and relaunch.

---
**Next plans (asked 2026-10-07):**
- Plan 9: the Windows installer (roadmap phase 4).
- Plan 10: macOS + Linux builds:
  - Platform layers for reveal-in-folder, notifications, tool binaries and the data dir.
  - Mac-style window buttons on macOS.
  - GitHub Actions builds producing a .dmg, an AppImage and a .deb. This needs a GitHub repo, and Apple signing is optional ($99/yr).
  - The Safari extension, built in Xcode on a Mac.
