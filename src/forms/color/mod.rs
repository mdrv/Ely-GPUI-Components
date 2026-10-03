mod dropper;
mod fields;
mod gradient;
mod picker;
mod swatch;

use gpui::Rgba;
use palette::rgb::Rgb;

pub use dropper::EyeDropper;
pub use gradient::{GradientEditor, GradientStop};
pub use picker::ColorPicker;
pub use swatch::{ColorPalette, ColorSwatch};

/// A color by hue in degrees, then saturation, value and alpha from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hsva {
    pub h: f32,
    pub s: f32,
    pub v: f32,
    pub a: f32,
}

impl Hsva {
    pub(crate) fn to_rgba(self) -> Rgba {
        let h = self.h.rem_euclid(360.0) / 60.0;
        let c = self.v * self.s;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let (r, g, b) = match h as u8 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = self.v - c;
        Rgba {
            color: Rgb::new(r + m, g + m, b + m),
            alpha: self.a,
        }
    }

    /// The HSV form of `rgba`; a gray keeps `hue`, which it cannot carry itself.
    pub(crate) fn from_rgba(rgba: Rgba, hue: f32) -> Self {
        let max = rgba.color.red.max(rgba.color.green).max(rgba.color.blue);
        let min = rgba.color.red.min(rgba.color.green).min(rgba.color.blue);
        let delta = max - min;
        let h = if delta == 0.0 {
            hue
        } else if max == rgba.color.red {
            60.0 * ((rgba.color.green - rgba.color.blue) / delta).rem_euclid(6.0)
        } else if max == rgba.color.green {
            60.0 * ((rgba.color.blue - rgba.color.red) / delta + 2.0)
        } else {
            60.0 * ((rgba.color.red - rgba.color.green) / delta + 4.0)
        };
        Self {
            h,
            s: if max == 0.0 { 0.0 } else { delta / max },
            v: max,
            a: rgba.alpha,
        }
    }
}

/// Reads `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`, with or without the hash.
pub(crate) fn parse_hex(text: &str) -> Result<Rgba, String> {
    let digits = text.trim().trim_start_matches('#');
    let doubled = || digits.chars().flat_map(|ch| [ch, ch]);
    let long: String = match digits.len() {
        3 => doubled().chain("ff".chars()).collect(),
        4 => doubled().collect(),
        6 => format!("{digits}ff"),
        8 => digits.to_string(),
        _ => return Err(format!("{text:?} is not a hex color")),
    };
    let value =
        u32::from_str_radix(&long, 16).map_err(|_| format!("{text:?} is not a hex color"))?;
    let channel = |shift: u32| ((value >> shift) & 0xff) as f32 / 255.0;
    Ok(Rgba {
        color: Rgb::new(channel(24), channel(16), channel(8)),
        alpha: channel(0),
    })
}

/// `#rrggbb`, with alpha appended when it is not opaque.
pub(crate) fn hex(rgba: Rgba) -> String {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let base = format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgba.color.red),
        byte(rgba.color.green),
        byte(rgba.color.blue)
    );
    if byte(rgba.alpha) == 255 {
        base
    } else {
        format!("{base}{:02x}", byte(rgba.alpha))
    }
}

#[cfg(test)]
mod tests {
    use gpui::Rgba;

    use super::{hex, parse_hex, Hsva};

    fn close(a: Rgba, b: Rgba) -> bool {
        [
            (a.color.red, b.color.red),
            (a.color.green, b.color.green),
            (a.color.blue, b.color.blue),
            (a.alpha, b.alpha),
        ]
        .iter()
        .all(|(x, y)| (x - y).abs() < 1e-4)
    }

    #[test]
    fn hsv_round_trips_and_grays_keep_their_hue() {
        let orange = Hsva {
            h: 30.0,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        };
        assert!(close(
            orange.to_rgba(),
            Rgba {
                r: 1.0,
                g: 0.5,
                b: 0.0,
                a: 1.0
            }
        ));
        let back = Hsva::from_rgba(orange.to_rgba(), 0.0);
        assert!((back.h - 30.0).abs() < 1e-3 && (back.s - 1.0).abs() < 1e-4);
        let gray = Hsva::from_rgba(
            Rgba {
                r: 0.5,
                g: 0.5,
                b: 0.5,
                a: 1.0,
            },
            210.0,
        );
        assert_eq!((gray.h, gray.s), (210.0, 0.0));
    }

    #[test]
    fn hex_reads_three_four_six_and_eight_digits() {
        assert_eq!(hex(parse_hex("#0af").unwrap()), "#00aaff");
        assert_eq!(hex(parse_hex("3772bb").unwrap()), "#3772bb");
        assert_eq!(hex(parse_hex("#3772bb80").unwrap()), "#3772bb80");
        assert_eq!(hex(parse_hex("#0008").unwrap()), "#00000088");
        assert!(parse_hex("#12345").is_err());
        assert!(parse_hex("#zzzzzz").is_err());
    }
}
