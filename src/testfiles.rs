// EH3D-PHOENIX-LASER - test projects that the program writes: the power-against-speed matrix and the test of the y axis.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Two projects (`.lbrn2`) that the program writes so that a person can find out something with a real material.
//!
//! The **matrix** is a grid of squares, one layer for each pair of power and speed, named after its pair (`P40 F1200`), so that the program that opens the project (this one or another) shows
//! what every square was meant to be. It is the project form of `calibrate`, which writes the program of the same grid.
//!
//! The **y axis test** is a small square, a large rectangle and an arrow that points toward the larger values of y, on one weak layer. Which way the y axis of a project points depends on
//! the program that wrote the project, so the file settles it: open it in the program that made the projects you have, look at where the arrow points, and set `lbrn2_flip_y` of `[read]`
//! to the answer. Nothing here says which one is right: the file only has coordinates.

use crate::calibrate::MAX_CELLS;

/// The most a square of the matrix may be, in millimetres.
pub const MAX_SQUARE_MM: f64 = 100.0;

fn number(v: f64) -> String {
    let t = format!("{v:.4}");
    t.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn head() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LightBurnProject AppVersion=\"1.0.00\" FormatVersion=\"1\">\n".to_string()
}

fn layer(kind: &str, index: usize, name: &str, power: f64, speed_mm_s: f64) -> String {
    format!(
        "    <CutSetting type=\"{kind}\">\n        <index Value=\"{index}\"/>\n        <name Value=\"{name}\"/>\n        <minPower Value=\"{p}\"/>\n        <maxPower Value=\"{p}\"/>\n        <speed Value=\"{s}\"/>\n        <numPasses Value=\"1\"/>\n    </CutSetting>\n",
        p = number(power),
        s = number(speed_mm_s)
    )
}

/// The project of the matrix: a square of `square_mm` for every power (a column, in percent of full power) and every feed (a row, in mm/min), `gap_mm` apart, each on its own layer that
/// is named `P<power> F<feed>` and filled with lines (`engrave`) or only outlined. The feed is written in mm/s, as the format has it. A pair that is not a power from above 0 to 100 and a
/// feed from 1 to 30000 is an error, and so are more than [`MAX_CELLS`] of either.
pub fn matrix_project(
    powers: &[f64],
    feeds_mm_min: &[f64],
    square_mm: f64,
    gap_mm: f64,
    engrave: bool,
) -> Result<String, String> {
    if powers.is_empty()
        || feeds_mm_min.is_empty()
        || powers.len() > MAX_CELLS
        || feeds_mm_min.len() > MAX_CELLS
    {
        return Err(format!(
            "the matrix needs from 1 to {MAX_CELLS} powers and from 1 to {MAX_CELLS} feeds"
        ));
    }
    if !(square_mm.is_finite() && square_mm > 0.0 && square_mm <= MAX_SQUARE_MM) {
        return Err(format!(
            "the square must be from above 0 to {MAX_SQUARE_MM} mm"
        ));
    }
    if !(gap_mm.is_finite() && (0.0..=MAX_SQUARE_MM).contains(&gap_mm)) {
        return Err(format!("the gap must be from 0 to {MAX_SQUARE_MM} mm"));
    }
    for &p in powers {
        if !(p.is_finite() && p > 0.0 && p <= 100.0) {
            return Err(format!(
                "a power of {p} is not above 0 and up to 100 percent"
            ));
        }
    }
    for &f in feeds_mm_min {
        if !(f.is_finite() && (1.0..=30_000.0).contains(&f)) {
            return Err(format!("a feed of {f} is not from 1 to 30000 mm/min"));
        }
    }
    let kind = if engrave { "Scan" } else { "Cut" };
    let mut cuts = String::new();
    let mut shapes = String::new();
    let pitch = square_mm + gap_mm;
    for (i, &p) in powers.iter().enumerate() {
        for (j, &f) in feeds_mm_min.iter().enumerate() {
            let index = i * feeds_mm_min.len() + j;
            cuts.push_str(&layer(
                kind,
                index,
                &format!("P{} F{}", number(p), number(f)),
                p,
                f / 60.0,
            ));
            let (x, y) = (
                i as f64 * pitch + square_mm / 2.0,
                j as f64 * pitch + square_mm / 2.0,
            );
            shapes.push_str(&format!(
                "    <Shape Type=\"Rect\" CutIndex=\"{index}\" W=\"{w}\" H=\"{w}\" Cr=\"0\"><XForm>1 0 0 1 {x} {y}</XForm></Shape>\n",
                w = number(square_mm),
                x = number(x),
                y = number(y)
            ));
        }
    }
    Ok(format!("{}{cuts}{shapes}</LightBurnProject>\n", head()))
}

/// The project of the y axis test: a square of 10 mm at (15, 15), a rectangle of 40 by 20 mm at (70, 60) and an arrow (a triangle of 12 mm at its base and 18 mm high) whose point is at
/// the larger y, with its base at (15, 40), all on one layer of 10 percent and 20 mm/s that only follows the lines.
pub fn y_axis_project() -> String {
    let layer = layer("Cut", 0, "y test", 10.0, 20.0);
    let square = "    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"10\" H=\"10\" Cr=\"0\"><XForm>1 0 0 1 15 15</XForm></Shape>\n";
    let large = "    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"40\" H=\"20\" Cr=\"0\"><XForm>1 0 0 1 70 60</XForm></Shape>\n";
    let arrow = "    <Shape Type=\"Path\" CutIndex=\"0\">\n        <XForm>1 0 0 1 15 40</XForm>\n        <VertList>V-6 0V6 0V0 18</VertList>\n        <PrimList>LineClosed</PrimList>\n    </Shape>\n";
    format!(
        "{}{layer}{square}{large}{arrow}</LightBurnProject>\n",
        head()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_are_written_without_noise() {
        assert_eq!(number(1200.0), "1200");
        assert_eq!(number(0.1 + 0.2), "0.3");
        assert_eq!(number(33.33333333), "33.3333");
    }

    #[test]
    fn a_matrix_outside_the_limits_is_refused() {
        assert!(matrix_project(&[], &[100.0], 8.0, 3.0, true).is_err());
        assert!(matrix_project(&[10.0], &[0.5], 8.0, 3.0, true).is_err());
        assert!(matrix_project(&[0.0], &[100.0], 8.0, 3.0, true).is_err());
        assert!(matrix_project(&[101.0], &[100.0], 8.0, 3.0, true).is_err());
        assert!(matrix_project(&[10.0], &[100.0], 0.0, 3.0, true).is_err());
        assert!(matrix_project(&[10.0], &[100.0], 8.0, -1.0, true).is_err());
        assert!(matrix_project(&[f64::NAN], &[100.0], 8.0, 3.0, true).is_err());
        assert!(matrix_project(&[10.0], &[100.0], 8.0, 3.0, true).is_ok());
    }
}
