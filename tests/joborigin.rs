//! Where the work goes (the nine anchor points, or the place it was drawn), the frame that shows it with the laser off, and how long it takes as a range with its braking and its passes. The
//! drawings are written here; nothing is sent to a machine.

use eh3d_phoenix_laser::cut::plan;
use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::job::{estimate, flat_seconds, ramp_seconds, work_box, work_frame, Anchor};
use eh3d_phoenix_laser::preview;
use eh3d_phoenix_laser::vconfig::{parse, Settings};
use eh3d_phoenix_laser::vector::{Drawing, ReadOptions};

/// A rectangle of `w` x `h` mm with its lower left corner at (`x`, `y`), as a light polyline in a DXF file on the layer `l`.
fn rect_dxf(x: f64, y: f64, w: f64, h: f64) -> String {
    format!(
        "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\nl\n90\n4\n70\n1\n10\n{x}\n20\n{y}\n10\n{}\n20\n{y}\n10\n{}\n20\n{}\n10\n{x}\n20\n{}\n0\nENDSEC\n0\nEOF\n",
        x + w,
        x + w,
        y + h,
        y + h
    )
}

fn drawing(x: f64, y: f64, w: f64, h: f64) -> Drawing {
    read_dxf(rect_dxf(x, y, w, h).as_bytes(), &ReadOptions::default()).unwrap()
}

fn settings(text: &str) -> Settings {
    parse(text).unwrap_or_else(|e| panic!("{e}"))
}

/// The box of the places the head goes to in a program.
fn program_box(g: &str) -> ((f64, f64), (f64, f64)) {
    let p = preview::read(g);
    let pts: Vec<(f64, f64)> = p.moves.iter().flat_map(|m| [m.from, m.to]).collect();
    let min = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::INFINITY, f64::min);
    let max = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
    ((min(|p| p.0), min(|p| p.1)), (max(|p| p.0), max(|p| p.1)))
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

#[test]
fn every_anchor_of_the_work_goes_to_the_origin_of_the_machine() {
    // a rectangle of 20 x 10 mm drawn far from the origin; the origin of the machine is (50, 40)
    let d = drawing(100.0, 200.0, 20.0, 10.0);
    for a in Anchor::ALL {
        let s = settings(&format!(
            "[machine]\norigin_x = 50\norigin_y = 40\nbed_x_mm = 200\nbed_y_mm = 200\n[job]\nanchor = {}\n[layer l]\nmode = cut\n",
            a.word()
        ));
        let (lo, hi) = work_box(&d, &s).unwrap();
        assert!(
            close(a.point(lo, hi), (50.0, 40.0)),
            "{}: {lo:?} {hi:?}",
            a.word()
        );
        assert!(((hi.0 - lo.0) - 20.0).abs() < 1e-9 && ((hi.1 - lo.1) - 10.0).abs() < 1e-9);
    }
}

#[test]
fn the_program_burns_where_the_box_of_the_work_says() {
    let d = drawing(100.0, 200.0, 20.0, 10.0);
    for a in [Anchor::BottomLeft, Anchor::Center, Anchor::TopRight] {
        let s = settings(&format!(
            "[machine]\norigin_x = 50\norigin_y = 40\n[job]\nanchor = {}\n[layer l]\nmode = cut\n",
            a.word()
        ));
        let (lo, hi) = work_box(&d, &s).unwrap();
        let g = plan(&d, &s).unwrap().gcode;
        let (plo, phi) = program_box(&g);
        // the head starts at the origin, so the box of the moves holds the origin too
        assert!(
            plo.0 <= lo.0 + 1e-6 && plo.1 <= lo.1 + 1e-6,
            "{plo:?} {lo:?}"
        );
        assert!(
            phi.0 >= hi.0 - 1e-6 && phi.1 >= hi.1 - 1e-6,
            "{phi:?} {hi:?}"
        );
        let burn = preview::read(&g);
        let burn_pts: Vec<(f64, f64)> = burn
            .moves
            .iter()
            .filter(|m| m.power > 0.0)
            .flat_map(|m| [m.from, m.to])
            .collect();
        for p in burn_pts {
            assert!(p.0 >= lo.0 - 1e-6 && p.0 <= hi.0 + 1e-6, "{p:?}");
            assert!(p.1 >= lo.1 - 1e-6 && p.1 <= hi.1 + 1e-6, "{p:?}");
        }
    }
}

#[test]
fn the_default_anchor_is_the_lower_left_corner_as_it_always_was() {
    let d = drawing(100.0, 200.0, 20.0, 10.0);
    let plain = settings("[machine]\norigin_x = 5\norigin_y = 7\n[layer l]\nmode = cut\n");
    assert_eq!(plain.job.anchor, Anchor::BottomLeft);
    let (lo, _) = work_box(&d, &plain).unwrap();
    assert!(close(lo, (5.0, 7.0)), "{lo:?}");
}

#[test]
fn an_absolute_start_keeps_the_drawing_where_it_was_drawn() {
    let d = drawing(100.0, 200.0, 20.0, 10.0);
    let s = settings("[read]\nalign = page\n[layer l]\nmode = cut\n[job]\nanchor = center\n");
    let (lo, hi) = work_box(&d, &s).unwrap();
    // the anchor does not matter when the drawing is not moved
    assert!(
        close(lo, (100.0, 200.0)) && close(hi, (120.0, 210.0)),
        "{lo:?} {hi:?}"
    );
}

#[test]
fn the_frame_is_the_box_with_its_margin_and_never_turns_the_laser_on() {
    let d = drawing(0.0, 0.0, 20.0, 10.0);
    let s = settings(
        "[frame]\nmargin_mm = 2\nlaps = 2\nfeed_mm_min = 1500\n[machine]\norigin_x = 30\norigin_y = 30\n[layer l]\nmode = cut\n",
    );
    let f = work_frame(&d, &s).unwrap();
    assert!(
        close(f.frame.lo, (28.0, 28.0)) && close(f.frame.hi, (52.0, 42.0)),
        "{:?}",
        f.frame
    );
    assert_eq!(f.frame.pilot_s, 0);
    assert_eq!(
        f.gcode
            .lines()
            .filter(|l| l.starts_with("G1 X52 Y28"))
            .count(),
        2
    );
    for l in f.gcode.lines() {
        assert!(
            !l.contains("M3") && !l.contains("M4") && !l.contains(" S"),
            "{l}"
        );
    }
    assert!(f.gcode.ends_with("M2\n"), "{}", f.gcode);
}

#[test]
fn a_frame_that_leaves_the_bed_is_refused_and_nothing_is_cut_down() {
    let d = drawing(0.0, 0.0, 10.0, 10.0);
    let fits = settings("[machine]\nbed_x_mm = 14\nbed_y_mm = 14\n[frame]\nmargin_mm = 2\n[layer l]\nmode = cut\n[machine]\norigin_x = 2\norigin_y = 2\n");
    assert!(work_frame(&d, &fits).is_ok());
    let over = settings("[machine]\nbed_x_mm = 13\nbed_y_mm = 14\norigin_x = 2\norigin_y = 2\n[frame]\nmargin_mm = 2\n[layer l]\nmode = cut\n");
    let e = work_frame(&d, &over).unwrap_err();
    assert!(e.contains("outside of the bed"), "{e}");
}

#[test]
fn there_is_nothing_to_frame_when_every_layer_is_skipped() {
    let d = drawing(0.0, 0.0, 10.0, 10.0);
    let s = settings("[layer l]\nmode = skip\n");
    assert!(work_box(&d, &s).is_none());
    let e = work_frame(&d, &s).unwrap_err();
    assert!(e.contains("nothing to frame"), "{e}");
}

#[test]
fn the_time_is_a_range_that_rises_with_the_braking_and_with_every_pass() {
    let d = drawing(0.0, 0.0, 30.0, 20.0);
    let base = |accel: f64, passes: u32| {
        settings(&format!(
            "[job]\naccel_mm_s2 = {accel}\n[layer l]\nmode = cut\npower = 30\nspeed = 600\npasses = {passes}\n"
        ))
    };
    let slow = estimate(&d, &base(100.0, 1)).unwrap();
    let fast = estimate(&d, &base(10_000.0, 1)).unwrap();
    // the low end does not depend on the acceleration; the high end falls as the acceleration rises
    assert!((slow.low_seconds - fast.low_seconds).abs() < 1e-9);
    assert!(slow.low_seconds <= slow.high_seconds && fast.low_seconds <= fast.high_seconds);
    assert!(slow.high_seconds > fast.high_seconds);
    assert!(slow.range_text().contains(" to "));
    // the burning time is exactly one pass of the contour at its feed: 100 mm at 600 mm/min
    assert!(
        (slow.burn_seconds - 10.0).abs() < 1e-6,
        "{}",
        slow.burn_seconds
    );
    // three passes burn three times as long, and the time of the operation says its passes
    let three = estimate(&d, &base(100.0, 3)).unwrap();
    assert!((three.burn_seconds / slow.burn_seconds - 3.0).abs() < 1e-9);
    assert!(three.high_seconds > slow.high_seconds * 2.0);
    assert_eq!(three.operations.len(), 1);
    assert_eq!(
        (
            three.operations[0].name.as_str(),
            three.operations[0].passes
        ),
        ("l", 3)
    );
}

#[test]
fn the_passes_of_a_sublayer_are_in_the_time_of_the_work() {
    let d = drawing(0.0, 0.0, 30.0, 20.0);
    let one = estimate(&d, &settings("[layer l]\nmode = cut\nspeed = 600\n")).unwrap();
    let two = estimate(
        &d,
        &settings(
            "[layer l]\nmode = cut\nspeed = 600\n[sublayer l/again]\nmode = cut\npasses = 2\n",
        ),
    )
    .unwrap();
    // the layer burns once and the pass twice more
    assert!((two.burn_seconds / one.burn_seconds - 3.0).abs() < 1e-9);
    let names: Vec<(&str, u32)> = two
        .operations
        .iter()
        .map(|o| (o.name.as_str(), o.passes))
        .collect();
    assert_eq!(names, [("l", 1), ("l/again", 2)]);
}

#[test]
fn the_estimate_is_the_same_every_time_and_a_bad_acceleration_is_refused() {
    let d = drawing(0.0, 0.0, 30.0, 20.0);
    let s = settings("[layer l]\nmode = cut\n");
    assert_eq!(estimate(&d, &s).unwrap(), estimate(&d, &s).unwrap());
    for bad in ["0", "0.5", "100001", "x"] {
        let e = parse(&format!("[job]\naccel_mm_s2 = {bad}\n")).unwrap_err();
        assert!(
            e.contains("accel_mm_s2") || e.contains("number"),
            "{bad}: {e}"
        );
    }
    let e = parse("[job]\nanchor = middle\n").unwrap_err();
    assert!(e.contains("line 2") && e.contains("bottom_left"), "{e}");
}

/// A small generator of numbers that gives the same series every time.
struct Lcg(u64);

impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

#[test]
fn a_move_from_rest_to_rest_takes_at_least_the_time_at_its_feed_and_less_with_more_acceleration() {
    let mut rng = Lcg(0xA11C_E5ED);
    for _ in 0..2000 {
        let d = 0.01 + rng.unit() * 500.0;
        let feed = 10.0 + rng.unit() * 20_000.0;
        let a = 1.0 + rng.unit() * 5_000.0;
        let t = ramp_seconds(d, feed, a);
        assert!(t >= d / (feed / 60.0) - 1e-9, "{d} {feed} {a}: {t}");
        assert!(ramp_seconds(d, feed, a * 2.0) <= t + 1e-9, "{d} {feed} {a}");
        assert!(t.is_finite() && t > 0.0);
    }
}

#[test]
fn the_flat_time_of_a_program_counts_the_rapid_moves_at_the_rapid_speed() {
    let p = preview::read("G21\nG90\nG0 X60 Y0 S0\nM4 S0\nG1 X60 Y60 F600 S500\nM5\nM2\n");
    // 60 mm at the rapid speed of 3000 mm/min is 1.2 s, and 60 mm at 600 mm/min is 6 s
    assert!((flat_seconds(&p, 3000.0) - 7.2).abs() < 1e-9);
}
