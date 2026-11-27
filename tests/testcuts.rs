//! The test of the spacing of the lines, the test of the width of the beam, the four further ways of dithering on the command line and the warning of a line that would be cut twice. The
//! programs are only read as text.

use eh3d_phoenix_laser::calibrate::{interval_test, kerf_from_gap, kerf_test, Grid, Kerf};
use eh3d_phoenix_laser::cut::{doubled_segments, plan};
use eh3d_phoenix_laser::order::Contour;
use eh3d_phoenix_laser::preview::{read, Move};
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::vconfig::Settings;
use std::path::Path;
use std::process::Command;

fn burns(program: &str) -> Vec<Move> {
    let p = read(program);
    assert!(p.ended && p.notes.is_empty(), "{:?} in\n{program}", p.notes);
    p.moves.into_iter().filter(|m| m.power > 0.0).collect()
}

#[test]
fn each_square_of_the_spacing_test_is_filled_at_its_own_spacing() {
    let g = Grid::default();
    let program = interval_test(&[1.0, 2.0, 4.0], 30.0, 2000.0, &g, None).unwrap();
    let moves = burns(&program);
    // 8 mm at 1, 2 and 4 mm: 9, 5 and 3 lines
    assert_eq!(moves.len(), 9 + 5 + 3);
    for (square, (spacing, lines)) in [(1.0, 9), (2.0, 5), (4.0, 3)].into_iter().enumerate() {
        let x0 = square as f64 * 11.0;
        let inside: Vec<&Move> = moves
            .iter()
            .filter(|m| m.from.0 >= x0 - 1e-9 && m.from.0 <= x0 + 8.0 + 1e-9)
            .collect();
        assert_eq!(inside.len(), lines, "square {square}");
        for (k, m) in inside.iter().enumerate() {
            assert!((m.from.1 - k as f64 * spacing).abs() < 1e-9);
            assert!((m.length() - 8.0).abs() < 1e-9);
            assert_eq!((m.power, m.feed), (300.0, 2000.0));
        }
    }
}

#[test]
fn the_figures_of_the_spacing_test_stand_below_the_squares() {
    let g = Grid::default();
    let plain = interval_test(&[1.0, 2.0], 30.0, 2000.0, &g, None).unwrap();
    let labelled = interval_test(&[1.0, 2.0], 30.0, 2000.0, &g, Some(2.0)).unwrap();
    let (a, b) = (burns(&plain), burns(&labelled));
    assert!(b.len() > a.len());
    // the squares are moved up by the figures and a gap, and the figures stay under them
    assert!(b[..a.len()].iter().all(|m| m.from.1 >= 3.5 - 1e-9));
    assert!(b[a.len()..]
        .iter()
        .all(|m| m.from.1 <= 2.0 + 1e-9 && m.to.1 <= 2.0 + 1e-9));
}

#[test]
fn the_spacing_test_is_refused_with_values_out_of_range() {
    let g = Grid::default();
    let ok = |s: &[f64], p: f64, f: f64, l: Option<f64>| interval_test(s, p, f, &g, l).is_ok();
    assert!(ok(&[0.02, 8.0], 100.0, 30_000.0, None));
    assert!(!ok(&[], 30.0, 2000.0, None));
    assert!(!ok(&[0.01], 30.0, 2000.0, None));
    assert!(!ok(&[8.5], 30.0, 2000.0, None));
    assert!(!ok(&[f64::NAN], 30.0, 2000.0, None));
    assert!(!ok(&[1.0; 13], 30.0, 2000.0, None));
    assert!(!ok(&[1.0], 0.0, 2000.0, None));
    assert!(!ok(&[1.0], 101.0, 2000.0, None));
    assert!(!ok(&[1.0], 0.01, 2000.0, None));
    assert!(!ok(&[1.0], 30.0, 0.0, None));
    assert!(!ok(&[1.0], 30.0, 2000.0, Some(0.5)));
    // figures wider than a square and its gap would run into each other
    assert!(!ok(&[0.125], 30.0, 2000.0, Some(8.0)));
}

fn kerf() -> Kerf {
    Kerf {
        strips: 4,
        strip_mm: 5.0,
        height_mm: 20.0,
        power_percent: 80.0,
        feed_mm_min: 300.0,
        passes: 1,
    }
}

#[test]
fn the_kerf_test_cuts_between_the_strips_first_and_the_rectangle_last() {
    let moves = burns(&kerf_test(&kerf(), 1000).unwrap());
    assert_eq!(moves.len(), 3 + 4);
    for (i, m) in moves[..3].iter().enumerate() {
        let x = (i + 1) as f64 * 5.0;
        assert_eq!((m.from.0, m.to.0), (x, x));
        assert!((m.length() - 20.0).abs() < 1e-9);
    }
    let outline: f64 = moves[3..].iter().map(Move::length).sum();
    assert!((outline - 80.0).abs() < 1e-9);
    assert!(moves.iter().all(|m| m.power == 800.0 && m.feed == 300.0));
    let twice = Kerf {
        passes: 2,
        ..kerf()
    };
    assert_eq!(burns(&kerf_test(&twice, 1000).unwrap()).len(), 14);
}

#[test]
fn the_kerf_test_is_refused_with_values_out_of_range() {
    let bad = [
        Kerf {
            strips: 1,
            ..kerf()
        },
        Kerf {
            strips: 41,
            ..kerf()
        },
        Kerf {
            strip_mm: 1.0,
            ..kerf()
        },
        Kerf {
            height_mm: 201.0,
            ..kerf()
        },
        Kerf {
            power_percent: 0.0,
            ..kerf()
        },
        Kerf {
            feed_mm_min: f64::NAN,
            ..kerf()
        },
        Kerf {
            passes: 0,
            ..kerf()
        },
    ];
    for k in bad {
        assert!(kerf_test(&k, 1000).is_err(), "{k:?}");
    }
    assert!(kerf_test(&kerf(), 0).is_err());
}

#[test]
fn the_width_of_the_beam_is_the_gap_over_the_cuts() {
    // ten strips are eleven cuts
    assert!((kerf_from_gap(1.65, 10).unwrap() - 0.15).abs() < 1e-12);
    for (gap, strips) in [(0.0, 10), (-1.0, 10), (f64::NAN, 10), (30.0, 10), (1.0, 1)] {
        assert!(kerf_from_gap(gap, strips).is_err(), "{gap} {strips}");
    }
}

fn line(from: (f64, f64), to: (f64, f64)) -> Contour {
    Contour {
        pts: vec![from, to],
        closed: false,
    }
}

#[test]
fn a_piece_drawn_twice_is_counted_whichever_way_round() {
    let a = line((0.0, 0.0), (10.0, 0.0));
    let back = line((10.0, 0.0), (0.0, 0.0));
    let other = line((0.0, 1.0), (10.0, 1.0));
    assert_eq!(doubled_segments([a.clone(), other].iter()), 0);
    assert_eq!(doubled_segments([a.clone(), back].iter()), 1);
    assert_eq!(doubled_segments([a.clone(), a.clone(), a].iter()), 2);
    let square = Contour {
        pts: vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0), (0.0, 5.0)],
        closed: true,
    };
    assert_eq!(doubled_segments([square.clone()].iter()), 0);
    assert_eq!(doubled_segments([square.clone(), square].iter()), 4);
}

#[test]
fn a_drawing_with_a_line_drawn_twice_says_it_in_the_plan() {
    let s = Settings::default();
    let svg = |paths: &str| {
        let text = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{paths}</svg>"##
        );
        plan(&read_svg(text.as_bytes(), &s.read).unwrap(), &s).unwrap()
    };
    let one = r##"<path d="M10 10 L40 10" stroke="#f00" fill="none"/>"##;
    let twice = svg(&format!("{one}{one}"));
    assert!(
        twice.warnings.iter().any(|w| w.contains("drawn twice")),
        "{:?}",
        twice.warnings
    );
    assert!(!svg(one).warnings.iter().any(|w| w.contains("drawn twice")));
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn out(name: &str) -> String {
    let path = format!("{}/testcuts-{name}", env!("CARGO_TARGET_TMPDIR"));
    let _ = std::fs::remove_file(&path);
    path
}

#[test]
fn the_command_line_writes_the_two_tests_and_writes_nothing_over_a_limit() {
    let nc = out("interval.nc");
    let args = [
        "--spacings",
        "0.1:0.3:0.1",
        "--power",
        "30",
        "--feed",
        "3000",
    ];
    let mut full = vec!["interval", &nc];
    full.extend(args);
    let (ok, said, err) = run(&full);
    assert!(ok, "{err}");
    assert!(said.contains("3 squares"), "{said}");
    let (ok, _, err) = run(&["verify", &nc]);
    assert!(ok, "{err}");

    let refused = out("refused.nc");
    let mut over = vec!["interval", &refused];
    over.extend(args);
    over.extend(["--max-power", "20"]);
    let (ok, _, err) = run(&over);
    assert!(!ok && err.contains("limit of power"), "{err}");
    assert!(!Path::new(&refused).exists());

    let outside = out("outside.nc");
    let (ok, _, err) = run(&[
        "kerf", &outside, "--power", "80", "--feed", "300", "--bed", "20,20",
    ]);
    assert!(!ok && err.contains("outside of the bed"), "{err}");
    assert!(!Path::new(&outside).exists());

    let nc = out("kerf.nc");
    let (ok, said, err) = run(&["kerf", &nc, "--power", "80", "--feed", "300"]);
    assert!(ok, "{err}");
    assert!(said.contains("--gap"), "{said}");
    assert!(Path::new(&nc).exists());
    let (ok, said, err) = run(&["kerf", "--gap", "1.65", "--strips", "10"]);
    assert!(ok, "{err}");
    assert!(said.contains("0.150 mm"), "{said}");
    assert!(!run(&["kerf", &nc, "--feed", "300"]).0);
}

#[test]
fn the_command_line_engraves_with_each_further_way_of_dithering() {
    let img = format!("{}/examples/gradient.pgm", env!("CARGO_MANIFEST_DIR"));
    let mut programs = Vec::new();
    for method in ["jarvis", "stucki", "sierra", "burkes"] {
        let nc = out(&format!("{method}.nc"));
        let (ok, _, err) = run(&[
            "engrave",
            &img,
            &nc,
            "--width-mm",
            "16",
            "--dot-mm",
            "0.5",
            "--dither",
            method,
        ]);
        assert!(ok, "{method}: {err}");
        // the head runs past the work with the laser off, and the program passes its check
        let (ok, said, err) = run(&["verify", &nc]);
        assert!(ok, "{method}: {said}{err}");
        programs.push(std::fs::read_to_string(&nc).unwrap());
    }
    programs.dedup();
    assert!(programs.len() > 1, "the ways give different dots");
}
