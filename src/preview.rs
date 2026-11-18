// EH3D-PHOENIX-LASER - the preview of a G-code program.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Reads the G-code of a laser (the ones this program writes and the common ones of other programs) and says what it would do before the laser does it: the moves, how much is burned and how
//! much is travelled, the box it covers, the powers it uses, a time that counts the speed and the corners, and the things that look dangerous (the laser on during a rapid move, a burning
//! move with no feed, a power above the full power of the controller, work outside the bed, no end). It also draws the path as an SVG picture: the travel thin and dashed, the burn in a
//! colour that goes from light to deep with the power. Nothing here moves or turns on anything; it only reads text.

use crate::vector::{bounds_of, Pt};

/// A program with more lines than this is read only up to there (and says so).
pub const MAX_LINES: usize = 5_000_000;
/// A program with more moves than this is read only up to there (and says so).
pub const MAX_MOVES: usize = 2_000_000;
/// The most notes a reading gives.
pub const MAX_NOTES: usize = 20;
/// A number with a bigger value than this (in mm, mm/min or S) is left out as rubbish.
pub const MAX_COORDINATE: f64 = 1e9;
/// The levels of power the picture tells apart.
pub const LEVELS: usize = 8;
/// A turn of more than this many degrees counts as a corner for the time.
pub const CORNER_DEGREES: f64 = 30.0;

/// One straight move in the plane, in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Move {
    pub from: Pt,
    pub to: Pt,
    pub rapid: bool,
    /// The `S` value while it burns; 0 when the laser is off during the move.
    pub power: f64,
    /// mm/min; 0 when the program gave none.
    pub feed: f64,
    pub line: usize,
}

impl Move {
    pub fn length(&self) -> f64 {
        ((self.to.0 - self.from.0).powi(2) + (self.to.1 - self.from.1).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub moves: Vec<Move>,
    /// Whether the program ends with `M2` or `M30` after its last move, or leaves the spindle off.
    pub ended: bool,
    /// The highest `S` the program sets, burning or not.
    pub max_s: f64,
    pub notes: Vec<String>,
}

fn note(notes: &mut Vec<String>, text: String) {
    if notes.len() < MAX_NOTES && !notes.contains(&text) {
        notes.push(text);
    }
}

/// The words of a line: a letter and the number after it. Comments in brackets or after `;`, a checksum after `*`, and the line number `N` are left out.
fn words(line: &str) -> Vec<(char, f64)> {
    let mut clean = String::with_capacity(line.len());
    let mut depth = 0u32;
    for c in line.chars() {
        match c {
            ';' | '*' if depth == 0 => break,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            _ => clean.push(c.to_ascii_uppercase()),
        }
    }
    let chars: Vec<char> = clean.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let start = j;
            if j < chars.len() && (chars[j] == '-' || chars[j] == '+') {
                j += 1;
            }
            while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                j += 1;
            }
            let text: String = chars[start..j].iter().collect();
            if let Ok(v) = text.parse::<f64>() {
                if v.is_finite() && v.abs() <= MAX_COORDINATE && c != 'N' {
                    out.push((c, v));
                }
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

/// Reads a program. It never fails: what it cannot understand is left out and told in the notes.
pub fn read(text: &str) -> Program {
    let mut p = Program::default();
    let (mut x, mut y) = (0.0f64, 0.0f64);
    let (mut absolute, mut inch) = (true, false);
    let mut motion: Option<u8> = None; // 0 rapid, 1 linear, 2 and 3 arcs
    let mut feed = 0.0f64;
    let mut s = 0.0f64;
    let mut spindle = false;
    let mut arcs = false;
    let mut ended_by_word = false;
    for (n, raw) in text.lines().enumerate() {
        if n >= MAX_LINES {
            note(
                &mut p.notes,
                format!("only the first {MAX_LINES} lines were read"),
            );
            break;
        }
        let line = n + 1;
        if raw.trim_start().starts_with('%') {
            continue;
        }
        let ws = words(raw);
        if ws.is_empty() {
            continue;
        }
        let (mut nx, mut ny): (Option<f64>, Option<f64>) = (None, None);
        let mut new_feed: Option<f64> = None;
        let mut ended_here = false;
        for &(letter, v) in &ws {
            let code = v.round() as i64;
            match letter {
                'G' => match code {
                    0 => motion = Some(0),
                    1 => motion = Some(1),
                    2 | 3 => {
                        motion = Some(code as u8);
                        arcs = true;
                    }
                    20 => inch = true,
                    21 => inch = false,
                    90 => absolute = true,
                    91 => absolute = false,
                    _ => {}
                },
                'M' => match code {
                    3 | 4 => spindle = true,
                    5 => spindle = false,
                    2 | 30 => ended_here = true,
                    _ => {}
                },
                'X' => nx = Some(v),
                'Y' => ny = Some(v),
                'F' => new_feed = Some(v),
                'S' => {
                    s = v.max(0.0);
                    if s > p.max_s {
                        p.max_s = s;
                    }
                }
                _ => {}
            }
        }
        let unit = if inch { 25.4 } else { 1.0 };
        if let Some(f) = new_feed {
            feed = (f * unit).max(0.0);
        }
        if nx.is_some() || ny.is_some() {
            let (tx, ty) = if absolute {
                (nx.map_or(x, |v| v * unit), ny.map_or(y, |v| v * unit))
            } else {
                (x + nx.unwrap_or(0.0) * unit, y + ny.unwrap_or(0.0) * unit)
            };
            let rapid = matches!(motion, Some(0));
            if motion.is_none() {
                note(
                    &mut p.notes,
                    format!("line {line}: a move before any G0 or G1 was left out"),
                );
            } else if (tx - x).abs() > 1e-9 || (ty - y).abs() > 1e-9 {
                if p.moves.len() >= MAX_MOVES {
                    note(
                        &mut p.notes,
                        format!("only the first {MAX_MOVES} moves were read"),
                    );
                    return p;
                }
                let burning = !rapid && spindle && s > 0.0;
                if rapid && spindle && s > 0.0 {
                    note(
                        &mut p.notes,
                        format!("line {line}: the laser is on (M3 or M4 with S above 0) during a rapid move"),
                    );
                }
                if burning && feed <= 0.0 {
                    note(
                        &mut p.notes,
                        format!("line {line}: a burning move has no feed (F)"),
                    );
                }
                p.moves.push(Move {
                    from: (x, y),
                    to: (tx, ty),
                    rapid,
                    power: if burning { s } else { 0.0 },
                    feed,
                    line,
                });
            }
            x = tx;
            y = ty;
            ended_by_word = false;
        }
        if ended_here {
            ended_by_word = true;
            spindle = false;
        }
    }
    // the end is M2 or M30 after the last move, or the spindle off
    p.ended = ended_by_word || !spindle;
    if arcs {
        note(
            &mut p.notes,
            "arcs (G2 and G3) are drawn as straight lines to their end".to_string(),
        );
    }
    p
}

/// What is asked of the analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// mm/s^2, for the corners.
    pub accel_mm_s2: f64,
    /// mm/min, for the rapid moves and for a move with no feed.
    pub rapid_mm_min: f64,
    /// The spindle value that is full power; a program that sets more is noted.
    pub s_max: Option<u32>,
    /// The size of the bed, when it is known.
    pub bed_mm: Option<Pt>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            accel_mm_s2: 1000.0,
            rapid_mm_min: 6000.0,
            s_max: None,
            bed_mm: None,
        }
    }
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("accel", self.accel_mm_s2, 1.0, 100_000.0)?;
        range("rapid", self.rapid_mm_min, 1.0, 100_000.0)?;
        if let Some((bx, by)) = self.bed_mm {
            range("bed x", bx, 1.0, 100_000.0)?;
            range("bed y", by, 1.0, 100_000.0)?;
        }
        if self.s_max == Some(0) {
            return Err("s_max is 0: it must be from 1 to 100000".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub moves: usize,
    pub burn_mm: f64,
    pub travel_mm: f64,
    /// The box of every move.
    pub bounds: Option<(Pt, Pt)>,
    /// The box of the moves that burn.
    pub burn_bounds: Option<(Pt, Pt)>,
    pub min_power: f64,
    pub max_power: f64,
    /// Length at the feed of each move, plus the time to brake and speed up at every corner.
    pub seconds: f64,
    pub notes: Vec<String>,
}

fn turn_degrees(a: &Move, b: &Move) -> f64 {
    let (ax, ay) = (a.to.0 - a.from.0, a.to.1 - a.from.1);
    let (bx, by) = (b.to.0 - b.from.0, b.to.1 - b.from.1);
    let (la, lb) = (a.length(), b.length());
    if la < 1e-9 || lb < 1e-9 {
        return 0.0;
    }
    let cos = ((ax * bx + ay * by) / (la * lb)).clamp(-1.0, 1.0);
    cos.acos().to_degrees()
}

/// Works out what a program does.
pub fn analyse(p: &Program, o: &Options) -> Result<Report, String> {
    o.validate()?;
    let mut notes = p.notes.clone();
    let (mut burn, mut travel, mut seconds) = (0.0f64, 0.0f64, 0.0f64);
    let (mut min_power, mut max_power) = (f64::INFINITY, 0.0f64);
    let mut no_feed = false;
    for (i, m) in p.moves.iter().enumerate() {
        let len = m.length();
        if m.power > 0.0 {
            burn += len;
            min_power = min_power.min(m.power);
            max_power = max_power.max(m.power);
        } else {
            travel += len;
        }
        let speed = if m.rapid || m.feed <= 0.0 {
            if !m.rapid {
                no_feed = true;
            }
            o.rapid_mm_min
        } else {
            m.feed
        };
        seconds += len / (speed / 60.0);
        if let Some(prev) = i.checked_sub(1).map(|k| &p.moves[k]) {
            if prev.to == m.from && turn_degrees(prev, m) > CORNER_DEGREES {
                let slow = speed.min(if prev.rapid || prev.feed <= 0.0 {
                    o.rapid_mm_min
                } else {
                    prev.feed
                });
                seconds += (slow / 60.0) / o.accel_mm_s2;
            }
        }
    }
    if no_feed {
        notes.push(format!(
            "a move with no feed is counted at the rapid speed, {} mm/min",
            o.rapid_mm_min
        ));
    }
    if let Some(limit) = o.s_max {
        if p.max_s > f64::from(limit) {
            notes.push(format!(
                "the program sets S{}, above the full power of the controller, S{limit}",
                p.max_s
            ));
        }
    }
    let all = || p.moves.iter().flat_map(|m| [m.from, m.to]);
    let bounds = bounds_of(all());
    let burn_bounds = bounds_of(
        p.moves
            .iter()
            .filter(|m| m.power > 0.0)
            .flat_map(|m| [m.from, m.to]),
    );
    if let (Some((bx, by)), Some((lo, hi))) = (o.bed_mm, bounds) {
        if lo.0 < -1e-6 || lo.1 < -1e-6 || hi.0 > bx + 1e-6 || hi.1 > by + 1e-6 {
            notes.push(format!(
                "the work goes outside of the bed: {:.1},{:.1} to {:.1},{:.1} mm on a bed of {bx} x {by} mm",
                lo.0, lo.1, hi.0, hi.1
            ));
        }
    }
    if !p.moves.is_empty() && !p.ended {
        notes.push("the program does not end with M2, M30 or M5".to_string());
    }
    notes.truncate(MAX_NOTES + 4);
    Ok(Report {
        moves: p.moves.len(),
        burn_mm: burn,
        travel_mm: travel,
        bounds,
        burn_bounds,
        min_power: if min_power.is_finite() {
            min_power
        } else {
            0.0
        },
        max_power,
        seconds,
        notes,
    })
}

fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '"' => "&quot;".chars().collect(),
            '\'' => "&apos;".chars().collect(),
            c if c.is_control() => vec![' '],
            c => vec![c],
        })
        .collect()
}

/// From light to deep: the colour of each level of power.
pub const RAMP: [&str; LEVELS] = [
    "#f6c28b", "#f3a469", "#ef8650", "#e86a3c", "#dc4c2f", "#c93426", "#b0231f", "#8c1519",
];

/// The time in the words of the title of the picture: `1 h 02 min`, `3 min 05 s` or `42 s`.
pub fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h} h {m:02} min")
    } else if m > 0 {
        format!("{m} min {s:02} s")
    } else {
        format!("{s} s")
    }
}

/// The path of the program as an SVG picture `width_px` wide: the travel thin and dashed, the burn in the colour of its power, the bed (when known) and the box of the work. The title says
/// what the program is and how long it takes.
pub fn svg(
    p: &Program,
    r: &Report,
    o: &Options,
    title: &str,
    width_px: f64,
) -> Result<String, String> {
    if !(200.0..=5000.0).contains(&width_px) || !width_px.is_finite() {
        return Err(format!(
            "the width of the picture is {width_px} px: it must be from 200 to 5000"
        ));
    }
    let (lo, hi) = r.bounds.ok_or("the program has no moves to draw")?;
    let mut min = lo;
    let mut max = hi;
    if let Some((bx, by)) = o.bed_mm {
        min = (min.0.min(0.0), min.1.min(0.0));
        max = (max.0.max(bx), max.1.max(by));
    }
    let span_x = (max.0 - min.0).max(1.0);
    let span_y = (max.1 - min.1).max(1.0);
    let margin = 30.0;
    let top = 44.0;
    // as wide as asked, unless that would make it more than four times as tall as it is wide
    let scale = ((width_px - 2.0 * margin) / span_x).min(4.0 * width_px / span_y);
    let width_px = span_x * scale + 2.0 * margin;
    let height_px = span_y * scale + margin + top;
    let px =
        |q: Pt| -> (f64, f64) { (margin + (q.0 - min.0) * scale, top + (max.1 - q.1) * scale) };
    let max_power = if r.max_power > 0.0 { r.max_power } else { 1.0 };
    let mut levels: Vec<String> = vec![String::new(); LEVELS];
    let mut ends: Vec<Option<Pt>> = vec![None; LEVELS];
    let mut travel = String::new();
    let mut travel_end: Option<Pt> = None;
    for m in &p.moves {
        let (a, b) = (px(m.from), px(m.to));
        if m.power > 0.0 {
            let level =
                (((m.power / max_power) * LEVELS as f64).ceil() as usize).clamp(1, LEVELS) - 1;
            if ends[level] == Some(m.from) {
                levels[level].push_str(&format!("L{:.2} {:.2}", b.0, b.1));
            } else {
                levels[level].push_str(&format!("M{:.2} {:.2}L{:.2} {:.2}", a.0, a.1, b.0, b.1));
            }
            ends[level] = Some(m.to);
        } else {
            if travel_end == Some(m.from) {
                travel.push_str(&format!("L{:.2} {:.2}", b.0, b.1));
            } else {
                travel.push_str(&format!("M{:.2} {:.2}L{:.2} {:.2}", a.0, a.1, b.0, b.1));
            }
            travel_end = Some(m.to);
        }
    }
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.2} {h:.2}\">\n",
        w = width_px,
        h = height_px
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"#fbfaf7\"/>\n<text x=\"{margin}\" y=\"26\" font-family=\"sans-serif\" font-size=\"14\" fill=\"#222\">{}</text>\n",
        escape(&format!(
            "{title}: {} moves, {:.0} mm burned, {:.0} mm travelled, {}",
            r.moves,
            r.burn_mm,
            r.travel_mm,
            clock(r.seconds)
        ))
    ));
    if let Some((bx, by)) = o.bed_mm {
        let (a, b) = (px((0.0, by)), px((bx, 0.0)));
        out.push_str(&format!(
            "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#8a8a8a\" stroke-width=\"1\"/>\n",
            a.0,
            a.1,
            b.0 - a.0,
            b.1 - a.1
        ));
    }
    let (a, b) = (px((lo.0, hi.1)), px((hi.0, lo.1)));
    out.push_str(&format!(
        "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#3b6fb6\" stroke-width=\"1\" stroke-dasharray=\"6 4\"/>\n",
        a.0,
        a.1,
        b.0 - a.0,
        b.1 - a.1
    ));
    if !travel.is_empty() {
        out.push_str(&format!(
            "<path d=\"{travel}\" fill=\"none\" stroke=\"#9a9a9a\" stroke-width=\"0.6\" stroke-dasharray=\"3 3\"/>\n"
        ));
    }
    for (k, d) in levels.iter().enumerate() {
        if !d.is_empty() {
            out.push_str(&format!(
                "<path d=\"{d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n",
                RAMP[k]
            ));
        }
    }
    out.push_str("</svg>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_ok(text: &str) -> Program {
        read(text)
    }

    #[test]
    fn a_program_is_read_into_moves_with_their_power_and_feed() {
        let p =
            read_ok("G21\nG90\nM4 S0\nF600\nG0 X10 Y0\nG1 X10 Y10 S500\nG1 X0 Y10 S0\nM5\nM2\n");
        assert_eq!(p.moves.len(), 3);
        assert!(p.moves[0].rapid && p.moves[0].power == 0.0);
        assert!(!p.moves[1].rapid && p.moves[1].power == 500.0 && p.moves[1].feed == 600.0);
        assert_eq!(p.moves[1].from, (10.0, 0.0));
        assert_eq!(p.moves[2].power, 0.0);
        assert!(p.ended);
        assert_eq!(p.max_s, 500.0);
    }

    #[test]
    fn comments_line_numbers_and_checksums_are_left_out() {
        let p = read_ok(
            "N10 G21 (millimetres)\n; a note\nF100 ; slow\nM3 S10\nG1 X5 (to here) Y0 *57\nM5\n",
        );
        assert_eq!(p.moves.len(), 1);
        assert_eq!(p.moves[0].to, (5.0, 0.0));
        assert_eq!(p.moves[0].power, 10.0);
    }

    #[test]
    fn relative_moves_and_inches_are_worked_out() {
        let p = read_ok("G20\nG91\nF1\nM3 S5\nG1 X1 Y0\nG1 X0 Y1\nM5\nM2\n");
        assert_eq!(p.moves[0].to, (25.4, 0.0));
        assert_eq!(p.moves[1].from, (25.4, 0.0));
        assert_eq!(p.moves[1].to, (25.4, 25.4));
        assert_eq!(p.moves[0].feed, 25.4);
    }

    #[test]
    fn the_laser_on_during_a_rapid_move_is_noted() {
        let p = read_ok("M4 S100\nG0 X10\nM5\nM2\n");
        assert!(
            p.notes.iter().any(|n| n.contains("rapid move")),
            "{:?}",
            p.notes
        );
        // a move that is not burning because the spindle is off raises nothing
        let q = read_ok("M5\nG0 X10\nM2\n");
        assert!(q.notes.is_empty(), "{:?}", q.notes);
    }

    #[test]
    fn a_move_with_no_mode_and_arcs_are_told_and_the_rest_is_read() {
        let p = read_ok("G90\nX5 Y5\nG2 X10 Y0 I5 J0\nM2\n");
        assert_eq!(p.moves.len(), 1);
        assert!(
            p.notes.iter().any(|n| n.contains("before any G0")),
            "{:?}",
            p.notes
        );
        assert!(p.notes.iter().any(|n| n.contains("arcs")), "{:?}", p.notes);
    }

    #[test]
    fn rubbish_never_panics_and_gives_no_move() {
        for text in [
            "",
            "\n\n",
            "G1 X",
            "G1 Xabc Y",
            "((((",
            "))))G1 X1",
            "G1 X1e5",
            "G1 X--5",
            "X5 Y5 Z5",
            "\u{0}\u{1}G1\u{2}",
            "G1 X9999999999999999999999999999999999999999999999 Y1",
        ] {
            let p = read(text);
            assert!(p
                .moves
                .iter()
                .all(|m| m.to.0.is_finite() && m.to.1.is_finite()));
        }
    }

    #[test]
    fn the_burnt_length_and_the_time_follow_the_program() {
        // 60 mm at 6000 mm/min is 0.6 s; the rapid of 10 mm at 6000 mm/min is 0.1 s
        let p = read_ok("F6000\nM4 S100\nG1 X60 Y0\nM5\nG0 X60 Y10\nM2\n");
        let r = analyse(&p, &Options::default()).unwrap();
        assert!((r.burn_mm - 60.0).abs() < 1e-9 && (r.travel_mm - 10.0).abs() < 1e-9);
        // a corner of 90 degrees adds the time to brake and speed up: 100 mm/s / 1000 mm/s^2 = 0.1 s
        assert!((r.seconds - 0.8).abs() < 1e-6, "{}", r.seconds);
        assert_eq!((r.min_power, r.max_power), (100.0, 100.0));
        assert_eq!(r.burn_bounds, Some(((0.0, 0.0), (60.0, 0.0))));
        assert_eq!(r.bounds, Some(((0.0, 0.0), (60.0, 10.0))));
    }

    #[test]
    fn a_straight_run_of_moves_has_no_corner_and_a_slow_acceleration_costs_more() {
        let p = read_ok("F6000\nM4 S100\nG1 X10\nG1 X20\nG1 X30\nM5\nM2\n");
        let r = analyse(&p, &Options::default()).unwrap();
        assert!((r.seconds - 0.3).abs() < 1e-6, "{}", r.seconds);
        let corner = read_ok("F6000\nM4 S100\nG1 X10\nG1 X10 Y10\nM5\nM2\n");
        let fast = analyse(&corner, &Options::default()).unwrap().seconds;
        let slow = analyse(
            &corner,
            &Options {
                accel_mm_s2: 100.0,
                ..Options::default()
            },
        )
        .unwrap()
        .seconds;
        assert!(slow > fast);
    }

    #[test]
    fn the_things_that_look_dangerous_are_noted() {
        let p = read_ok("M4 S2000\nF100\nG1 X10\nG1 X50 Y2000\n");
        let o = Options {
            s_max: Some(1000),
            bed_mm: Some((100.0, 100.0)),
            ..Options::default()
        };
        let r = analyse(&p, &o).unwrap();
        let all = r.notes.join(" | ");
        assert!(all.contains("above the full power"), "{all}");
        assert!(all.contains("outside of the bed"), "{all}");
        assert!(all.contains("does not end"), "{all}");
        let nofeed = read_ok("M4 S100\nG1 X10\nM5\nM2\n");
        let all = analyse(&nofeed, &Options::default())
            .unwrap()
            .notes
            .join(" | ");
        assert!(all.contains("no feed"), "{all}");
    }

    #[test]
    fn the_options_have_a_range() {
        for bad in [
            Options {
                accel_mm_s2: 0.5,
                ..Options::default()
            },
            Options {
                rapid_mm_min: f64::NAN,
                ..Options::default()
            },
            Options {
                bed_mm: Some((0.0, 10.0)),
                ..Options::default()
            },
            Options {
                s_max: Some(0),
                ..Options::default()
            },
        ] {
            assert!(analyse(&Program::default(), &bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_clock_says_hours_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0 s");
        assert_eq!(clock(42.4), "42 s");
        assert_eq!(clock(185.0), "3 min 05 s");
        assert_eq!(clock(3720.0), "1 h 02 min");
        assert_eq!(clock(-5.0), "0 s");
    }

    #[test]
    fn the_picture_is_an_svg_with_the_travel_and_a_colour_for_each_level_of_power() {
        let p = read_ok("F1000\nM4 S100\nG1 X10 Y0\nG1 X10 Y10 S900\nM5\nG0 X0 Y0\nM2\n");
        let r = analyse(&p, &Options::default()).unwrap();
        let s = svg(&p, &r, &Options::default(), "a <test> & \"more\"", 900.0).unwrap();
        assert!(s.starts_with("<svg ") && s.trim_end().ends_with("</svg>"));
        assert!(s.contains("a &lt;test&gt; &amp; &quot;more&quot;"), "{s}");
        assert!(s.contains(RAMP[0]) && s.contains(RAMP[LEVELS - 1]));
        assert!(s.contains("stroke-dasharray=\"3 3\""));
        assert_eq!(s.matches("<svg").count(), 1);
        assert!(!s.contains("NaN") && !s.contains("inf"));
    }

    #[test]
    fn the_bed_is_drawn_and_a_program_with_no_moves_has_no_picture() {
        let p = read_ok("M4 S100\nF500\nG1 X10\nM5\nM2\n");
        let o = Options {
            bed_mm: Some((100.0, 50.0)),
            ..Options::default()
        };
        let r = analyse(&p, &o).unwrap();
        let s = svg(&p, &r, &o, "bed", 600.0).unwrap();
        assert!(s.contains("stroke=\"#8a8a8a\""));
        let empty = Program::default();
        let r = analyse(&empty, &Options::default()).unwrap();
        assert!(svg(&empty, &r, &Options::default(), "x", 600.0).is_err());
        let r = analyse(&p, &o).unwrap();
        assert!(svg(&p, &r, &o, "x", 100.0).is_err());
        assert!(svg(&p, &r, &o, "x", f64::NAN).is_err());
    }

    #[test]
    fn what_the_engraving_writes_is_read_back_with_the_same_length_and_power() {
        use crate::dither::Dots;
        use crate::raster::{engrave, Params};
        let (w, h) = (9, 5);
        let burn: Vec<bool> = (0..w * h).map(|i| (i % w * 3 + i / w) % 4 < 2).collect();
        let dots = Dots { w, h, burn };
        let params = Params {
            pixel_mm: 0.5,
            ..Params::default()
        };
        let e = engrave(&dots, &params).unwrap();
        let p = read(&e.gcode);
        let r = analyse(&p, &Options::default()).unwrap();
        assert!(
            (r.burn_mm - e.burn_mm).abs() < 1e-6,
            "{} {}",
            r.burn_mm,
            e.burn_mm
        );
        assert_eq!((r.min_power, r.max_power), (500.0, 500.0));
        assert!(p.ended && p.notes.is_empty(), "{:?}", p.notes);
    }
}
