//! The two test projects the program writes (the power-against-speed matrix and the y axis test): they are read back by the reader of the program, with the values of every layer, and the
//! commands that write them refuse what is wrong. A series of random matrices with a fixed seed is written and read back.

use eh3d_phoenix_laser::calibrate::series;
use eh3d_phoenix_laser::lbrn2::read_project;
use eh3d_phoenix_laser::testfiles::{matrix_project, y_axis_project};
use eh3d_phoenix_laser::vector::{bounds_of, ReadOptions};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

#[test]
fn a_matrix_is_read_back_with_one_layer_for_each_pair_and_its_own_power_and_speed() {
    let powers = series("20:80:20", "p").unwrap();
    let feeds = series("600:2400:600", "f").unwrap();
    let text = matrix_project(&powers, &feeds, 8.0, 3.0, true).unwrap();
    let p = read_project(text.as_bytes(), &ReadOptions::default()).unwrap();
    assert_eq!(p.cut_settings.len(), 16);
    assert_eq!(p.drawing.shapes.len(), 16);
    for (i, power) in powers.iter().enumerate() {
        for (j, feed) in feeds.iter().enumerate() {
            let name = format!("P{power} F{feed}");
            let c = p
                .cut_settings
                .iter()
                .find(|c| c.name == name)
                .unwrap_or_else(|| panic!("no layer {name}"));
            assert_eq!(c.max_power(), Some(*power));
            assert!(
                (c.speed_mm_s().unwrap() * 60.0 - feed).abs() < 1e-2,
                "{name}"
            );
            // the square of the pair is where the grid puts it: columns for the powers, rows for the feeds
            let key = c.label().to_lowercase();
            let s = p
                .drawing
                .shapes
                .iter()
                .find(|s| s.layer.to_lowercase() == key)
                .unwrap();
            let b = bounds_of(s.pts.iter().copied()).unwrap();
            assert!((b.0 .0 - i as f64 * 11.0).abs() < 1e-3, "{name} {b:?}");
            assert!((b.0 .1 - j as f64 * 11.0).abs() < 1e-3, "{name} {b:?}");
            assert!((b.1 .0 - b.0 .0 - 8.0).abs() < 1e-3);
        }
    }
    // the layers become the hints of the table of layers, so the window and the commands show the pair of each square
    let hints = p.hints();
    assert_eq!(hints.len(), 16);
    let h = hints.iter().find(|(k, _)| k == "p40 f1200").unwrap().1;
    assert_eq!((h.power_percent, h.speed_mm_min), (40.0, 1200.0));
}

#[test]
fn an_outline_matrix_follows_the_lines_and_a_fill_matrix_fills() {
    let m = |engrave| matrix_project(&[30.0], &[900.0], 8.0, 3.0, engrave).unwrap();
    let outline = read_project(m(false).as_bytes(), &ReadOptions::default()).unwrap();
    let fill = read_project(m(true).as_bytes(), &ReadOptions::default()).unwrap();
    assert_eq!(
        outline.hints()[0].1.mode,
        eh3d_phoenix_laser::vconfig::LayerMode::Cut
    );
    assert_eq!(
        fill.hints()[0].1.mode,
        eh3d_phoenix_laser::vconfig::LayerMode::Engrave
    );
}

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn random_matrices_are_written_and_read_back_exactly_with_a_fixed_seed() {
    let mut rng = Rng(0x7e57_f11e_0000_0007);
    for _ in 0..60 {
        let np = 1 + (rng.next() % 12) as usize;
        let nf = 1 + (rng.next() % 12) as usize;
        let powers: Vec<f64> = (0..np)
            .map(|k| 1.0 + k as f64 * 7.5 + (rng.next() % 3) as f64)
            .collect();
        let feeds: Vec<f64> = (0..nf)
            .map(|k| 100.0 + k as f64 * 250.0 + (rng.next() % 50) as f64)
            .collect();
        let square = 2.0 + (rng.next() % 20) as f64;
        let text = matrix_project(&powers, &feeds, square, 1.0, (rng.next() & 1) == 0).unwrap();
        let p = read_project(text.as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(p.cut_settings.len(), np * nf);
        assert_eq!(p.drawing.shapes.len(), np * nf);
        // the same call twice gives the same text
        assert_eq!(
            text,
            matrix_project(
                &powers,
                &feeds,
                square,
                1.0,
                p.hints()[0].1.mode == eh3d_phoenix_laser::vconfig::LayerMode::Engrave
            )
            .unwrap()
        );
        for c in &p.cut_settings {
            assert!(powers.contains(&c.max_power().unwrap()));
        }
    }
}

fn arrow_and_squares(flip: bool) -> (f64, f64, f64, f64) {
    let opts = ReadOptions {
        lbrn2_flip_y: flip,
        ..ReadOptions::default()
    };
    let p = read_project(y_axis_project().as_bytes(), &opts).unwrap();
    assert_eq!(p.drawing.shapes.len(), 3, "{:?}", p.drawing.warnings);
    let by_size = |f: &dyn Fn((f64, f64), (f64, f64)) -> bool| {
        p.drawing
            .shapes
            .iter()
            .find(|s| {
                let b = bounds_of(s.pts.iter().copied()).unwrap();
                f((b.1 .0 - b.0 .0, b.1 .1 - b.0 .1), b.0)
            })
            .map(|s| bounds_of(s.pts.iter().copied()).unwrap())
            .unwrap()
    };
    let square = by_size(&|size, _| (size.0 - 10.0).abs() < 1e-6 && (size.1 - 10.0).abs() < 1e-6);
    let large = by_size(&|size, _| (size.0 - 40.0).abs() < 1e-6);
    let arrow = by_size(&|size, _| (size.0 - 12.0).abs() < 1e-6);
    // the centre of the square, of the large rectangle, and the y of the point and of the base of the arrow
    let point = p
        .drawing
        .shapes
        .iter()
        .find(|s| {
            let b = bounds_of(s.pts.iter().copied()).unwrap();
            (b.1 .0 - b.0 .0 - 12.0).abs() < 1e-6
        })
        .unwrap()
        .pts
        .iter()
        .find(|q| (q.0 - 15.0).abs() < 1e-9)
        .unwrap()
        .1;
    (
        (square.0 .1 + square.1 .1) / 2.0,
        (large.0 .1 + large.1 .1) / 2.0,
        point,
        (arrow.0 .1 + arrow.1 .1) / 2.0,
    )
}

#[test]
fn the_y_axis_test_puts_the_arrow_toward_the_larger_y_of_the_file_and_the_setting_turns_it() {
    let (square, large, point, arrow_mid) = arrow_and_squares(true);
    // with the setting on, y of the file is y of the drawing: the square is near 15, the large rectangle near 60, the point of the arrow is the larger y of its three corners
    assert!((square - 15.0).abs() < 1e-6 && (large - 60.0).abs() < 1e-6);
    assert!(point > arrow_mid);
    // with the setting off the drawing is turned over: the order of the three is the other way round
    let (square, large, point, arrow_mid) = arrow_and_squares(false);
    assert!((square + 15.0).abs() < 1e-6 && (large + 60.0).abs() < 1e-6);
    assert!(point < arrow_mid);
}

fn run(args: &[&str]) -> (bool, String, String) {
    let o = Command::new(BIN).args(args).output().unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn the_commands_write_the_projects_and_refuse_what_is_wrong() {
    let dir = std::env::temp_dir().join(format!("eh3d-laser-testfiles-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let m = dir.join("m.lbrn2");
    let y = dir.join("y.lbrn2");
    let (ok, out, err) = run(&[
        "matrix",
        &m.to_string_lossy(),
        "--powers",
        "20:60:20",
        "--feeds",
        "600:1200:600",
        "--outline",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("6 layers"), "{out}");
    let text = std::fs::read_to_string(&m).unwrap();
    assert!(text.contains("type=\"Cut\"") && text.contains("P40 F1200"));
    // the file written by the command is the file of the library
    assert_eq!(
        text,
        matrix_project(&[20.0, 40.0, 60.0], &[600.0, 1200.0], 8.0, 3.0, false).unwrap()
    );
    let (ok, _, err) = run(&["ytest", &y.to_string_lossy()]);
    assert!(ok, "{err}");
    assert_eq!(std::fs::read_to_string(&y).unwrap(), y_axis_project());
    // what is wrong: another extension, a power over 100, a missing option, an extra word
    let bad = dir.join("m.nc");
    for args in [
        vec![
            "matrix",
            &bad.to_string_lossy(),
            "--powers",
            "10",
            "--feeds",
            "600",
        ],
        vec![
            "matrix",
            &m.to_string_lossy(),
            "--powers",
            "150",
            "--feeds",
            "600",
        ],
        vec!["matrix", &m.to_string_lossy(), "--feeds", "600"],
        vec!["ytest", &bad.to_string_lossy()],
        vec!["ytest", &y.to_string_lossy(), "extra"],
    ] {
        let args: Vec<&str> = args.iter().map(|s| &**s).collect();
        let (ok, _, err) = run(&args);
        assert!(!ok, "{args:?}");
        assert!(!err.is_empty());
    }
    assert!(!bad.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
