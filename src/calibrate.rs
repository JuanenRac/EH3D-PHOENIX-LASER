//! A test grid to find the power and the feed that suit a material: a square for every pair of power and feed, burned line by line.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub square_mm: f64,
    pub gap_mm: f64,
    pub line_spacing_mm: f64,
    pub s_max: u32,
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            square_mm: 8.0,
            gap_mm: 3.0,
            line_spacing_mm: 0.1,
            s_max: 1000,
        }
    }
}

pub const MAX_CELLS: usize = 12;

use crate::gcode_out::number as n;

/// The G-code of the grid: one column for each power (a percent of full power) and one row for each feed (mm/min).
pub fn grid(powers_percent: &[f64], feeds_mm_min: &[f64], g: &Grid) -> Result<String, String> {
    if powers_percent.is_empty()
        || feeds_mm_min.is_empty()
        || powers_percent.len() > MAX_CELLS
        || feeds_mm_min.len() > MAX_CELLS
    {
        return Err(format!(
            "the grid needs from 1 to {MAX_CELLS} powers and from 1 to {MAX_CELLS} feeds"
        ));
    }
    if powers_percent
        .iter()
        .any(|p| !p.is_finite() || !(0.0..=100.0).contains(p))
    {
        return Err("a power must be a percent from 0 to 100".into());
    }
    if feeds_mm_min
        .iter()
        .any(|f| !f.is_finite() || !(1.0..=30_000.0).contains(f))
    {
        return Err("a feed must be from 1 to 30000 mm/min".into());
    }
    if !(g.square_mm.is_finite()
        && g.square_mm >= 1.0
        && g.square_mm <= 50.0
        && g.gap_mm.is_finite()
        && g.gap_mm >= 0.0
        && g.gap_mm <= 50.0
        && g.line_spacing_mm.is_finite()
        && g.line_spacing_mm >= 0.02
        && g.line_spacing_mm <= g.square_mm
        && g.s_max > 0
        && g.s_max <= 100_000)
    {
        return Err(
            "the size of the squares, the gap, the spacing of the lines or s_max are out of range"
                .into(),
        );
    }
    let mut out = String::from("G21\nG90\nM4 S0\n");
    let lines = (g.square_mm / g.line_spacing_mm).floor() as usize + 1;
    let mut last_s = 0u32;
    for (row, feed) in feeds_mm_min.iter().enumerate() {
        for (col, power) in powers_percent.iter().enumerate() {
            let x0 = col as f64 * (g.square_mm + g.gap_mm);
            let y0 = row as f64 * (g.square_mm + g.gap_mm);
            let s = (f64::from(g.s_max) * power / 100.0).round() as u32;
            // no rapid move with the power of the last square still set
            if last_s != 0 {
                out.push_str("S0\n");
            }
            out.push_str(&format!(
                "(power {}% feed {})\nG0 X{} Y{}\n",
                n(*power)?,
                n(*feed)?,
                n(x0)?,
                n(y0)?
            ));
            for k in 0..lines {
                let y = y0 + (k as f64 * g.line_spacing_mm).min(g.square_mm);
                let (a, b) = if k % 2 == 0 {
                    (x0, x0 + g.square_mm)
                } else {
                    (x0 + g.square_mm, x0)
                };
                out.push_str(&format!("G1 X{} Y{} S0 F{}\n", n(a)?, n(y)?, n(*feed)?));
                out.push_str(&format!("G1 X{} S{s} F{}\n", n(b)?, n(*feed)?));
            }
            last_s = s;
        }
    }
    out.push_str("M5\nM2\n");
    Ok(out)
}

use crate::vconfig::FocusSettings;

/// The G-code of the focus test: short lines side by side, each burnt with the head at another height (Z), so that the sharpest and thinnest line shows where the beam is in focus. The
/// step of the height has no default: which way is down depends on the machine, and a head sent the wrong way can hit the material. The power of the test is limited by the limit of power
/// of the machine, and the head goes back to the first height at the end.
pub fn focus_test(f: &FocusSettings, s_max: u32, max_power_percent: f64) -> Result<String, String> {
    f.validate()?;
    if f.z_step_mm == 0.0 {
        return Err(
            "the focus step is needed: --z-step MM or z_step_mm in [focus] (which way is down depends on the machine)"
                .to_string(),
        );
    }
    crate::safety::check_power(
        "the power of the focus test",
        f.power_percent,
        max_power_percent,
    )?;
    if s_max == 0 || s_max > 100_000 {
        return Err(format!("s_max is {s_max}: it must be from 1 to 100000"));
    }
    let s = (f64::from(s_max) * f.power_percent / 100.0).round() as u32;
    if s == 0 {
        return Err(format!(
            "a power of {} % of S{s_max} is less than one step of S",
            f.power_percent
        ));
    }
    let mut out = format!("G21\nG90\nM4 S0\nF{}\n", n(f.feed_mm_min)?);
    for k in 0..f.steps {
        let z = n(f.z_start_mm + f64::from(k) * f.z_step_mm)?;
        let y = n(f64::from(k) * f.spacing_mm)?;
        out.push_str(&format!(
            "(line {} of {} at Z{z})\nG0 X0 Y{y}\nG0 Z{z}\nG1 X{} S{s}\nS0\n",
            k + 1,
            f.steps,
            n(f.length_mm)?
        ));
    }
    out.push_str(&format!("G0 Z{}\nM5\nM2\n", n(f.z_start_mm)?));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{analyse, read, Options};

    fn settings() -> FocusSettings {
        FocusSettings {
            z_step_mm: -0.5,
            steps: 5,
            ..FocusSettings::default()
        }
    }

    #[test]
    fn the_focus_test_burns_one_line_for_each_height_and_goes_back() {
        let g = focus_test(&settings(), 1000, 100.0).unwrap();
        for z in ["Z0", "Z-0.5", "Z-1", "Z-1.5", "Z-2"] {
            assert!(g.contains(&format!("G0 {z}\n")), "{z} in {g}");
        }
        assert!(g.ends_with("G0 Z0\nM5\nM2\n"), "{g}");
        let p = read(&g);
        let r = analyse(&p, &Options::default()).unwrap();
        assert!((r.burn_mm - 5.0 * 10.0).abs() < 1e-9, "{}", r.burn_mm);
        assert_eq!((r.min_power, r.max_power), (300.0, 300.0));
        assert!(p.ended && p.notes.is_empty(), "{:?}", p.notes);
        // the lines are 2 mm apart
        assert_eq!(r.burn_bounds, Some(((0.0, 0.0), (10.0, 8.0))));
    }

    #[test]
    fn the_height_goes_the_way_the_step_says() {
        let up = FocusSettings {
            z_step_mm: 0.25,
            z_start_mm: 3.0,
            steps: 3,
            ..FocusSettings::default()
        };
        let g = focus_test(&up, 255, 100.0).unwrap();
        assert!(g.contains("G0 Z3\n") && g.contains("G0 Z3.25\n") && g.contains("G0 Z3.5\n"));
        assert!(g.contains(" S77\n"), "30 % of 255: {g}");
    }

    #[test]
    fn without_a_step_the_test_is_refused() {
        let e = focus_test(&FocusSettings::default(), 1000, 100.0).unwrap_err();
        assert!(e.contains("--z-step"), "{e}");
    }

    #[test]
    fn the_power_of_the_test_obeys_the_limit_and_must_be_at_least_one_step() {
        let e = focus_test(&settings(), 1000, 20.0).unwrap_err();
        assert!(e.contains("max_power_percent"), "{e}");
        assert!(focus_test(&settings(), 1000, 30.0).is_ok());
        let weak = FocusSettings {
            power_percent: 0.01,
            ..settings()
        };
        assert!(focus_test(&weak, 100, 100.0)
            .unwrap_err()
            .contains("one step"));
        assert!(focus_test(&settings(), 0, 100.0).is_err());
        assert!(focus_test(&settings(), 100_001, 100.0).is_err());
    }

    #[test]
    fn a_value_out_of_range_is_refused() {
        for bad in [
            FocusSettings {
                steps: 1,
                ..settings()
            },
            FocusSettings {
                steps: 31,
                ..settings()
            },
            FocusSettings {
                z_step_mm: -20.0,
                steps: 30,
                ..settings()
            },
            FocusSettings {
                length_mm: f64::NAN,
                ..settings()
            },
            FocusSettings {
                spacing_mm: 0.1,
                ..settings()
            },
            FocusSettings {
                z_start_mm: 101.0,
                ..settings()
            },
        ] {
            assert!(focus_test(&bad, 1000, 100.0).is_err(), "{bad:?}");
        }
    }
}
