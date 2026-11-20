// EH3D-PHOENIX-LASER - where the work lies, from a picture of the bed.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Where the work lies, from a picture of the bed: the positioning marks that were burnt (an L at each corner, and a cross when it was asked for) are found in a picture taken from above, and the
//! distance and the turn between where they should be and where they are tell how far the material has moved. The program does not read a camera: it reads a picture file that is looked at from
//! straight above, with the axes of the machine along the rows and the columns and a known number of pixels to the millimetre (`px_per_mm`), the way a calibrated camera would give it.
//!
//! **This is a simulator.** The pictures the tests use are drawn by [`simulate`] from the strokes of the marks, moved and turned by a known amount, and the search finds that amount again; nothing here
//! has read a picture from a real camera, and the lens, the light and the dust of a real bed are still to be tried.

use crate::vector::Pt;
use eh3d_image::Gray;

/// The most dark spots in a picture that are looked at; a picture with more than this is not a picture of marks.
pub const MAX_BLOBS: usize = 256;
/// A dark spot of fewer pixels than this is dust, not a mark.
pub const MIN_BLOB_PX: usize = 4;
/// The most pixels to the millimetre.
pub const MAX_PX_PER_MM: f64 = 1000.0;

/// A dark spot of the picture: where its middle is (in pixels, the first row on top) and how many pixels it has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blob {
    pub cx: f64,
    pub cy: f64,
    pub area: usize,
}

/// The dark spots of a picture (the pixels at or below `threshold`, joined along the rows and the columns) that have at least `min_area` pixels, the biggest first. A picture with more than
/// [`MAX_BLOBS`] of them is refused.
pub fn blobs(img: &Gray, threshold: u8, min_area: usize) -> Result<Vec<Blob>, String> {
    let (w, h) = (img.w, img.h);
    let dark = |i: usize| img.data[i] <= threshold;
    let mut seen = vec![false; w * h];
    let mut out: Vec<Blob> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for start in 0..w * h {
        if seen[start] || !dark(start) {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut n, mut sx, mut sy) = (0usize, 0.0_f64, 0.0_f64);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            n += 1;
            sx += x as f64;
            sy += y as f64;
            let mut visit = |j: usize| {
                if !seen[j] && dark(j) {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < w {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - w);
            }
            if y + 1 < h {
                visit(i + w);
            }
        }
        if n >= min_area.max(1) {
            out.push(Blob {
                cx: sx / n as f64,
                cy: sy / n as f64,
                area: n,
            });
            if out.len() > MAX_BLOBS {
                return Err(format!(
                    "the picture has more than {MAX_BLOBS} dark spots: it is not a picture of the marks"
                ));
            }
        }
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.area));
    Ok(out)
}

/// A camera that looks at the bed from above: how many pixels to the millimetre, how big the picture is, how thick the burnt line looks, and how the marks lie (turned by `angle_deg` about the
/// origin of the machine and then moved by `dx_mm`, `dy_mm`). The origin of the machine is the lower left corner of the picture; the first row of the picture is the top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub px_per_mm: f64,
    pub width_px: usize,
    pub height_px: usize,
    /// The width of the burnt line in the picture, in pixels.
    pub line_px: f64,
    pub dx_mm: f64,
    pub dy_mm: f64,
    pub angle_deg: f64,
}

impl Camera {
    /// A camera with the marks where they should be.
    pub fn new(px_per_mm: f64, width_px: usize, height_px: usize) -> Camera {
        Camera {
            px_per_mm,
            width_px,
            height_px,
            line_px: 3.0,
            dx_mm: 0.0,
            dy_mm: 0.0,
            angle_deg: 0.0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let ok = self.px_per_mm.is_finite()
            && (0.1..=MAX_PX_PER_MM).contains(&self.px_per_mm)
            && (1..=20_000).contains(&self.width_px)
            && (1..=20_000).contains(&self.height_px)
            && self.line_px.is_finite()
            && (1.0..=100.0).contains(&self.line_px)
            && self.dx_mm.is_finite()
            && self.dy_mm.is_finite()
            && self.angle_deg.is_finite()
            && self.dx_mm.abs() <= 100_000.0
            && self.dy_mm.abs() <= 100_000.0
            && self.angle_deg.abs() <= 360.0;
        if ok {
            Ok(())
        } else {
            Err("the camera needs 0.1 to 1000 pixels to the millimetre, a picture of 1 to 20000 pixels on each side, a line of 1 to 100 pixels, and a shift and a turn that are numbers".to_string())
        }
    }

    /// A point of the machine (mm) as a place in the picture (pixels), after the marks were moved and turned.
    fn pixel_of(&self, p: Pt) -> (f64, f64) {
        let (s, c) = self.angle_deg.to_radians().sin_cos();
        let (x, y) = (
            c * p.0 - s * p.1 + self.dx_mm,
            s * p.0 + c * p.1 + self.dy_mm,
        );
        (
            x * self.px_per_mm,
            self.height_px as f64 - y * self.px_per_mm,
        )
    }
}

/// The picture a camera would take of burnt strokes (dark lines on a light bed): the simulator of this module. A stroke outside of the picture is simply cut by its edge.
pub fn simulate(strokes: &[Vec<Pt>], cam: &Camera) -> Result<Gray, String> {
    cam.validate()?;
    let mut img = Gray::blank(cam.width_px, cam.height_px, 230)?;
    let r = cam.line_px / 2.0;
    for stroke in strokes {
        for pair in stroke.windows(2) {
            let (a, b) = (cam.pixel_of(pair[0]), cam.pixel_of(pair[1]));
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let steps = (len * 2.0).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let t = k as f64 / steps as f64;
                img.disc(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, r, 20);
            }
        }
    }
    Ok(img)
}

/// Where the marks lie compared with where they should be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Registration {
    /// How far the marks are moved (mm) after being turned about the origin of the machine: add it to the origin of the work to put the work on the marks.
    pub dx_mm: f64,
    pub dy_mm: f64,
    /// How far they are turned (degrees, counter-clockwise).
    pub angle_deg: f64,
    /// The mean distance (mm) between each mark seen and where the fit puts it: a large one says that the marks do not move as one piece (a lens, a mark that is not one).
    pub residual_mm: f64,
    /// How many marks were found and matched.
    pub found: usize,
}

/// What may be asked of the search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Search {
    pub px_per_mm: f64,
    /// A pixel at or below this is dark.
    pub threshold: u8,
    /// A mark further than this from where it should be (mm) is not matched.
    pub max_shift_mm: f64,
    /// A turn over this (degrees) is refused.
    pub max_angle_deg: f64,
    /// A residual over this (mm) is refused.
    pub max_residual_mm: f64,
}

impl Search {
    pub fn new(px_per_mm: f64) -> Search {
        Search {
            px_per_mm,
            threshold: 128,
            max_shift_mm: 20.0,
            max_angle_deg: 5.0,
            max_residual_mm: 1.0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let ok = self.px_per_mm.is_finite()
            && (0.1..=MAX_PX_PER_MM).contains(&self.px_per_mm)
            && self.max_shift_mm.is_finite()
            && (0.1..=1000.0).contains(&self.max_shift_mm)
            && self.max_angle_deg.is_finite()
            && (0.0..=90.0).contains(&self.max_angle_deg)
            && self.max_residual_mm.is_finite()
            && (0.0..=100.0).contains(&self.max_residual_mm);
        if ok {
            Ok(())
        } else {
            Err("the search needs 0.1 to 1000 pixels to the millimetre, a largest shift of 0.1 to 1000 mm, a largest turn of 0 to 90 degrees and a largest residual of 0 to 100 mm".to_string())
        }
    }
}

/// The middle of each burnt stroke group as the picture shows it, in millimetres of the machine (the origin at the lower left corner of the picture).
fn centres_mm(img: &Gray, s: &Search) -> Result<Vec<Pt>, String> {
    Ok(blobs(img, s.threshold, MIN_BLOB_PX)?
        .into_iter()
        .map(|b| (b.cx / s.px_per_mm, (img.h as f64 - b.cy) / s.px_per_mm))
        .collect())
}

/// Finds the marks in a picture and says how far they lie from the strokes they should be. The strokes are the ones of the marks (`frame::marks_strokes`) in millimetres of the machine; the
/// picture has the origin of the machine at its lower left corner. Every mark that should be there must be found, within `max_shift_mm` of its place, one picture spot for each; the turn and the
/// shift that fit them best are given, and a turn or a residual over the limits of the search is refused.
pub fn register(img: &Gray, strokes: &[Vec<Pt>], s: &Search) -> Result<Registration, String> {
    s.validate()?;
    // the marks as they should be, seen by a camera with nothing moved: the same way the picture is read, so that a bias of the middle of an L cancels out
    let reference = simulate(strokes, &Camera::new(s.px_per_mm, img.w, img.h))?;
    let want = centres_mm(&reference, s)?;
    if want.len() < 2 {
        return Err("the marks are not inside the picture: at least two must be seen".to_string());
    }
    let seen = centres_mm(img, s)?;
    // each mark that should be there takes the nearest spot that is still free, the nearest pairs first
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (i, w) in want.iter().enumerate() {
        for (j, o) in seen.iter().enumerate() {
            let d = ((w.0 - o.0).powi(2) + (w.1 - o.1).powi(2)).sqrt();
            if d <= s.max_shift_mm {
                pairs.push((d, i, j));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut took_want, mut took_seen) = (vec![false; want.len()], vec![false; seen.len()]);
    let mut fit: Vec<(Pt, Pt)> = Vec::new();
    for (_, i, j) in pairs {
        if !took_want[i] && !took_seen[j] {
            took_want[i] = true;
            took_seen[j] = true;
            fit.push((want[i], seen[j]));
        }
    }
    if fit.len() < want.len() {
        return Err(format!(
            "{} of {} marks were found within {} mm of their place",
            fit.len(),
            want.len(),
            s.max_shift_mm
        ));
    }
    let n = fit.len() as f64;
    let mean = |f: &dyn Fn(&(Pt, Pt)) -> f64| fit.iter().map(f).sum::<f64>() / n;
    let (ew, eo) = (
        (mean(&|p| p.0 .0), mean(&|p| p.0 .1)),
        (mean(&|p| p.1 .0), mean(&|p| p.1 .1)),
    );
    let (mut dot, mut cross) = (0.0_f64, 0.0_f64);
    for (e, o) in &fit {
        let (a, b) = ((e.0 - ew.0, e.1 - ew.1), (o.0 - eo.0, o.1 - eo.1));
        dot += a.0 * b.0 + a.1 * b.1;
        cross += a.0 * b.1 - a.1 * b.0;
    }
    let angle = cross.atan2(dot);
    let (sin, cos) = angle.sin_cos();
    let t = (
        eo.0 - (cos * ew.0 - sin * ew.1),
        eo.1 - (sin * ew.0 + cos * ew.1),
    );
    let residual = fit
        .iter()
        .map(|(e, o)| {
            let p = (cos * e.0 - sin * e.1 + t.0, sin * e.0 + cos * e.1 + t.1);
            ((p.0 - o.0).powi(2) + (p.1 - o.1).powi(2)).sqrt()
        })
        .sum::<f64>()
        / n;
    let reg = Registration {
        dx_mm: t.0,
        dy_mm: t.1,
        angle_deg: angle.to_degrees(),
        residual_mm: residual,
        found: fit.len(),
    };
    if reg.angle_deg.abs() > s.max_angle_deg {
        return Err(format!(
            "the marks are turned by {:.2} degrees: more than the {} degrees that are allowed",
            reg.angle_deg, s.max_angle_deg
        ));
    }
    if reg.residual_mm > s.max_residual_mm {
        return Err(format!(
            "the marks do not lie as one piece: they are {:.2} mm from the best fit, more than the {} mm that are allowed",
            reg.residual_mm, s.max_residual_mm
        ));
    }
    Ok(reg)
}

/// A picture as a binary PGM file, for the picture the simulator draws.
pub fn to_pgm(img: &Gray) -> Vec<u8> {
    let mut out = format!("P5\n{} {}\n255\n", img.w, img.h).into_bytes();
    out.extend_from_slice(&img.data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::marks_strokes;
    use crate::vconfig::MarksSettings;

    fn strokes(cross: bool) -> Vec<Vec<Pt>> {
        let k = MarksSettings {
            centre_cross: cross,
            ..MarksSettings::default()
        };
        marks_strokes((60.0, 40.0), (10.0, 10.0), &k).unwrap()
    }

    fn search() -> Search {
        Search::new(10.0)
    }

    fn camera() -> Camera {
        Camera::new(10.0, 900, 700)
    }

    fn assert_close(a: f64, b: f64, tol: f64, what: &str) {
        assert!((a - b).abs() <= tol, "{what}: {a} against {b}");
    }

    #[test]
    fn spots_are_found_by_their_middle_and_dust_is_left_out() {
        let mut img = Gray::blank(60, 40, 255).unwrap();
        img.disc(10.0, 10.0, 4.0, 0);
        img.disc(40.0, 30.0, 6.0, 0);
        img.data[5 * 60 + 55] = 0;
        let b = blobs(&img, 128, MIN_BLOB_PX).unwrap();
        assert_eq!(b.len(), 2, "{b:?}");
        assert!(b[0].area > b[1].area);
        assert_close(b[0].cx, 40.0, 0.1, "x of the big one");
        assert_close(b[0].cy, 30.0, 0.1, "y of the big one");
        assert_close(b[1].cx, 10.0, 0.1, "x of the small one");
        // everything counts when no minimum is asked
        assert_eq!(blobs(&img, 128, 1).unwrap().len(), 3);
        // a light picture has none
        assert!(blobs(&Gray::blank(10, 10, 200).unwrap(), 128, 1)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn marks_that_are_where_they_should_be_give_a_zero_shift() {
        let img = simulate(&strokes(false), &camera()).unwrap();
        let r = register(&img, &strokes(false), &search()).unwrap();
        assert_eq!(r.found, 4);
        assert_close(r.dx_mm, 0.0, 1e-9, "dx");
        assert_close(r.dy_mm, 0.0, 1e-9, "dy");
        assert_close(r.angle_deg, 0.0, 1e-9, "angle");
        assert!(r.residual_mm < 1e-9);
    }

    #[test]
    fn a_shift_of_the_material_is_found_again_to_a_fraction_of_a_millimetre() {
        for (dx, dy) in [(2.0, 0.0), (0.0, -3.5), (-4.2, 5.7), (12.0, 12.0)] {
            let cam = Camera {
                dx_mm: dx,
                dy_mm: dy,
                ..camera()
            };
            let img = simulate(&strokes(true), &cam).unwrap();
            let r = register(&img, &strokes(true), &search()).unwrap();
            assert_eq!(r.found, 5, "four corners and the cross");
            assert_close(r.dx_mm, dx, 0.15, "dx");
            assert_close(r.dy_mm, dy, 0.15, "dy");
            assert_close(r.angle_deg, 0.0, 0.3, "angle");
        }
    }

    #[test]
    fn a_turn_of_the_material_is_found_again() {
        for a in [-3.0, -0.5, 0.8, 2.5] {
            let cam = Camera {
                dx_mm: 1.0,
                dy_mm: -1.0,
                angle_deg: a,
                ..camera()
            };
            let img = simulate(&strokes(false), &cam).unwrap();
            let r = register(&img, &strokes(false), &search()).unwrap();
            assert_close(r.angle_deg, a, 0.25, "angle");
            assert!(r.residual_mm < 0.3, "{}", r.residual_mm);
            // the shift is the one that was made, to half a millimetre
            assert_close(r.dx_mm, 1.0, 0.5, "dx");
            assert_close(r.dy_mm, -1.0, 0.5, "dy");
        }
    }

    #[test]
    fn what_cannot_be_matched_or_is_too_far_off_is_refused_with_the_reason() {
        let img = simulate(&strokes(false), &camera()).unwrap();
        // a picture of nothing
        let empty = Gray::blank(900, 700, 230).unwrap();
        let e = register(&empty, &strokes(false), &search()).unwrap_err();
        assert!(e.contains("0 of 4"), "{e}");
        // a shift beyond the reach of the search
        let far = simulate(
            &strokes(false),
            &Camera {
                dx_mm: 30.0,
                ..camera()
            },
        )
        .unwrap();
        let e = register(&far, &strokes(false), &search()).unwrap_err();
        assert!(e.contains("marks were found within 20 mm"), "{e}");
        // a turn over the limit
        let turned = simulate(
            &strokes(false),
            &Camera {
                angle_deg: 8.0,
                ..camera()
            },
        )
        .unwrap();
        let strict = Search {
            max_angle_deg: 5.0,
            max_shift_mm: 100.0,
            ..search()
        };
        let e = register(&turned, &strokes(false), &strict).unwrap_err();
        assert!(e.contains("turned by"), "{e}");
        // strokes outside of the picture
        let small = Gray::blank(100, 100, 230).unwrap();
        let e = register(&small, &strokes(false), &search()).unwrap_err();
        assert!(e.contains("inside the picture"), "{e}");
        // a limit of the search that is not a number
        let bad = Search {
            px_per_mm: f64::NAN,
            ..search()
        };
        assert!(register(&img, &strokes(false), &bad).is_err());
        let bad = Search {
            max_residual_mm: -1.0,
            ..search()
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn a_spot_that_is_not_a_mark_and_a_missing_mark_are_told_apart() {
        let mut cam = camera();
        cam.dx_mm = 1.5;
        let mut img = simulate(&strokes(false), &cam).unwrap();
        // a smudge far from the marks does not matter: each mark takes the nearest spot
        img.disc(450.0, 350.0, 8.0, 10);
        let r = register(&img, &strokes(false), &search()).unwrap();
        assert_close(r.dx_mm, 1.5, 0.15, "dx");
        // a mark that is covered is missing (the lower left L, about 100 to 150 pixels from the left and 100 to 150 from the bottom)
        let mut covered = simulate(&strokes(false), &cam).unwrap();
        for y in 540..640 {
            for x in 90..190 {
                covered.data[y * 900 + x] = 230;
            }
        }
        let e = register(&covered, &strokes(false), &search()).unwrap_err();
        assert!(e.contains("of 4 marks"), "{e}");
    }

    #[test]
    fn the_camera_and_the_picture_are_checked() {
        for bad in [
            Camera::new(0.0, 100, 100),
            Camera::new(f64::NAN, 100, 100),
            Camera::new(10.0, 0, 100),
            Camera::new(10.0, 100, 30_000),
            Camera {
                line_px: 0.0,
                ..Camera::new(10.0, 100, 100)
            },
            Camera {
                angle_deg: f64::INFINITY,
                ..Camera::new(10.0, 100, 100)
            },
        ] {
            assert!(simulate(&strokes(false), &bad).is_err(), "{bad:?}");
        }
        // a picture with too many spots is not a picture of marks
        let mut busy = Gray::blank(400, 400, 255).unwrap();
        for k in 0..(MAX_BLOBS + 5) {
            busy.disc(
                6.0 + (k % 20) as f64 * 19.0,
                6.0 + (k / 20) as f64 * 19.0,
                3.0,
                0,
            );
        }
        assert!(blobs(&busy, 128, 1).unwrap_err().contains("dark spots"));
        let pgm = to_pgm(&Gray::blank(3, 2, 7).unwrap());
        assert_eq!(&pgm[..11], b"P5\n3 2\n255\n");
        assert_eq!(&pgm[11..], &[7, 7, 7, 7, 7, 7]);
    }

    #[test]
    fn random_shifts_and_turns_with_a_fixed_seed_are_found_again() {
        // xorshift64: the same sequence on every machine
        let mut state = 0x5eed_1a5e_0000_0010_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 10_000) as f64 / 10_000.0
        };
        for round in 0..40 {
            // the marks stay inside the picture: a mark cut by the edge of the picture is not the mark
            let (dx, dy) = (next() * 12.0 - 6.0, next() * 12.0 - 6.0);
            let angle = next() * 5.0 - 2.5;
            let with_cross = round % 2 == 0;
            let cam = Camera {
                dx_mm: dx,
                dy_mm: dy,
                angle_deg: angle,
                ..camera()
            };
            let img = simulate(&strokes(with_cross), &cam).unwrap();
            let r = register(&img, &strokes(with_cross), &search())
                .unwrap_or_else(|e| panic!("{dx} {dy} {angle}: {e}"));
            assert_eq!(r.found, if with_cross { 5 } else { 4 });
            assert_close(r.angle_deg, angle, 0.3, "angle");
            assert_close(r.dx_mm, dx, 0.6, "dx");
            assert_close(r.dy_mm, dy, 0.6, "dy");
        }
    }

    #[test]
    fn a_picture_of_noise_never_panics_and_is_refused_or_gives_finite_numbers() {
        let mut state = 0x5eed_1a5e_0000_0011_u64;
        for _ in 0..30 {
            let data: Vec<u8> = (0..300 * 200)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state % 256) as u8
                })
                .collect();
            let img = Gray::new(300, 200, data).unwrap();
            if let Ok(r) = register(&img, &strokes(false), &Search::new(5.0)) {
                assert!(r.dx_mm.is_finite() && r.dy_mm.is_finite() && r.angle_deg.is_finite());
            }
        }
    }
}
