// EH3D-PHOENIX-LASER - dynamic (M4) or constant (M3) power in the programs that are written.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! `laser_mode` in `[machine]` says how the programs turn the laser on. With `m4` (the default) the controller makes the power follow the speed of the head and puts the beam out when the
//! head stands. With `m3` the power is constant: a head that stands with the beam on burns through. The programs are written with `M4` and, for `m3`, written again here: `M3`, the power at
//! 0 before every rapid move and every stop, and a run-up and a run-out with the laser off, `overscan_mm` long, before and after every burn that begins or ends with the head at rest or
//! turning back on itself, so that the head is at speed while it burns. The corners inside a burnt contour are left as they are: the head slows there and burns more.

use crate::gcode_out::number as n;
use crate::vector::Pt;

/// How the programs turn the laser on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaserMode {
    /// `M4`: the power follows the speed of the head.
    #[default]
    Dynamic,
    /// `M3`: the power is constant.
    Constant,
}

impl LaserMode {
    pub const ALL: [LaserMode; 2] = [LaserMode::Dynamic, LaserMode::Constant];

    pub fn from_word(word: &str) -> Option<LaserMode> {
        match word.trim().to_ascii_lowercase().as_str() {
            "m4" => Some(LaserMode::Dynamic),
            "m3" => Some(LaserMode::Constant),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            LaserMode::Dynamic => "m4",
            LaserMode::Constant => "m3",
        }
    }

    /// The program (written with `M4`) in this mode. `lead_mm` is the run-up and run-out that `m3` needs. With `m4` the program is returned as it is. The bed is not looked at here: the
    /// check that a program goes through before it is written or sent judges the moves that were added like any other.
    pub fn apply(self, program: &str, lead_mm: f64) -> Result<Written, String> {
        match self {
            LaserMode::Dynamic => Ok(Written {
                gcode: program.to_string(),
                extra_mm: 0.0,
                extra_seconds: 0.0,
            }),
            LaserMode::Constant => constant(program, lead_mm),
        }
    }
}

/// A program in its mode, and what the run-ups and run-outs added to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    pub gcode: String,
    /// The length of the moves with the laser off that were added (mm).
    pub extra_mm: f64,
    /// Their time at the feed of the burn they belong to (the rapid moves are not counted).
    pub extra_seconds: f64,
}

/// One line of the program that is written again.
struct Line {
    text: String,
    /// A comment or an empty line: it is copied and counts for nothing.
    comment: bool,
    /// The line names `G0` or `G1`.
    names_motion: bool,
    /// The way of moving after the line: 0 rapid, 1 linear.
    motion: u8,
    from: Pt,
    to: Pt,
    moves_xy: bool,
    /// Both X and Y are written.
    both: bool,
    moves_z: bool,
    /// The line holds an `M` word.
    m_word: bool,
    names_s: bool,
    s: f64,
    feed: f64,
}

impl Line {
    fn plain(text: &str) -> Line {
        Line {
            text: text.to_string(),
            comment: true,
            names_motion: false,
            motion: 0,
            from: (0.0, 0.0),
            to: (0.0, 0.0),
            moves_xy: false,
            both: false,
            moves_z: false,
            m_word: false,
            names_s: false,
            s: 0.0,
            feed: 0.0,
        }
    }

    fn burns(&self) -> bool {
        self.moves_xy && self.motion == 1 && self.s > 0.0
    }

    fn length(&self) -> f64 {
        (self.to.0 - self.from.0).hypot(self.to.1 - self.from.1)
    }

    /// The unit vector of the move (only asked of a line that moves in the plane).
    fn direction(&self) -> Pt {
        let l = self.length();
        ((self.to.0 - self.from.0) / l, (self.to.1 - self.from.1) / l)
    }
}

fn dot(a: Pt, b: Pt) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn distance(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The program as lines with what each does. Only the words the program writes are taken.
fn parse(program: &str) -> Result<Vec<Line>, String> {
    let mut lines = Vec::new();
    let (mut motion, mut s, mut feed) = (0u8, 0.0f64, 0.0f64);
    let (mut x, mut y): (Option<f64>, Option<f64>) = (None, None);
    for (number, raw) in program.lines().enumerate() {
        let code = raw.trim();
        if code.is_empty() || code.starts_with(';') || code.starts_with('(') {
            lines.push(Line::plain(raw));
            continue;
        }
        let at = number + 1;
        let mut line = Line::plain(raw);
        line.comment = false;
        let (mut nx, mut ny) = (None, None);
        let mut words = Vec::new();
        for token in code.split_whitespace() {
            let letter = token.chars().next().unwrap_or(' ');
            let value: f64 = token
                .get(letter.len_utf8()..)
                .and_then(|v| v.parse().ok())
                .filter(|v: &f64| v.is_finite())
                .ok_or_else(|| format!("line {at}: '{token}' is not a word of the program"))?;
            let mut word = token.to_string();
            match letter {
                'G' if value == 0.0 || value == 1.0 => {
                    motion = u8::from(value == 1.0);
                    line.names_motion = true;
                }
                'G' => {}
                'M' => {
                    line.m_word = true;
                    if value == 4.0 {
                        word = "M3".to_string();
                    }
                }
                'X' => nx = Some(value),
                'Y' => ny = Some(value),
                'Z' => line.moves_z = true,
                'F' => feed = value,
                'S' => {
                    s = value;
                    line.names_s = true;
                }
                _ => {
                    return Err(format!(
                        "line {at}: '{token}' is not a word the program writes"
                    ))
                }
            }
            words.push(word);
        }
        line.text = words.join(" ");
        line.motion = motion;
        line.s = s;
        line.feed = feed;
        line.both = nx.is_some() && ny.is_some();
        let known = x.is_some() && y.is_some();
        line.from = (x.unwrap_or(0.0), y.unwrap_or(0.0));
        line.to = (nx.unwrap_or(line.from.0), ny.unwrap_or(line.from.1));
        if nx.is_some() || ny.is_some() {
            line.moves_xy = !known || line.length() > 1e-9;
            if line.burns() && !known {
                return Err(format!("line {at}: a burn from a place that is not known"));
            }
            x = nx.or(x);
            y = ny.or(y);
        }
        lines.push(line);
    }
    Ok(lines)
}

/// The straight move that comes before (`step` -1) or after (`step` 1) line `i` with the head not stopping in between; `None` when a rapid move, a move of Z or an `M` word is met first.
fn neighbour(lines: &[Line], i: usize, step: isize) -> Option<usize> {
    let mut k = i as isize + step;
    while k >= 0 && (k as usize) < lines.len() {
        let l = &lines[k as usize];
        if !l.comment {
            if l.m_word || l.moves_z || (l.moves_xy && l.motion != 1) {
                return None;
            }
            if l.moves_xy {
                return Some(k as usize);
            }
        }
        k += step;
    }
    None
}

/// Whether the burn of line `i` needs a run-up (`step` -1) or a run-out (`step` 1) of its own.
fn needs_lead(lines: &[Line], i: usize, step: isize, lead: f64) -> bool {
    let direction = lines[i].direction();
    let Some(mut k) = neighbour(lines, i, step) else {
        return true;
    };
    if lines[k].burns() {
        // a burn that goes on is left as it is, unless it turns back on itself
        return dot(lines[k].direction(), direction) < -0.99;
    }
    // the straight line that the head already runs on before (or after) the burn
    let mut run = 0.0;
    while dot(lines[k].direction(), direction) > 0.999_999 {
        run += lines[k].length();
        match neighbour(lines, k, step) {
            Some(next) => k = next,
            None => break,
        }
    }
    // the places of a program have three decimals
    run < lead - 0.002
}

/// What has been written so far and where it leaves the controller.
struct Out {
    text: String,
    /// Where the head is.
    head: Pt,
    s: f64,
    motion: u8,
    extra_mm: f64,
    extra_seconds: f64,
}

impl Out {
    fn line(&mut self, text: &str) {
        self.text.push_str(text);
        self.text.push('\n');
    }

    fn laser_off(&mut self) {
        if self.s > 0.0 {
            self.line("S0");
            self.s = 0.0;
        }
    }

    fn rapid(&mut self, to: Pt) -> Result<(), String> {
        self.laser_off();
        self.line(&format!("G0 X{} Y{}", n(to.0)?, n(to.1)?));
        self.extra_mm += distance(self.head, to);
        self.head = to;
        self.motion = 0;
        Ok(())
    }

    /// A straight move with the laser off at `feed`.
    fn dark(&mut self, to: Pt, feed: f64) -> Result<(), String> {
        let mut text = format!("G1 X{} Y{} S0", n(to.0)?, n(to.1)?);
        if feed > 0.0 {
            text.push_str(&format!(" F{}", n(feed)?));
            self.extra_seconds += distance(self.head, to) / (feed / 60.0);
        }
        self.line(&text);
        self.extra_mm += distance(self.head, to);
        self.head = to;
        self.s = 0.0;
        self.motion = 1;
        Ok(())
    }
}

/// The point `length` from `from` in the direction of the unit vector `direction`.
fn along(from: Pt, direction: Pt, length: f64) -> Pt {
    (from.0 + direction.0 * length, from.1 + direction.1 * length)
}

fn constant(program: &str, lead: f64) -> Result<Written, String> {
    if !(lead.is_finite() && lead > 0.0) {
        return Err(
            "laser_mode = m3 needs a run-up with the laser off: overscan_mm must be more than 0"
                .to_string(),
        );
    }
    let lines = parse(program)?;
    let mut out = Out {
        text: String::with_capacity(program.len()),
        head: (0.0, 0.0),
        s: 0.0,
        motion: 0,
        extra_mm: 0.0,
        extra_seconds: 0.0,
    };
    for (i, l) in lines.iter().enumerate() {
        if l.comment {
            out.line(&l.text);
            continue;
        }
        // the beam is out before anything that is not a straight move in the plane with its own power
        let stops = l.m_word || l.moves_z || (l.moves_xy && l.motion == 0);
        if stops && !(l.names_s && l.s == 0.0) {
            out.laser_off();
        }
        let mut run_out = false;
        if l.burns() {
            run_out = needs_lead(&lines, i, 1, lead);
            if needs_lead(&lines, i, -1, lead) {
                let d = l.direction();
                let start = along(l.from, (-d.0, -d.1), lead);
                if distance(out.head, start) > 1e-9 {
                    out.rapid(start)?;
                }
                out.dark(l.from, l.feed)?;
            }
        }
        // a run-out left the head past the end: a line that counts on where it was takes it back first
        if l.moves_xy && distance(out.head, l.from) > 1e-9 && !(l.motion == 0 && l.both) {
            if l.motion == 0 {
                out.rapid(l.from)?;
            } else {
                out.dark(l.from, l.feed)?;
            }
        }
        let mut text = l.text.clone();
        if l.moves_xy {
            if !l.names_motion && out.motion != l.motion {
                text = format!("G{} {text}", l.motion);
            }
            if !l.names_s && l.motion == 1 && out.s != l.s {
                text.push_str(&format!(" S{}", n(l.s)?));
                out.s = l.s;
            }
        }
        out.line(&text);
        if l.names_s {
            out.s = l.s;
        }
        if l.names_motion || l.moves_xy {
            out.motion = l.motion;
        }
        if l.moves_xy {
            out.head = l.to;
        }
        if run_out {
            let end = along(l.to, l.direction(), lead);
            out.dark(end, l.feed)?;
        }
    }
    Ok(Written {
        gcode: out.text,
        extra_mm: out.extra_mm,
        extra_seconds: out.extra_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: &str = "G21\nG90\nM4 S0\nF600\nG0 X10 Y10 S0\nG1 X20 Y10 S500\nG1 X20 Y20\nG1 X10 Y20\nG1 X10 Y10\nS0\nM5\nM2\n";

    fn m3(program: &str) -> String {
        LaserMode::Constant.apply(program, 2.0).unwrap().gcode
    }

    #[test]
    fn the_words_of_the_modes_are_m4_and_m3_and_m4_is_the_default() {
        assert_eq!(LaserMode::default(), LaserMode::Dynamic);
        for m in LaserMode::ALL {
            assert_eq!(LaserMode::from_word(m.word()), Some(m));
        }
        assert_eq!(LaserMode::from_word(" M3 "), Some(LaserMode::Constant));
        assert_eq!(LaserMode::from_word("m5"), None);
    }

    #[test]
    fn dynamic_power_leaves_the_program_as_it_is() {
        let w = LaserMode::Dynamic.apply(SQUARE, 0.0).unwrap();
        assert_eq!((w.gcode.as_str(), w.extra_mm), (SQUARE, 0.0));
    }

    #[test]
    fn constant_power_runs_up_and_out_with_the_laser_off_and_keeps_the_power_of_the_contour() {
        assert_eq!(
            m3(SQUARE),
            "G21\nG90\nM3 S0\nF600\nG0 X10 Y10 S0\nG0 X8 Y10\nG1 X10 Y10 S0 F600\nG1 X20 Y10 S500\nG1 X20 Y20\nG1 X10 Y20\nG1 X10 Y10\nG1 X10 Y8 S0 F600\nS0\nM5\nM2\n"
        );
    }

    #[test]
    fn a_line_that_already_has_its_overscan_gets_nothing_more() {
        let rows = "G21\nG90\nM4 S0\nF600\nG0 X0 Y0\nG1 X2 S0\nG1 X5 S300\nG1 X6 S0\nG1 X9 S300\nG1 X11 S0\nM5\nM2\n";
        assert_eq!(m3(rows), rows.replace("M4", "M3"));
    }

    #[test]
    fn the_beam_is_out_before_a_stop_and_before_a_rapid_move() {
        let g = m3("M4 S0\nF600\nG0 X5 Y5\nG1 X9 S100\nG0 X20 Y5\nG1 X25 S100\nM5\nM2\n");
        assert!(g.contains("G1 X11 Y5 S0 F600\nG0 X20 Y5\n"), "{g}");
        assert!(g.ends_with("G1 X27 Y5 S0 F600\nM5\nM2\n"), "{g}");
    }

    #[test]
    fn a_pass_that_turns_back_runs_out_and_comes_back_in() {
        let g = m3("M4 S0\nF600\nG0 X10 Y10\nG1 X20 S100\nG1 X10\nS0\nM5\nM2\n");
        assert!(
            g.contains("G1 X20 S100\nG1 X22 Y10 S0 F600\nG1 X20 Y10 S0 F600\nG1 X10 S100\n"),
            "{g}"
        );
    }

    #[test]
    fn no_run_up_at_all_and_a_word_the_program_does_not_write_are_refused() {
        for lead in [0.0, -1.0, f64::NAN] {
            assert!(LaserMode::Constant.apply(SQUARE, lead).is_err());
        }
        assert!(LaserMode::Constant.apply("G4 P1\n", 2.0).is_err());
    }
}
