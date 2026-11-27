//! A line of text burnt with a font of strokes of the program's own: the capitals, the figures and a few signs, each drawn as a few lines on a lattice 4 wide and 6 high, so that a name, a
//! serial number or a mark can be burnt with no picture and no font file. A letter that the font has not is refused, never skipped, and the height, the number of characters, the power and
//! the speed are held between limits that the settings can narrow.

use crate::gcode_out::number as n;
use crate::vconfig::{TextAlign, TextSettings};

/// The width of a letter on the lattice of the font.
const GLYPH_W: f64 = 4.0;
/// The height of a capital letter on the lattice of the font.
const GLYPH_H: f64 = 6.0;

type Stroke = &'static [(f64, f64)];

/// The strokes of a character (capitals, figures and `. , - + / : ! ? ( ) = _ '`), or `None` when the font has not it. A space has no strokes.
fn glyph(c: char) -> Option<&'static [Stroke]> {
    Some(match c {
        ' ' => &[],
        'A' => &[
            &[(0.0, 0.0), (0.0, 4.0), (2.0, 6.0), (4.0, 4.0), (4.0, 0.0)],
            &[(0.0, 2.0), (4.0, 2.0)],
        ],
        'B' => &[
            &[
                (0.0, 0.0),
                (0.0, 6.0),
                (3.0, 6.0),
                (4.0, 5.0),
                (4.0, 4.0),
                (3.0, 3.0),
                (0.0, 3.0),
            ],
            &[(3.0, 3.0), (4.0, 2.0), (4.0, 1.0), (3.0, 0.0), (0.0, 0.0)],
        ],
        'C' => &[&[
            (4.0, 5.0),
            (3.0, 6.0),
            (1.0, 6.0),
            (0.0, 5.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (3.0, 0.0),
            (4.0, 1.0),
        ]],
        'D' => &[&[
            (0.0, 0.0),
            (0.0, 6.0),
            (2.0, 6.0),
            (4.0, 4.0),
            (4.0, 2.0),
            (2.0, 0.0),
            (0.0, 0.0),
        ]],
        'E' => &[
            &[(4.0, 6.0), (0.0, 6.0), (0.0, 0.0), (4.0, 0.0)],
            &[(0.0, 3.0), (3.0, 3.0)],
        ],
        'F' => &[
            &[(4.0, 6.0), (0.0, 6.0), (0.0, 0.0)],
            &[(0.0, 3.0), (3.0, 3.0)],
        ],
        'G' => &[&[
            (4.0, 5.0),
            (3.0, 6.0),
            (1.0, 6.0),
            (0.0, 5.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (3.0, 0.0),
            (4.0, 1.0),
            (4.0, 3.0),
            (2.0, 3.0),
        ]],
        'H' => &[
            &[(0.0, 0.0), (0.0, 6.0)],
            &[(4.0, 0.0), (4.0, 6.0)],
            &[(0.0, 3.0), (4.0, 3.0)],
        ],
        'I' => &[
            &[(1.0, 6.0), (3.0, 6.0)],
            &[(2.0, 6.0), (2.0, 0.0)],
            &[(1.0, 0.0), (3.0, 0.0)],
        ],
        'J' => &[&[(4.0, 6.0), (4.0, 1.0), (3.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        'K' => &[
            &[(0.0, 0.0), (0.0, 6.0)],
            &[(4.0, 6.0), (0.0, 2.0)],
            &[(1.0, 3.0), (4.0, 0.0)],
        ],
        'L' => &[&[(0.0, 6.0), (0.0, 0.0), (4.0, 0.0)]],
        'M' => &[&[(0.0, 0.0), (0.0, 6.0), (2.0, 3.0), (4.0, 6.0), (4.0, 0.0)]],
        'N' => &[&[(0.0, 0.0), (0.0, 6.0), (4.0, 0.0), (4.0, 6.0)]],
        'O' => &[&[
            (1.0, 0.0),
            (0.0, 1.0),
            (0.0, 5.0),
            (1.0, 6.0),
            (3.0, 6.0),
            (4.0, 5.0),
            (4.0, 1.0),
            (3.0, 0.0),
            (1.0, 0.0),
        ]],
        'P' => &[&[
            (0.0, 0.0),
            (0.0, 6.0),
            (3.0, 6.0),
            (4.0, 5.0),
            (4.0, 4.0),
            (3.0, 3.0),
            (0.0, 3.0),
        ]],
        'Q' => &[
            &[
                (1.0, 0.0),
                (0.0, 1.0),
                (0.0, 5.0),
                (1.0, 6.0),
                (3.0, 6.0),
                (4.0, 5.0),
                (4.0, 1.0),
                (3.0, 0.0),
                (1.0, 0.0),
            ],
            &[(2.0, 2.0), (4.0, 0.0)],
        ],
        'R' => &[
            &[
                (0.0, 0.0),
                (0.0, 6.0),
                (3.0, 6.0),
                (4.0, 5.0),
                (4.0, 4.0),
                (3.0, 3.0),
                (0.0, 3.0),
            ],
            &[(2.0, 3.0), (4.0, 0.0)],
        ],
        'S' => &[&[
            (4.0, 5.0),
            (3.0, 6.0),
            (1.0, 6.0),
            (0.0, 5.0),
            (0.0, 4.0),
            (1.0, 3.0),
            (3.0, 3.0),
            (4.0, 2.0),
            (4.0, 1.0),
            (3.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
        ]],
        'T' => &[&[(0.0, 6.0), (4.0, 6.0)], &[(2.0, 6.0), (2.0, 0.0)]],
        'U' => &[&[
            (0.0, 6.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (3.0, 0.0),
            (4.0, 1.0),
            (4.0, 6.0),
        ]],
        'V' => &[&[(0.0, 6.0), (2.0, 0.0), (4.0, 6.0)]],
        'W' => &[&[(0.0, 6.0), (1.0, 0.0), (2.0, 3.0), (3.0, 0.0), (4.0, 6.0)]],
        'X' => &[&[(0.0, 6.0), (4.0, 0.0)], &[(4.0, 6.0), (0.0, 0.0)]],
        'Y' => &[
            &[(0.0, 6.0), (2.0, 3.0), (4.0, 6.0)],
            &[(2.0, 3.0), (2.0, 0.0)],
        ],
        'Z' => &[&[(0.0, 6.0), (4.0, 6.0), (0.0, 0.0), (4.0, 0.0)]],
        '0' => &[
            &[
                (1.0, 0.0),
                (0.0, 1.0),
                (0.0, 5.0),
                (1.0, 6.0),
                (3.0, 6.0),
                (4.0, 5.0),
                (4.0, 1.0),
                (3.0, 0.0),
                (1.0, 0.0),
            ],
            &[(1.0, 1.0), (3.0, 5.0)],
        ],
        '1' => &[
            &[(1.0, 5.0), (2.0, 6.0), (2.0, 0.0)],
            &[(1.0, 0.0), (3.0, 0.0)],
        ],
        '2' => &[&[
            (0.0, 5.0),
            (1.0, 6.0),
            (3.0, 6.0),
            (4.0, 5.0),
            (4.0, 4.0),
            (0.0, 0.0),
            (4.0, 0.0),
        ]],
        '3' => &[
            &[
                (0.0, 5.0),
                (1.0, 6.0),
                (3.0, 6.0),
                (4.0, 5.0),
                (4.0, 4.0),
                (3.0, 3.0),
                (1.5, 3.0),
            ],
            &[
                (3.0, 3.0),
                (4.0, 2.0),
                (4.0, 1.0),
                (3.0, 0.0),
                (1.0, 0.0),
                (0.0, 1.0),
            ],
        ],
        '4' => &[&[(3.0, 0.0), (3.0, 6.0), (0.0, 2.0), (4.0, 2.0)]],
        '5' => &[&[
            (4.0, 6.0),
            (0.0, 6.0),
            (0.0, 3.0),
            (3.0, 3.0),
            (4.0, 2.0),
            (4.0, 1.0),
            (3.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
        ]],
        '6' => &[&[
            (4.0, 5.0),
            (3.0, 6.0),
            (1.0, 6.0),
            (0.0, 5.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (3.0, 0.0),
            (4.0, 1.0),
            (4.0, 2.0),
            (3.0, 3.0),
            (0.0, 3.0),
        ]],
        '7' => &[&[(0.0, 6.0), (4.0, 6.0), (1.5, 0.0)]],
        '8' => &[&[
            (1.0, 3.0),
            (0.0, 4.0),
            (0.0, 5.0),
            (1.0, 6.0),
            (3.0, 6.0),
            (4.0, 5.0),
            (4.0, 4.0),
            (3.0, 3.0),
            (1.0, 3.0),
            (0.0, 2.0),
            (0.0, 1.0),
            (1.0, 0.0),
            (3.0, 0.0),
            (4.0, 1.0),
            (4.0, 2.0),
            (3.0, 3.0),
        ]],
        '9' => &[&[
            (0.0, 1.0),
            (1.0, 0.0),
            (3.0, 0.0),
            (4.0, 1.0),
            (4.0, 5.0),
            (3.0, 6.0),
            (1.0, 6.0),
            (0.0, 5.0),
            (0.0, 4.0),
            (1.0, 3.0),
            (4.0, 3.0),
        ]],
        '.' => &[&[(2.0, 0.0), (2.0, 0.5)]],
        ',' => &[&[(2.0, 1.0), (1.5, 0.0)]],
        '-' => &[&[(1.0, 3.0), (3.0, 3.0)]],
        '+' => &[&[(1.0, 3.0), (3.0, 3.0)], &[(2.0, 2.0), (2.0, 4.0)]],
        '/' => &[&[(0.0, 0.0), (4.0, 6.0)]],
        ':' => &[&[(2.0, 1.0), (2.0, 1.5)], &[(2.0, 4.0), (2.0, 4.5)]],
        '!' => &[&[(2.0, 6.0), (2.0, 2.0)], &[(2.0, 0.0), (2.0, 0.5)]],
        '?' => &[
            &[
                (0.0, 5.0),
                (1.0, 6.0),
                (3.0, 6.0),
                (4.0, 5.0),
                (4.0, 4.0),
                (2.0, 3.0),
                (2.0, 2.0),
            ],
            &[(2.0, 0.0), (2.0, 0.5)],
        ],
        '(' => &[&[(3.0, 6.0), (1.0, 4.0), (1.0, 2.0), (3.0, 0.0)]],
        ')' => &[&[(1.0, 6.0), (3.0, 4.0), (3.0, 2.0), (1.0, 0.0)]],
        '=' => &[&[(0.0, 2.0), (4.0, 2.0)], &[(0.0, 4.0), (4.0, 4.0)]],
        '_' => &[&[(0.0, 0.0), (4.0, 0.0)]],
        '\'' => &[&[(2.0, 6.0), (2.0, 4.5)]],
        _ => return None,
    })
}

/// Whether the font has a character (lower case is the same as its capital).
pub fn has_glyph(c: char) -> bool {
    glyph(c.to_ascii_uppercase()).is_some()
}

/// The characters of the font, for the message of a refusal.
pub const ALPHABET: &str = "A-Z, 0-9, space and . , - + / : ! ? ( ) = _ '";

/// The text laid out: the strokes (each a list of points, in mm, with the lower left corner of the whole text at `0, 0`) and the size of the box that holds them.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub strokes: Vec<Vec<(f64, f64)>>,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Lays a text out. Lower case is burnt as capitals, a new line starts a new row, and a character that the font has not, a text with no character to burn, a text longer than
/// `max_chars` or a height over `max_height_mm` is an error.
pub fn layout(text: &str, t: &TextSettings) -> Result<Layout, String> {
    t.validate()?;
    let count = text.chars().count();
    if count > t.max_chars as usize {
        return Err(format!(
            "the text has {count} characters: it can have at most {} (max_chars in [text])",
            t.max_chars
        ));
    }
    let rows: Vec<&str> = text.trim_end_matches(['\n', '\r']).split('\n').collect();
    let mut chars: Vec<Vec<char>> = Vec::new();
    for row in &rows {
        let mut line = Vec::new();
        for c in row.trim_end_matches('\r').chars() {
            let up = c.to_ascii_uppercase();
            if glyph(up).is_none() {
                return Err(format!(
                    "the character {c:?} is not in the font (it has {ALPHABET})"
                ));
            }
            line.push(up);
        }
        chars.push(line);
    }
    if chars.iter().all(|l| l.iter().all(|c| *c == ' ')) {
        return Err("the text has nothing to burn".to_string());
    }
    let unit = t.height_mm / GLYPH_H;
    let gap = t.height_mm * t.spacing_percent / 100.0;
    let advance = GLYPH_W * unit + gap;
    let pitch = t.height_mm * (1.0 + t.line_spacing_percent / 100.0);
    let rows_n = chars.len();
    let mut strokes: Vec<Vec<(f64, f64)>> = Vec::new();
    // the width of a row stops at its last letter that is not a space, and the text is as wide as its widest row
    let row_width: Vec<f64> = chars
        .iter()
        .map(|line| {
            line.iter()
                .rposition(|c| *c != ' ')
                .map_or(0.0, |last| last as f64 * advance + GLYPH_W * unit)
        })
        .collect();
    let width = row_width.iter().copied().fold(0.0f64, f64::max);
    for (k, line) in chars.iter().enumerate() {
        let y = (rows_n - 1 - k) as f64 * pitch;
        // a row that is narrower than the text starts at the left, in the middle or so that it ends at the right
        let shift = match t.align {
            TextAlign::Left => 0.0,
            TextAlign::Centre => (width - row_width[k]) / 2.0,
            TextAlign::Right => width - row_width[k],
        };
        for (i, c) in line.iter().enumerate() {
            let x = shift + i as f64 * advance;
            for s in glyph(*c).unwrap_or(&[]) {
                strokes.push(
                    s.iter()
                        .map(|&(gx, gy)| (x + gx * unit, y + gy * unit))
                        .collect(),
                );
            }
        }
    }
    let height = (rows_n - 1) as f64 * pitch + t.height_mm;
    if t.mirror {
        // the mirror image: every point is as far from the right edge as it was from the left one
        for stroke in &mut strokes {
            for p in stroke.iter_mut() {
                p.0 = width - p.0;
            }
        }
    }
    Ok(Layout {
        strokes,
        width_mm: width,
        height_mm: height,
    })
}

/// What a program of text needs from the machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// The spindle value that is full power.
    pub s_max: u32,
    pub max_power_percent: f64,
    pub max_speed_mm_min: f64,
    /// The size of the bed, when it is known: text that would go outside of it is refused.
    pub bed_mm: Option<(f64, f64)>,
}

/// The G-code of a text whose lower left corner is at `origin`: the head goes to each stroke with the laser off and burns it at the power and the speed of the settings, as many times as
/// `passes` says. A power or a speed over the limit, or a text that would go outside of the bed, is refused and nothing is written.
pub fn program(
    text: &str,
    origin: (f64, f64),
    t: &TextSettings,
    m: &Machine,
) -> Result<(String, Layout), String> {
    crate::safety::validate_max(m.max_power_percent)?;
    crate::safety::check_power(
        "the power of the text",
        t.power_percent,
        m.max_power_percent,
    )?;
    crate::safety::check_speed("the speed of the text", t.speed_mm_min, m.max_speed_mm_min)?;
    if m.s_max == 0 || m.s_max > 100_000 {
        return Err("s_max must be from 1 to 100000".to_string());
    }
    if !(origin.0.is_finite() && origin.1.is_finite()) {
        return Err("the origin of the text is not a number".to_string());
    }
    let l = layout(text, t)?;
    if let Some((bx, by)) = m.bed_mm {
        if origin.0 < 0.0
            || origin.1 < 0.0
            || origin.0 + l.width_mm > bx + 1e-9
            || origin.1 + l.height_mm > by + 1e-9
        {
            return Err(format!(
                "the text is {:.1} x {:.1} mm and, from {}, {}, it goes outside of the bed of {bx} x {by} mm",
                l.width_mm, l.height_mm, origin.0, origin.1
            ));
        }
    }
    // the time of the burning itself: the length of every stroke, in every pass, at the speed of the settings
    let burn_mm: f64 = l
        .strokes
        .iter()
        .map(|s| {
            s.windows(2)
                .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
                .sum::<f64>()
        })
        .sum::<f64>()
        * f64::from(t.passes);
    let minutes = burn_mm / t.speed_mm_min;
    if minutes > t.max_minutes + 1e-9 {
        return Err(format!(
            "burning the text would take about {minutes:.1} minutes ({burn_mm:.0} mm at {} mm/min): it can take at most {} (max_minutes in [text])",
            t.speed_mm_min, t.max_minutes
        ));
    }
    let s = crate::safety::spindle(m.s_max, t.power_percent);
    let feed = n(t.speed_mm_min)?;
    let mut out = String::from("G21\nG90\nM4 S0\n");
    out.push_str(&format!(
        "(text of {} strokes, {} passes)\n",
        l.strokes.len(),
        t.passes
    ));
    let mut lit = false;
    for _ in 0..t.passes {
        for stroke in &l.strokes {
            let Some(((fx, fy), rest)) = stroke.split_first() else {
                continue;
            };
            // no rapid move with the power of the last stroke still set
            if lit {
                out.push_str("S0\n");
                lit = false;
            }
            out.push_str(&format!(
                "G0 X{} Y{}\n",
                n(origin.0 + fx)?,
                n(origin.1 + fy)?
            ));
            for (x, y) in rest {
                out.push_str(&format!(
                    "G1 X{} Y{} S{s} F{feed}\n",
                    n(origin.0 + x)?,
                    n(origin.1 + y)?
                ));
                lit = true;
            }
        }
    }
    out.push_str("S0\nM5\nM2\n");
    Ok((out, l))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::{verify, Limits};

    fn settings() -> TextSettings {
        TextSettings::default()
    }

    fn machine() -> Machine {
        Machine {
            s_max: 1000,
            max_power_percent: 100.0,
            max_speed_mm_min: 30_000.0,
            bed_mm: None,
        }
    }

    #[test]
    fn the_mirror_image_has_every_point_as_far_from_the_right_edge_as_it_was_from_the_left_one() {
        let plain = settings();
        let flipped = TextSettings {
            mirror: true,
            ..plain
        };
        let a = layout("EH3D 42\nLASER", &plain).unwrap();
        let b = layout("EH3D 42\nLASER", &flipped).unwrap();
        assert_eq!((a.width_mm, a.height_mm), (b.width_mm, b.height_mm));
        assert_eq!(a.strokes.len(), b.strokes.len());
        for (s, m) in a.strokes.iter().zip(&b.strokes) {
            assert_eq!(s.len(), m.len());
            for (p, q) in s.iter().zip(m) {
                assert!((q.0 - (a.width_mm - p.0)).abs() < 1e-9 && q.1 == p.1);
                assert!(q.0 >= -1e-9 && q.0 <= a.width_mm + 1e-9);
            }
        }
        // an E has its spine on the left; mirrored, the spine is on the right
        let e = layout("E", &flipped).unwrap();
        assert!(e.strokes[0].iter().any(|p| (p.0 - e.width_mm).abs() < 1e-9));
        // the program is another one, passes its own check and burns the same length
        let (g, _) = program("EH3D 42", (5.0, 5.0), &plain, &machine()).unwrap();
        let (h, _) = program("EH3D 42", (5.0, 5.0), &flipped, &machine()).unwrap();
        assert_ne!(g, h);
        let lim = Limits::default();
        let (v, w) = (
            verify(g.as_bytes(), &lim).unwrap(),
            verify(h.as_bytes(), &lim).unwrap(),
        );
        assert!(w.passes(), "{:?}", w.problems);
        assert!(
            (v.burn_mm - w.burn_mm).abs() < 0.05,
            "{} {}",
            v.burn_mm,
            w.burn_mm
        );
        // the bed holds the same box
        let bed = Machine {
            bed_mm: Some((5.0 + a.width_mm, 5.0 + a.height_mm)),
            ..machine()
        };
        assert!(program("EH3D 42", (5.0, 5.0), &flipped, &bed).is_ok());
    }

    #[test]
    fn the_rows_start_at_the_left_in_the_middle_or_end_at_the_right_inside_the_same_box() {
        let text = "EH3D 42\nLA";
        let at = |align: TextAlign| {
            layout(
                text,
                &TextSettings {
                    align,
                    ..settings()
                },
            )
            .unwrap()
        };
        let (left, centre, right) = (
            at(TextAlign::Left),
            at(TextAlign::Centre),
            at(TextAlign::Right),
        );
        // the box is the same, and so is what is burnt
        for l in [&centre, &right] {
            assert_eq!((l.width_mm, l.height_mm), (left.width_mm, left.height_mm));
            assert_eq!(l.strokes.len(), left.strokes.len());
        }
        // the short row is the lower one: its strokes lie under the pitch of the rows (10 mm of height and 50 % of gap)
        let low = |l: &Layout| -> (f64, f64) {
            let xs: Vec<f64> = l
                .strokes
                .iter()
                .filter(|s| s.iter().all(|p| p.1 < 12.0))
                .flatten()
                .map(|p| p.0)
                .collect();
            (
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        let short = layout("LA", &settings()).unwrap().width_mm;
        let (lo, hi) = low(&left);
        assert!(lo.abs() < 1e-9 && (hi - short).abs() < 1e-9, "{lo} {hi}");
        let (lo, hi) = low(&centre);
        assert!((lo - (left.width_mm - short) / 2.0).abs() < 1e-9, "{lo}");
        assert!((hi - lo - short).abs() < 1e-9);
        let (lo, hi) = low(&right);
        assert!((hi - left.width_mm).abs() < 1e-9 && (hi - lo - short).abs() < 1e-9);
        // the long row does not move, and one row is the same whatever the alignment
        let upper = |l: &Layout| {
            l.strokes
                .iter()
                .filter(|s| s.iter().any(|p| p.1 > 12.0))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(upper(&left), upper(&right));
        assert_eq!(
            layout("EH3D", &settings()).unwrap(),
            layout(
                "EH3D",
                &TextSettings {
                    align: TextAlign::Right,
                    ..settings()
                }
            )
            .unwrap()
        );
        // a mirror image turns the aligned text about the same axis, and the program passes its own check
        let both = TextSettings {
            align: TextAlign::Right,
            mirror: true,
            ..settings()
        };
        let (g, _) = program(text, (2.0, 2.0), &both, &machine()).unwrap();
        assert!(verify(g.as_bytes(), &Limits::default()).unwrap().passes());
    }

    #[test]
    fn a_burning_that_would_take_longer_than_the_limit_is_refused_never_shortened() {
        let t = settings();
        let (g, _) = program("EH3D 42", (0.0, 0.0), &t, &machine()).unwrap();
        let v = verify(g.as_bytes(), &Limits::default()).unwrap();
        let minutes = v.burn_mm / t.speed_mm_min;
        // a limit just over the time is fine, and one under it is refused with the time it would take
        let over = TextSettings {
            max_minutes: 1.0f64.max(minutes + 0.5),
            ..t
        };
        assert!(program("EH3D 42", (0.0, 0.0), &over, &machine()).is_ok());
        let slow = TextSettings {
            speed_mm_min: 1.0,
            passes: 20,
            ..t
        };
        let e = program("EH3D 42", (0.0, 0.0), &slow, &machine()).unwrap_err();
        assert!(e.contains("max_minutes") && e.contains("[text]"), "{e}");
        let many = TextSettings {
            passes: 20,
            speed_mm_min: 100.0,
            max_minutes: 5.0,
            ..t
        };
        assert!(program("EH3D 42", (0.0, 0.0), &many, &machine()).is_err());
        // the passes add up: 20 passes of a text that takes less than a minute fit in the default limit
        let passes = TextSettings { passes: 20, ..t };
        assert!(program("A", (0.0, 0.0), &passes, &machine()).is_ok());
        for bad in [0.0, 0.5, 1441.0, f64::INFINITY] {
            let b = TextSettings {
                max_minutes: bad,
                ..t
            };
            assert!(program("A", (0.0, 0.0), &b, &machine()).is_err(), "{bad}");
        }
    }

    #[test]
    fn every_glyph_stays_on_the_lattice_and_has_a_stroke_with_two_points() {
        for c in ('A'..='Z').chain('0'..='9').chain(".,-+/:!?()=_'".chars()) {
            let g = glyph(c).unwrap_or_else(|| panic!("{c}"));
            assert!(!g.is_empty(), "{c}");
            for s in g {
                assert!(s.len() >= 2, "{c}");
                for &(x, y) in *s {
                    assert!((0.0..=GLYPH_W).contains(&x), "{c}: {x}");
                    assert!((0.0..=GLYPH_H).contains(&y), "{c}: {y}");
                }
            }
        }
        // a space is a gap, and what the font has not is not a glyph
        assert_eq!(glyph(' ').unwrap().len(), 0);
        for c in ['a', '#', '@', '\u{e9}', '\n', '\u{0}'] {
            assert!(glyph(c).is_none(), "{c:?}");
        }
    }

    #[test]
    fn the_size_of_the_text_follows_the_height_and_the_gap() {
        let t = TextSettings {
            height_mm: 6.0,
            spacing_percent: 0.0,
            ..settings()
        };
        // two letters of 4 mm with no gap
        let l = layout("AB", &t).unwrap();
        assert!((l.width_mm - 8.0).abs() < 1e-9 && (l.height_mm - 6.0).abs() < 1e-9);
        let wide = TextSettings {
            spacing_percent: 50.0,
            ..t
        };
        let l = layout("AB", &wide).unwrap();
        assert!((l.width_mm - 11.0).abs() < 1e-9, "{}", l.width_mm);
        // the strokes start at the corner 0, 0
        let min_x = l
            .strokes
            .iter()
            .flatten()
            .map(|p| p.0)
            .fold(f64::MAX, f64::min);
        let min_y = l
            .strokes
            .iter()
            .flatten()
            .map(|p| p.1)
            .fold(f64::MAX, f64::min);
        assert!(min_x.abs() < 1e-9 && min_y.abs() < 1e-9);
        // two rows: the first is on top
        let two = layout("A\nA", &wide).unwrap();
        assert!(
            (two.height_mm - (6.0 + 9.0)).abs() < 1e-9,
            "{}",
            two.height_mm
        );
        let max_y = two
            .strokes
            .iter()
            .flatten()
            .map(|p| p.1)
            .fold(0.0, f64::max);
        assert!((max_y - two.height_mm).abs() < 1e-9);
        // trailing spaces and a carriage return take no width
        assert_eq!(
            layout("AB  \r\n", &t).unwrap().width_mm,
            layout("AB", &t).unwrap().width_mm
        );
    }

    #[test]
    fn lower_case_is_burnt_as_capitals_and_what_the_font_has_not_is_refused() {
        let a = layout("hello 42", &settings()).unwrap();
        let b = layout("HELLO 42", &settings()).unwrap();
        assert_eq!(a, b);
        let e = layout("caf\u{e9}", &settings()).unwrap_err();
        assert!(e.contains("not in the font") && e.contains("A-Z"), "{e}");
        assert!(layout("#", &settings()).is_err());
        assert!(layout("a\u{0}b", &settings()).is_err());
        assert!(layout("   ", &settings()).is_err());
        assert!(layout("", &settings()).is_err());
        assert!(layout("\n\n", &settings()).is_err());
    }

    #[test]
    fn the_number_of_characters_and_the_height_have_limits() {
        let t = TextSettings {
            max_chars: 5,
            ..settings()
        };
        assert!(layout("ABCDE", &t).is_ok());
        let e = layout("ABCDEF", &t).unwrap_err();
        assert!(e.contains("max_chars"), "{e}");
        let tall = TextSettings {
            height_mm: 120.0,
            ..settings()
        };
        assert!(layout("A", &tall).unwrap_err().contains("height_mm"));
        let tall = TextSettings {
            height_mm: 120.0,
            max_height_mm: 200.0,
            ..settings()
        };
        assert!(layout("A", &tall).is_ok());
    }

    #[test]
    fn the_program_burns_every_stroke_at_the_power_and_speed_asked_and_passes_the_check() {
        let t = TextSettings {
            power_percent: 25.0,
            speed_mm_min: 1200.0,
            ..settings()
        };
        let (g, l) = program("Phoenix 2", (5.0, 5.0), &t, &machine()).unwrap();
        assert!(g.starts_with("G21\nG90\nM4 S0\n"));
        assert!(g.ends_with("S0\nM5\nM2\n"));
        assert!(g.contains("S250 F1200"), "{g}");
        assert_eq!(g.matches("\nG0 ").count(), l.strokes.len());
        // nothing moves fast with the laser on, and the check of the program passes
        let lim = Limits {
            bed_mm: Some((100.0, 50.0)),
            ..Limits::default()
        };
        let v = verify(g.as_bytes(), &lim).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
        assert!(v.burn_mm > 0.0);
        // the box holds the start of the head too, so only the far corner says where the text ends
        let (_, (x1, y1)) = v.bounds.unwrap();
        assert!((x1 - (5.0 + l.width_mm)).abs() < 1e-3 && (y1 - (5.0 + l.height_mm)).abs() < 1e-3);
        // two passes burn twice the length
        let two = TextSettings { passes: 2, ..t };
        let (g2, _) = program("Phoenix 2", (5.0, 5.0), &two, &machine()).unwrap();
        let v2 = verify(g2.as_bytes(), &lim).unwrap();
        assert!((v2.burn_mm - 2.0 * v.burn_mm).abs() < 1e-3);
    }

    #[test]
    fn a_power_or_a_speed_over_the_limit_or_work_outside_of_the_bed_is_refused() {
        let t = settings();
        let weak = Machine {
            max_power_percent: 10.0,
            ..machine()
        };
        let e = program("A", (0.0, 0.0), &t, &weak).unwrap_err();
        assert!(e.contains("the power of the text"), "{e}");
        let slow = Machine {
            max_speed_mm_min: 500.0,
            ..machine()
        };
        let e = program("A", (0.0, 0.0), &t, &slow).unwrap_err();
        assert!(e.contains("the speed of the text"), "{e}");
        // exactly the limit is allowed
        let exact = Machine {
            max_power_percent: 20.0,
            max_speed_mm_min: 1000.0,
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &t, &exact).is_ok());
        let small = Machine {
            bed_mm: Some((10.0, 10.0)),
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &t, &small).is_ok());
        let e = program("AB", (0.0, 0.0), &t, &small).unwrap_err();
        assert!(e.contains("outside of the bed"), "{e}");
        assert!(program("A", (4.0, 0.0), &t, &small).is_err());
        assert!(program("A", (-1.0, 0.0), &t, &small).is_err());
        assert!(program("A", (f64::NAN, 0.0), &t, &small).is_err());
        let zero = Machine {
            s_max: 0,
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &t, &zero).is_err());
    }

    #[test]
    fn the_text_is_never_written_into_a_comment_of_the_program() {
        // a parenthesis in the text would close a comment and let the rest through
        let (g, _) = program("A) (B", (0.0, 0.0), &settings(), &machine()).unwrap();
        for line in g.lines().filter(|l| l.starts_with('(')) {
            assert!(
                line.starts_with("(text of ") && line.ends_with(')'),
                "{line}"
            );
            assert_eq!(line.matches(')').count(), 1);
        }
    }
}
