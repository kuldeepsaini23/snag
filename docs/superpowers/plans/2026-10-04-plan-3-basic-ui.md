# Plan 3 — Basic test UI (`rdm-app`, iced 0.14)

> Compact format (interfaces + tests), executed inline. **Spec:** §7 addendum + reorder note. **Branch:** `feat/basic-ui` (from `feat/core`).

**Goal:** a plain window built from default iced widgets that exercises every `core` feature end to end. The Figma pass later replaces only `ui/` (and adds a theme); `state.rs`, `update.rs` and `format.rs` stay.

## Layout of the crate
```
crates/app/src/
├─ main.rs      tokio runtime + Manager::start(%APPDATA%\rdm\state.json) + iced::application
├─ format.rs    bytes(), speed(), eta(), status_label()          (pure, tested)
├─ state.rs     Model { items, settings, url, selected, screen, draft, notice }  (pure, tested)
│               Model::apply(Event), Model::load(AppState), Draft::from/to_settings, Model::totals()
├─ update.rs    App { model, manager }, enum Message, update(&mut App, Message) -> Task<Message>,
│               subscription() → core events as Message::Core(Event)
└─ ui/mod.rs    view(&App) -> Element  (replaced by the Figma pass later)
```

## Screens (default widgets)
- **Downloads:**
  - Top bar: URL text field (Enter or the Add button) and a Settings button.
  - Rows: name, status, progress bar, "x of y · speed · eta", and the buttons Pause/Resume, Remove, Remove+file, and Show in folder (Done only).
  - Footer: active count and total speed.
- **Settings:**
  - Fields: download folder, connections (1–16), max at once (1–10), speed limit in KB/s (0 = off).
  - Checkboxes: sort into folders, start immediately.
  - Save / Cancel. Validation errors show inline.

## Tests (unit, no window)
- `format`: `bytes_units`, `speed_suffix`, `eta_seconds_minutes_hours`, `eta_none_without_speed`
- `state`: `apply_added_appends`, `apply_updated_replaces_in_place`, `apply_removed_deletes_and_clears_selection`, `apply_settings_updates_and_resets_draft`, `load_replaces_everything`, `draft_round_trip`, `draft_rejects_bad_numbers`, `totals_count_running_and_sum_speed`

## Manual check
Run `rdm`, add a real URL, and watch the progress. Then pause/resume, change the speed limit, restart the app (the item comes back paused), and remove an item. Screenshot the window as evidence.
