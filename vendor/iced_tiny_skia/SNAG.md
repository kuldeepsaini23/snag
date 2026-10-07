# iced_tiny_skia 0.14.1, patched for Snag

A copy of [iced_tiny_skia](https://crates.io/crates/iced_tiny_skia) 0.14.1 (MIT, by the iced
authors), used through `[patch.crates-io]` in the workspace `Cargo.toml`. Changes, all in
`src/window/compositor.rs` and marked "Snag:":

1. **Full redraw on any change.** Upstream redraws only damaged regions. While a list scrolls the
   damage splits into ~20 rectangles and every layer is drawn once per rectangle: 100-120 ms a
   frame on a 1382x864 window (8 fps, "laggy"), and some old pixels were never cleared (trails of
   bullet dots under What's new, piled-up dialog shadows). One full redraw measured ~30 ms while scrolling (no clipping needed); a single clipped
   rectangle around the changes was slower (~38 ms).
   Frames where nothing changed are still skipped.
2. **Frame timing log** for Snag's tests, only when `SNAG_RENDER_LOG` names a file.

Measure with the debug scene `scroll-test` (see `crates/app/src/snap.rs`).
