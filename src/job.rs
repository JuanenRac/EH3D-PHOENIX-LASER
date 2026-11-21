// EH3D-PHOENIX-LASER - where the work starts, the frame that holds it and how long it takes.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Three questions that a person asks before the work is run, answered without touching a machine.
//!
//! Where does the work go? The work is placed by one of its nine anchor points (a corner, the middle of a side or the centre of its box), and that point goes to the origin of the machine
//! (`origin_x_mm` and `origin_y_mm` of `[machine]`); `align = page` keeps the drawing where it was drawn instead (an absolute start) and the anchor does not matter then. Which rectangle does
//! the head trace to show it? The box of the work, `margin_mm` of `[frame]` larger on every side, traced with the laser off (the program written here never sets a power), and refused when it
//! would leave the bed. How long does it take? A range: its low end is every move at its own feed with no braking at all, and its high end is the time of the preview of the program, which also
//! brakes and speeds up at every corner at `accel_mm_s2` of `[job]`; the program already holds every pass, so the passes of a layer are in both. Nothing here sends anything to a machine.

use crate::cut;
use crate::frame::{self, Frame};
use crate::preview;
use crate::vconfig::Settings;
use crate::vector::{Drawing, Pt};

/// The nine points of the box of the work that can be put at the origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    BottomLeft,
    Bottom,
    BottomRight,
    Left,
    Center,
    Right,
    TopLeft,
    Top,
    TopRight,
}

impl Anchor {
    /// Every anchor, in the order the words below are listed.
    pub const ALL: [Anchor; 9] = [
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
    ];

    /// The word of the settings file.
    pub fn word(self) -> &'static str {
        match self {
            Anchor::BottomLeft => "bottom_left",
            Anchor::Bottom => "bottom",
            Anchor::BottomRight => "bottom_right",
            Anchor::Left => "left",
            Anchor::Center => "center",
            Anchor::Right => "right",
            Anchor::TopLeft => "top_left",
            Anchor::Top => "top",
            Anchor::TopRight => "top_right",
        }
    }

    /// The anchor a word names (capitals, spaces and hyphens do not matter).
    pub fn from_word(word: &str) -> Option<Anchor> {
        let w = word.trim().to_ascii_lowercase().replace(['-', ' '], "_");
        Anchor::ALL.into_iter().find(|a| a.word() == w)
    }

    /// The point of a box (its lower left and upper right corners) that this anchor names.
    pub fn point(self, lo: Pt, hi: Pt) -> Pt {
        let x = match self {
            Anchor::BottomLeft | Anchor::Left | Anchor::TopLeft => lo.0,
            Anchor::Bottom | Anchor::Center | Anchor::Top => (lo.0 + hi.0) / 2.0,
            Anchor::BottomRight | Anchor::Right | Anchor::TopRight => hi.0,
        };
        let y = match self {
            Anchor::BottomLeft | Anchor::Bottom | Anchor::BottomRight => lo.1,
            Anchor::Left | Anchor::Center | Anchor::Right => (lo.1 + hi.1) / 2.0,
            Anchor::TopLeft | Anchor::Top | Anchor::TopRight => hi.1,
        };
        (x, y)
    }
}

/// The `[job]` section of the settings file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JobSettings {
    /// The point of the box of the work that goes to the origin of the machine.
    pub anchor: Anchor,
    /// The acceleration of the head (mm/s2) that the time of the work counts at every corner.
    pub accel_mm_s2: f64,
}

impl Default for JobSettings {
    fn default() -> Self {
        JobSettings {
            anchor: Anchor::BottomLeft,
            accel_mm_s2: preview::Options::default().accel_mm_s2,
        }
    }
}

impl JobSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.accel_mm_s2.is_finite() && (1.0..=100_000.0).contains(&self.accel_mm_s2) {
            Ok(())
        } else {
            Err(format!(
                "job accel_mm_s2 is {}: it must be from 1 to 100000",
                self.accel_mm_s2
            ))
        }
    }
}

/// The rectangle that the head traces to show where the work lands, with the program that traces it.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkFrame {
    pub frame: Frame,
    /// The G-code of the frame: the laser is never turned on.
    pub gcode: String,
}

/// The box of the work as the plan places it (the shapes of the layers that are not skipped), or `None` when there is nothing to work.
pub fn work_box(drawing: &Drawing, s: &Settings) -> Option<(Pt, Pt)> {
    crate::vector::bounds_of(
        cut::placed_paths(drawing, s)
            .into_iter()
            .flat_map(|(_, pts, _)| pts),
    )
}

/// The frame of the work: its box, `margin_mm` of `[frame]` larger, traced `laps` times at the feed of `[frame]` with the laser off. A frame outside the bed is an error and nothing is cut down.
pub fn work_frame(drawing: &Drawing, s: &Settings) -> Result<WorkFrame, String> {
    s.validate()?;
    let (lo, hi) = work_box(drawing, s)
        .ok_or("there is nothing to frame: every layer is skipped or the drawing is empty")?;
    let (lo, hi) = frame::grow(lo, hi, s.frame.margin_mm);
    let f = Frame {
        lo,
        hi,
        feed_mm_min: s.frame.feed_mm_min,
        laps: s.frame.laps,
        s_max: s.machine.s_max,
        pilot_s: 0,
    };
    let gcode = frame::gcode(&f, s.machine.bed_mm)?;
    Ok(WorkFrame { frame: f, gcode })
}

/// How long the work takes, as a range, and where the time goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// Every move at its own feed (the rapid moves at the rapid speed) with no braking at all: the least the work can take.
    pub low_seconds: f64,
    /// The time of the preview of the program, which brakes and speeds up at every corner at the acceleration of `[job]`.
    pub high_seconds: f64,
    /// The part of the high time in which the laser burns.
    pub burn_seconds: f64,
    /// The acceleration the high time was worked out with (mm/s2).
    pub accel_mm_s2: f64,
    /// The time of each operation (a layer or a sub-layer) with its passes, in the order it is worked.
    pub operations: Vec<OperationTime>,
}

/// The time of one operation.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationTime {
    /// The name the program writes in its comment: `red` or `red/outline`.
    pub name: String,
    pub passes: u32,
    /// With the braking at the corners.
    pub seconds: f64,
}

/// The time of a straight move from rest to rest at a feed (mm/min) and an acceleration (mm/s2): it speeds up, runs at the feed and brakes, and a move too short to reach the feed
/// speeds up to half of its length and brakes for the rest.
pub fn ramp_seconds(distance_mm: f64, feed_mm_min: f64, accel_mm_s2: f64) -> f64 {
    if !(distance_mm.is_finite() && distance_mm > 0.0 && feed_mm_min > 0.0 && accel_mm_s2 > 0.0) {
        return 0.0;
    }
    let v = feed_mm_min / 60.0;
    let to_speed = v * v / accel_mm_s2;
    if distance_mm >= to_speed {
        distance_mm / v + v / accel_mm_s2
    } else {
        2.0 * (distance_mm / accel_mm_s2).sqrt()
    }
}

/// The time of a program with no braking: each move at its own feed, a rapid move or one with no feed at the rapid speed.
pub fn flat_seconds(p: &preview::Program, rapid_mm_min: f64) -> f64 {
    p.moves
        .iter()
        .map(|m| {
            let speed = if m.rapid || m.feed <= 0.0 {
                rapid_mm_min
            } else {
                m.feed
            };
            m.length() / (speed / 60.0)
        })
        .sum()
}

/// How long a drawing takes with the settings given: the whole work is planned once, as it would be written, and read back.
pub fn estimate(drawing: &Drawing, s: &Settings) -> Result<Estimate, String> {
    s.validate()?;
    let planned = cut::plan(drawing, s)?;
    let options = preview::Options {
        accel_mm_s2: s.job.accel_mm_s2,
        rapid_mm_min: s.machine.rapid_mm_min,
        ..preview::Options::default()
    };
    let program = preview::read(&planned.gcode);
    let report = preview::analyse(&program, &options)?;
    let order = crate::sublayers::execution_order(drawing, s);
    let operations = cut::layer_times(drawing, s, s.job.accel_mm_s2)?
        .into_iter()
        .map(|t| {
            let passes = order
                .iter()
                .find(|op| cut::comment_text(&op.name()) == t.key)
                .map_or(1, |op| op.params.passes);
            OperationTime {
                name: t.key,
                passes,
                seconds: t.seconds,
            }
        })
        .collect();
    Ok(Estimate {
        low_seconds: flat_seconds(&program, s.machine.rapid_mm_min),
        high_seconds: report.seconds,
        burn_seconds: report.burn_seconds,
        accel_mm_s2: s.job.accel_mm_s2,
        operations,
    })
}

impl Estimate {
    /// The range in words: `1 min 05 s to 1 min 20 s`.
    pub fn range_text(&self) -> String {
        format!(
            "{} to {}",
            preview::clock(self.low_seconds),
            preview::clock(self.high_seconds)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_anchors_name_nine_points_of_a_box() {
        let (lo, hi) = ((10.0, 20.0), (30.0, 60.0));
        let want = [
            (10.0, 20.0),
            (20.0, 20.0),
            (30.0, 20.0),
            (10.0, 40.0),
            (20.0, 40.0),
            (30.0, 40.0),
            (10.0, 60.0),
            (20.0, 60.0),
            (30.0, 60.0),
        ];
        for (a, w) in Anchor::ALL.into_iter().zip(want) {
            assert_eq!(a.point(lo, hi), w, "{}", a.word());
            assert_eq!(Anchor::from_word(a.word()), Some(a));
        }
        assert_eq!(Anchor::from_word(" Top-Left "), Some(Anchor::TopLeft));
        assert_eq!(Anchor::from_word("middle"), None);
    }

    #[test]
    fn a_short_move_never_reaches_the_feed_and_a_long_one_runs_at_it() {
        // 600 mm/min = 10 mm/s; at 100 mm/s2 it takes 0.5 mm to reach the feed
        let long = ramp_seconds(100.0, 600.0, 100.0);
        assert!((long - (10.0 + 0.1)).abs() < 1e-9, "{long}");
        let short = ramp_seconds(0.1, 600.0, 100.0);
        assert!((short - 2.0 * (0.1f64 / 100.0).sqrt()).abs() < 1e-12);
        // the short move takes less than the feed alone would say plus the braking, and more than the feed alone
        assert!(short > 0.1 / 10.0);
        for bad in [0.0, -1.0, f64::NAN] {
            assert_eq!(ramp_seconds(bad, 600.0, 100.0), 0.0);
        }
        assert_eq!(ramp_seconds(5.0, 0.0, 100.0), 0.0);
    }

    #[test]
    fn the_acceleration_has_a_range() {
        assert!(JobSettings::default().validate().is_ok());
        for bad in [0.0, 0.5, 100_001.0, f64::NAN] {
            let j = JobSettings {
                accel_mm_s2: bad,
                ..JobSettings::default()
            };
            assert!(j.validate().is_err(), "{bad}");
        }
    }
}
