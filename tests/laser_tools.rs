//! The tools around the engraving, through the command line: the tone curve, the ordered dithering, the library of materials, the limit of power, the frame, the preview and the focus test.

use std::process::Command;

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A path in the temporary folder of the tests, with the name of the test in it so that two tests never share a file.
fn tmp(name: &str) -> String {
    format!("{}/laser-tools-{name}", env!("CARGO_TARGET_TMPDIR"))
}

const GRADIENT: &str = "examples/gradient.pgm";

#[test]
fn a_brightness_of_100_leaves_nothing_to_burn() {
    let (ok, out, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dither",
        "threshold",
        "--brightness",
        "100",
        "--dry-run",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("0 lines"), "{out}");
}

#[test]
fn inverting_the_picture_changes_what_burns() {
    let (plain, flipped) = (tmp("plain.nc"), tmp("flipped.nc"));
    let base = [
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dither",
        "threshold",
    ];
    let mut a = base.to_vec();
    a.push(&plain);
    let (ok, _, err) = run(&a);
    assert!(ok, "{err}");
    let mut b = base.to_vec();
    b.extend([flipped.as_str(), "--invert"]);
    let (ok, _, err) = run(&b);
    assert!(ok, "{err}");
    assert_ne!(
        std::fs::read_to_string(&plain).unwrap(),
        std::fs::read_to_string(&flipped).unwrap()
    );
}

#[test]
fn a_tone_value_out_of_range_is_an_error_that_names_it() {
    for (option, value) in [("--gamma", "0"), ("--gamma", "11"), ("--contrast", "101")] {
        let (ok, _, err) = run(&[
            "engrave",
            GRADIENT,
            "--width-mm",
            "16",
            "--dry-run",
            option,
            value,
        ]);
        assert!(!ok, "{option} {value}");
        assert!(err.contains(&option[2..]), "{err}");
    }
}

#[test]
fn the_tone_options_do_not_change_the_picture_when_they_are_left_out() {
    let args = [
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dry-run",
    ];
    let (_, plain, _) = run(&args);
    let mut neutral = args.to_vec();
    neutral.extend(["--gamma", "1", "--contrast", "0", "--brightness", "0"]);
    let (ok, same, err) = run(&neutral);
    assert!(ok, "{err}");
    assert_eq!(plain, same);
}
