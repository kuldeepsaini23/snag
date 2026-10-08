//! How the window is drawn. Snag's window is mostly still, so the CPU (iced's tiny-skia
//! renderer) draws it with ~35 MB; the GPU path loads every graphics driver on the PC (~200 MB,
//! and it wakes a laptop's discrete GPU). The GPU stays one switch away (Settings → Appearance).
//! macOS draws with the GPU (Metal) always: those costs are a PC's, while drawing a Retina window
//! (four times the pixels) on the CPU, all of it each frame (vendor/iced_tiny_skia), is not cheap.

/// The `ICED_BACKEND` to set before the window opens, or `None` to leave it as it is: a value the
/// user already set wins; otherwise CPU drawing unless the GPU was chosen.
pub fn backend(already_set: Option<&str>, use_gpu: bool) -> Option<&'static str> {
    let set_by_hand = already_set.is_some_and(|v| !v.trim().is_empty());
    (!set_by_hand && !use_gpu).then_some("tiny-skia")
}

use std::sync::atomic::{AtomicBool, Ordering};

/// Set once in `main`, before the window opens.
static CPU: AtomicBool = AtomicBool::new(false);

/// The window is drawn by the CPU for this value of `ICED_BACKEND` (iced's default is the GPU).
pub fn on_cpu(backend: Option<&str>) -> bool {
    backend.is_some_and(|v| v.trim().eq_ignore_ascii_case("tiny-skia"))
}

pub fn set_cpu(cpu: bool) {
    CPU.store(cpu, Ordering::Relaxed);
}

/// The shadow to draw: none on the CPU. iced's CPU renderer doesn't clear the area a shadow
/// covers before redrawing it, so each redraw darkens it further; a dialog turns black.
pub fn shadow(s: iced::Shadow) -> iced::Shadow {
    shadow_for(CPU.load(Ordering::Relaxed), s)
}

fn shadow_for(cpu: bool, s: iced::Shadow) -> iced::Shadow {
    if cpu { iced::Shadow::default() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_draws_without_shadows() {
        let s = iced::Shadow { color: iced::Color::BLACK, offset: iced::Vector::new(0.0, 18.0), blur_radius: 48.0 };
        // iced's CPU renderer doesn't clear the area a shadow covers before redrawing it, so each
        // redraw darkens it further: after a few seconds of downloads a dialog turns black.
        assert_eq!(shadow_for(true, s), iced::Shadow::default());
        assert_eq!(shadow_for(false, s), s, "the GPU draws them fine");
        assert!(on_cpu(Some("tiny-skia")) && on_cpu(Some(" TINY-SKIA ")));
        assert!(!on_cpu(Some("wgpu")) && !on_cpu(None), "iced's default is the GPU");
    }

    #[test]
    fn the_cpu_draws_unless_the_gpu_was_chosen() {
        assert_eq!(backend(None, false), Some("tiny-skia"));
        assert_eq!(backend(None, true), None, "the GPU: iced's default");
        assert_eq!(backend(Some("wgpu"), false), None, "a value set by hand is kept");
        assert_eq!(backend(Some(""), false), Some("tiny-skia"), "an empty value counts as unset");
    }
}
