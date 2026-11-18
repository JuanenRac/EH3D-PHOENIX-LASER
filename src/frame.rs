// EH3D-PHOENIX-LASER - the frame of the work.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Before the work is run the head traces the rectangle that holds it, so that the person sees where on the material it will land. The frame is traced with the laser off: the program never
//! sets a power unless a pilot power was asked for (and confirmed, see `safety`), and then it is a small constant power (`M3`, so it does not dip in the corners) that is turned off with `M5`
//! as soon as the lap is done. The rectangle comes from a size and a corner, or from the moves that burn in a G-code program, so that the real place of the work is framed.

use crate::gcode_out::number as n;
use crate::vector::Pt;

/// The most laps of the frame.
pub const MAX_LAPS: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The lower left corner of the rectangle.
    pub lo: Pt,
    /// The upper right corner of the rectangle.
    pub hi: Pt,
    pub feed_mm_min: f64,
    pub laps: u32,
    /// The spindle value that is full power for the controller.
    pub s_max: u32,
    /// The spindle value of the pilot power; 0 keeps the laser off.
    pub pilot_s: u32,
}

/// The spindle value of a pilot power given as a percent of full power. A power that rounds to 0 is an error: the person asked for a light and would get none.
pub fn pilot_value(s_max: u32, percent: f64) -> Result<u32, String> {
    if !(percent.is_finite() && percent >= 0.0) {
        return Err(format!(
            "the power of the frame is {percent}: it must be 0 or more"
        ));
    }
    let s = (f64::from(s_max) * percent / 100.0).round() as u32;
    if percent > 0.0 && s == 0 {
        return Err(format!(
            "a power of {percent} % of S{s_max} is less than one step of S: raise it or leave the laser off"
        ));
    }
    Ok(s)
}

/// The rectangle `margin` mm larger on every side.
pub fn grow(lo: Pt, hi: Pt, margin: f64) -> (Pt, Pt) {
    (
        (lo.0 - margin, lo.1 - margin),
        (hi.0 + margin, hi.1 + margin),
    )
}

/// The G-code of the frame. `bed` is the size of the bed when it is known: a frame outside of it is an error.
pub fn gcode(f: &Frame, bed: Option<Pt>) -> Result<String, String> {
    let ok = |v: f64| v.is_finite() && v.abs() <= 100_000.0;
    if !(ok(f.lo.0) && ok(f.lo.1) && ok(f.hi.0) && ok(f.hi.1)) {
        return Err("a corner of the frame is not a number from -100000 to 100000".into());
    }
    if f.hi.0 - f.lo.0 < 0.1 || f.hi.1 - f.lo.1 < 0.1 {
        return Err("the frame is smaller than 0.1 mm on a side".into());
    }
    if !(f.feed_mm_min.is_finite() && (1.0..=30_000.0).contains(&f.feed_mm_min)) {
        return Err(format!(
            "the feed of the frame is {}: it must be from 1 to 30000",
            f.feed_mm_min
        ));
    }
    if f.laps == 0 || f.laps > MAX_LAPS {
        return Err(format!(
            "the laps of the frame are {}: they must be from 1 to {MAX_LAPS}",
            f.laps
        ));
    }
    if f.s_max == 0 || f.s_max > 100_000 || f.pilot_s > f.s_max {
        return Err("s_max is out of range, or the pilot power is above it".into());
    }
    if let Some((bx, by)) = bed {
        if f.lo.0 < -1e-6 || f.lo.1 < -1e-6 || f.hi.0 > bx + 1e-6 || f.hi.1 > by + 1e-6 {
            return Err(format!(
                "the frame goes outside of the bed: {:.1},{:.1} to {:.1},{:.1} mm on a bed of {bx} x {by} mm",
                f.lo.0, f.lo.1, f.hi.0, f.hi.1
            ));
        }
    }
    let (x0, y0, x1, y1) = (n(f.lo.0)?, n(f.lo.1)?, n(f.hi.0)?, n(f.hi.1)?);
    let mut g = format!("G21\nG90\nM5\nF{}\nG0 X{x0} Y{y0}\n", n(f.feed_mm_min)?);
    if f.pilot_s > 0 {
        g.push_str(&format!("M3 S{}\n", f.pilot_s));
    }
    for _ in 0..f.laps {
        g.push_str(&format!(
            "G1 X{x1} Y{y0}\nG1 X{x1} Y{y1}\nG1 X{x0} Y{y1}\nG1 X{x0} Y{y0}\n"
        ));
    }
    if f.pilot_s > 0 {
        g.push_str("M5\n");
    }
    g.push_str("M2\n");
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{analyse, read, Options};

    fn frame() -> Frame {
        Frame {
            lo: (5.0, 10.0),
            hi: (45.0, 30.0),
            feed_mm_min: 3000.0,
            laps: 1,
            s_max: 1000,
            pilot_s: 0,
        }
    }

    #[test]
    fn the_frame_with_the_laser_off_never_sets_a_power_or_turns_the_spindle_on() {
        let g = gcode(&frame(), None).unwrap();
        assert!(
            !g.contains(" S") && !g.contains("M3") && !g.contains("M4"),
            "{g}"
        );
        let p = read(&g);
        assert_eq!(p.moves.len(), 5);
        assert!(p.moves[0].rapid && p.moves[1..].iter().all(|m| !m.rapid && m.power == 0.0));
        let r = analyse(&p, &Options::default()).unwrap();
        assert_eq!(r.burn_mm, 0.0);
        assert!(r.notes.is_empty(), "{:?}", r.notes);
        // the box holds the start of the head at 0,0 too
        assert_eq!(r.bounds, Some(((0.0, 0.0), (45.0, 30.0))));
        // the rapid move to the corner and the four sides: 120 mm
        assert!(
            (r.travel_mm - (125.0f64.sqrt() + 120.0)).abs() < 1e-9,
            "{}",
            r.travel_mm
        );
    }

    #[test]
    fn a_pilot_power_is_a_constant_power_that_is_off_before_the_end() {
        let f = Frame {
            pilot_s: 20,
            laps: 2,
            ..frame()
        };
        let g = gcode(&f, None).unwrap();
        assert!(g.contains("M3 S20\n") && g.contains("M5\nM2\n"), "{g}");
        // the head is at the corner before the spindle turns on
        assert!(g.find("G0 X5 Y10").unwrap() < g.find("M3 S20").unwrap());
        let p = read(&g);
        let r = analyse(&p, &Options::default()).unwrap();
        assert!((r.burn_mm - 240.0).abs() < 1e-9, "{}", r.burn_mm);
        assert_eq!((r.min_power, r.max_power), (20.0, 20.0));
        assert!(p.ended && p.notes.is_empty(), "{:?}", p.notes);
    }

    #[test]
    fn every_lap_goes_round_the_rectangle_and_comes_back_to_the_corner() {
        let f = Frame { laps: 3, ..frame() };
        let p = read(&gcode(&f, None).unwrap());
        assert_eq!(p.moves.len(), 1 + 4 * 3);
        assert_eq!(p.moves.last().unwrap().to, (5.0, 10.0));
        for lap in 0..3 {
            assert_eq!(p.moves[1 + 4 * lap].from, (5.0, 10.0));
            assert_eq!(p.moves[2 + 4 * lap].to, (45.0, 30.0));
        }
    }

    #[test]
    fn a_frame_outside_of_the_bed_or_out_of_range_is_refused() {
        assert!(gcode(&frame(), Some((100.0, 100.0))).is_ok());
        assert!(gcode(&frame(), Some((40.0, 100.0)))
            .unwrap_err()
            .contains("outside of the bed"));
        let low = Frame {
            lo: (-1.0, 10.0),
            ..frame()
        };
        assert!(gcode(&low, Some((100.0, 100.0))).is_err());
        for bad in [
            Frame {
                hi: (5.05, 30.0),
                ..frame()
            },
            Frame {
                feed_mm_min: 0.0,
                ..frame()
            },
            Frame {
                feed_mm_min: f64::NAN,
                ..frame()
            },
            Frame { laps: 0, ..frame() },
            Frame {
                laps: MAX_LAPS + 1,
                ..frame()
            },
            Frame {
                s_max: 0,
                ..frame()
            },
            Frame {
                pilot_s: 1001,
                ..frame()
            },
            Frame {
                lo: (f64::INFINITY, 0.0),
                ..frame()
            },
            Frame {
                hi: (1e9, 30.0),
                ..frame()
            },
        ] {
            assert!(gcode(&bad, None).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_pilot_value_is_a_percent_of_full_power_and_never_rounds_to_nothing() {
        assert_eq!(pilot_value(1000, 0.0).unwrap(), 0);
        assert_eq!(pilot_value(1000, 2.0).unwrap(), 20);
        assert_eq!(pilot_value(255, 5.0).unwrap(), 13);
        assert!(pilot_value(100, 0.4).unwrap_err().contains("one step"));
        assert!(pilot_value(1000, -1.0).is_err());
        assert!(pilot_value(1000, f64::NAN).is_err());
    }

    #[test]
    fn the_rectangle_grows_by_the_margin_on_every_side() {
        assert_eq!(
            grow((5.0, 10.0), (45.0, 30.0), 2.5),
            ((2.5, 7.5), (47.5, 32.5))
        );
        assert_eq!(grow((0.0, 0.0), (1.0, 1.0), 0.0), ((0.0, 0.0), (1.0, 1.0)));
    }
}
