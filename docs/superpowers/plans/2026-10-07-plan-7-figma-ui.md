# Plan 7 — Figma look (roadmap phase 3)

> Compact format, executed natively with TDD. **Spec:** §2.5 and §8.3. **Figma:** https://www.figma.com/design/oe5r4M2xEWvmWfUCmT8Bsr, page "Screens" (4:8), frames 01–13. Frame 14 is the browser extension and is out of scope here. **Branch:** `feat/figma-ui`.

## Global constraints
- iced 0.14, Rust 2024. Logic lives in `state.rs` / `update.rs` / pure helper modules and is unit-tested. `ui/` only draws.
- **Colours** come from spec §2.5 tokens:
  - Accent: `Settings.accent`, an existing field, default `#ff9f0a`.
  - Text on the accent is chosen by luminance: above 0.6 → `#1a1816`, otherwise white.
- **Fonts:** Inter (Regular/Medium/SemiBold, OFL) and Phosphor regular + fill (MIT) are bundled from `crates/app/assets/fonts` with `include_bytes!`. Licence files sit next to them. Numbers use the system mono font (Consolas).
- **Window:** no native decorations. A 52 px toolbar is the drag region; double-click toggles maximise; min/max/close sit on the right. Default 1280×800, minimum 960×600.
- **Never ship a control that does nothing.** Figma rows that have no backing behaviour are left out, and each one gets a ruling:
  - Settings: retries, proxy, back-off toggle, "catch files larger than", "never catch", "ask where to save", "when all downloads finish", "only while I'm using the PC", per-download limit, "shut down PC", acrylic, compact list.
  - Elsewhere: segment bar, thumbnails/durations, the tools-download card, and Refresh link.
  - "Launch with Windows" belongs to Phase 4.

## Review focus
1. **Long names and paths:** a 200-character file name or download folder must not push buttons off-screen or overlap the speed/ETA column; the text ellipsises or wraps. → test `row_meta_never_empty` plus a manual long-name screenshot.
2. **Invalid accent hex:** typing `#ff`, `orange` or an empty value in Custom colour must not crash or blank the UI. The previous accent stays until the value is valid. → test `parse_hex_cases`, `draft_accent_falls_back`.
3. **Filters hide the selected item:** if the selected item is filtered out, the inspector must not show a stale or removed item. → test `inspector_item_respects_filters`.
4. **Slider spam:** dragging the speed slider must not restart videos on every pixel. The limit applies only on release. → test `slider_preview_does_not_apply`.
5. **Playlist with everything unchecked:** the Download button is disabled and nothing is added. → test `playlist_none_selected_adds_nothing`.

---

### Task 1: theme, fonts, icons
**Files:** create `crates/app/src/ui/theme.rs`, `crates/app/src/ui/icon.rs`, `crates/app/assets/fonts/*`

```rust
pub struct Colors { canvas, panel, sidebar, surface, raised, hover, line, line_strong, text, text2, text3, success, danger, accent, accent_soft, on_accent: Color }
pub fn parse_hex(s: &str) -> Option<Color>;            // "#rgb" | "#rrggbb", case-insensitive, '#' optional
pub fn on_accent(accent: Color) -> Color;              // luminance > 0.6 → #1a1816 else white
pub fn colors(accent_hex: &str) -> Colors;             // invalid hex → default accent
pub const SWATCHES: [(&str, &str); 7];                 // Orange #ff9f0a, White #f5f5f7, Green #32d74b, Mint #63e6e2, Blue #0a84ff, Purple #bf5af2, Pink #ff375f
pub fn theme(c: &Colors) -> iced::Theme;               // Theme::custom with the palette
pub enum Icon { … } impl Icon { fn ch(self) -> char; fn fill(self) -> bool }
pub fn icon<'a, M>(i: Icon, size: f32) -> Text<'a>;
```
- Tests:
  - `parse_hex_cases`: valid forms; bad forms (`""`, `"#ff"`, `"orange"`, `"#gggggg"`) → None.
  - `text_on_accent_by_luminance`: white accent → dark text; orange → dark; blue → white.
  - `colors_fall_back_on_bad_accent`
  - `swatches_parse`

### Task 2: core — add a video straight into a queue
**Files:** `crates/core/src/manager.rs`, `crates/core/tests/manager.rs`

- `Manager::add_media_to(url, title, format, queue: QueueId) -> ItemId`. An unknown queue falls back to Main.
- `add_media` stays as `add_media_to(.., 0)`. The item is placed in the queue before the next `schedule()`, so it never starts in Main first.
- Test: `add_media_to_scheduled_queue_waits`. The queue's schedule is inactive, so the item stays Queued and keeps `queue == id`.

### Task 3: view-model logic
**Files:** create `crates/app/src/view.rs`; modify `state.rs`, `update.rs`

```rust
pub enum Filter { All, Active, Done, Scheduled }        // Active = Running|Queued|Paused|Failed not waiting; Scheduled = waiting_for_schedule
pub enum Library { All, Category(Category), Queue(QueueId) }
pub struct Counts { all, active, done, scheduled, per_category: [usize; 6], per_queue: Vec<(QueueId, usize)> }
impl Model {
    pub fn counts(&self) -> Counts;
    pub fn visible(&self) -> (Vec<&Item>, Vec<&Item>);   // (downloading, recent), newest first; filter + library + search
    pub fn inspected(&self) -> Option<&Item>;            // selected AND visible
}
pub enum LinkTag { Video, Playlist, File }
pub fn link_tag(url: &str) -> Option<LinkTag>;          // None for non-http(s)/empty
pub fn row_meta(item: &Item) -> String;                 // "1080p · MP4 · 642 MB of 1.2 GB", "Paused · 2.1 of 5.8 GB", "Completed · 12 MB", "Failed: …"
pub fn host(url: &str) -> String;
pub fn next_queue_start(queues: &[Queue], now: Now) -> Option<String>;   // "23:00", or None
pub fn slider_to_bps(v: f32) -> u64; pub fn bps_to_slider(bps: u64) -> f32;   // log scale 64 KB/s … 50 MB/s, rounded to 64 KB
pub fn segment_choices(base: &[usize], current: usize) -> Vec<usize>;   // base plus current, sorted, deduped
```
- **Picker additions:**
  - `tab: MediaTab { Video, Audio }`, `selected: Vec<bool>` (one per playlist entry, all true at first) and `queue: QueueId`.
  - `Picker::visible_options() -> Vec<(usize, &QualityOption)>`.
  - `requests()` keeps only the selected entries.
- **Model additions:**
  - `filter`, `library`, `search`, `sidebar_open: bool` (default true), `speed_open: bool`, `speed_preview: Option<u64>`, `settings_tab: SettingsTab { General, Appearance, Connections, Speed, Extension, Tools }`, `toast: Option<String>` (clipboard link).
  - Settings drafts: `draft.accent: String`. The theme previews `draft.accent` while Settings is open and the value parses.
- **Settings sheet:**
  - It replaces the Settings and Queues screens: `Screen::Settings` with tabs. The Queues editor moves to the "Speed & Schedule" tab.
  - Closing the sheet (X) saves both the settings draft and the queue drafts. If either is invalid, it stays open and shows the error.
- **Clipboard:** a detected link becomes `toast = Some(link)`; the URL bar is no longer filled. "Download" adds the link; X dismisses it.
- **Tests:**
  - `filter_counts`
  - `visible_splits_and_filters` (filter × library × search)
  - `inspector_item_respects_filters`
  - `link_tag_cases`
  - `row_meta_never_empty`
  - `next_queue_start_picks_soonest`
  - `slider_round_trip`
  - `slider_preview_does_not_apply`
  - `segment_choices_include_current`
  - `picker_tabs_split_options`
  - `playlist_selection_filters_requests`
  - `playlist_none_selected_adds_nothing`
  - `draft_accent_falls_back`
  - `clipboard_link_becomes_toast`
  - `closing_settings_saves_or_stays_open`

### Task 4: the Figma UI
**Files:** replace `crates/app/src/ui/mod.rs`; create `ui/{style,toolbar,sidebar,list,inspector,picker,settings,popover}.rs`; modify `main.rs`

- **Frame 01, main window:**
  - Toolbar: logo, sidebar toggle, title + "N active · speed", filter pills with counts, search (Ctrl K), gauge popover, settings, white Add URL, window buttons.
  - Sidebar: Library categories with counts, Queues with counts, and Sources (extension and clipboard On/Off). It is collapsible.
  - Centre: a URL bar with an accent outline and detection tag, then a "Downloading" group and a "Recent" group. Each row has a tile with a kind icon, the name, the meta line, a 3 px accent progress bar, speed/ETA in mono, and an action icon.
  - Inspector:
    - A preview tile, the title, and a details table (Source, Quality, Format, Size, Speed, Save to, Queue picker).
    - A Failed item shows an error card with Retry. A missing file shows "Download again".
    - Remove and Delete use the two-click Delete.
    - At the bottom: Pause/Resume and Show in folder.
  - Footer: "↓ speed" on the left; "Limit: … · Next queue HH:MM" on the right.
- **Frames 02–04, Add sheet over a scrim:**
  - Title card, a Video/Audio switch, a radio list with sizes, Save to (read-only), a Queue picker, and a Download button.
  - Playlist: Select all, a Quality-for-all picker, a checkbox list, and "Download N videos".
- **Frame 05:** the speed popover, with a toggle, a big value, a slider (applied on release), presets and an "Applies to N active downloads" note.
- **Frame 06:** red error rows, the inspector error card, and the bottom-right toast for clipboard links and notices.
- **Frames 07–12:** the Settings sheet with a left nav and grouped rows using toggles, segmented controls and inputs. Appearance has swatches and a Custom hex field. Tools has Update yt-dlp, App data "Open folder" and "Clear finished" (removes Done items from the list only; files stay).
- **Frame 13:** an empty state with Add URL (focuses the URL bar) and "Set up Chrome extension" (opens Settings → Extension).

### Task 5: verify
- Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
- Build the release and relaunch. Screenshot frames 01, 02, 05, 07, 08 and 13 with `win.ps1` and compare each against the Figma image: layout, spacing, colours, and the accent switch to blue and white.
- Check that the window drags, maximises, minimises, closes and resizes at its edges.
- Review the whole branch with a fresh opus reviewer, fix its findings, then merge to master.
