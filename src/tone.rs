// EH3D-PHOENIX-LASER - the tone curve of a picture before it becomes dots.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A photograph rarely burns well as it is: the greys are too dark for the material, or the picture is flat. The tone curve changes the grey levels before the dithering: the levels are
//! stretched to the full range (`auto_level`), the contrast and the brightness are changed, a gamma lightens or darkens the middle greys and the picture can be inverted. Every step is a
//! table of 256 entries, so the cost is one pass over the picture. With the default values the picture comes out exactly as it went in.

use crate::image::Gray;

/// The changes to the grey levels of a picture. The default changes nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    /// From -100 to 100: a percent of the full range added to every level (positive makes the picture lighter, so it burns less).
    pub brightness: f64,
    /// From -100 to 100: negative flattens the picture, positive pushes the greys away from the middle.
    pub contrast: f64,
    /// From 0.1 to 10: more than 1 lightens the middle greys, less than 1 darkens them; black and white stay where they are.
    pub gamma: f64,
    /// Black becomes white and white becomes black.
    pub invert: bool,
    /// The darkest level of the picture becomes black and the lightest becomes white.
    pub auto_level: bool,
}

impl Default for Tone {
    fn default() -> Self {
        Tone {
            brightness: 0.0,
            contrast: 0.0,
            gamma: 1.0,
            invert: false,
            auto_level: false,
        }
    }
}

impl Tone {
    /// Whether the curve leaves a picture as it is.
    pub fn is_identity(&self) -> bool {
        *self == Tone::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("brightness", self.brightness, -100.0, 100.0)?;
        range("contrast", self.contrast, -100.0, 100.0)?;
        range("gamma", self.gamma, 0.1, 10.0)
    }

    /// The new level for each of the 256 levels, given the darkest and the lightest level of the picture (used only by `auto_level`).
    fn table(&self, lowest: u8, highest: u8) -> [u8; 256] {
        // the usual contrast factor, with the contrast taken from -255 to 255
        let c = self.contrast * 2.55;
        let factor = (259.0 * (c + 255.0)) / (255.0 * (259.0 - c));
        let mut out = [0u8; 256];
        for (level, slot) in out.iter_mut().enumerate() {
            let mut v = level as f64 / 255.0;
            if self.auto_level && highest > lowest {
                let (lo, hi) = (f64::from(lowest) / 255.0, f64::from(highest) / 255.0);
                v = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
            }
            v = ((v - 0.5) * factor + 0.5).clamp(0.0, 1.0);
            v = (v + self.brightness / 100.0).clamp(0.0, 1.0);
            v = v.powf(1.0 / self.gamma);
            if self.invert {
                v = 1.0 - v;
            }
            *slot = (v * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    /// The picture with the curve applied. A curve that is out of range is an error; one that changes nothing gives the picture back as it is.
    pub fn apply(&self, img: &Gray) -> Result<Gray, String> {
        self.validate()?;
        if self.is_identity() {
            return Ok(img.clone());
        }
        let lowest = img.data.iter().copied().min().unwrap_or(0);
        let highest = img.data.iter().copied().max().unwrap_or(255);
        let table = self.table(lowest, highest);
        Ok(Gray {
            w: img.w,
            h: img.h,
            data: img.data.iter().map(|&v| table[usize::from(v)]).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> Gray {
        Gray {
            w: 256,
            h: 1,
            data: (0..=255u8).collect(),
        }
    }

    #[test]
    fn the_default_curve_changes_nothing() {
        let t = Tone::default();
        assert!(t.is_identity());
        assert_eq!(t.apply(&ramp()).unwrap(), ramp());
        // even the table of the default values is the identity, so a curve that only looks neutral cannot drift
        let table = t.table(0, 255);
        for (i, v) in table.iter().enumerate() {
            assert_eq!(usize::from(*v), i);
        }
    }

    #[test]
    fn invert_turns_black_into_white() {
        let t = Tone {
            invert: true,
            ..Tone::default()
        };
        let out = t.apply(&ramp()).unwrap();
        for (i, v) in out.data.iter().enumerate() {
            assert_eq!(usize::from(*v), 255 - i);
        }
    }

    #[test]
    fn brightness_moves_every_level_and_stops_at_the_ends() {
        let lighter = Tone {
            brightness: 20.0,
            ..Tone::default()
        }
        .apply(&ramp())
        .unwrap();
        assert_eq!(lighter.data[0], 51);
        assert_eq!(lighter.data[255], 255);
        assert_eq!(lighter.data[230], 255);
        let darker = Tone {
            brightness: -20.0,
            ..Tone::default()
        }
        .apply(&ramp())
        .unwrap();
        assert_eq!(darker.data[255], 204);
        assert_eq!(darker.data[0], 0);
    }

    #[test]
    fn gamma_moves_the_middle_and_keeps_black_and_white() {
        let lighter = Tone {
            gamma: 2.0,
            ..Tone::default()
        }
        .apply(&ramp())
        .unwrap();
        assert_eq!(lighter.data[0], 0);
        assert_eq!(lighter.data[255], 255);
        assert!(lighter.data[128] > 128);
        let darker = Tone {
            gamma: 0.5,
            ..Tone::default()
        }
        .apply(&ramp())
        .unwrap();
        assert!(darker.data[128] < 128);
        assert_eq!(darker.data[255], 255);
    }

    #[test]
    fn contrast_pushes_the_greys_away_from_the_middle() {
        let sharper = Tone {
            contrast: 50.0,
            ..Tone::default()
        }
        .apply(&ramp())
        .unwrap();
        assert!(sharper.data[64] < 64);
        assert!(sharper.data[192] > 192);
        let flatter = Tone {
            contrast: -50.0,
            ..Tone::default()
        }
        .apply(&ramp())
        .unwrap();
        assert!(flatter.data[64] > 64);
        assert!(flatter.data[192] < 192);
    }

    #[test]
    fn auto_level_stretches_a_flat_picture_to_the_full_range() {
        let flat = Gray {
            w: 3,
            h: 1,
            data: vec![100, 150, 200],
        };
        let out = Tone {
            auto_level: true,
            ..Tone::default()
        }
        .apply(&flat)
        .unwrap();
        assert_eq!(out.data[0], 0);
        assert!((127..=128).contains(&out.data[1]), "{:?}", out.data);
        assert_eq!(out.data[2], 255);
        // a picture of one level has nothing to stretch
        let one = Gray {
            w: 2,
            h: 1,
            data: vec![90, 90],
        };
        let out = Tone {
            auto_level: true,
            ..Tone::default()
        }
        .apply(&one)
        .unwrap();
        assert_eq!(out.data, vec![90, 90]);
    }

    #[test]
    fn without_inversion_a_darker_level_never_becomes_lighter_than_a_lighter_one() {
        for (b, c, g) in [(30.0, 40.0, 0.7), (-30.0, -40.0, 3.0), (100.0, 100.0, 10.0)] {
            let t = Tone {
                brightness: b,
                contrast: c,
                gamma: g,
                ..Tone::default()
            };
            let out = t.apply(&ramp()).unwrap();
            assert!(out.data.windows(2).all(|p| p[0] <= p[1]), "{t:?}");
        }
    }

    #[test]
    fn values_out_of_range_are_refused() {
        for t in [
            Tone {
                brightness: 100.5,
                ..Tone::default()
            },
            Tone {
                contrast: -101.0,
                ..Tone::default()
            },
            Tone {
                gamma: 0.0,
                ..Tone::default()
            },
            Tone {
                gamma: 10.5,
                ..Tone::default()
            },
            Tone {
                gamma: f64::NAN,
                ..Tone::default()
            },
        ] {
            assert!(t.apply(&ramp()).is_err(), "{t:?}");
        }
        // the edges are accepted
        for t in [
            Tone {
                brightness: -100.0,
                contrast: 100.0,
                gamma: 0.1,
                ..Tone::default()
            },
            Tone {
                brightness: 100.0,
                contrast: -100.0,
                gamma: 10.0,
                ..Tone::default()
            },
        ] {
            assert!(t.apply(&ramp()).is_ok(), "{t:?}");
        }
    }
}
