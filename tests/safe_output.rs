//! `engrave` and `calibrate` write like `cut` does: never over their own input, a whole `--smax`, and no half file.

use std::process::Command;

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn engrave_never_writes_over_the_image_it_is_made_from() {
    let copy = format!("{}/safe-output-image.pgm", env!("CARGO_TARGET_TMPDIR"));
    let source = format!("{}/examples/gradient.pgm", env!("CARGO_MANIFEST_DIR"));
    std::fs::copy(&source, &copy).unwrap();
    let before = std::fs::read(&copy).unwrap();
    let (ok, err) = run(&[
        "engrave",
        &copy,
        &copy,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
    ]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    assert_eq!(std::fs::read(&copy).unwrap(), before);
    std::fs::remove_file(&copy).unwrap();
}

#[test]
fn smax_takes_a_whole_number_in_engrave_and_in_calibrate() {
    let out = format!("{}/safe-output-smax.nc", env!("CARGO_TARGET_TMPDIR"));
    let source = format!("{}/examples/gradient.pgm", env!("CARGO_MANIFEST_DIR"));
    let (ok, err) = run(&[
        "engrave",
        &source,
        &out,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--smax",
        "1000.7",
    ]);
    assert!(!ok);
    assert!(err.contains("--smax"), "{err}");
    let (ok, err) = run(&[
        "calibrate",
        &out,
        "--powers",
        "10,20",
        "--feeds",
        "1000,2000",
        "--smax",
        "255.5",
    ]);
    assert!(!ok);
    assert!(err.contains("--smax"), "{err}");
    assert!(!std::path::Path::new(&out).exists());
}

#[test]
fn calibrate_writes_a_complete_program() {
    let out = format!("{}/safe-output-grid.nc", env!("CARGO_TARGET_TMPDIR"));
    let (ok, err) = run(&[
        "calibrate",
        &out,
        "--powers",
        "10,20",
        "--feeds",
        "1000,2000",
    ]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.trim_end().ends_with("M2"), "{text}");
    std::fs::remove_file(&out).unwrap();
}
