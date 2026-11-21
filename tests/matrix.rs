//! The series of the test grid (`10:50:10`) and the power-against-speed matrix that is made from them, with a random series of a fixed seed.

use eh3d_phoenix_laser::calibrate::{grid, series, Grid, MAX_CELLS, MAX_SERIES};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

#[test]
fn a_range_is_every_value_from_the_first_to_the_last_at_its_step() {
    assert_eq!(
        series("10:50:10", "--powers").unwrap(),
        vec![10.0, 20.0, 30.0, 40.0, 50.0]
    );
    // a step that does not reach the last value stops below it
    assert_eq!(
        series("100:350:100", "x").unwrap(),
        vec![100.0, 200.0, 300.0]
    );
    // a mixture of numbers and ranges keeps the order it was written in
    assert_eq!(
        series("5,10:30:10,45", "x").unwrap(),
        vec![5.0, 10.0, 20.0, 30.0, 45.0]
    );
    // a single value range
    assert_eq!(series("7:7:1", "x").unwrap(), vec![7.0]);
    // the sum of a step that has no exact binary form is rounded
    assert_eq!(
        series("0.1:0.5:0.1", "x").unwrap(),
        vec![0.1, 0.2, 0.3, 0.4, 0.5]
    );
    // plain lists are what they were
    assert_eq!(series("20,40,60", "x").unwrap(), vec![20.0, 40.0, 60.0]);
}

#[test]
fn a_range_that_is_wrong_is_an_error_that_names_the_option() {
    for bad in [
        "10:50",
        "10:50:0",
        "10:50:-5",
        "50:10:5",
        "a:b:c",
        "1:2:3:4",
        "",
        "10:50:nan",
        "1:1e9:1",
    ] {
        let e = series(bad, "--powers").unwrap_err();
        assert!(e.contains("--powers"), "{bad}: {e}");
    }
    // the series is limited
    let e = series(&format!("1:{}:1", MAX_SERIES + 5), "--feeds").unwrap_err();
    assert!(e.contains("more than"), "{e}");
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
fn random_ranges_have_the_number_of_values_that_the_arithmetic_says_with_a_fixed_seed() {
    let mut rng = Rng(0x0a7e_5eed_0000_0021);
    for _ in 0..300 {
        let first = (rng.next() % 1000) as f64 / 10.0;
        let step = ((rng.next() % 50) + 1) as f64 / 10.0;
        let count = (rng.next() % 40) as usize;
        let last = first + step * count as f64;
        let got = series(&format!("{first}:{last}:{step}"), "x").unwrap();
        assert_eq!(got.len(), count + 1, "{first}:{last}:{step}");
        assert!(got.windows(2).all(|w| w[1] > w[0]));
        assert!((got[0] - first).abs() < 1e-6);
    }
}

#[test]
fn a_matrix_of_ranges_burns_every_pair_and_never_goes_over_the_limits() {
    let powers = series("10:60:10", "p").unwrap();
    let feeds = series("600:2400:600", "f").unwrap();
    assert_eq!((powers.len(), feeds.len()), (6, 4));
    assert!(powers.len() <= MAX_CELLS && feeds.len() <= MAX_CELLS);
    let program = grid(&powers, &feeds, &Grid::default()).unwrap();
    // the same grid twice is the same program
    assert_eq!(program, grid(&powers, &feeds, &Grid::default()).unwrap());
    // every feed of the series is written, and the strongest power is 60 % of the maximum
    for f in &feeds {
        assert!(program.contains(&format!("F{}", *f as u32)), "{f}");
    }
    assert!(program.contains("S600"), "the 60 percent square");
    // more cells than the grid holds is refused
    let many = series("1:13:1", "p").unwrap();
    assert!(grid(&many, &feeds, &Grid::default()).is_err());
}

#[test]
fn the_command_takes_ranges_for_the_powers_and_the_feeds() {
    let dir = std::env::temp_dir().join(format!("eh3d-laser-matrix-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("a.nc");
    let b = dir.join("b.nc");
    let run = |out: &std::path::Path, p: &str, f: &str| {
        let r = Command::new(BIN)
            .args([
                "calibrate",
                &out.to_string_lossy(),
                "--powers",
                p,
                "--feeds",
                f,
            ])
            .output()
            .unwrap();
        assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
        std::fs::read_to_string(out).unwrap()
    };
    let (short, long) = (
        run(&a, "20:60:20", "600:1800:600"),
        run(&b, "20,40,60", "600,1200,1800"),
    );
    assert_eq!(short, long);
    // a range that is wrong is refused with the option named
    let r = Command::new(BIN)
        .args([
            "calibrate",
            &a.to_string_lossy(),
            "--powers",
            "60:20:5",
            "--feeds",
            "600",
        ])
        .output()
        .unwrap();
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("--powers"));
    let _ = std::fs::remove_dir_all(&dir);
}
