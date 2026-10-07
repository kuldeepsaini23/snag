//! How the window is drawn. Snag's window is mostly still, so the CPU (iced's tiny-skia
//! renderer) draws it with ~35 MB; the GPU path loads every graphics driver on the PC (~200 MB,
//! and it wakes a laptop's discrete GPU). The GPU stays one switch away (Settings → Appearance).

/// The `ICED_BACKEND` to set before the window opens, or `None` to leave it as it is: a value the
/// user already set wins; otherwise CPU drawing unless the GPU was chosen.
pub fn backend(already_set: Option<&str>, use_gpu: bool) -> Option<&'static str> {
    let set_by_hand = already_set.is_some_and(|v| !v.trim().is_empty());
    (!set_by_hand && !use_gpu).then_some("tiny-skia")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_draws_unless_the_gpu_was_chosen() {
        assert_eq!(backend(None, false), Some("tiny-skia"));
        assert_eq!(backend(None, true), None, "the GPU: iced's default");
        assert_eq!(backend(Some("wgpu"), false), None, "a value set by hand is kept");
        assert_eq!(backend(Some(""), false), Some("tiny-skia"), "an empty value counts as unset");
    }
}
