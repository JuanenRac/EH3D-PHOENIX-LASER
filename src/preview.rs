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

/// How far (mm) the straight pieces that an arc (G2 or G3) is cut into may lie from the true arc, when nothing says another (`arc_tolerance_mm` in `[verify]`).
pub const DEFAULT_ARC_TOLERANCE_MM: f64 = 0.01;

/// The smallest and the largest tolerance of an arc (mm).
pub const ARC_TOLERANCE_RANGE_MM: (f64, f64) = (0.0005, 1.0);

/// An arc is never cut into more pieces than this.
pub const MAX_ARC_SEGMENTS: usize = 2000;

/// The centre of the arc of radius `r` (G2 when `clockwise`) that goes from `from` to `to`: a positive radius is the arc of at most half a turn, a negative one the other. `None` when the
/// two points are the same (a full turn needs the centre) or the radius is shorter than half the distance between them.
fn arc_centre(from: Pt, to: Pt, r: f64, clockwise: bool) -> Option<Pt> {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let d = dx.hypot(dy);
    if d < 1e-12 {
        return None;
    }
    let half = d / 2.0;
    let radius = r.abs();
    if radius < half - 1e-9 {
        return None;
    }
    let h = (radius * radius - half * half).max(0.0).sqrt();
    // the centre of a counter-clockwise arc of at most half a turn lies to the left of the chord; clockwise, or the long way round, it is the other side
    let side = if clockwise == (r < 0.0) { 1.0 } else { -1.0 };
    let (px, py) = (-dy / d, dx / d);
    Some((
        (from.0 + to.0) / 2.0 + side * h * px,
        (from.1 + to.1) / 2.0 + side * h * py,
    ))
}

/// The points of an arc round `c` from `from` to `to` (the last one is `to` itself), no piece lying farther than `tolerance` from the arc. The same point at both ends is a full turn.
fn arc_points(from: Pt, to: Pt, c: Pt, clockwise: bool, tolerance: f64) -> Vec<Pt> {
    let r = (from.0 - c.0).hypot(from.1 - c.1);
    let r_end = (to.0 - c.0).hypot(to.1 - c.1);
    let a0 = (from.1 - c.1).atan2(from.0 - c.0);
    let a1 = (to.1 - c.1).atan2(to.0 - c.0);
    let mut sweep = a1 - a0;
    if clockwise {
        if sweep >= -1e-12 {
            sweep -= std::f64::consts::TAU;
        }
    } else if sweep <= 1e-12 {
        sweep += std::f64::consts::TAU;
    }
    let step = if r > tolerance {
        2.0 * (1.0 - tolerance / r).acos()
    } else {
        std::f64::consts::PI
    };
    let n = ((sweep.abs() / step).ceil() as usize).clamp(1, MAX_ARC_SEGMENTS);
    (1..=n)
        .map(|k| {
            if k == n {
                to
            } else {
                let t = k as f64 / n as f64;
                let angle = a0 + sweep * t;
                let radius = r + (r_end - r) * t;
                (c.0 + radius * angle.cos(), c.1 + radius * angle.sin())
            }
        })
        .collect()
}

/// Reads a program; the arcs (G2 and G3) are cut into pieces no farther than [`DEFAULT_ARC_TOLERANCE_MM`] from the true arc. It never fails: what it cannot understand is left out and told in the notes.
pub fn read(text: &str) -> Program {
    read_with(text, DEFAULT_ARC_TOLERANCE_MM)
}

/// Reads a program like [`read`], with the tolerance of the arcs given (mm; out of [`ARC_TOLERANCE_RANGE_MM`] it is held to the nearest end).
pub fn read_with(text: &str, arc_tolerance_mm: f64) -> Program {
    let tolerance = if arc_tolerance_mm.is_finite() {
        arc_tolerance_mm.clamp(ARC_TOLERANCE_RANGE_MM.0, ARC_TOLERANCE_RANGE_MM.1)
    } else {
        DEFAULT_ARC_TOLERANCE_MM
    };
    let mut p = Program::default();
    let (mut x, mut y) = (0.0f64, 0.0f64);
    let (mut absolute, mut inch) = (true, false);
    let mut motion: Option<u8> = None; // 0 rapid, 1 linear, 2 and 3 arcs
    let mut feed = 0.0f64;
    let mut s = 0.0f64;
    let mut spindle = false;
    // the plane of the arcs (17, 18 or 19) and whether the centre of an arc is given as a point (G90.1) and not as the offset from where it starts (G91.1, the usual way)
    let mut plane = 17u32;
    let mut centre_absolute = false;
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
        let (mut ci, mut cj, mut radius): (Option<f64>, Option<f64>, Option<f64>) =
            (None, None, None);
        let mut new_feed: Option<f64> = None;
        let mut ended_here = false;
        for &(letter, v) in &ws {
            let code = v.round() as i64;
            match letter {
                'G' => match (v * 10.0).round() as i64 {
                    0 => motion = Some(0),
                    10 => motion = Some(1),
                    20 => motion = Some(2),
                    30 => motion = Some(3),
                    170 => plane = 17,
                    180 => plane = 18,
                    190 => plane = 19,
                    200 => inch = true,
                    210 => inch = false,
                    900 => absolute = true,
                    901 => centre_absolute = true,
                    910 => absolute = false,
                    911 => centre_absolute = false,
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
                'I' => ci = Some(v),
                'J' => cj = Some(v),
                'R' => radius = Some(v),
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
        let is_arc = matches!(motion, Some(2 | 3));
        let has_centre = ci.is_some() || cj.is_some() || radius.is_some();
        // an arc with its centre and no end is a full turn
        if nx.is_some() || ny.is_some() || (is_arc && has_centre) {
            let (tx, ty) = if absolute {
                (nx.map_or(x, |v| v * unit), ny.map_or(y, |v| v * unit))
            } else {
                (x + nx.unwrap_or(0.0) * unit, y + ny.unwrap_or(0.0) * unit)
            };
            let rapid = matches!(motion, Some(0));
            let path: Vec<Pt> = if motion.is_none() {
                note(
                    &mut p.notes,
                    format!("line {line}: a move before any G0 or G1 was left out"),
                );
                Vec::new()
            } else if is_arc {
                let clockwise = motion == Some(2);
                let centre = if !has_centre {
                    None
                } else if let Some(r) = radius.filter(|_| ci.is_none() && cj.is_none()) {
                    arc_centre((x, y), (tx, ty), r * unit, clockwise)
                } else if centre_absolute {
                    Some((ci.map_or(x, |v| v * unit), cj.map_or(y, |v| v * unit)))
                } else {
                    Some((x + ci.unwrap_or(0.0) * unit, y + cj.unwrap_or(0.0) * unit))
                };
                match centre {
                    Some(c) if plane == 17 => {
                        let radius_from = (x - c.0).hypot(y - c.1);
                        let radius_to = (tx - c.0).hypot(ty - c.1);
                        let gap = (radius_from - radius_to).abs();
                        if gap > 0.5 || gap > 0.005 * radius_to {
                            note(
                                &mut p.notes,
                                format!("line {line}: the two ends of an arc are not at the same distance from its centre ({gap:.3} mm apart): GRBL refuses it"),
                            );
                        }
                        arc_points((x, y), (tx, ty), c, clockwise, tolerance)
                    }
                    Some(_) => {
                        note(
                            &mut p.notes,
                            format!("line {line}: an arc outside of the XY plane (G18 or G19) is drawn as a straight line to its end"),
                        );
                        vec![(tx, ty)]
                    }
                    None => {
                        note(
                            &mut p.notes,
                            format!("line {line}: an arc with no centre, or a radius too short for its ends, is drawn as a straight line to its end"),
                        );
                        vec![(tx, ty)]
                    }
                }
            } else {
                vec![(tx, ty)]
            };
            for to in path {
                if (to.0 - x).abs() > 1e-9 || (to.1 - y).abs() > 1e-9 {
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
                        to,
                        rapid,
                        power: if burning { s } else { 0.0 },
                        feed,
                        line,
                    });
                }
                x = to.0;
                y = to.1;
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
    /// The optical power of the laser at full power (W): with it the report tells the energy that the program burns. It needs `s_max`.
    pub laser_watts: Option<f64>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            accel_mm_s2: 1000.0,
            rapid_mm_min: 6000.0,
            s_max: None,
            bed_mm: None,
            laser_watts: None,
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
        if let Some(w) = self.laser_watts {
            range("laser watts", w, 0.01, 10_000.0)?;
            if self.s_max.is_none() {
                return Err(
                    "the energy needs the full power of the controller: give s_max (--smax) with the power of the laser"
                        .to_string(),
                );
            }
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
    /// The time of the moves that burn, at their feeds (the corners are not counted: the head is not at speed there).
    pub burn_seconds: f64,
    /// The energy that the moves that burn put in the material (J): the power of each move, as a part of the power of the laser, over the time it burns. Known only when the power of the
    /// laser was given.
    pub energy_j: Option<f64>,
    /// The energy over the time of the moves that burn (W), when the energy is known and something burns.
    pub mean_power_w: Option<f64>,
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
    let (mut burn_seconds, mut power_seconds) = (0.0f64, 0.0f64);
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
        let moving = len / (speed / 60.0);
        seconds += moving;
        if m.power > 0.0 {
            burn_seconds += moving;
            power_seconds += m.power * moving;
        }
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
    // the energy: S over the full S is the part of the power of the laser that burns
    let energy_j = match (o.laser_watts, o.s_max) {
        (Some(w), Some(full)) => Some(w * power_seconds / f64::from(full)),
        _ => None,
    };
    let mean_power_w = energy_j
        .filter(|_| burn_seconds > 0.0)
        .map(|e| e / burn_seconds);
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
        burn_seconds,
        energy_j,
        mean_power_w,
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
    fn a_move_with_no_mode_is_told_and_an_arc_after_it_is_still_read() {
        let p = read_ok("G90\nX5 Y5\nG2 X10 Y0 I5 J0\nM2\n");
        assert!(p.moves.len() > 1, "the arc is cut into pieces");
        assert_eq!(p.moves.last().unwrap().to, (10.0, 0.0));
        assert!(
            p.notes.iter().any(|n| n.contains("before any G0")),
            "{:?}",
            p.notes
        );
        assert!(!p.notes.iter().any(|n| n.contains("arc")), "{:?}", p.notes);
    }

    /// The length of the moves of a program.
    fn total_mm(p: &Program) -> f64 {
        p.moves.iter().map(Move::length).sum()
    }

    #[test]
    fn a_half_turn_counter_clockwise_is_a_half_circle_and_clockwise_goes_the_other_way() {
        // from (0, 0) to (10, 0) about (5, 0)
        let ccw = read_ok("G90\nG0 X0 Y0\nG3 X10 Y0 I5 J0\n");
        let cw = read_ok("G90\nG0 X0 Y0\nG2 X10 Y0 I5 J0\n");
        // a half turn of radius 5 is 5 pi long
        for p in [&ccw, &cw] {
            let len = total_mm(p);
            assert!((len - 5.0 * std::f64::consts::PI).abs() < 0.03, "{len}");
        }
        // G3 turns counter-clockwise: from the point at the left of the centre it goes down first
        let first = ccw.moves[0].to;
        assert!(first.1 < 0.0, "{first:?}");
        let first = cw.moves[0].to;
        assert!(first.1 > 0.0, "{first:?}");
        let (lo, hi) =
            crate::vector::bounds_of(cw.moves.iter().flat_map(|m| [m.from, m.to])).unwrap();
        assert!(
            (hi.1 - 5.0).abs() < 0.01 && lo.1.abs() < 1e-9,
            "{lo:?} {hi:?}"
        );
    }

    #[test]
    fn an_arc_with_a_centre_and_no_end_is_a_full_circle() {
        let p = read_ok("G90\nG0 X10 Y0\nG3 I-10 J0\n");
        let len: f64 = p.moves.iter().filter(|m| !m.rapid).map(Move::length).sum();
        assert!((len - 20.0 * std::f64::consts::PI).abs() < 0.05, "{len}");
        assert_eq!(p.moves.last().unwrap().to, (10.0, 0.0));
        let (lo, hi) =
            crate::vector::bounds_of(p.moves.iter().flat_map(|m| [m.from, m.to])).unwrap();
        assert!(
            (lo.0 + 10.0).abs() < 0.01 && (hi.0 - 10.0).abs() < 1e-9,
            "{lo:?} {hi:?}"
        );
        assert!(
            (lo.1 + 10.0).abs() < 0.01 && (hi.1 - 10.0).abs() < 0.01,
            "{lo:?} {hi:?}"
        );
    }

    #[test]
    fn the_radius_form_takes_the_short_way_with_a_positive_radius_and_the_long_way_with_a_negative_one(
    ) {
        // a quarter circle of radius 10 from (0, 0) to (10, 10) counter-clockwise
        let short = read_ok("G90\nG0 X0 Y0\nG3 X10 Y10 R10\n");
        assert!((total_mm(&short) - 5.0 * std::f64::consts::PI).abs() < 0.02);
        // the long way round is three quarters of the same circle
        let long = read_ok("G90\nG0 X0 Y0\nG3 X10 Y10 R-10\n");
        assert!((total_mm(&long) - 15.0 * std::f64::consts::PI).abs() < 0.05);
        // clockwise with a positive radius is the other quarter circle, bulging the other way
        let cw = read_ok("G90\nG0 X0 Y0\nG2 X10 Y10 R10\n");
        assert!((total_mm(&cw) - 5.0 * std::f64::consts::PI).abs() < 0.02);
        let high = |p: &Program| p.moves.iter().map(|m| m.to.1).fold(f64::MIN, f64::max);
        let right = |p: &Program| p.moves.iter().map(|m| m.to.0).fold(f64::MIN, f64::max);
        // counter-clockwise the centre is at (0, 10) and the arc bulges to the lower right; clockwise it is at (10, 0) and the arc bulges to the upper left
        let (lo_short, _) = crate::vector::bounds_of(short.moves.iter().map(|m| m.to)).unwrap();
        assert!(lo_short.1 >= 0.0 && short.moves[0].to.0 > short.moves[0].to.1);
        assert!(cw.moves[0].to.1 > cw.moves[0].to.0);
        assert!(right(&short) <= 10.0 + 1e-9 && high(&short) <= 10.0 + 1e-9);
        assert!(right(&cw) <= 10.0 + 1e-9 && high(&cw) <= 10.0 + 1e-9);
        // a radius shorter than half the chord cannot be drawn: a straight line and a note
        let short_radius = read_ok("G90\nG0 X0 Y0\nG2 X10 Y0 R2\n");
        assert_eq!(short_radius.moves.len(), 1);
        assert!(short_radius
            .notes
            .iter()
            .any(|n| n.contains("radius too short")));
    }

    #[test]
    fn the_pieces_of_an_arc_lie_within_the_tolerance_and_a_finer_tolerance_gives_more_pieces() {
        let text = "G90\nG0 X10 Y0\nG3 X-10 Y0 I-10 J0\n";
        let coarse = read_with(text, 0.1);
        let fine = read_with(text, 0.001);
        assert!(fine.moves.len() > coarse.moves.len());
        for p in [&coarse, &fine] {
            for m in &p.moves {
                // the points are on the circle and the middle of each piece is no farther from the circle than the tolerance asked
                let r = m.to.0.hypot(m.to.1);
                assert!((r - 10.0).abs() < 1e-9, "{r}");
            }
        }
        for (p, tol) in [(&coarse, 0.1), (&fine, 0.001)] {
            for m in p.moves.iter().filter(|m| !m.rapid) {
                let mid = ((m.from.0 + m.to.0) / 2.0, (m.from.1 + m.to.1) / 2.0);
                assert!(10.0 - mid.0.hypot(mid.1) <= tol + 1e-9, "{m:?}");
            }
        }
        // the tolerance is held to its range and a number that is not one means the default
        assert_eq!(read_with(text, f64::NAN), read(text));
        assert_eq!(
            read_with(text, 0.0),
            read_with(text, ARC_TOLERANCE_RANGE_MM.0)
        );
        assert_eq!(
            read_with(text, 50.0),
            read_with(text, ARC_TOLERANCE_RANGE_MM.1)
        );
    }

    #[test]
    fn the_centre_as_a_point_and_the_other_planes_are_read_as_they_are_written() {
        // G90.1 gives the centre as a point
        let abs = read_ok("G90\nG90.1\nG0 X0 Y0\nG3 X10 Y0 I5 J0\n");
        let rel = read_ok("G90\nG0 X0 Y0\nG3 X10 Y0 I5 J0\n");
        let ends = |p: &Program| p.moves.iter().map(|m| m.to).collect::<Vec<Pt>>();
        assert_eq!(ends(&abs), ends(&rel));
        // G91.1 goes back to the offset, and neither changes how the moves are positioned
        let again = read_ok("G90\nG90.1\nG91.1\nG0 X0 Y0\nG3 X10 Y0 I5 J0\n");
        assert_eq!(ends(&again), ends(&rel));
        let line = read_ok("G90\nG91.1\nG0 X5\nG1 X8\n");
        assert_eq!(line.moves.last().unwrap().to, (8.0, 0.0));
        // an arc in the plane G18 is drawn straight, with a note
        let other = read_ok("G90\nG18\nG0 X0 Y0\nG2 X10 Y10 I5 K0\n");
        assert_eq!(other.moves.len(), 1);
        assert!(other.notes.iter().any(|n| n.contains("G18 or G19")));
    }

    #[test]
    fn an_arc_whose_ends_are_not_at_the_same_distance_from_its_centre_is_told() {
        let p = read_ok("G90\nG0 X0 Y0\nG3 X10 Y3 I5 J0\n");
        assert!(
            p.notes.iter().any(|n| n.contains("same distance")),
            "{:?}",
            p.notes
        );
        assert_eq!(p.moves.last().unwrap().to, (10.0, 3.0));
    }

    #[test]
    fn the_burning_arcs_keep_the_power_the_feed_and_the_line() {
        let p = read_ok("G21\nG90\nM4 S0\nF600\nG0 X0 Y0\nG3 X10 Y0 I5 J0 S250\nM5\nM2\n");
        let arc: Vec<&Move> = p.moves.iter().filter(|m| !m.rapid).collect();
        assert!(arc.len() > 10);
        assert!(arc
            .iter()
            .all(|m| m.power == 250.0 && m.feed == 600.0 && m.line == 6));
        assert_eq!(p.max_s, 250.0);
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
            "G2 I1e5 J1e5",
            "G2 X1e9 Y1e9 R1e9",
            "G3 X0 Y0 R0",
            "G3 I0 J0",
            "G2 X1 Y1 R-0.0001",
            "G90.1 G91.1 G2 X5 I9999999999",
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
    fn the_energy_is_the_power_of_the_laser_over_the_time_each_move_burns() {
        // 60 mm at 600 mm/min is 6 s, at half of the full power of a 10 W laser: 5 W, 30 J
        let p = read_ok("M4 S500\nF600\nG1 X60\nS0\nM5\nM2\n");
        let o = Options {
            s_max: Some(1000),
            laser_watts: Some(10.0),
            ..Options::default()
        };
        let r = analyse(&p, &o).unwrap();
        assert!((r.burn_seconds - 6.0).abs() < 1e-9, "{}", r.burn_seconds);
        assert!(
            (r.energy_j.unwrap() - 30.0).abs() < 1e-9,
            "{:?}",
            r.energy_j
        );
        assert!((r.mean_power_w.unwrap() - 5.0).abs() < 1e-9);
        // without the power of the laser nothing is said about the energy, though the time of the burning still is
        let plain = analyse(&p, &Options::default()).unwrap();
        assert_eq!((plain.energy_j, plain.mean_power_w), (None, None));
        assert!((plain.burn_seconds - 6.0).abs() < 1e-9);
        // two powers and two feeds add up: 20 mm at S1000 and 300 mm/min (4 s) and 30 mm at S250 and 900 mm/min (2 s)
        let p = read_ok("M4 S1000\nF300\nG1 X20\nS250\nF900\nG1 X50\nS0\nM5\nM2\n");
        let r = analyse(&p, &o).unwrap();
        assert!((r.burn_seconds - 6.0).abs() < 1e-9, "{}", r.burn_seconds);
        assert!(
            (r.energy_j.unwrap() - (10.0 * 4.0 + 2.5 * 2.0)).abs() < 1e-9,
            "{:?}",
            r.energy_j
        );
        // the moves with the laser off add nothing, and a program that burns nothing has no mean power
        let off = read_ok("F600\nG1 X60\nM5\nM2\n");
        let r = analyse(&off, &o).unwrap();
        assert_eq!((r.energy_j, r.mean_power_w), (Some(0.0), None));
        assert_eq!(r.burn_seconds, 0.0);
        // a corner costs time but not energy
        let corner = read_ok("M4 S1000\nF6000\nG1 X10\nG1 X10 Y10\nS0\nM5\nM2\n");
        let slow = Options {
            accel_mm_s2: 10.0,
            ..o
        };
        let (a, b) = (
            analyse(&corner, &o).unwrap(),
            analyse(&corner, &slow).unwrap(),
        );
        assert!(b.seconds > a.seconds);
        assert_eq!(a.energy_j, b.energy_j);
        assert!((a.energy_j.unwrap() - 10.0 * 0.2).abs() < 1e-9);
    }

    #[test]
    fn the_power_of_the_laser_needs_the_full_power_of_the_controller_and_has_a_range() {
        let p = read_ok("M4 S500\nF600\nG1 X60\nM5\nM2\n");
        let no_smax = Options {
            laser_watts: Some(10.0),
            ..Options::default()
        };
        assert!(analyse(&p, &no_smax).unwrap_err().contains("s_max"));
        for watts in [0.0, -1.0, 0.001, 10_001.0, f64::NAN, f64::INFINITY] {
            let o = Options {
                s_max: Some(1000),
                laser_watts: Some(watts),
                ..Options::default()
            };
            assert!(analyse(&p, &o).is_err(), "{watts}");
        }
        for watts in [0.01, 10_000.0] {
            let o = Options {
                s_max: Some(1000),
                laser_watts: Some(watts),
                ..Options::default()
            };
            assert!(analyse(&p, &o).is_ok(), "{watts}");
        }
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
