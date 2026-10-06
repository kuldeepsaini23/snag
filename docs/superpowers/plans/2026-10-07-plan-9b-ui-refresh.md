# Plan 9b — UI refresh (user feedback, 2026-10-07)

> **User:**
> - "add micro animation, tabs changes, sidebar open close and change upper menu bar look — icons look cheap"
> - "UI can be better, I want it UI/UX-wise simple"
> - "show download options in the extension too"
> - "thumbnail and extension colour should change with appearance"
> - "why no colour picker in Appearance"
> - "a change log and help & bug button"
>
> Runs before the remaining plan 9 tasks (7–11, 13–17). **Branch:** `feat/anything` (continues).

## Global constraints
- Logic stays in pure, tested functions; `ui/` only draws. No dead controls.
- **Animations:**
  - iced 0.14 `Animation` driven by `window::frames()` only while something animates.
  - 120–200 ms ease-out.
  - All of them respect Windows "Animation effects" off (SPI_GETCLIENTAREAANIMATION).
- **Simpler means fewer things on screen, not fewer features.** Advanced controls move behind one click.

## Review focus
1. **Animation never blocks input:** a click during a transition acts on the target state. There's no frame loop when idle (CPU near 0).
2. **Reduced motion:** with Windows animations off, everything jumps instantly.
3. **Colour picker edge cases:** grey (no hue) and black/white keep the hue the user chose. The hex box and the picker never fight.
4. **Bug report:** it never contains the pairing token, cookies or full file paths of the user's downloads beyond the folder names. It's written locally; nothing is sent anywhere.
5. **Extension quality picker:** if the app answers slowly (yt-dlp probe up to ~10 s), the popup shows progress and never hangs. Closing the popup loses nothing: the app picker still works.

---

### Task R1: toolbar and window chrome
- Icons:
  - The Phosphor **Bold** font replaces Regular for toolbar and sidebar icons. Active filter and nav items use **Fill**.
  - 18 px icons in a 32 px hit area, borderless, with a soft hover background.
- Filter tabs:
  - text only (no icons); the count sits as a small muted number;
  - the active tab has an accent underline indicator that slides between tabs (animated).
- Search:
  - a borderless magnifier that expands into the field on click or Ctrl+K;
  - it collapses again when empty and focus leaves.
- Speed and settings become plain icon buttons. Add URL is one accent pill ("+ Add").
- Window buttons: Windows 11 caption sizes (46×32), thin glyphs, a red close hover.
- Tests:
  - `tab_indicator_targets` (the indicator x and width per tab from the measured label widths)
  - `search_collapses_when_empty`

### Task R2: simpler layout, sidebar animation
- The sidebar slides (width animates 0↔208) when toggled.
- Sources and Watching collapse into one "More" group by default.
- The inspector opens and closes with a slide when an item is selected or deselected, and Esc deselects.
- Rows:
  - one line of meta, with the progress bar under it;
  - the action button appears on hover or selection only (less noise);
  - speed and ETA stay.
- Tests:
  - `sidebar_animation_state`
  - `row_actions_show_on_hover_or_selected`

### Task R3: micro-animations
- Sheets (picker, settings, quit confirm): fade plus a 0.97→1 scale-in.
- Popover: slide down.
- Toasts: slide up.
- Progress bars: ease to the new value.
- New rows fade in.
- A finished row shows a short accent check pulse.
- A frame subscription runs only while `any_animating(now)`. Reduced motion is honoured.
- Tests:
  - `idle_means_no_frames`
  - `reduced_motion_jumps`
  - `progress_eases_monotonic`

### Task R4: accent everywhere
- File tiles use the accent (soft gradient from the accent to the panel colour) instead of fixed orange or purple. Video and audio tiles without a thumbnail too.
- The extension:
  - `/ping` returns `accent`;
  - the popup and grab page set `--accent` (and dark/light text on it) from it;
  - the extension icon badge uses it.
- Tests:
  - `tile_colors_follow_accent`
  - bridge `ping_reports_accent`
  - node `accentVars` (text colour on the accent)

### Task R5: colour picker
- Appearance gets a saturation/value square plus a hue strip (canvas), next to the swatches and the hex box, all kept in sync.
- Pure HSV↔RGB conversion. The hue is kept for greys.
- Tests:
  - `hsv_round_trip`
  - `grey_keeps_hue`
  - `hex_and_picker_agree`

### Task R6: quality choice in the extension
- Bridge `POST /probe {url}` returns options (label, size) after yt-dlp reads the page. `POST /add-media {url, choice, referrer}` adds it.
- The popup's "Download video" on a video page and in the sniffer list shows the options inline (with a spinner while reading), then "Download".
- Tests:
  - bridge `probe_returns_options` (fake yt-dlp)
  - `add_media_with_choice`
  - node `optionLabels`

### Task R7: What's new
- `CHANGELOG.md` is the source.
- The app embeds it, parsed into versions with sections (pure parser, tested).
- After an update, `Settings.last_seen_version < current` shows a "What's new in Snag X" sheet once.
- The Help menu has "What's new" for the full list.
- Tests:
  - `changelog_parse`
  - `shows_once_after_update`

### Task R8: Help and Report a bug
- A "?" toolbar menu: Keyboard shortcuts, Help (an in-app sheet: basics, browser extension, phone, torrents, troubleshooting), What's new, Report a bug.
- **Report a bug:**
  - A sheet: "What happened?" plus a checkbox "include diagnostics".
  - It writes `Snag-bug-report-<date>.txt` to the Desktop with: version, Windows version, settings minus token and cookies, tool versions (yt-dlp, gallery-dl), counts per status, and the last 50 errors (download names only).
  - It opens the file's folder and copies the text. "Where to send it" shows the project's issue page once it has one.
- Tests:
  - `bug_report_has_no_secrets`
  - `bug_report_contents`

### Task R9: verify
- Run the tests and clippy, plus snapshots of every changed screen (before and after).
- Review with a fresh opus reviewer, fix its findings, then merge (with plan 9's finished tasks), rebuild the release and relaunch.
