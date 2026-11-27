//! No program is written that the check of the program would fail: the commands and the window are tried with a bed, a forbidden zone and broken programs. Nothing is run on a machine.

use eh3d_phoenix_laser::gate;
use eh3d_phoenix_laser::lang::{Lang, Msg};
use eh3d_phoenix_laser::window::{Key, Mode, Status, Window};
use std::path::Path;
use std::process::Command;

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

fn path(name: &str) -> String {
    format!("{}/gate-{name}", env!("CARGO_TARGET_TMPDIR"))
}

fn file(name: &str, text: &str) -> String {
    let path = path(name);
    std::fs::write(&path, text).unwrap();
    path
}

const ZONE: &str =
    "[zone clamp]\nx_mm = 0\ny_mm = 0\nwidth_mm = 50\nheight_mm = 50\nkind = no_burn\n";

/// A command that writes a program, run with a settings file: whether it succeeded, what it said and whether the file is there.
fn written(name: &str, config: &str, args: &[&str]) -> (bool, String, bool) {
    let cfg = file(&format!("{name}.cfg"), config);
    let out = path(&format!("{name}.nc"));
    let _ = std::fs::remove_file(&out);
    let mut all = args.to_vec();
    all.push(&out);
    all.extend(["--config", &cfg]);
    let (ok, _, err) = run(&all);
    (ok, err, Path::new(&out).exists())
}

#[test]
fn a_program_that_burns_in_a_forbidden_zone_is_not_written() {
    let commands: [&[&str]; 4] = [
        &["text", "--text", "A", "--power", "10", "--feed", "1000"],
        &["qr", "--text", "A", "--power", "10", "--feed", "1000"],
        &["barcode", "--text", "A", "--power", "10", "--feed", "1000"],
        &["calibrate", "--powers", "10", "--feeds", "1000"],
    ];
    for (i, args) in commands.iter().enumerate() {
        let (ok, err, there) = written(&format!("zone{i}"), ZONE, args);
        assert!(!ok && !there, "{args:?}: {err}");
        assert!(
            err.contains("was not written") && err.contains("forbidden zone clamp"),
            "{args:?}: {err}"
        );
        let (ok, err, there) = written(&format!("free{i}"), "", args);
        assert!(ok && there, "{args:?}: {err}");
    }
}

#[test]
fn the_grid_and_the_engraving_keep_the_bed_of_the_settings() {
    let bed = "[machine]\nbed_x_mm = 5\nbed_y_mm = 5\n";
    let (ok, err, there) = written(
        "bedgrid",
        bed,
        &["calibrate", "--powers", "10,20", "--feeds", "1000"],
    );
    assert!(!ok && !there && err.contains("outside of the bed"), "{err}");
    let picture = file("dots.pgm", "P2\n4 1\n255\n0 0 0 0\n");
    let args = [
        "engrave",
        picture.as_str(),
        "--width-mm",
        "20",
        "--power",
        "10",
        "--feed",
        "1000",
        "--overscan",
        "0",
    ];
    let (ok, err, there) = written("bedpic", bed, &args);
    assert!(!ok && !there && err.contains("outside of the bed"), "{err}");
    let (ok, err, there) = written("freepic", "", &args);
    assert!(ok && there, "{err}");
}

#[test]
fn a_series_writes_nothing_when_one_row_fails_the_check() {
    // the first text fits left of the zone, the longer second one reaches it
    let zone = "[zone far]\nx_mm = 20\ny_mm = 0\nwidth_mm = 100\nheight_mm = 50\nkind = no_burn\n";
    let csv = file("rows.csv", "name\nI\nIIIIIIIIII\n");
    let (ok, err, _) = written(
        "series",
        zone,
        &[
            "text",
            "--text",
            "{name}",
            "--csv",
            &csv,
            "--power",
            "10",
            "--feed",
            "1000",
            "--height-mm",
            "8",
        ],
    );
    assert!(!ok && err.contains("row 2"), "{err}");
    assert!(!Path::new(&path("series_001.nc")).exists());
}

#[test]
fn a_pressed_stop_does_not_stop_a_file_from_being_written() {
    let (ok, err, there) = written(
        "stopped",
        "[interlocks]\nestop = pressed\n",
        &["text", "--text", "A", "--power", "10", "--feed", "1000"],
    );
    assert!(ok && there, "{err}");
}

#[test]
fn no_frame_is_made_of_a_program_that_fails_the_check() {
    let bad = file(
        "bad.nc",
        "G21\nG90\nF600\nM4 S100\n(a (b)) G1 X10\nM5\nM2\n",
    );
    let out = path("frame.nc");
    let _ = std::fs::remove_file(&out);
    let (ok, _, err) = run(&["frame", "--gcode", &bad, &out]);
    assert!(!ok && err.contains("no frame is made"), "{err}");
    assert!(!Path::new(&out).exists());
}

#[test]
fn the_gate_says_every_reason() {
    let limits = gate::limits(1000, 20.0, 500.0, Some((10.0, 10.0)), 0.0);
    let e =
        gate::before_write("G21\nG90\nF600\nM4 S500\nG1 X20\nM5\nM2\n", &limits, &[]).unwrap_err();
    for words in ["limit of power", "limit of speed", "outside of the bed"] {
        assert!(e.contains(words), "{e}");
    }
}

#[test]
fn the_window_does_not_export_a_program_that_burns_in_a_forbidden_zone() {
    let cfg = file("window.cfg", ZONE);
    let out = path("window.nc");
    let _ = std::fs::remove_file(&out);
    let mut w = Window::new(Lang::En);
    assert!(w.load_config(Path::new(&cfg)));
    w.set_mode(Mode::Calibrate);
    assert!(!w.export(Path::new(&out)));
    assert!(
        matches!(&w.status, Status::Failed(e) if e.contains("forbidden zone clamp")),
        "{:?}",
        w.status.text(Lang::En)
    );
    assert!(!Path::new(&out).exists());
    // a bad value in the box of the check does not let the program be written without it
    let mut w = Window::new(Lang::En);
    w.set_mode(Mode::Calibrate);
    let i = w
        .fields
        .iter()
        .position(|f| f.key == Key::VerifyOverscan)
        .unwrap();
    w.set_field(i, "x".to_string());
    assert!(!w.export(Path::new(&out)));
    assert_eq!(w.status, Status::BadValue(Msg::VerifyOverscan));
    assert!(!Path::new(&out).exists());
}
