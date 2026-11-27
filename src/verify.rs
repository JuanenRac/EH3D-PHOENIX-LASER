// EH3D-PHOENIX-LASER - the checks a program passes before anyone starts it.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Before a laser program is started, someone has to be able to say exactly what was looked at. `verify` reads a G-code program (this program's or another's) as text, never runs it, and
//! gives its SHA-256 (the hash of the exact file, so that an authorisation can be bound to it and any change afterwards is seen), what it does (moves, burnt and travelled length, time, the
//! box, the highest power and speed) and a verdict: the program passes only when it stays under the limit of power and of speed of the machine, inside its bed, with the laser off during
//! every rapid move and a feed on every burning move. Everything else it finds is a note. The report is plain text or JSON, so that another program can keep it.
//!
//! Work outside the bed is a failure. A program from another program often runs the head a little past the bed with the laser off (the overscan of its hatch lines), so the person can say how
//! far that is allowed (`overscan_mm`, 0 by default): the head may go that far past the bed with the laser off, and anything that burns outside the bed fails whatever the allowance.

use crate::gcode_out::sha256_hex;
use crate::preview::{analyse, read_with, Options};
use crate::vector::Pt;

/// What the program is checked against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// The spindle value that is full power for the controller.
    pub s_max: u32,
    /// The limit of power, a percent of full power.
    pub max_power_percent: f64,
    /// The limit of speed, mm/min.
    pub max_speed_mm_min: f64,
    /// The size of the bed, when it is known.
    pub bed_mm: Option<Pt>,
    /// The acceleration the time is worked out with, mm/s2.
    pub accel_mm_s2: f64,
    /// The speed of the moves with the laser off, mm/min.
    pub rapid_mm_min: f64,
    /// How far (mm) the head may go past the bed with the laser off, as the overscan of another program does. Burning outside the bed always fails.
    pub overscan_mm: f64,
    /// How far (mm) the pieces that an arc (G2 or G3) is cut into may lie from the true arc.
    pub arc_tolerance_mm: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            s_max: 1000,
            max_power_percent: crate::safety::DEFAULT_MAX_POWER_PERCENT,
            max_speed_mm_min: crate::safety::DEFAULT_MAX_SPEED_MM_MIN,
            bed_mm: None,
            accel_mm_s2: 1000.0,
            rapid_mm_min: 6000.0,
            overscan_mm: 0.0,
            arc_tolerance_mm: crate::preview::DEFAULT_ARC_TOLERANCE_MM,
        }
    }
}

/// The checks that depend on the settings file and not on the machine alone: the forbidden zones of the bed and the interlocks.
#[derive(Debug, Clone, Copy, Default)]
pub struct Guard<'a> {
    /// The rectangles the program must keep out of.
    pub zones: &'a [crate::guard::Zone],
    /// The interlocks, when they are to be checked.
    pub interlocks: Option<&'a crate::guard::Interlocks>,
}

/// The zones and the interlocks of a settings file.
pub fn guard_of(s: &crate::vconfig::Settings) -> Guard<'_> {
    Guard {
        zones: &s.zones,
        interlocks: Some(&s.interlocks),
    }
}

/// The most the head may be allowed to go past the bed (mm).
pub const MAX_OVERSCAN_MM: f64 = 100.0;

/// What was found in a program.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    /// The SHA-256 of the bytes of the file, as 64 lowercase hex digits.
    pub sha256: String,
    pub bytes: usize,
    pub moves: usize,
    pub burn_mm: f64,
    pub travel_mm: f64,
    pub seconds: f64,
    /// The box of every move.
    pub bounds: Option<(Pt, Pt)>,
    /// The highest `S` that burns, and the same as a percent of full power.
    pub max_s: f64,
    pub max_power_percent: f64,
    /// The highest feed of a move that is not rapid, mm/min.
    pub max_feed: f64,
    /// The names of the checks of the settings file that were made besides the usual ones (`forbidden_zones`, `interlocks`).
    pub guards: Vec<&'static str>,
    /// What makes the program fail.
    pub problems: Vec<String>,
    /// What is worth knowing and does not make it fail.
    pub notes: Vec<String>,
}

impl Verdict {
    pub fn passes(&self) -> bool {
        self.problems.is_empty()
    }

    /// Adds what a saved listing of the settings of the controller says against this program and its limits (see [`crate::grblset`]): the serious findings make it fail, the others are
    /// notes. The name `controller_settings` goes to the list of checks.
    pub fn add_controller(&mut self, dump: &crate::grblset::Dump, l: &Limits) {
        let found = crate::grblset::check(
            dump,
            &crate::grblset::Expect {
                s_max: l.s_max,
                bed_mm: l.bed_mm,
                max_feed_mm_min: (self.max_feed > 0.0).then_some(self.max_feed),
            },
        );
        self.guards.push("controller_settings");
        for f in found {
            let text = f.text(crate::lang::Lang::En);
            if f.kind.serious() {
                self.problems.push(text);
            } else {
                self.notes.push(text);
            }
        }
    }
}

/// Whether a note of the analysis makes a program fail: a power above the full power of the controller or work outside of the bed. What the reading itself finds (the laser on during a
/// rapid move, a burning move with no feed and the rest) is in the faults of the program.
fn is_problem(note: &str) -> bool {
    ["above the full power", "outside of the bed"]
        .iter()
        .any(|w| note.contains(w))
}

/// Checks a program. `bytes` are the bytes of the file (their hash is the one that is reported).
pub fn verify(bytes: &[u8], l: &Limits) -> Result<Verdict, String> {
    verify_with(bytes, l, &Guard::default())
}

/// Checks a program like [`verify`], and also against the forbidden zones and the interlocks of the settings file: a program that burns in a `no_burn` zone or enters a `no_go` zone fails, and so does
/// any program while the emergency stop is pressed or a required interlock is not in its safe state.
pub fn verify_with(bytes: &[u8], l: &Limits, guard: &Guard) -> Result<Verdict, String> {
    crate::safety::validate_max(l.max_power_percent)?;
    crate::safety::validate_max_speed(l.max_speed_mm_min)?;
    if l.s_max == 0 || l.s_max > 100_000 {
        return Err(format!("s_max is {}: it must be from 1 to 100000", l.s_max));
    }
    if !(l.overscan_mm.is_finite() && (0.0..=MAX_OVERSCAN_MM).contains(&l.overscan_mm)) {
        return Err(format!(
            "overscan_mm is {}: it must be from 0 to {MAX_OVERSCAN_MM}",
            l.overscan_mm
        ));
    }
    let (lo, hi) = crate::preview::ARC_TOLERANCE_RANGE_MM;
    if !(l.arc_tolerance_mm.is_finite() && (lo..=hi).contains(&l.arc_tolerance_mm)) {
        return Err(format!(
            "arc_tolerance_mm is {}: it must be from {lo} to {hi}",
            l.arc_tolerance_mm
        ));
    }
    let program = read_with(&String::from_utf8_lossy(bytes), l.arc_tolerance_mm);
    let report = analyse(
        &program,
        &Options {
            accel_mm_s2: l.accel_mm_s2,
            rapid_mm_min: l.rapid_mm_min,
            s_max: Some(l.s_max),
            // the bed is judged here, where the allowance for the overscan of another program can be given
            bed_mm: None,
            laser_watts: None,
        },
    )?;
    let mut problems: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    if report.moves == 0 {
        problems.push("the program has no move in the plane".to_string());
    }
    if let (Some((bx, by)), Some((lo, hi))) = (l.bed_mm, report.bounds) {
        // how far a box goes past the edges of the bed (0 when it is inside)
        let past = |(lo, hi): (Pt, Pt)| (-lo.0).max(-lo.1).max(hi.0 - bx).max(hi.1 - by).max(0.0);
        let all_past = past((lo, hi));
        let burn_past = report.burn_bounds.map_or(0.0, past);
        let place = format!(
            "{:.1},{:.1} to {:.1},{:.1} mm on a bed of {bx} x {by} mm",
            lo.0, lo.1, hi.0, hi.1
        );
        if burn_past > 1e-6 {
            problems.push(format!("the work burns outside of the bed: {place}"));
        } else if all_past > l.overscan_mm + 1e-6 {
            problems.push(if l.overscan_mm > 0.0 {
                format!("the work goes outside of the bed by {all_past:.1} mm, more than the {} mm allowed for the overscan: {place}", l.overscan_mm)
            } else {
                format!("the work goes outside of the bed: {place}")
            });
        } else if all_past > 1e-6 {
            notes.push(format!(
                "the head goes up to {all_past:.1} mm past the bed with the laser off (the overscan allowed is {} mm)",
                l.overscan_mm
            ));
        }
    }
    // what the reading itself found dangerous or beyond judging, whatever its words
    problems.extend(program.faults.iter().cloned());
    for n in report.notes.iter().filter(|n| !program.faults.contains(n)) {
        if is_problem(n) {
            problems.push(n.clone());
        } else {
            notes.push(n.clone());
        }
    }
    let allowed_s = f64::from(l.s_max) * l.max_power_percent / 100.0;
    let max_burn_s = program
        .moves
        .iter()
        .filter(|m| m.power > 0.0)
        .fold(0.0f64, |a, m| a.max(m.power));
    if max_burn_s > allowed_s + 1e-9 {
        problems.push(format!(
            "the program burns at S{max_burn_s}, which is {:.1} % of full power: more than the limit of power, {} % (S{allowed_s})",
            max_burn_s / f64::from(l.s_max) * 100.0,
            l.max_power_percent
        ));
    }
    let max_feed = program
        .moves
        .iter()
        .filter(|m| !m.rapid)
        .fold(0.0f64, |a, m| a.max(m.feed));
    if max_feed > l.max_speed_mm_min + 1e-9 {
        problems.push(format!(
            "the program moves at {max_feed} mm/min: more than the limit of speed, {} mm/min",
            l.max_speed_mm_min
        ));
    }
    let mut guards: Vec<&'static str> = Vec::new();
    if !guard.zones.is_empty() {
        guards.push("forbidden_zones");
        problems.extend(crate::guard::program_problems(&program, guard.zones));
    }
    if let Some(i) = guard.interlocks.filter(|i| i.is_active()) {
        guards.push("interlocks");
        problems.extend(i.problems());
    }
    Ok(Verdict {
        sha256: sha256_hex(bytes),
        bytes: bytes.len(),
        moves: report.moves,
        burn_mm: report.burn_mm,
        travel_mm: report.travel_mm,
        seconds: report.seconds,
        bounds: report.bounds,
        max_s: max_burn_s,
        max_power_percent: max_burn_s / f64::from(l.s_max) * 100.0,
        max_feed,
        guards,
        problems,
        notes,
    })
}

/// A text for a JSON string: quotes, backslashes and control characters are escaped.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|i| json_string(i)).collect();
    format!("[{}]", parts.join(", "))
}

/// Where the limits of a check came from, for the record that is kept next to an authorisation: the machine profile that was chosen and the SHA-256 of the settings file that was read, so
/// that a change to either afterwards can be seen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Provenance {
    pub profile: Option<String>,
    pub config_sha256: Option<String>,
}

/// The checks that decide the verdict, as they are named in the report. The bed is only checked when its size is known.
pub fn validations(l: &Limits) -> Vec<String> {
    let mut v: Vec<&str> = vec![
        "has_moves",
        "power_limit",
        "speed_limit",
        "laser_off_during_rapid_moves",
        "feed_on_burning_moves",
    ];
    if l.bed_mm.is_some() {
        v.push("inside_bed");
    }
    v.into_iter().map(str::to_string).collect()
}

impl Verdict {
    /// The report as JSON, one key per line, for another program to keep next to the authorisation.
    pub fn json(&self, l: &Limits) -> String {
        self.json_with(l, &Provenance::default())
    }

    /// The report as JSON with the profile and the settings file the limits came from, and the names of the checks that were made.
    pub fn json_with(&self, l: &Limits, p: &Provenance) -> String {
        let text_or_null =
            |v: &Option<String>| v.as_deref().map_or("null".to_string(), json_string);
        let (profile, config) = (text_or_null(&p.profile), text_or_null(&p.config_sha256));
        let bounds = self.bounds.map_or("null".to_string(), |(lo, hi)| {
            format!("[{}, {}, {}, {}]", lo.0, lo.1, hi.0, hi.1)
        });
        let bed = l
            .bed_mm
            .map_or("null".to_string(), |(x, y)| format!("[{x}, {y}]"));
        format!(
            "{{\n  \"toolpath_sha256\": {},\n  \"bytes\": {},\n  \"verdict\": {},\n  \"moves\": {},\n  \"burn_mm\": {},\n  \"travel_mm\": {},\n  \"seconds\": {},\n  \"bounds_mm\": {},\n  \"max_s\": {},\n  \"max_power_percent\": {},\n  \"max_feed_mm_min\": {},\n  \"limits\": {{\"s_max\": {}, \"max_power_percent\": {}, \"max_speed_mm_min\": {}, \"bed_mm\": {}, \"overscan_mm\": {}}},\n  \"machine\": {{\"profile\": {}, \"config_sha256\": {}}},\n  \"validations\": {},\n  \"problems\": {},\n  \"notes\": {}\n}}\n",
            json_string(&self.sha256),
            self.bytes,
            json_string(if self.passes() { "pass" } else { "fail" }),
            self.moves,
            self.burn_mm,
            self.travel_mm,
            self.seconds,
            bounds,
            self.max_s,
            self.max_power_percent,
            self.max_feed,
            l.s_max,
            l.max_power_percent,
            l.max_speed_mm_min,
            bed,
            l.overscan_mm,
            profile,
            config,
            json_list(
                &validations(l)
                    .into_iter()
                    .chain(self.guards.iter().map(|g| g.to_string()))
                    .collect::<Vec<String>>()
            ),
            json_list(&self.problems),
            json_list(&self.notes),
        )
    }

    /// The report as lines of text.
    pub fn text(&self) -> String {
        let mut out = format!(
            "toolpath sha256: {}\n{} bytes, {} moves, {:.0} mm burnt, {:.0} mm of travel, about {}\nhighest power S{} ({:.1} % of full power), highest feed {} mm/min\n",
            self.sha256,
            self.bytes,
            self.moves,
            self.burn_mm,
            self.travel_mm,
            crate::preview::clock(self.seconds),
            self.max_s,
            self.max_power_percent,
            self.max_feed
        );
        for p in &self.problems {
            out.push_str(&format!("problem: {p}\n"));
        }
        for n in &self.notes {
            out.push_str(&format!("note: {n}\n"));
        }
        out.push_str(if self.passes() {
            "verdict: pass\n"
        } else {
            "verdict: FAIL\n"
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "G21\nG90\nF1000\nG0 X0 Y0\nM3 S200\nG1 X10 Y0\nG1 X10 Y10\nM5\nM2\n";

    #[test]
    fn a_good_program_passes_and_its_hash_is_the_one_of_its_bytes() {
        let v = verify(GOOD.as_bytes(), &Limits::default()).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
        assert_eq!(v.sha256, sha256_hex(GOOD.as_bytes()));
        assert_eq!(v.sha256.len(), 64);
        assert_eq!(v.bytes, GOOD.len());
        assert_eq!(v.moves, 2);
        assert!((v.burn_mm - 20.0).abs() < 1e-9);
        assert_eq!(
            (v.max_s, v.max_power_percent, v.max_feed),
            (200.0, 20.0, 1000.0)
        );
        assert_eq!(v.bounds, Some(((0.0, 0.0), (10.0, 10.0))));
        // one more byte, another hash
        let more = format!("{GOOD};");
        assert_ne!(
            verify(more.as_bytes(), &Limits::default()).unwrap().sha256,
            v.sha256
        );
    }

    #[test]
    fn a_program_that_burns_in_a_forbidden_zone_fails_and_the_report_names_the_check() {
        use crate::guard::{Zone, ZoneKind};
        // GOOD burns from 0, 0 to 10, 0 and on to 10, 10
        let zones = [Zone {
            name: "clamp".to_string(),
            x_mm: 8.0,
            y_mm: 4.0,
            width_mm: 5.0,
            height_mm: 2.0,
            kind: ZoneKind::NoBurn,
        }];
        let guard = Guard {
            zones: &zones,
            interlocks: None,
        };
        let v = verify_with(GOOD.as_bytes(), &Limits::default(), &guard).unwrap();
        assert!(!v.passes());
        assert!(
            v.problems
                .iter()
                .any(|p| p.contains("forbidden zone clamp")),
            "{:?}",
            v.problems
        );
        assert_eq!(v.guards, ["forbidden_zones"]);
        let j = v.json(&Limits::default());
        assert!(j.contains("\"forbidden_zones\""), "{j}");
        // the same program with a zone it does not reach passes, and says it was checked
        let away = [Zone {
            y_mm: 50.0,
            ..zones[0].clone()
        }];
        let ok = verify_with(
            GOOD.as_bytes(),
            &Limits::default(),
            &Guard {
                zones: &away,
                interlocks: None,
            },
        )
        .unwrap();
        assert!(ok.passes(), "{:?}", ok.problems);
        assert_eq!(ok.guards, ["forbidden_zones"]);
        // without a guard nothing changes: the checks are the usual six and the hash is the one of the bytes
        let plain = verify(GOOD.as_bytes(), &Limits::default()).unwrap();
        assert!(plain.guards.is_empty());
        assert_eq!(plain.sha256, ok.sha256);
        assert!(!plain.json(&Limits::default()).contains("forbidden_zones"));
    }

    #[test]
    fn a_pressed_emergency_stop_fails_every_program_and_a_released_one_adds_nothing() {
        use crate::guard::{Estop, Interlocks};
        let pressed = Interlocks {
            estop: Estop::Pressed,
            ..Interlocks::default()
        };
        let v = verify_with(
            GOOD.as_bytes(),
            &Limits::default(),
            &Guard {
                zones: &[],
                interlocks: Some(&pressed),
            },
        )
        .unwrap();
        assert!(!v.passes());
        assert!(v.problems[0].contains("emergency stop is pressed"));
        assert_eq!(v.guards, ["interlocks"]);
        // the defaults require nothing: the report has no interlocks check
        let idle = Interlocks::default();
        let ok = verify_with(
            GOOD.as_bytes(),
            &Limits::default(),
            &Guard {
                zones: &[],
                interlocks: Some(&idle),
            },
        )
        .unwrap();
        assert!(ok.passes() && ok.guards.is_empty());
        // a required enclosure whose state is not known stops the program
        let lid = Interlocks {
            require_enclosure: true,
            ..Interlocks::default()
        };
        let stopped = verify_with(
            GOOD.as_bytes(),
            &Limits::default(),
            &Guard {
                zones: &[],
                interlocks: Some(&lid),
            },
        )
        .unwrap();
        assert!(stopped
            .problems
            .iter()
            .any(|p| p.contains("enclosure must be closed")));
    }

    #[test]
    fn the_guard_of_the_settings_holds_their_zones_and_interlocks() {
        let s = crate::vconfig::parse(
            "[zone a]\nwidth_mm = 5\nheight_mm = 5\n[interlocks]\nestop = pressed\n",
        )
        .unwrap();
        let g = guard_of(&s);
        assert_eq!(g.zones.len(), 1);
        assert!(g.interlocks.is_some_and(|i| i.is_active()));
        let plain = crate::vconfig::Settings::default();
        let none = guard_of(&plain);
        assert!(none.zones.is_empty());
    }

    #[test]
    fn a_power_over_the_limit_fails_and_a_power_at_the_limit_does_not() {
        let at = Limits {
            max_power_percent: 20.0,
            ..Limits::default()
        };
        assert!(verify(GOOD.as_bytes(), &at).unwrap().passes());
        let under = Limits {
            max_power_percent: 19.9,
            ..Limits::default()
        };
        let v = verify(GOOD.as_bytes(), &under).unwrap();
        assert!(!v.passes());
        assert!(v.problems[0].contains("limit of power"), "{:?}", v.problems);
        // the full power of the controller decides what S means
        let other = Limits {
            s_max: 100,
            ..Limits::default()
        };
        let v = verify(GOOD.as_bytes(), &other).unwrap();
        assert!(v
            .problems
            .iter()
            .any(|p| p.contains("above the full power")));
    }

    #[test]
    fn a_speed_over_the_limit_fails() {
        let slow = Limits {
            max_speed_mm_min: 999.0,
            ..Limits::default()
        };
        let v = verify(GOOD.as_bytes(), &slow).unwrap();
        assert!(v.problems.iter().any(|p| p.contains("limit of speed")));
        // rapid moves are not counted: they have no feed of their own
        let rapid = "G21\nG90\nM5\nG0 X0 Y0\nG0 X50 Y50\nM5\nM2\n";
        let l = Limits {
            max_speed_mm_min: 100.0,
            ..Limits::default()
        };
        assert!(verify(rapid.as_bytes(), &l).unwrap().passes());
    }

    #[test]
    fn the_laser_on_during_a_rapid_move_and_a_burning_move_with_no_feed_fail() {
        let v = verify(
            b"G21\nG90\nG0 X0 Y0\nM3 S100\nG0 X10 Y0\nG1 X20 Y0\nM5\nM2\n",
            &Limits::default(),
        )
        .unwrap();
        assert!(v.problems.iter().any(|p| p.contains("during a rapid move")));
        assert!(v.problems.iter().any(|p| p.contains("no feed")));
        assert!(!v.passes());
    }

    #[test]
    fn work_outside_the_bed_fails_and_the_end_of_the_program_is_only_a_note() {
        let l = Limits {
            bed_mm: Some((5.0, 5.0)),
            ..Limits::default()
        };
        let v = verify(GOOD.as_bytes(), &l).unwrap();
        assert!(v.problems.iter().any(|p| p.contains("outside of the bed")));
        // with M4 the beam goes out when the head stops, so a missing end is only told
        let no_end = "G21\nG90\nF1000\nG0 X0 Y0\nM4 S100\nG1 X10 Y0\n";
        let v = verify(no_end.as_bytes(), &Limits::default()).unwrap();
        assert!(v.passes());
        assert!(
            v.notes.iter().any(|n| n.contains("does not end")),
            "{:?}",
            v.notes
        );
    }

    #[test]
    fn a_file_with_nothing_to_do_fails_and_a_limit_that_is_not_possible_is_an_error() {
        for junk in [&b""[..], b"not a program", b"(a comment)\n"] {
            let v = verify(junk, &Limits::default()).unwrap();
            assert!(!v.passes());
            assert!(v.problems[0].contains("no move"));
        }
        for bad in [
            Limits {
                max_power_percent: 0.0,
                ..Limits::default()
            },
            Limits {
                max_speed_mm_min: 0.0,
                ..Limits::default()
            },
            Limits {
                s_max: 0,
                ..Limits::default()
            },
            Limits {
                accel_mm_s2: 0.0,
                ..Limits::default()
            },
        ] {
            assert!(verify(GOOD.as_bytes(), &bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_report_is_valid_json_with_the_hash_and_the_verdict() {
        let l = Limits::default();
        let v = verify(GOOD.as_bytes(), &l).unwrap();
        let j = v.json(&l);
        assert!(j.contains(&format!("\"toolpath_sha256\": \"{}\"", v.sha256)));
        assert!(j.contains("\"verdict\": \"pass\""));
        assert!(j.contains("\"bounds_mm\": [0, 0, 10, 10]"), "{j}");
        assert!(j.contains("\"bed_mm\": null"));
        assert!(j.contains("\"problems\": []"));
        assert!(j.starts_with("{\n") && j.ends_with("}\n"));
        let bad = verify(
            b"G0 X0 Y0\nM3 S100\nG0 X1 Y0\nM2\n",
            &Limits {
                bed_mm: Some((1.0, 1.0)),
                ..l
            },
        )
        .unwrap();
        let j = bad.json(&Limits {
            bed_mm: Some((1.0, 1.0)),
            ..l
        });
        assert!(j.contains("\"verdict\": \"fail\"") && j.contains("\"bed_mm\": [1, 1]"));
        assert!(!j.contains("\"problems\": []"));
    }

    #[test]
    fn a_text_for_json_is_escaped() {
        assert_eq!(
            json_string("a\"b\\c\nd\te\u{1}"),
            "\"a\\\"b\\\\c\\nd\\te\\u0001\""
        );
        assert_eq!(
            json_list(&["x".to_string(), "y\"".to_string()]),
            "[\"x\", \"y\\\"\"]"
        );
        assert_eq!(json_list(&[]), "[]");
    }

    #[test]
    fn the_text_says_the_verdict_in_its_last_line() {
        let v = verify(GOOD.as_bytes(), &Limits::default()).unwrap();
        assert!(v.text().ends_with("verdict: pass\n"));
        assert!(v
            .text()
            .starts_with(&format!("toolpath sha256: {}\n", v.sha256)));
        let bad = verify(b"nothing", &Limits::default()).unwrap();
        let t = bad.text();
        assert!(t.contains("problem: the program has no move") && t.ends_with("verdict: FAIL\n"));
    }

    /// A program from another program: the head runs 2 mm past the bed on both sides with the laser off, and burns inside the bed.
    const OVERSCAN: &str =
        "G21\nG90\nF1000\nG0 X-2 Y5\nM3 S0\nG1 X0 Y5\nG1 X10 Y5 S300\nG1 X12 Y5 S0\nM5\nM2\n";

    fn bed10(overscan_mm: f64) -> Limits {
        Limits {
            bed_mm: Some((10.0, 10.0)),
            overscan_mm,
            ..Limits::default()
        }
    }

    #[test]
    fn the_overscan_of_another_program_is_allowed_only_as_far_as_the_person_says() {
        // nothing is allowed by default
        let v = verify(OVERSCAN.as_bytes(), &bed10(0.0)).unwrap();
        assert!(!v.passes());
        assert!(
            v.problems
                .iter()
                .any(|p| p.contains("goes outside of the bed")),
            "{:?}",
            v.problems
        );
        // exactly as far as it goes
        let v = verify(OVERSCAN.as_bytes(), &bed10(2.0)).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
        assert!(
            v.notes
                .iter()
                .any(|n| n.contains("2.0 mm past the bed with the laser off")),
            "{:?}",
            v.notes
        );
        // a little less is a failure that says the allowance
        let v = verify(OVERSCAN.as_bytes(), &bed10(1.9)).unwrap();
        assert!(
            v.problems
                .iter()
                .any(|p| p.contains("more than the 1.9 mm allowed")),
            "{:?}",
            v.problems
        );
        // a program inside the bed has nothing to say about it
        let v = verify(GOOD.as_bytes(), &bed10(5.0)).unwrap();
        assert!(v.passes() && !v.notes.iter().any(|n| n.contains("past the bed")));
    }

    #[test]
    fn burning_outside_the_bed_fails_whatever_the_allowance() {
        let burns = "G21\nG90\nF1000\nG0 X0 Y5\nM3 S300\nG1 X11 Y5\nM5\nM2\n";
        for allowance in [0.0, 2.0, MAX_OVERSCAN_MM] {
            let v = verify(burns.as_bytes(), &bed10(allowance)).unwrap();
            assert!(!v.passes(), "{allowance}");
            assert!(
                v.problems
                    .iter()
                    .any(|p| p.contains("burns outside of the bed")),
                "{:?}",
                v.problems
            );
        }
        // the corner of the bed itself is inside
        let edge = "G21\nG90\nF1000\nG0 X0 Y0\nM3 S300\nG1 X10 Y10\nM5\nM2\n";
        assert!(verify(edge.as_bytes(), &bed10(0.0)).unwrap().passes());
        // with no bed there is nothing to be outside of
        let v = verify(OVERSCAN.as_bytes(), &Limits::default()).unwrap();
        assert!(v.passes());
    }

    #[test]
    fn an_allowance_that_is_not_possible_is_an_error_and_the_report_says_it() {
        for bad in [-0.1, MAX_OVERSCAN_MM + 0.1, f64::NAN, f64::INFINITY] {
            assert!(verify(GOOD.as_bytes(), &bed10(bad)).is_err(), "{bad}");
        }
        let l = bed10(2.0);
        let v = verify(OVERSCAN.as_bytes(), &l).unwrap();
        let j = v.json(&l);
        assert!(
            j.contains("\"bed_mm\": [10, 10], \"overscan_mm\": 2}"),
            "{j}"
        );
        assert!(j.contains("\"verdict\": \"pass\""));
    }

    #[test]
    fn the_report_tells_the_profile_the_settings_hash_and_the_checks_that_were_made() {
        let l = bed10(0.0);
        let v = verify(GOOD.as_bytes(), &l).unwrap();
        // with nothing to say, the machine is null and the checks are named
        let j = v.json(&l);
        assert!(
            j.contains("\"machine\": {\"profile\": null, \"config_sha256\": null}"),
            "{j}"
        );
        assert!(
            j.contains("\"validations\": [\"has_moves\", \"power_limit\", \"speed_limit\", \"laser_off_during_rapid_moves\", \"feed_on_burning_moves\", \"inside_bed\"]"),
            "{j}"
        );
        // with a profile and a settings file
        let p = Provenance {
            profile: Some("small \"diode\"".to_string()),
            config_sha256: Some("ab".repeat(32)),
        };
        let j = v.json_with(&l, &p);
        assert!(
            j.contains(&format!(
                "\"machine\": {{\"profile\": \"small \\\"diode\\\"\", \"config_sha256\": \"{}\"}}",
                "ab".repeat(32)
            )),
            "{j}"
        );
        // the bed is only a check when its size is known
        let names = validations(&Limits::default());
        assert!(!names.iter().any(|n| n == "inside_bed") && names.len() == 5);
    }
}
