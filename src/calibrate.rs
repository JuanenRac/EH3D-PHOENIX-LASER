//! A test grid to find the power and the feed that suit a material: a square for every pair of power and feed, burned line by line. The powers and the feeds can be spread around what a
//! material of the library says ([`ladder`]), and the grid can burn the number of each column (the power) and of each row (the feed) next to it with a font of strokes ([`Labels`]), so
//! that the squares can be read on the material when the program is long gone.

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

/// The numbers that are burnt next to the squares: the height of the figures, and the power and the feed they are burnt with. A figure is a line, so it is burnt at the power and the feed of
/// the material, not at the weak or strong ones of the squares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Labels {
    /// The height of a figure (mm).
    pub height_mm: f64,
    /// A percent of full power.
    pub power_percent: f64,
    pub feed_mm_min: f64,
}

/// The tallest figure that is burnt.
pub const MAX_LABEL_MM: f64 = 20.0;

/// The strokes of a figure on a lattice 2 wide and 4 high. Only the figures and the point are drawn.
fn glyph(c: char) -> Option<&'static [&'static [(f64, f64)]]> {
    Some(match c {
        '0' => &[&[(0.0, 0.0), (2.0, 0.0), (2.0, 4.0), (0.0, 4.0), (0.0, 0.0)]],
        '1' => &[&[(0.0, 3.0), (1.0, 4.0), (1.0, 0.0)]],
        '2' => &[&[
            (0.0, 4.0),
            (2.0, 4.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (0.0, 0.0),
            (2.0, 0.0),
        ]],
        '3' => &[
            &[(0.0, 4.0), (2.0, 4.0), (2.0, 0.0), (0.0, 0.0)],
            &[(0.0, 2.0), (2.0, 2.0)],
        ],
        '4' => &[
            &[(0.0, 4.0), (0.0, 2.0), (2.0, 2.0)],
            &[(2.0, 4.0), (2.0, 0.0)],
        ],
        '5' => &[&[
            (2.0, 4.0),
            (0.0, 4.0),
            (0.0, 2.0),
            (2.0, 2.0),
            (2.0, 0.0),
            (0.0, 0.0),
        ]],
        '6' => &[&[
            (2.0, 4.0),
            (0.0, 4.0),
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
        ]],
        '7' => &[&[(0.0, 4.0), (2.0, 4.0), (2.0, 0.0)]],
        '8' => &[
            &[(0.0, 0.0), (2.0, 0.0), (2.0, 4.0), (0.0, 4.0), (0.0, 0.0)],
            &[(0.0, 2.0), (2.0, 2.0)],
        ],
        '9' => &[&[
            (2.0, 2.0),
            (0.0, 2.0),
            (0.0, 4.0),
            (2.0, 4.0),
            (2.0, 0.0),
            (0.0, 0.0),
        ]],
        '.' => &[&[(1.0, 0.0), (1.0, 0.4)]],
        _ => return None,
    })
}

/// A number as it is burnt: whole when it is whole, with one decimal otherwise.
fn label_text(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

/// How wide a text is when the figures are `height_mm` high (a figure is half as wide as it is high, with a quarter of the height between two).
fn text_width(text: &str, height_mm: f64) -> f64 {
    let unit = height_mm / 4.0;
    (text.chars().count() as f64 * 3.0 - 1.0).max(0.0) * unit
}

/// The strokes of a text whose lower left corner is at `(x, y)`.
fn text_strokes(text: &str, x: f64, y: f64, height_mm: f64) -> Vec<Vec<(f64, f64)>> {
    let unit = height_mm / 4.0;
    let mut out = Vec::new();
    for (k, c) in text.chars().enumerate() {
        let Some(strokes) = glyph(c) else { continue };
        let left = x + k as f64 * 3.0 * unit;
        for s in strokes {
            out.push(
                s.iter()
                    .map(|&(gx, gy)| (left + gx * unit, y + gy * unit))
                    .collect(),
            );
        }
    }
    out
}

/// The most values a series of the command line may give.
pub const MAX_SERIES: usize = 1_000;

/// A list of numbers for a test grid: numbers separated by commas, where a part written `FIRST:LAST:STEP` stands for every value from the first to the last (both included) at that step
/// (`10:50:10` is 10, 20, 30, 40 and 50). The step is above 0, the last is not below the first, and the whole series holds at most [`MAX_SERIES`] values. The values are rounded to six
/// places, so that `0.1:0.5:0.1` gives 0.1, 0.2, 0.3, 0.4 and 0.5 and not 0.30000000000000004.
pub fn series(text: &str, what: &str) -> Result<Vec<f64>, String> {
    let number = |p: &str| -> Result<f64, String> {
        p.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{what} '{}' is not a number", p.trim()))
    };
    let mut out: Vec<f64> = Vec::new();
    for part in text.split(',') {
        if part.contains(':') {
            let bits: Vec<&str> = part.split(':').collect();
            if bits.len() != 3 {
                return Err(format!("{what} '{}' is not FIRST:LAST:STEP", part.trim()));
            }
            let (first, last, step) = (number(bits[0])?, number(bits[1])?, number(bits[2])?);
            if step <= 0.0 || last < first {
                return Err(format!(
                    "{what} '{}': the step must be above 0 and the last not below the first",
                    part.trim()
                ));
            }
            let count = ((last - first) / step + 1e-9).floor();
            if count + 1.0 + out.len() as f64 > MAX_SERIES as f64 {
                return Err(format!("{what} gives more than {MAX_SERIES} values"));
            }
            for k in 0..=(count as usize) {
                out.push(((first + step * k as f64) * 1e6).round() / 1e6);
            }
        } else {
            out.push(number(part)?);
        }
        if out.len() > MAX_SERIES {
            return Err(format!("{what} gives more than {MAX_SERIES} values"));
        }
    }
    Ok(out)
}

/// `steps` values spread evenly around `center`, from `spread_percent` below it to `spread_percent` above it (one step gives the centre alone), rounded to `decimals` places and kept
/// between `lo` and `hi`. A value that the range cuts off is held at the end of the range, and a value that comes out twice is given once, so the ladder never asks for more than the range
/// allows and may be shorter than `steps`.
pub fn ladder(
    center: f64,
    steps: usize,
    spread_percent: f64,
    decimals: u32,
    lo: f64,
    hi: f64,
) -> Result<Vec<f64>, String> {
    if !(1..=MAX_CELLS).contains(&steps) {
        return Err(format!("the steps must be from 1 to {MAX_CELLS}"));
    }
    if !(spread_percent.is_finite() && (0.0..=90.0).contains(&spread_percent)) {
        return Err("the spread must be a percent from 0 to 90".to_string());
    }
    if !(center.is_finite() && lo.is_finite() && hi.is_finite() && lo <= center && center <= hi) {
        return Err(format!(
            "the middle of the ladder, {center}, is outside of {lo} to {hi}"
        ));
    }
    let scale = 10f64.powi(decimals as i32);
    let mut out: Vec<f64> = Vec::new();
    for k in 0..steps {
        let t = if steps == 1 {
            0.0
        } else {
            2.0 * k as f64 / (steps - 1) as f64 - 1.0
        };
        let v = (center * (1.0 + spread_percent / 100.0 * t) * scale).round() / scale;
        let v = v.clamp(lo, hi);
        if out.last() != Some(&v) {
            out.push(v);
        }
    }
    Ok(out)
}

/// The G-code of the grid: one column for each power (a percent of full power) and one row for each feed (mm/min).
pub fn grid(powers_percent: &[f64], feeds_mm_min: &[f64], g: &Grid) -> Result<String, String> {
    build(powers_percent, feeds_mm_min, g, None)
}

/// The grid with the power of each column burnt below it and the feed of each row burnt on its left, at the power and the feed of `labels`. The whole drawing starts at the corner `0, 0`:
/// the squares are moved up and to the right to make room for the figures.
pub fn labelled_grid(
    powers_percent: &[f64],
    feeds_mm_min: &[f64],
    g: &Grid,
    labels: &Labels,
) -> Result<String, String> {
    build(powers_percent, feeds_mm_min, g, Some(labels))
}

fn build(
    powers_percent: &[f64],
    feeds_mm_min: &[f64],
    g: &Grid,
    labels: Option<&Labels>,
) -> Result<String, String> {
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
    // the room for the figures: on the left, as wide as the widest feed, and below, as high as a figure
    let (mut ox, mut oy) = (0.0, 0.0);
    let mut label_strokes: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut label_s = 0u32;
    if let Some(l) = labels {
        if !(l.height_mm.is_finite() && (1.0..=MAX_LABEL_MM).contains(&l.height_mm)) {
            return Err(format!(
                "the height of the figures must be from 1 to {MAX_LABEL_MM} mm"
            ));
        }
        if !(l.power_percent.is_finite() && (0.0..=100.0).contains(&l.power_percent))
            || !(l.feed_mm_min.is_finite() && (1.0..=30_000.0).contains(&l.feed_mm_min))
        {
            return Err("the power or the feed of the figures is out of range".to_string());
        }
        label_s = (f64::from(g.s_max) * l.power_percent / 100.0).round() as u32;
        let widest = feeds_mm_min
            .iter()
            .map(|f| text_width(&label_text(*f), l.height_mm))
            .fold(0.0f64, f64::max);
        (ox, oy) = (widest + 2.0, l.height_mm + 1.5);
        let pitch = g.square_mm + g.gap_mm;
        for (col, power) in powers_percent.iter().enumerate() {
            let text = label_text(*power);
            let x = ox + col as f64 * pitch + (g.square_mm - text_width(&text, l.height_mm)) / 2.0;
            label_strokes.extend(text_strokes(&text, x, 0.5, l.height_mm));
        }
        for (row, feed) in feeds_mm_min.iter().enumerate() {
            let text = label_text(*feed);
            let x = ox - 1.5 - text_width(&text, l.height_mm);
            let y = oy + row as f64 * pitch + (g.square_mm - l.height_mm) / 2.0;
            label_strokes.extend(text_strokes(&text, x, y, l.height_mm));
        }
    }
    let mut out = String::from("G21\nG90\nM4 S0\n");
    let lines = (g.square_mm / g.line_spacing_mm).floor() as usize + 1;
    let mut last_s = 0u32;
    for (row, feed) in feeds_mm_min.iter().enumerate() {
        for (col, power) in powers_percent.iter().enumerate() {
            let x0 = ox + col as f64 * (g.square_mm + g.gap_mm);
            let y0 = oy + row as f64 * (g.square_mm + g.gap_mm);
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
    if let Some(l) = labels {
        let feed = n(l.feed_mm_min)?;
        out.push_str(&format!(
            "(figures at {}% feed {feed})\n",
            n(l.power_percent)?
        ));
        for stroke in &label_strokes {
            // no rapid move with the power of the last stroke still set
            if last_s != 0 {
                out.push_str("S0\n");
                last_s = 0;
            }
            let Some(((fx, fy), rest)) = stroke.split_first() else {
                continue;
            };
            out.push_str(&format!("G0 X{} Y{}\n", n(*fx)?, n(*fy)?));
            for (x, y) in rest {
                out.push_str(&format!("G1 X{} Y{} S{label_s} F{feed}\n", n(*x)?, n(*y)?));
                last_s = label_s;
            }
        }
    }
    out.push_str("M5\nM2\n");
    Ok(out)
}

/// A value as it is burnt under a square of the test of the spacing: up to three decimals, without zeros at the end.
fn spacing_text(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The test of the spacing of the lines: a row of squares burnt at one power and one feed, each filled with lines at its own spacing (`spacings_mm`, in the order given), so that the widest
/// spacing that still leaves no gap can be read off. With `label_mm` the spacing of each square is burnt below it.
pub fn interval_test(
    spacings_mm: &[f64],
    power_percent: f64,
    feed_mm_min: f64,
    g: &Grid,
    label_mm: Option<f64>,
) -> Result<String, String> {
    if spacings_mm.is_empty() || spacings_mm.len() > MAX_CELLS {
        return Err(format!(
            "the test of the spacing needs from 1 to {MAX_CELLS} spacings"
        ));
    }
    if !(g.square_mm.is_finite()
        && (1.0..=50.0).contains(&g.square_mm)
        && g.gap_mm.is_finite()
        && (0.0..=50.0).contains(&g.gap_mm))
    {
        return Err("the size of the squares or the gap is out of range".into());
    }
    if spacings_mm
        .iter()
        .any(|v| !v.is_finite() || !(0.02..=g.square_mm).contains(v))
    {
        return Err("a spacing must be from 0.02 mm to the side of a square".into());
    }
    if !(power_percent.is_finite() && power_percent > 0.0 && power_percent <= 100.0) {
        return Err("a power must be a percent above 0 and up to 100".into());
    }
    if !(feed_mm_min.is_finite() && (1.0..=30_000.0).contains(&feed_mm_min)) {
        return Err("a feed must be from 1 to 30000 mm/min".into());
    }
    if g.s_max == 0 || g.s_max > 100_000 {
        return Err(format!("s_max is {}: it must be from 1 to 100000", g.s_max));
    }
    let s = (f64::from(g.s_max) * power_percent / 100.0).round() as u32;
    if s == 0 {
        return Err(format!(
            "a power of {power_percent} % of S{} is less than one step of S",
            g.s_max
        ));
    }
    let pitch = g.square_mm + g.gap_mm;
    let oy = match label_mm {
        Some(h) => {
            if !(h.is_finite() && (1.0..=MAX_LABEL_MM).contains(&h)) {
                return Err(format!(
                    "the height of the figures must be from 1 to {MAX_LABEL_MM} mm"
                ));
            }
            if spacings_mm
                .iter()
                .any(|v| text_width(&spacing_text(*v), h) > pitch)
            {
                return Err(
                    "the figures are wider than a square and its gap: make them lower (--label-mm) or leave them out (--no-labels)"
                        .into(),
                );
            }
            h + 1.5
        }
        None => 0.0,
    };
    let feed = n(feed_mm_min)?;
    let mut out = String::from("G21\nG90\nM4 S0\n");
    for (col, spacing) in spacings_mm.iter().enumerate() {
        let x0 = col as f64 * pitch;
        out.push_str(&format!(
            "S0\n(spacing {} mm)\nG0 X{} Y{}\n",
            n(*spacing)?,
            n(x0)?,
            n(oy)?
        ));
        // the small amount keeps the last line when the side is a whole number of spacings
        let lines = (g.square_mm / spacing + 1e-6).floor() as usize + 1;
        for k in 0..lines {
            let y = oy + (k as f64 * spacing).min(g.square_mm);
            let (a, b) = if k % 2 == 0 {
                (x0, x0 + g.square_mm)
            } else {
                (x0 + g.square_mm, x0)
            };
            out.push_str(&format!("G1 X{} Y{} S0 F{feed}\n", n(a)?, n(y)?));
            out.push_str(&format!("G1 X{} S{s} F{feed}\n", n(b)?));
        }
    }
    if let Some(h) = label_mm {
        for (col, spacing) in spacings_mm.iter().enumerate() {
            let text = spacing_text(*spacing);
            let x = col as f64 * pitch + ((g.square_mm - text_width(&text, h)) / 2.0).max(0.0);
            for stroke in text_strokes(&text, x, 0.0, h) {
                let Some(((fx, fy), rest)) = stroke.split_first() else {
                    continue;
                };
                out.push_str(&format!("S0\nG0 X{} Y{}\n", n(*fx)?, n(*fy)?));
                for (px, py) in rest {
                    out.push_str(&format!("G1 X{} Y{} S{s} F{feed}\n", n(*px)?, n(*py)?));
                }
            }
        }
    }
    out.push_str("M5\nM2\n");
    Ok(out)
}

/// The most strips of the test of the width of the beam.
pub const MAX_STRIPS: u32 = 40;

/// The test of the width of the beam (kerf): a rectangle cut into strips of the same width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kerf {
    pub strips: u32,
    pub strip_mm: f64,
    pub height_mm: f64,
    pub power_percent: f64,
    pub feed_mm_min: f64,
    pub passes: u32,
}

impl Default for Kerf {
    fn default() -> Self {
        Kerf {
            strips: 10,
            strip_mm: 5.0,
            height_mm: 20.0,
            power_percent: 0.0,
            feed_mm_min: 0.0,
            passes: 1,
        }
    }
}

/// The program of the test: first the cuts between the strips, then the rectangle round them, with the beam on the line (nothing is taken into account of its width: that is what is
/// measured). Pushing every strip to one side leaves a gap that is the width of the beam once for each of the `strips + 1` cuts.
pub fn kerf_test(k: &Kerf, s_max: u32) -> Result<String, String> {
    if !(2..=MAX_STRIPS).contains(&k.strips) {
        return Err(format!("the strips must be from 2 to {MAX_STRIPS}"));
    }
    if !(k.strip_mm.is_finite() && (2.0..=50.0).contains(&k.strip_mm)) {
        return Err("a strip must be from 2 to 50 mm wide".into());
    }
    if !(k.height_mm.is_finite() && (5.0..=200.0).contains(&k.height_mm)) {
        return Err("the height must be from 5 to 200 mm".into());
    }
    if !(k.power_percent.is_finite() && k.power_percent > 0.0 && k.power_percent <= 100.0) {
        return Err("a power must be a percent above 0 and up to 100".into());
    }
    if !(k.feed_mm_min.is_finite() && (1.0..=30_000.0).contains(&k.feed_mm_min)) {
        return Err("a feed must be from 1 to 30000 mm/min".into());
    }
    if !(1..=20).contains(&k.passes) {
        return Err("the passes must be from 1 to 20".into());
    }
    if s_max == 0 || s_max > 100_000 {
        return Err(format!("s_max is {s_max}: it must be from 1 to 100000"));
    }
    let s = (f64::from(s_max) * k.power_percent / 100.0).round() as u32;
    if s == 0 {
        return Err(format!(
            "a power of {} % of S{s_max} is less than one step of S",
            k.power_percent
        ));
    }
    let width = f64::from(k.strips) * k.strip_mm;
    let (feed, h, w) = (n(k.feed_mm_min)?, n(k.height_mm)?, n(width)?);
    let mut out = String::from("G21\nG90\nM4 S0\n");
    for pass in 0..k.passes {
        out.push_str(&format!("(pass {} of {})\n", pass + 1, k.passes));
        for i in 1..k.strips {
            let x = n(f64::from(i) * k.strip_mm)?;
            let (a, b) = if i % 2 == 1 {
                ("0", h.as_str())
            } else {
                (h.as_str(), "0")
            };
            out.push_str(&format!("S0\nG0 X{x} Y{a}\nG1 Y{b} S{s} F{feed}\n"));
        }
    }
    // the rectangle last, so that nothing moves before the cuts inside it are done
    out.push_str("S0\nG0 X0 Y0\n");
    for _ in 0..k.passes {
        out.push_str(&format!(
            "G1 X{w} Y0 S{s} F{feed}\nG1 X{w} Y{h}\nG1 X0 Y{h}\nG1 X0 Y0\n"
        ));
    }
    out.push_str("S0\nM5\nM2\n");
    Ok(out)
}

/// The width of the beam (mm) from the gap left after every strip of [`kerf_test`] is pushed to one side. A width over 2 mm is not believed: the gap was measured wrong or the strips were not
/// all pushed.
pub fn kerf_from_gap(gap_mm: f64, strips: u32) -> Result<f64, String> {
    if !(2..=MAX_STRIPS).contains(&strips) {
        return Err(format!("the strips must be from 2 to {MAX_STRIPS}"));
    }
    let kerf = gap_mm / f64::from(strips + 1);
    if !(kerf.is_finite() && kerf > 0.0 && kerf <= 2.0) {
        return Err(format!(
            "a gap of {gap_mm} mm over {} cuts is not a width of a beam: measure it again",
            strips + 1
        ));
    }
    Ok(kerf)
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

/// The most powers of a focus grid: one column for each.
pub const MAX_FOCUS_POWERS: usize = 10;

/// The G-code of a focus grid: the lines of the focus test, once for each power in `powers_percent`, side by side (a column for each power, `spacing_mm` apart, in the order given) and one row for
/// each height, so that the power that burns the thinnest line at the height of the focus can be read off. The head goes to a height once and burns all the columns of that row, and goes back to the
/// first height at the end. A power that is not more than 0 up to 100 or over the limit of power of the machine, one that is less than one step of S, no powers or more than
/// [`MAX_FOCUS_POWERS`], and a missing step of the height are refused and nothing is written.
pub fn focus_grid(
    f: &FocusSettings,
    powers_percent: &[f64],
    s_max: u32,
    max_power_percent: f64,
) -> Result<String, String> {
    f.validate()?;
    if f.z_step_mm == 0.0 {
        return Err(
            "the focus step is needed: --z-step MM or z_step_mm in [focus] (which way is down depends on the machine)"
                .to_string(),
        );
    }
    if powers_percent.is_empty() || powers_percent.len() > MAX_FOCUS_POWERS {
        return Err(format!(
            "the focus grid has {} powers: it must have from 1 to {MAX_FOCUS_POWERS}",
            powers_percent.len()
        ));
    }
    if s_max == 0 || s_max > 100_000 {
        return Err(format!("s_max is {s_max}: it must be from 1 to 100000"));
    }
    let mut values = Vec::new();
    for &p in powers_percent {
        if !(p.is_finite() && p > 0.0 && p <= 100.0) {
            return Err(format!(
                "a power of the focus grid is {p} %: it must be more than 0 up to 100"
            ));
        }
        crate::safety::check_power("a power of the focus grid", p, max_power_percent)?;
        let s = (f64::from(s_max) * p / 100.0).round() as u32;
        if s == 0 {
            return Err(format!(
                "a power of {p} % of S{s_max} is less than one step of S"
            ));
        }
        values.push(s);
    }
    let mut out = format!("G21\nG90\nM4 S0\nF{}\n", n(f.feed_mm_min)?);
    for k in 0..f.steps {
        let z = n(f.z_start_mm + f64::from(k) * f.z_step_mm)?;
        let y = n(f64::from(k) * f.spacing_mm)?;
        out.push_str(&format!(
            "(row {} of {} at Z{z})\nG0 Z{z}\n",
            k + 1,
            f.steps
        ));
        for (j, s) in values.iter().enumerate() {
            let x0 = j as f64 * (f.length_mm + f.spacing_mm);
            out.push_str(&format!(
                "G0 X{} Y{y}\nG1 X{} S{s}\nS0\n",
                n(x0)?,
                n(x0 + f.length_mm)?
            ));
        }
    }
    out.push_str(&format!("G0 Z{}\nM5\nM2\n", n(f.z_start_mm)?));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{analyse, read, Options};

    #[test]
    fn the_focus_grid_burns_a_column_for_each_power_and_a_row_for_each_height() {
        let f = settings();
        let g = focus_grid(&f, &[10.0, 20.0, 30.0], 1000, 100.0).unwrap();
        let p = read(&g);
        assert!(p.ended && p.notes.is_empty(), "{:?}", p.notes);
        let burns: Vec<_> = p.moves.iter().filter(|m| m.power > 0.0).collect();
        assert_eq!(burns.len(), 5 * 3);
        // the powers go along each row in the order they were given, and every line is a length long
        for row in burns.chunks(3) {
            assert_eq!(
                row.iter().map(|m| m.power).collect::<Vec<_>>(),
                vec![100.0, 200.0, 300.0]
            );
            assert!(row.iter().all(|m| (m.length() - 10.0).abs() < 1e-9));
            // the columns are a length and a gap apart
            assert_eq!(
                row.iter().map(|m| m.from.0).collect::<Vec<_>>(),
                vec![0.0, 12.0, 24.0]
            );
            assert!(row.iter().all(|m| m.from.1 == row[0].from.1));
        }
        // the rows are 2 mm apart, one for each height, and the head goes down once for each row
        let rows: Vec<f64> = burns.chunks(3).map(|r| r[0].from.1).collect();
        assert_eq!(rows, vec![0.0, 2.0, 4.0, 6.0, 8.0]);
        for z in ["Z0", "Z-0.5", "Z-1", "Z-1.5", "Z-2"] {
            assert_eq!(
                g.matches(&format!("G0 {z}\n")).count(),
                if z == "Z0" { 2 } else { 1 },
                "{z}"
            );
        }
        assert!(g.ends_with("G0 Z0\nM5\nM2\n"), "{g}");
        let r = analyse(&p, &Options::default()).unwrap();
        assert_eq!(r.burn_bounds, Some(((0.0, 0.0), (34.0, 8.0))));
        let v = crate::verify::verify(g.as_bytes(), &crate::verify::Limits::default()).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
    }

    #[test]
    fn a_grid_of_one_power_burns_the_lines_of_the_focus_test() {
        let f = settings();
        let one = focus_grid(&f, &[30.0], 1000, 100.0).unwrap();
        let test = focus_test(&f, 1000, 100.0).unwrap();
        let (a, b) = (read(&one), read(&test));
        let key = |p: &crate::preview::Program| {
            p.moves
                .iter()
                .filter(|m| m.power > 0.0)
                .map(|m| (m.from, m.to, m.power))
                .collect::<Vec<_>>()
        };
        assert_eq!(key(&a), key(&b));
    }

    #[test]
    fn a_focus_grid_with_a_power_or_a_count_that_is_not_allowed_is_refused() {
        let f = settings();
        assert!(focus_grid(&f, &[], 1000, 100.0)
            .unwrap_err()
            .contains("0 powers"));
        let many = [10.0; MAX_FOCUS_POWERS + 1];
        assert!(focus_grid(&f, &many, 1000, 100.0)
            .unwrap_err()
            .contains("11 powers"));
        assert!(focus_grid(&f, &[10.0; MAX_FOCUS_POWERS], 1000, 100.0).is_ok());
        for bad in [0.0, -5.0, 100.5, f64::NAN, f64::INFINITY] {
            assert!(focus_grid(&f, &[10.0, bad], 1000, 100.0).is_err(), "{bad}");
        }
        // over the limit of power of the machine, and under one step of S
        let e = focus_grid(&f, &[10.0, 40.0], 1000, 30.0).unwrap_err();
        assert!(e.contains("max_power_percent"), "{e}");
        assert!(focus_grid(&f, &[10.0, 30.0], 1000, 30.0).is_ok());
        assert!(focus_grid(&f, &[0.01], 100, 100.0)
            .unwrap_err()
            .contains("one step"));
        // the step of the height is needed, as for the focus test
        let e = focus_grid(&FocusSettings::default(), &[10.0], 1000, 100.0).unwrap_err();
        assert!(e.contains("--z-step"), "{e}");
        assert!(focus_grid(&f, &[10.0], 0, 100.0).is_err());
        assert!(focus_grid(&f, &[10.0], 100_001, 100.0).is_err());
    }

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

    #[test]
    fn a_ladder_spreads_around_the_middle_and_stays_in_its_range() {
        // 5 steps from 40 % below to 40 % above 50
        assert_eq!(
            ladder(50.0, 5, 40.0, 1, 1.0, 100.0).unwrap(),
            vec![30.0, 40.0, 50.0, 60.0, 70.0]
        );
        // a single step is the middle alone, and a spread of 0 repeats nothing
        assert_eq!(ladder(50.0, 1, 40.0, 1, 1.0, 100.0).unwrap(), vec![50.0]);
        assert_eq!(ladder(50.0, 5, 0.0, 1, 1.0, 100.0).unwrap(), vec![50.0]);
        // what goes over the end of the range is held at it and given once
        assert_eq!(
            ladder(90.0, 5, 40.0, 1, 1.0, 100.0).unwrap(),
            vec![54.0, 72.0, 90.0, 100.0]
        );
        assert_eq!(
            ladder(10.0, 3, 90.0, 1, 5.0, 100.0).unwrap(),
            vec![5.0, 10.0, 19.0]
        );
        // the places after the point
        assert_eq!(
            ladder(33.0, 3, 10.0, 1, 1.0, 100.0).unwrap(),
            vec![29.7, 33.0, 36.3]
        );
        assert_eq!(
            ladder(333.0, 3, 10.0, 0, 10.0, 30_000.0).unwrap(),
            vec![300.0, 333.0, 366.0]
        );
        // it never leaves the range, whatever it is given
        for center in [1.0, 25.0, 100.0] {
            for spread in [0.0, 33.0, 90.0] {
                for v in ladder(center, 12, spread, 1, 1.0, 100.0).unwrap() {
                    assert!((1.0..=100.0).contains(&v), "{center} {spread} {v}");
                }
            }
        }
    }

    #[test]
    fn a_ladder_that_cannot_be_made_is_an_error() {
        for (steps, spread, center) in [
            (0, 40.0, 50.0),
            (MAX_CELLS + 1, 40.0, 50.0),
            (5, -1.0, 50.0),
            (5, 90.5, 50.0),
            (5, f64::NAN, 50.0),
            (5, 40.0, 0.5),
            (5, 40.0, 101.0),
            (5, 40.0, f64::NAN),
        ] {
            assert!(
                ladder(center, steps, spread, 1, 1.0, 100.0).is_err(),
                "{steps} {spread} {center}"
            );
        }
    }

    fn labels() -> Labels {
        Labels {
            height_mm: 3.0,
            power_percent: 50.0,
            feed_mm_min: 1000.0,
        }
    }

    #[test]
    fn the_figures_of_a_label_are_strokes_inside_their_cell_and_a_missing_glyph_draws_nothing() {
        for c in "0123456789.".chars() {
            let strokes = glyph(c).unwrap();
            assert!(!strokes.is_empty());
            for s in strokes {
                assert!(s.len() >= 2, "{c}");
                for &(x, y) in *s {
                    assert!((0.0..=2.0).contains(&x) && (0.0..=4.0).contains(&y), "{c}");
                }
            }
        }
        assert!(glyph('x').is_none() && glyph(' ').is_none());
        // 4 pt high figures: a unit is 1 mm, a figure is 2 wide and the next one starts 3 on
        let s = text_strokes("10", 5.0, 7.0, 4.0);
        assert_eq!(s.len(), 2);
        assert!(s[0].iter().all(|p| (5.0..=7.0).contains(&p.0)));
        assert!(s[1].iter().all(|p| (8.0..=10.0).contains(&p.0)));
        assert!(s.iter().flatten().all(|p| (7.0..=11.0).contains(&p.1)));
        assert_eq!(text_width("10", 4.0), 5.0);
        assert_eq!(text_width("", 4.0), 0.0);
        assert!(text_strokes("x y", 0.0, 0.0, 4.0).is_empty());
        assert_eq!(label_text(40.0), "40");
        assert_eq!(label_text(37.5), "37.5");
        assert_eq!(label_text(33.333), "33.3");
    }

    #[test]
    fn the_grid_with_figures_is_the_grid_moved_to_make_room_and_passes_the_check_of_the_program() {
        let (powers, feeds) = ([20.0, 40.0], [1000.0, 2000.0]);
        let g = Grid::default();
        let plain = grid(&powers, &feeds, &g).unwrap();
        let text = labelled_grid(&powers, &feeds, &g, &labels()).unwrap();
        let check = |program: &str| {
            let v = crate::verify::verify(program.as_bytes(), &crate::verify::Limits::default());
            assert!(v.as_ref().is_ok_and(|v| v.passes()), "{v:?}");
        };
        check(&text);
        check(&plain);
        let o = Options::default();
        let a = analyse(&read(&plain), &o).unwrap();
        let b = analyse(&read(&text), &o).unwrap();
        // room for the widest feed ("2000" is 8.25 mm wide, 2 mm of air) on the left and for the figures below
        let (ox, oy) = (8.25 + 2.0, 3.0 + 1.5);
        let ((alx, aly), (ahx, ahy)) = a.bounds.unwrap();
        let ((blx, bly), (bhx, bhy)) = b.bounds.unwrap();
        assert!((ahx + ox - bhx).abs() < 1e-6 && (ahy + oy - bhy).abs() < 1e-6);
        assert_eq!((alx, aly), (0.0, 0.0));
        // the figures start at 0.5 mm from the edge and nothing goes below zero
        assert!(blx >= 0.0 && bly >= 0.0);
        let (lo, _) = b.burn_bounds.unwrap();
        assert!(
            (lo.0 - 0.5).abs() < 1e-6 && (lo.1 - 0.5).abs() < 1e-6,
            "{lo:?}"
        );
        // one move to start each stroke, after the 4 squares: 5 strokes for "20" and "40" (the 4 has two) and 8 for "1000" and "2000"
        let rapid = |t: &str| t.lines().filter(|l| l.starts_with("G0 ")).count();
        assert_eq!(rapid(&plain), 4);
        assert_eq!(rapid(&text), 4 + 5 + 8);
        assert!(text.contains("(figures at 50% feed 1000)"));
        // the figures are burnt at S500 (50 % of 1000), the squares at their own power
        assert!(text.contains(" S500 F1000"));
        // no rapid move with the power of the stroke before still set
        let mut s = 0u32;
        for line in text.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            for w in &words {
                if let Some(v) = w.strip_prefix('S').and_then(|v| v.parse::<u32>().ok()) {
                    s = v;
                }
            }
            if words.first() == Some(&"G0") {
                assert_eq!(s, 0, "{line}");
            }
        }
        // the squares of the grid with figures are the ones of the plain grid
        let squares = |t: &str| t.lines().filter(|l| l.starts_with("(power ")).count();
        assert_eq!(squares(&text), squares(&plain));
    }

    #[test]
    fn the_figures_obey_their_own_ranges() {
        let g = Grid::default();
        for bad in [
            Labels {
                height_mm: 0.5,
                ..labels()
            },
            Labels {
                height_mm: MAX_LABEL_MM + 1.0,
                ..labels()
            },
            Labels {
                height_mm: f64::NAN,
                ..labels()
            },
            Labels {
                power_percent: 101.0,
                ..labels()
            },
            Labels {
                feed_mm_min: 0.0,
                ..labels()
            },
        ] {
            assert!(
                labelled_grid(&[20.0], &[1000.0], &g, &bad).is_err(),
                "{bad:?}"
            );
        }
        // a grid with no figures is the grid it always was
        assert_eq!(
            grid(&[20.0], &[1000.0], &g).unwrap(),
            build(&[20.0], &[1000.0], &g, None).unwrap()
        );
    }
}
