//! The G-code that the program writes is read back with the reader of the suite and walked the way the G-code debugger walks it in its strict mode: nothing in it may be a warning of
//! the reader, a cut with the laser off or a cutting move without a feed, and the program must end.

use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Severity};
use eh3d_phoenix_laser::calibrate::{grid, Grid};
use eh3d_phoenix_laser::dither::Dots;
use eh3d_phoenix_laser::raster::{engrave, Params};

/// What the strict mode of the debugger would report for `text`.
fn alerts(text: &str) -> Vec<String> {
    let trace = interpret(&parse(text, Dialect::Grbl));
    let mut found: Vec<String> = trace
        .diagnostics
        .iter()
        .filter(|d| d.severity >= Severity::Warning)
        .map(|d| format!("line {}: {}", d.line, d.message))
        .collect();
    for s in &trace.segments {
        let rapid = matches!(s.motion, Motion::Rapid);
        if !rapid && !s.spindle_on {
            found.push(format!("line {}: it cuts with the laser off", s.line));
        }
        if !rapid && s.feed <= 0.0 {
            found.push(format!("line {}: a cutting move has no feed", s.line));
        }
    }
    if !trace.ended {
        found.push("the program does not end".into());
    }
    found
}

#[test]
fn the_checker_used_here_does_find_a_cut_with_the_laser_off() {
    assert!(!alerts("G21\nG90\nF600\nG1 X5\nM2\n").is_empty());
    assert!(!alerts("G21\nG90\nM3 S100\nG1 X5\nM2\n").is_empty());
    assert!(alerts("G21\nG90\nF600\nM3 S100\nG1 X5\nM5\nM2\n").is_empty());
}

#[test]
fn the_calibration_grid_has_no_alert() {
    let text = grid(&[10.0, 50.0, 100.0], &[600.0, 1200.0], &Grid::default()).unwrap();
    let found = alerts(&text);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_engraving_has_no_alert() {
    // a small chequerboard of dots
    let (w, h) = (8, 6);
    let burn: Vec<bool> = (0..w * h).map(|i| (i % w + i / w) % 2 == 0).collect();
    let dots = Dots { w, h, burn };
    let engraving = engrave(&dots, &Params::default()).unwrap();
    assert!(engraving.runs > 0);
    let found = alerts(&engraving.gcode);
    assert!(found.is_empty(), "{found:?}");
}
