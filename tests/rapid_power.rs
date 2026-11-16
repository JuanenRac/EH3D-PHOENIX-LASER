//! No rapid move (G0) travels with a power still set: the power is put to zero first, in the image and in the grid.

use eh3d_phoenix_laser::calibrate::{grid, Grid};
use eh3d_phoenix_laser::dither::Dots;
use eh3d_phoenix_laser::raster::{engrave, Params};

/// The power (`S`) in force at each G0 of a program, from the words of the lines before it.
fn power_at_each_rapid(gcode: &str) -> Vec<u32> {
    let mut s = 0u32;
    let mut at_rapid = Vec::new();
    for line in gcode.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.first() == Some(&"G0") {
            at_rapid.push(s);
        }
        for w in &words {
            if let Some(v) = w.strip_prefix('S') {
                s = v.parse().unwrap_or(s);
            }
        }
    }
    at_rapid
}

#[test]
fn a_rapid_move_between_rows_of_an_image_has_the_power_at_zero_even_without_overscan() {
    let dots = Dots {
        w: 3,
        h: 3,
        burn: vec![true, true, true, false, false, false, true, true, true],
    };
    let p = Params {
        pixel_mm: 1.0,
        feed_mm_min: 6000.0,
        s_max: 1000,
        power_percent: 50.0,
        overscan_mm: 0.0,
        origin: (0.0, 0.0),
    };
    let e = engrave(&dots, &p).unwrap();
    let at = power_at_each_rapid(&e.gcode);
    assert_eq!(at.len(), 2, "{}", e.gcode);
    assert!(at.iter().all(|&s| s == 0), "{}", e.gcode);
}

#[test]
fn a_rapid_move_between_the_squares_of_the_grid_has_the_power_at_zero() {
    let text = grid(&[10.0, 50.0, 100.0], &[600.0, 1200.0], &Grid::default()).unwrap();
    let at = power_at_each_rapid(&text);
    assert_eq!(at.len(), 6, "{text}");
    assert!(at.iter().all(|&s| s == 0), "{text}");
}
