# Plan 6 — Missing features (roadmap phase 2)

> Compact format, executed natively with TDD. **Spec:** §8.2, plus review leftovers I7 (cookies, referrer), I8 (yt-dlp update) and M5 (speed limit for videos). **Branch:** `feat/features`.

## Global constraints
- Rust 2024, iced 0.14. Every new persisted field is `#[serde(default)]` so older `state.json` files still load.
- Cookies are **never written to `state.json`**. They live in memory only. Cookie files for yt-dlp go to `%APPDATA%\rdm\cookies\` and are deleted once yt-dlp exits.
- UI changes stay in the basic `ui/mod.rs` style; the Figma pass restyles them later. All logic goes in `state.rs` / `update.rs` and is unit-tested.

## Review focus
1. **Old state:** a `state.json` from before this plan, with media items that have no `work_dir`, must load. Such an item should resume into a fresh temp folder. → test `old_media_item_without_work_dir_loads`.
2. **Removing a paused video:** removing it must delete its `.rdm-parts/<id>` folder. The finished video file and other items' folders must stay. → test `removing_media_item_deletes_its_parts_only`.
3. **Deleting a queue:** if a queue still holds items, they move to Main and keep downloading. They must not vanish or get stuck. → test `deleting_queue_moves_items_to_main`.
4. **Cookie scope:** cookies for `.youtube.com` must never be sent to an unrelated host. Neither may cookies for a parent domain that is host-only. → tests in `cookies.rs`.
5. **Bad typing in the queue form:** a start time like `25:00`, `7` or an empty field shows a message instead of saving garbage. → test `queue_draft_rejects_bad_times`.

---

### Task 1: media options (cookies, rate limit, temp folder), quality choice, self-update
**Files:** `crates/media/src/lib.rs`, `crates/media/tests/run.rs`

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaOptions {
    pub cookies: Option<PathBuf>,   // --cookies <file>
    pub limit_bps: u64,             // 0 = none; else --limit-rate <n>
    pub temp_dir: Option<PathBuf>,  // -P temp:<dir> (partial files live here)
}
pub fn build_args(format: &MediaFormat, out_dir: &Path, url: &str, opts: &MediaOptions) -> Vec<String>;
pub fn probe_args(url: &str, cookies: Option<&Path>) -> Vec<String>;
pub async fn probe(ytdlp: &Path, url: &str, cookies: Option<&Path>) -> Result<MediaInfo, String>;
pub async fn download(ytdlp, url, format, out_dir, opts: &MediaOptions, cancel, progress) -> Result<MediaOutcome, String>;
impl MediaInfo {
    /// Index of the option that best matches `pref` (None = best = 0).
    /// Video{h}: the highest video option ≤ h, else the lowest video option. AudioMp3: the MP3 option.
    pub fn preferred(&self, pref: Option<&MediaFormat>) -> usize;
    /// (url, title, format) to add: one item, or one per playlist entry.
    pub fn requests(&self, url: &str, choice: usize) -> Vec<(String, String, MediaFormat)>;
}
/// `yt-dlp -U`: returns the last meaningful output line ("Updated yt-dlp to …" / "yt-dlp is up to date …").
pub async fn self_update(ytdlp: &Path) -> Result<String, String>;
```
- Tests (unit):
  - `args_carry_cookies_limit_and_temp_dir`
  - `args_without_options_have_none_of_them`
  - `probe_args_carry_cookies`
  - `preferred_picks_highest_not_above`
  - `preferred_falls_back_to_lowest_video`
  - `preferred_mp3`
  - `requests_single_and_playlist` (moved here from the app's `Picker::requests`)
- `fake_ytdlp` handles `-U` by printing `yt-dlp is up to date (fake)`. Test: `self_update_reports_last_line`.

### Task 2: engine — per-item request headers
**Files:** `crates/engine/src/lib.rs`, `crates/engine/tests/*`

```rust
/// `default_client()` plus headers sent on every request (Cookie, Referer).
/// reqwest drops Cookie on a redirect to another host.
pub fn client_with(headers: reqwest::header::HeaderMap) -> reqwest::Client;
```
- Test: `client_with_sends_cookie_and_referer`. The local test server echoes the request headers, and the test asserts both headers arrive on the probe and on the range requests.

### Task 3: core — cookie jar
**Files:** create `crates/core/src/cookies.rs`; modify `crates/core/src/lib.rs`

```rust
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie { pub domain: String, pub host_only: bool, pub path: String, pub secure: bool,
                    pub expiration_date: Option<f64>, pub name: String, pub value: String }
#[derive(Default)]
pub struct Jar { cookies: Vec<Cookie> }
impl Jar {
    pub fn add(&mut self, cookies: Vec<Cookie>);                 // same (domain, path, name) replaces
    pub fn for_url(&self, url: &str) -> Vec<&Cookie>;            // domain, host-only, path, secure and expiry rules
    pub fn header(&self, url: &str) -> Option<String>;           // "a=1; b=2"
    pub fn netscape(&self, url: &str) -> Option<String>;         // cookies.txt body for yt-dlp, None if empty
}
```
- Tests:
  - `domain_cookie_matches_subdomains`
  - `host_only_cookie_matches_exact_host`
  - `unrelated_host_gets_nothing`
  - `secure_cookie_needs_https`
  - `path_prefix_rule`
  - `expired_cookie_skipped`
  - `same_cookie_replaced`
  - `netscape_format_lines`

### Task 4: core — Manager features
**Files:** `crates/core/src/{model,manager}.rs`, `crates/core/tests/manager.rs`

- **Model fields** (all `#[serde(default)]`):
  - `Item.referrer: Option<String>`
  - `Item.work_dir: Option<PathBuf>`
  - `Settings.ask_quality: bool` (default true)
  - `Settings.preferred_quality: Option<MediaFormat>` (default None = best)
- **Queues:**
  - `Manager::set_queues(Vec<Queue>)`:
    - Main (id 0) is always kept; if it's missing, it's re-added unchanged.
    - Items in queues that no longer exist move to Main.
    - Emits `Event::Queues(Vec<Queue>)` and then runs `schedule()`.
  - `Manager::move_to_queue(ItemId, QueueId)` ignores unknown queues and emits `Updated`.
  - `Manager::add_to(url, queue)` is not needed (YAGNI): new items go to Main, and the user moves them.
- **Quality:**
  - `offer_media(url, info)` becomes an actor command. With `ask_quality` it emits `PickMedia { url, info, choice: info.preferred(pref) }`.
  - Without it, the actor adds `info.requests(url, choice)` directly and emits `Notice("Added N …")`.
- **Speed limit for videos:**
  - On start, a media item gets `limit_bps = speed_limit / running_count` (0 stays 0).
  - When `speed_limit_bps` changes, running media items are requeued (`Stop::Schedule`). yt-dlp continues its parts, so they restart with the new rate.
- **Partial files:**
  - On first start a media item gets `work_dir = <out_dir>/.rdm-parts/<id>`, passed as `temp_dir`.
  - On Done, on `remove_now` and on `redownload`, the folder is deleted, along with `.rdm-parts` if it's then empty.
- **Cookies:**
  - `Manager::remember_cookies(Vec<Cookie>)` stores them in a shared `Arc<std::sync::Mutex<Jar>>`.
  - HTTP items use `client_with(Cookie + Referer)` when either is present.
  - Media items and `probe_media` write `cookies/<id or probe-n>.txt` from `jar.netscape(url)` and delete it afterwards.
  - `Manager::add_with(url, referrer: Option<String>) -> ItemId` is used by the bridge; `add(url)` = `add_with(url, None)`.
- **yt-dlp update:**
  - `Manager::update_ytdlp() -> Result<String, String>` runs under the tools lock.
  - The weekly automatic `-U` happens inside `Tools::ytdlp()` when `bin/yt-dlp.checked` is older than 7 days or missing. The marker is touched even on failure, and the result is ignored.
- **Tests:**
  - `set_queues_keeps_main_and_emits`
  - `deleting_queue_moves_items_to_main`
  - `move_to_queue_and_schedule_holds_it`: a queue with an inactive schedule keeps the item `Queued`, not running.
  - `offer_media_asks_with_preferred_choice`
  - `offer_media_without_asking_adds_items`
  - `old_media_item_without_work_dir_loads`
  - `removing_media_item_deletes_its_parts_only`
  - `http_item_sends_browser_cookies_and_referrer`: uses the local server's header echo.
  - Media-running tests use the fake yt-dlp: `media_limit_and_cookies_reach_ytdlp`. The fake writes its args to a file next to it.

### Task 5: bridge + extension — forward cookies and referrer
**Files:** `crates/bridge/src/lib.rs`, `crates/bridge/tests/bridge.rs`, `extension/{manifest.json,background.js}`

- `AddRequest` gains `#[serde(default)] cookies: Vec<Cookie>` and `referrer: Option<String>`.
  - `add` calls `remember_cookies` first, then `add_with(url, referrer)` (file) or probe → `offer_media` (media).
  - Test: `add_with_cookies_and_referrer_reaches_item`.
- Extension:
  - Adds the `cookies` permission and the `<all_urls>` host permission.
  - `sendToApp(url, kind, referrer)` attaches `await chrome.cookies.getAll({ url })`, mapped to the Cookie fields.
  - Catching a download passes `item.referrer`.
  - Version 0.1.3. Checked with `node --check` plus the existing node tests.

### Task 6: app — queues screen, quick speed limit, quality settings, yt-dlp update
**Files:** `crates/app/src/{state,update}.rs`, `crates/app/src/ui/mod.rs`

- `Model.queues: Vec<Queue>`, filled by `load` and by `Event::Queues`. `PickMedia` uses the event's `choice`.
- `QueueDraft { id, name, max_concurrent: String, scheduled: bool, start: String, stop: String, days: [bool;7] }`:
  - `QueueDraft::from_queue` and `drafts_to_queues(&[QueueDraft]) -> Result<Vec<Queue>, String>`.
  - Times are `HH:MM`; stop may be empty (until midnight).
  - The name can't be empty; max is 1–10 (Main has no limit field).
  - New queues get the id `max+1`.
- `parse_hhmm(&str) -> Option<u16>`.
- **Screens and controls:**
  - A Queues screen: list, Add queue, Delete (not Main), Save/Cancel.
  - Each item row gets a `pick_list` to choose its queue.
  - The footer gets a speed-limit `pick_list` with presets Off, 256 KB/s, 512 KB/s, 1 MB/s, 2 MB/s, 5 MB/s and 10 MB/s. A custom value is shown as-is.
  - Settings gets a quality `pick_list` (Best, 2160p, 1440p, 1080p, 720p, 480p, 360p, MP3), an "Ask every time" checkbox and an "Update yt-dlp" button that shows the result as a notice.
- The paste flow calls `manager.offer_media` (one path for app and browser).
- **Tests:**
  - `parse_hhmm_cases`
  - `queue_draft_round_trip`
  - `queue_draft_rejects_bad_times`
  - `new_queue_gets_next_id`
  - `queues_event_updates_model`
  - `speed_presets_include_custom_value`

### Task 7: verify
- Run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and the node tests.
- Run the opt-in real tests (`--ignored`) for media and core with a real YouTube video. This checks that `-P temp:` leaves no parts in the output folder and that `--limit-rate` holds speed near the limit.
- Build the release and relaunch. Take screenshots of the Queues screen, the footer limit and the Settings quality controls.
- Review the whole branch with a fresh opus reviewer, fix its findings, then merge to master.
