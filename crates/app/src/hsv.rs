//! Hue, saturation and value for the accent picker, and the conversions to and from the hex the
//! settings store. A grey has no hue and black no saturation: those keep the ones the user chose.

/// h in degrees 0 … 360, s and v 0 … 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn new(h: f32, s: f32, v: f32) -> Self {
        Hsv { h: h.rem_euclid(360.0), s: s.clamp(0.0, 1.0), v: v.clamp(0.0, 1.0) }
    }

    /// The 8-bit colour.
    pub fn to_rgb(self) -> [u8; 3] {
        let c = self.v * self.s;
        let sector = self.h.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (sector % 2.0 - 1.0).abs());
        let (r, g, b) = match sector as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.v - c;
        [r, g, b].map(|k| ((k + m) * 255.0).round().clamp(0.0, 255.0) as u8)
    }

    /// The colour as `#rrggbb`.
    pub fn to_hex(self) -> String {
        let [r, g, b] = self.to_rgb();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// The HSV of `rgb`. What the colour doesn't say (the hue of a grey, the saturation of
    /// black) comes from `keep`.
    pub fn from_rgb(rgb: [u8; 3], keep: Hsv) -> Hsv {
        let [r, g, b] = rgb.map(|k| k as f32 / 255.0);
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        if max == 0.0 {
            return Hsv { v: 0.0, ..keep };
        }
        if delta == 0.0 {
            return Hsv { s: 0.0, v: max, ..keep };
        }
        let h = if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        Hsv::new(h, delta / max, max)
    }
}

/// `#rgb` / `#rrggbb` (any case, `#` optional) as 8-bit channels.
pub fn parse(hex: &str) -> Option<[u8; 3]> {
    let c = crate::ui::theme::parse_hex(hex)?.into_rgba8();
    Some([c[0], c[1], c[2]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(rgb: [u8; 3]) -> String {
        format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
    }

    #[test]
    fn hsv_round_trip() {
        let any = Hsv::new(0.0, 0.0, 0.0);
        // Every 15th level of every channel: rgb → hsv → rgb gives the same 8-bit colour.
        for r in (0..=255).step_by(15) {
            for g in (0..=255).step_by(15) {
                for b in (0..=255).step_by(15) {
                    let rgb = [r as u8, g as u8, b as u8];
                    assert_eq!(Hsv::from_rgb(rgb, any).to_rgb(), rgb, "{}", hex(rgb));
                }
            }
        }
        for swatch in crate::ui::theme::SWATCHES.map(|(_, h)| h) {
            assert_eq!(Hsv::from_rgb(parse(swatch).unwrap(), any).to_hex(), swatch);
        }
        let red = Hsv::from_rgb([255, 0, 0], any);
        assert_eq!((red.h, red.s, red.v), (0.0, 1.0, 1.0));
        let blue = Hsv::from_rgb([0x0a, 0x84, 0xff], any);
        assert!((blue.h - 211.0).abs() < 1.0, "{blue:?}");
        assert_eq!(Hsv::new(120.0, 1.0, 1.0).to_hex(), "#00ff00");
        assert_eq!(Hsv::new(360.0, 1.0, 1.0).to_hex(), "#ff0000", "360° is red again");
        assert_eq!(Hsv::new(-10.0, 2.0, -1.0), Hsv::new(350.0, 1.0, 0.0), "out of range is folded/clamped");
    }

    #[test]
    fn grey_keeps_hue() {
        let chosen = Hsv::new(200.0, 0.8, 0.6);
        let grey = Hsv::from_rgb([128, 128, 128], chosen);
        assert_eq!(grey.h, 200.0, "a grey has no hue: the chosen one stays");
        assert_eq!(grey.s, 0.0);
        assert!((grey.v - 128.0 / 255.0).abs() < 1e-6);
        let white = Hsv::from_rgb([255, 255, 255], chosen);
        assert_eq!((white.h, white.s, white.v), (200.0, 0.0, 1.0));
        // Black has neither hue nor saturation: the cursor stays in its column.
        let black = Hsv::from_rgb([0, 0, 0], chosen);
        assert_eq!((black.h, black.s, black.v), (200.0, 0.8, 0.0));
        // A colour with a hue of its own takes it.
        assert!((Hsv::from_rgb([255, 0, 0], chosen).h).abs() < 1e-6);
    }
}
