//! RGB color type and helpers used by the lighting engine.

use serde::{Deserialize, Serialize};

/// An 8-bit-per-channel RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);
    pub const RED: Rgb = Rgb::new(255, 0, 0);
    pub const GREEN: Rgb = Rgb::new(0, 255, 0);
    pub const BLUE: Rgb = Rgb::new(0, 0, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    /// Parses `#RRGGBB` or `RRGGBB`.
    pub fn from_hex(s: &str) -> Option<Rgb> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() != 6 || !s.is_ascii() {
            return None;
        }
        let v = u32::from_str_radix(s, 16).ok()?;
        Some(Rgb::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    /// Formats as `#RRGGBB`.
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Multiplies every channel by `factor` (clamped to 0..=1).
    pub fn scale(self, factor: f32) -> Rgb {
        let f = factor.clamp(0.0, 1.0);
        let s = |c: u8| (c as f32 * f + 0.5) as u8;
        Rgb::new(s(self.r), s(self.g), s(self.b))
    }

    /// Linear interpolation from `self` (t = 0) to `other` (t = 1).
    pub fn lerp(self, other: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t + 0.5) as u8;
        Rgb::new(l(self.r, other.r), l(self.g, other.g), l(self.b, other.b))
    }

    /// Builds a color from hue (degrees, any value), saturation and value (0..=1).
    pub fn from_hsv(hue: f32, saturation: f32, value: f32) -> Rgb {
        let h = hue.rem_euclid(360.0) / 60.0;
        let s = saturation.clamp(0.0, 1.0);
        let v = value.clamp(0.0, 1.0);
        let c = v * s;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let q = |f: f32| ((f + m) * 255.0 + 0.5) as u8;
        Rgb::new(q(r), q(g), q(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Rgb::from_hex("#1a2B3c").unwrap();
        assert_eq!(c, Rgb::new(0x1A, 0x2B, 0x3C));
        assert_eq!(c.to_hex(), "#1A2B3C");
        assert_eq!(Rgb::from_hex("123"), None);
        assert_eq!(Rgb::from_hex("zzzzzz"), None);
    }

    #[test]
    fn hsv_primaries() {
        assert_eq!(Rgb::from_hsv(0.0, 1.0, 1.0), Rgb::RED);
        assert_eq!(Rgb::from_hsv(120.0, 1.0, 1.0), Rgb::GREEN);
        assert_eq!(Rgb::from_hsv(240.0, 1.0, 1.0), Rgb::BLUE);
        assert_eq!(Rgb::from_hsv(360.0, 1.0, 1.0), Rgb::RED);
        assert_eq!(Rgb::from_hsv(-120.0, 1.0, 1.0), Rgb::BLUE);
        assert_eq!(Rgb::from_hsv(42.0, 0.0, 1.0), Rgb::WHITE);
    }

    #[test]
    fn scale_and_lerp() {
        assert_eq!(Rgb::WHITE.scale(0.0), Rgb::BLACK);
        assert_eq!(Rgb::WHITE.scale(2.0), Rgb::WHITE);
        assert_eq!(Rgb::new(200, 100, 0).scale(0.5), Rgb::new(100, 50, 0));
        assert_eq!(Rgb::BLACK.lerp(Rgb::WHITE, 0.5), Rgb::new(128, 128, 128));
        assert_eq!(Rgb::RED.lerp(Rgb::BLUE, 1.0), Rgb::BLUE);
    }
}
