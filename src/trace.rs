// EH3D-PHOENIX-LASER - the contours of a picture.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A picture of a shape (a logo, a silhouette, a stencil) can be cut or engraved by its outline instead of being burnt dot by dot. The grey levels are split at a threshold into ink and paper,
//! the border between them is found with marching squares (the crossing of every edge is placed by the grey levels on both sides of it, so the line is smooth and does not follow the steps of
//! the pixels), the pieces of border are joined into closed rings and each ring is simplified with the Douglas-Peucker rule until no point is farther than `simplify_mm` from the line. What
//! comes out is a drawing like the ones read from an SVG, so it is cut and engraved by the same layers, the same settings and the same checks. The picture is never trusted: its size, the
//! number of pieces of border, of contours and of points are all limited by settings, and a picture that asks for more is an error and not a slow program.

use crate::image::Gray;
use crate::vconfig::TraceSettings;
use crate::vector::{signed_area, Collector, Drawing, Limits, Pt, Source};
use std::collections::HashMap;

/// The layer that the contours of a picture get.
pub const LAYER: &str = "trace";

/// The widest work a picture is traced for (mm).
pub const MAX_WIDTH_MM: f64 = 5000.0;

/// The corners of a cell of the lattice and its four edges: the top (between the corners `tl` and `tr`), the right, the bottom and the left.
#[derive(Clone, Copy)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// The grey levels as a field that is positive on the ink and zero or negative on the paper, with the border of the picture counted as paper.
struct Field<'a> {
    w: isize,
    h: isize,
    f: &'a [i16],
}

impl Field<'_> {
    fn at(&self, x: isize, y: isize) -> Option<i16> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| self.f[(y * self.w + x) as usize])
    }

    fn ink(&self, x: isize, y: isize) -> bool {
        self.at(x, y).is_some_and(|v| v > 0)
    }

    /// The point where the edge `key` is crossed by the border, in pixel units where the picture covers `0..w` and `0..h`.
    fn crossing(&self, key: u64) -> Pt {
        let (horizontal, x, y) = decode(key);
        let (bx, by) = if horizontal { (x + 1, y) } else { (x, y + 1) };
        let t = match (self.at(x, y), self.at(bx, by)) {
            (Some(a), Some(b)) => {
                let (a, b) = (f64::from(a), f64::from(b));
                (a / (a - b)).clamp(0.0, 1.0)
            }
            // the border of the picture: the line runs along the edge of the picture
            _ => 0.5,
        };
        (
            x as f64 + 0.5 + t * (bx - x) as f64,
            y as f64 + 0.5 + t * (by - y) as f64,
        )
    }
}

/// The key of the edge from the sample `(x, y)` to the sample on its right (`horizontal`) or below it.
fn key(horizontal: bool, x: isize, y: isize) -> u64 {
    (((y + 1) as u64) << 33) | (((x + 1) as u64) << 1) | u64::from(horizontal)
}

fn decode(key: u64) -> (bool, isize, isize) {
    (
        key & 1 == 1,
        ((key >> 1) & 0xffff_ffff) as isize - 1,
        (key >> 33) as isize - 1,
    )
}

/// The edge of the cell `(i, j)` (its corners are the samples `(i, j)`, `(i + 1, j)`, `(i + 1, j + 1)` and `(i, j + 1)`).
fn edge(side: Side, i: isize, j: isize) -> u64 {
    match side {
        Side::Top => key(true, i, j),
        Side::Bottom => key(true, i, j + 1),
        Side::Left => key(false, i, j),
        Side::Right => key(false, i + 1, j),
    }
}

/// The pieces of border inside one cell, from the corners that are ink. The cases with two opposite corners are saddles: the border goes round the ink when the centre is paper and
/// round the paper when the centre is ink, so two cells that touch never leave a gap.
fn pieces(case: u8, centre_ink: bool) -> &'static [(Side, Side)] {
    use Side::{Bottom as B, Left as L, Right as R, Top as T};
    match case {
        1 => &[(L, B)],
        2 => &[(B, R)],
        3 => &[(L, R)],
        4 => &[(T, R)],
        5 if centre_ink => &[(T, L), (B, R)],
        5 => &[(T, R), (L, B)],
        6 => &[(T, B)],
        7 => &[(T, L)],
        8 => &[(T, L)],
        9 => &[(T, B)],
        10 if centre_ink => &[(T, R), (L, B)],
        10 => &[(T, L), (B, R)],
        11 => &[(T, R)],
        12 => &[(L, R)],
        13 => &[(B, R)],
        14 => &[(L, B)],
        _ => &[],
    }
}

/// The closed rings of border of the field, in pixel units. A picture with more pieces of border than `max_segments` is an error.
fn rings(field: &Field, max_segments: usize) -> Result<Vec<Vec<Pt>>, String> {
    let mut segments: Vec<(u64, u64)> = Vec::new();
    for j in -1..field.h {
        for i in -1..field.w {
            let corners = [
                field.ink(i, j),
                field.ink(i + 1, j),
                field.ink(i + 1, j + 1),
                field.ink(i, j + 1),
            ];
            let case = corners
                .iter()
                .enumerate()
                .fold(0u8, |c, (k, &on)| c | (u8::from(on) << (3 - k)));
            if case == 0 || case == 15 {
                continue;
            }
            let centre_ink = if case == 5 || case == 10 {
                let sum: i32 = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)]
                    .iter()
                    .map(|&(x, y)| field.at(x, y).map_or(-1, i32::from))
                    .sum();
                sum > 0
            } else {
                false
            };
            for &(a, b) in pieces(case, centre_ink) {
                if segments.len() >= max_segments {
                    return Err(format!(
                        "the picture has more than {max_segments} pieces of border (max_segments): it is too detailed to follow its outline, so raise the threshold, clean the picture or lower max_pixels"
                    ));
                }
                segments.push((edge(a, i, j), edge(b, i, j)));
            }
        }
    }
    // every edge that is crossed belongs to exactly two pieces of border
    let mut at_edge: HashMap<u64, [u32; 2]> = HashMap::with_capacity(segments.len());
    for (k, &(a, b)) in segments.iter().enumerate() {
        for e in [a, b] {
            let slot = at_edge.entry(e).or_insert([u32::MAX; 2]);
            if slot[0] == u32::MAX {
                slot[0] = k as u32;
            } else {
                slot[1] = k as u32;
            }
        }
    }
    let mut used = vec![false; segments.len()];
    let mut out: Vec<Vec<Pt>> = Vec::new();
    for first in 0..segments.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let (start, mut at) = segments[first];
        let mut ring = vec![field.crossing(start)];
        let mut piece = first;
        while at != start {
            ring.push(field.crossing(at));
            let pair = at_edge[&at];
            let next = if pair[0] as usize == piece {
                pair[1]
            } else {
                pair[0]
            };
            if next == u32::MAX || used[next as usize] {
                break;
            }
            piece = next as usize;
            used[piece] = true;
            at = if segments[piece].0 == at {
                segments[piece].1
            } else {
                segments[piece].0
            };
        }
        out.push(ring);
    }
    Ok(out)
}

/// The distance from `p` to the line through `a` and `b` (the distance to `a` when they are the same point).
fn line_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    if len == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / len
}

/// Douglas-Peucker on an open chain: the points that can be dropped without moving the line more than `eps` are dropped. It uses a stack and not recursion, because a chain of a million
/// points would overflow the call stack.
fn simplify_chain(chain: &[Pt], eps: f64) -> Vec<Pt> {
    if chain.len() < 3 {
        return chain.to_vec();
    }
    let mut keep = vec![false; chain.len()];
    keep[0] = true;
    keep[chain.len() - 1] = true;
    let mut stack = vec![(0usize, chain.len() - 1)];
    while let Some((lo, hi)) = stack.pop() {
        let mut far = (0.0f64, lo);
        for (k, &p) in chain.iter().enumerate().take(hi).skip(lo + 1) {
            let d = line_distance(p, chain[lo], chain[hi]);
            if d > far.0 {
                far = (d, k);
            }
        }
        if far.0 > eps {
            keep[far.1] = true;
            stack.push((lo, far.1));
            stack.push((far.1, hi));
        }
    }
    chain
        .iter()
        .zip(&keep)
        .filter(|(_, &k)| k)
        .map(|(&p, _)| p)
        .collect()
}

/// A closed ring simplified: it is split at its point farthest from the first one, so that both halves are chains whose ends stay where they are.
pub fn simplify_ring(ring: &[Pt], eps: f64) -> Vec<Pt> {
    if ring.len() < 4 || eps <= 0.0 {
        return ring.to_vec();
    }
    let far = ring
        .iter()
        .enumerate()
        .max_by(|a, b| {
            let d = |p: &Pt| (p.0 - ring[0].0).hypot(p.1 - ring[0].1);
            d(a.1).total_cmp(&d(b.1))
        })
        .map_or(0, |(k, _)| k);
    if far == 0 {
        return ring.to_vec();
    }
    let mut second: Vec<Pt> = ring[far..].to_vec();
    second.push(ring[0]);
    let mut out = simplify_chain(&ring[..=far], eps);
    out.pop();
    let mut tail = simplify_chain(&second, eps);
    tail.pop();
    out.extend(tail);
    out
}

/// The picture as a drawing in millimetres, `width_mm` wide, with the y axis down like a picture (the same as an SVG, so the top of the picture is the top of the work). The contours are all
/// on the layer [`LAYER`]. Rings whose area is under `min_area_mm2` (specks) are dropped. A picture with no contour is an error.
pub fn trace(
    img: &Gray,
    width_mm: f64,
    t: &TraceSettings,
    limits: Limits,
) -> Result<Drawing, String> {
    t.validate()?;
    if !(width_mm.is_finite() && width_mm > 0.0 && width_mm <= MAX_WIDTH_MM) {
        return Err(format!(
            "the width of the work is {width_mm}: it must be more than 0 and at most {MAX_WIDTH_MM} mm"
        ));
    }
    let mut notes: Vec<String> = Vec::new();
    let reduced;
    let img = if img.w.saturating_mul(img.h) > t.max_pixels {
        let k = (t.max_pixels as f64 / (img.w as f64 * img.h as f64)).sqrt();
        let (w, h) = (
            ((img.w as f64 * k).floor() as usize).max(1),
            ((img.h as f64 * k).floor() as usize).max(1),
        );
        reduced = img.resize(w, h)?;
        notes.push(format!(
            "the picture was reduced from {} x {} to {} x {} pixels (max_pixels)",
            img.w, img.h, reduced.w, reduced.h
        ));
        &reduced
    } else {
        img
    };
    let level = i16::from(t.threshold);
    let field: Vec<i16> = img
        .data
        .iter()
        .map(|&v| {
            if t.invert {
                i16::from(v) - level + 1
            } else {
                level - i16::from(v)
            }
        })
        .collect();
    let field = Field {
        w: img.w as isize,
        h: img.h as isize,
        f: &field,
    };
    let scale = width_mm / img.w as f64;
    let (max_x, max_y) = (img.w as f64 * scale, img.h as f64 * scale);
    let mut kept: Vec<Vec<Pt>> = Vec::new();
    let mut specks = 0usize;
    for ring in rings(&field, t.max_segments)? {
        let mm: Vec<Pt> = ring
            .iter()
            .map(|&(x, y)| ((x * scale).clamp(0.0, max_x), (y * scale).clamp(0.0, max_y)))
            .collect();
        let mm = simplify_ring(&mm, t.simplify_mm);
        if mm.len() < 3 || signed_area(&mm).abs() < t.min_area_mm2 {
            specks += 1;
            continue;
        }
        kept.push(mm);
        if kept.len() > t.max_contours {
            return Err(format!(
                "the picture has more than {} contours (max_contours): raise min_area_mm2 or the threshold, or clean the picture",
                t.max_contours
            ));
        }
    }
    if kept.is_empty() {
        return Err(format!(
            "there is no contour to follow in the picture at the threshold {}{}: try another threshold{}",
            t.threshold,
            if t.invert { " (inverted)" } else { "" },
            if t.invert { "" } else { " or --invert" }
        ));
    }
    if specks > 0 {
        notes.push(format!(
            "{specks} speck(s) smaller than {} mm2 were left out (min_area_mm2)",
            t.min_area_mm2
        ));
    }
    let mut c = Collector::new(Source::Svg, limits);
    for n in notes {
        c.warn(n);
    }
    for ring in kept {
        c.push(LAYER, None, ring, true)?;
    }
    Ok(c.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paper(w: usize, h: usize) -> Gray {
        Gray::blank(w, h, 255).unwrap()
    }

    fn settings() -> TraceSettings {
        TraceSettings {
            min_area_mm2: 0.0,
            simplify_mm: 0.0,
            ..TraceSettings::default()
        }
    }

    fn disc(w: usize, r: f64) -> Gray {
        let mut g = paper(w, w);
        g.disc(w as f64 / 2.0, w as f64 / 2.0, r, 0);
        g
    }

    fn deviation_from_circle(ring: &[Pt], centre: Pt, r: f64) -> f64 {
        ring.iter()
            .map(|&(x, y)| ((x - centre.0).hypot(y - centre.1) - r).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_disc_becomes_one_closed_smooth_ring_of_the_right_size() {
        let d = trace(&disc(40, 12.0), 40.0, &settings(), Limits::default()).unwrap();
        assert_eq!(d.shapes.len(), 1);
        let s = &d.shapes[0];
        assert!(s.closed && s.layer == LAYER && s.color.is_none());
        // 1 mm is one pixel here (the disc is centred on the pixel 20, which is at 20.5 mm); the border stays within a pixel of the true circle
        assert!(deviation_from_circle(&s.pts, (20.5, 20.5), 12.0) < 0.8);
        // a smooth line, not the steps of the pixels
        let area = signed_area(&s.pts).abs();
        assert!((area - std::f64::consts::PI * 144.0).abs() < 20.0, "{area}");
    }

    #[test]
    fn a_ring_has_an_outer_contour_and_a_hole() {
        let mut g = disc(60, 20.0);
        g.disc(30.0, 30.0, 8.0, 255);
        let d = trace(&g, 60.0, &settings(), Limits::default()).unwrap();
        assert_eq!(d.shapes.len(), 2);
        let mut areas: Vec<f64> = d.shapes.iter().map(|s| signed_area(&s.pts).abs()).collect();
        areas.sort_by(f64::total_cmp);
        assert!((areas[0] - std::f64::consts::PI * 64.0).abs() < 15.0);
        assert!((areas[1] - std::f64::consts::PI * 400.0).abs() < 40.0);
    }

    #[test]
    fn the_work_is_as_wide_as_asked_and_the_top_of_the_picture_is_y_zero() {
        let mut g = paper(100, 50);
        for y in 10..20 {
            for x in 20..70 {
                g.data[y * 100 + x] = 0;
            }
        }
        let d = trace(&g, 200.0, &settings(), Limits::default()).unwrap();
        let ((x0, y0), (x1, y1)) = d.bounds().unwrap();
        // 2 mm a pixel; the rectangle of ink is 50 x 10 pixels at (20, 10), and the line runs half way between the ink and the paper, which is the edge of the pixels
        assert!(
            (x0 - 40.0).abs() < 0.1 && (x1 - 140.0).abs() < 0.1,
            "{x0} {x1}"
        );
        assert!(
            (y0 - 20.0).abs() < 0.1 && (y1 - 40.0).abs() < 0.1,
            "{y0} {y1}"
        );
        assert_eq!(d.source, Source::Svg);
    }

    #[test]
    fn the_border_of_the_picture_is_followed_and_never_left_outside() {
        let g = Gray::blank(10, 8, 0).unwrap();
        let d = trace(&g, 10.0, &settings(), Limits::default()).unwrap();
        assert_eq!(d.shapes.len(), 1);
        let ((x0, y0), (x1, y1)) = d.bounds().unwrap();
        assert_eq!((x0, y0, x1, y1), (0.0, 0.0, 10.0, 8.0));
    }

    #[test]
    fn invert_swaps_ink_and_paper() {
        let mut g = Gray::blank(40, 40, 0).unwrap();
        g.disc(20.0, 20.0, 10.0, 255);
        // dark ink: the contour of the picture and the hole of the disc
        let dark = trace(&g, 40.0, &settings(), Limits::default()).unwrap();
        assert_eq!(dark.shapes.len(), 2);
        // light ink: the disc alone
        let light = trace(
            &g,
            40.0,
            &TraceSettings {
                invert: true,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(light.shapes.len(), 1);
        assert!(deviation_from_circle(&light.shapes[0].pts, (20.5, 20.5), 10.0) < 0.8);
    }

    #[test]
    fn the_threshold_decides_what_is_ink() {
        let mut g = paper(30, 30);
        g.disc(15.0, 15.0, 8.0, 100);
        // grey 100 is ink at the threshold 128 and paper at the threshold 50
        assert!(trace(&g, 30.0, &settings(), Limits::default()).is_ok());
        let e = trace(
            &g,
            30.0,
            &TraceSettings {
                threshold: 50,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap_err();
        assert!(e.contains("no contour") && e.contains("--invert"), "{e}");
        // a threshold of 0 has no ink at all
        let e = trace(
            &g,
            30.0,
            &TraceSettings {
                threshold: 0,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap_err();
        assert!(e.contains("no contour"), "{e}");
    }

    #[test]
    fn a_soft_edge_puts_the_line_where_the_grey_crosses_the_threshold() {
        // a row of pixels that goes from black to white: the line is between the pixels where the grey passes 128
        let data: Vec<u8> = (0..10)
            .flat_map(|_| [0u8, 0, 0, 0, 100, 156, 255, 255, 255, 255])
            .collect();
        let g = Gray::new(10, 10, data).unwrap();
        let d = trace(&g, 10.0, &settings(), Limits::default()).unwrap();
        let ((_, _), (x1, _)) = d.bounds().unwrap();
        // 100 is ink and 156 is paper: the crossing is half way between their centres (4.5 and 5.5 px), at 5.0
        assert!((x1 - 5.0).abs() < 0.01, "{x1}");
    }

    #[test]
    fn specks_are_dropped_with_a_note_and_the_shape_stays() {
        let mut g = paper(60, 60);
        g.disc(30.0, 30.0, 15.0, 0);
        g.data[3 * 60 + 3] = 0;
        g.data[50 * 60 + 5] = 0;
        let keep_all = trace(&g, 60.0, &settings(), Limits::default()).unwrap();
        assert_eq!(keep_all.shapes.len(), 3);
        let clean = trace(
            &g,
            60.0,
            &TraceSettings {
                min_area_mm2: 2.0,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(clean.shapes.len(), 1);
        assert!(
            clean.warnings.iter().any(|w| w.contains("2 speck")),
            "{:?}",
            clean.warnings
        );
    }

    #[test]
    fn simplifying_drops_points_and_keeps_the_line_within_the_tolerance() {
        let g = disc(80, 30.0);
        let raw = trace(&g, 80.0, &settings(), Limits::default()).unwrap();
        let eps = 0.2;
        let slim = trace(
            &g,
            80.0,
            &TraceSettings {
                simplify_mm: eps,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap();
        let (a, b) = (&raw.shapes[0].pts, &slim.shapes[0].pts);
        assert!(b.len() * 2 < a.len(), "{} {}", b.len(), a.len());
        // no point of the raw line is farther than the tolerance from the simplified line
        for &p in a {
            let d = b
                .iter()
                .zip(b.iter().cycle().skip(1))
                .map(|(&s, &e)| crate::vector::dist_to_segment(p, s, e))
                .fold(f64::INFINITY, f64::min);
            assert!(d <= eps + 1e-9, "{d}");
        }
    }

    #[test]
    fn a_straight_ring_loses_every_point_on_a_side() {
        let ring = [
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (10.0, 10.0),
            (5.0, 10.0),
            (0.0, 10.0),
            (0.0, 5.0),
        ];
        let s = simplify_ring(&ring, 0.01);
        assert_eq!(s.len(), 4, "{s:?}");
        // without a tolerance nothing changes
        assert_eq!(simplify_ring(&ring, 0.0), ring.to_vec());
        // a triangle is already the simplest ring
        let tri = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        assert_eq!(simplify_ring(&tri, 5.0), tri.to_vec());
    }

    #[test]
    fn two_pixels_that_touch_at_a_corner_never_leave_a_gap_or_panic() {
        let mut g = paper(4, 4);
        g.data[4 + 1] = 0;
        g.data[2 * 4 + 2] = 0;
        let d = trace(&g, 4.0, &settings(), Limits::default()).unwrap();
        // the centre of the cell is ink or paper by the mean of its corners: two black pixels outweigh two white ones, so the ink is joined into one piece
        assert_eq!(d.shapes.len(), 1);
        // and the other diagonal
        let mut g = paper(4, 4);
        g.data[4 + 2] = 0;
        g.data[2 * 4 + 1] = 0;
        let d = trace(&g, 4.0, &settings(), Limits::default()).unwrap();
        assert_eq!(d.shapes.len(), 1);
        // two faint pixels are outweighed by the paper round them: two separate pieces
        let mut g = paper(4, 4);
        g.data[4 + 1] = 100;
        g.data[2 * 4 + 2] = 100;
        let d = trace(&g, 4.0, &settings(), Limits::default()).unwrap();
        assert_eq!(d.shapes.len(), 2);
    }

    #[test]
    fn a_picture_with_too_many_pieces_of_border_is_an_error_and_not_a_slow_program() {
        // a chessboard has a piece of border round every square
        let data: Vec<u8> = (0..64 * 64)
            .map(|k| if (k / 64 + k % 64) % 2 == 0 { 0 } else { 255 })
            .collect();
        let g = Gray::new(64, 64, data).unwrap();
        let e = trace(
            &g,
            64.0,
            &TraceSettings {
                max_segments: 1000,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap_err();
        assert!(e.contains("max_segments"), "{e}");
        let e = trace(
            &g,
            64.0,
            &TraceSettings {
                max_contours: 100,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap_err();
        assert!(e.contains("max_contours"), "{e}");
    }

    #[test]
    fn a_big_picture_is_reduced_to_max_pixels_with_a_note() {
        let g = disc(200, 80.0);
        let d = trace(
            &g,
            100.0,
            &TraceSettings {
                max_pixels: 2500,
                ..settings()
            },
            Limits::default(),
        )
        .unwrap();
        assert!(
            d.warnings.iter().any(|w| w.contains("reduced")),
            "{:?}",
            d.warnings
        );
        let ((x0, _), (x1, _)) = d.bounds().unwrap();
        // still 100 mm wide in the picture, the disc of radius 80 / 200 of it
        assert!((x1 - x0 - 80.0).abs() < 4.0, "{}", x1 - x0);
    }

    #[test]
    fn the_limits_of_the_drawing_hold_over_the_contours() {
        let g = disc(40, 12.0);
        let tight = Limits {
            max_points: 5,
            ..Limits::default()
        };
        assert!(trace(&g, 40.0, &settings(), tight).is_err());
    }

    #[test]
    fn a_width_that_is_not_possible_is_an_error() {
        let g = disc(20, 5.0);
        for bad in [0.0, -3.0, f64::NAN, f64::INFINITY, MAX_WIDTH_MM + 1.0] {
            assert!(
                trace(&g, bad, &settings(), Limits::default()).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_contours_of_a_picture_are_cut_by_the_same_plan_as_a_drawing() {
        let mut g = disc(60, 20.0);
        g.disc(30.0, 30.0, 8.0, 255);
        let d = trace(&g, 30.0, &TraceSettings::default(), Limits::default()).unwrap();
        let mut s = crate::vconfig::Settings::default();
        s.defaults.mode = crate::vconfig::LayerMode::Cut;
        s.defaults.power_percent = 60.0;
        s.defaults.speed_mm_min = 300.0;
        let plan = crate::cut::plan(&d, &s).unwrap();
        assert_eq!(plan.stats.contours, 2);
        assert!(plan.gcode.contains("; layer trace: cut"));
        // the hole is cut before the piece it is a hole of
        let first = plan.gcode.find("G1").unwrap();
        assert!(plan.gcode[first..].contains("S600"));
        crate::cut::verify_gcode(&plan.gcode, 1000, None).unwrap();
    }
}
