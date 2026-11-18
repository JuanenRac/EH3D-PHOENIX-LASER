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

#[test]
fn the_ordered_dithering_engraves_a_picture() {
    let out = tmp("bayer.nc");
    let (ok, text, err) = run(&[
        "engrave",
        GRADIENT,
        &out,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dither",
        "bayer",
    ]);
    assert!(ok, "{err}");
    assert!(text.contains("dots"), "{text}");
    let gcode = std::fs::read_to_string(&out).unwrap();
    assert!(gcode.contains("M4 S0") && gcode.trim_end().ends_with("M2"));
}

#[test]
fn a_method_of_dithering_that_does_not_exist_names_the_ones_that_do() {
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dry-run",
        "--dither",
        "dots",
    ]);
    assert!(!ok);
    assert!(err.contains("fs, atkinson, threshold or bayer"), "{err}");
}

#[test]
fn a_power_over_the_limit_of_power_is_refused_and_one_at_the_limit_is_not() {
    let args = |power: &str| {
        run(&[
            "engrave",
            GRADIENT,
            "--width-mm",
            "16",
            "--dry-run",
            "--power",
            power,
            "--max-power",
            "50",
        ])
    };
    let (ok, _, err) = args("60");
    assert!(!ok);
    assert!(
        err.contains("max_power_percent") && err.contains("--power"),
        "{err}"
    );
    let (ok, _, err) = args("50");
    assert!(ok, "{err}");
}

#[test]
fn the_grid_obeys_the_limit_of_power_too() {
    let out = tmp("limit-grid.nc");
    let (ok, _, err) = run(&[
        "calibrate",
        &out,
        "--powers",
        "20,80",
        "--feeds",
        "1000",
        "--max-power",
        "50",
    ]);
    assert!(!ok);
    assert!(err.contains("max_power_percent"), "{err}");
    let (ok, _, err) = run(&[
        "calibrate",
        &out,
        "--powers",
        "20,50",
        "--feeds",
        "1000",
        "--max-power",
        "50",
    ]);
    assert!(ok, "{err}");
}

#[test]
fn a_drawing_over_the_limit_of_power_is_refused_unless_the_power_is_lowered() {
    let base = [
        "cut",
        "examples/plate.svg",
        "--dry-run",
        "--config",
        "examples/plate.cfg",
        "--max-power",
        "50",
    ];
    let (ok, _, err) = run(&base);
    assert!(!ok);
    assert!(
        err.contains("max_power_percent") && err.contains("layer"),
        "{err}"
    );
    let mut lowered = base.to_vec();
    lowered.extend(["--power", "40"]);
    let (ok, _, err) = run(&lowered);
    assert!(ok, "{err}");
}

#[test]
fn the_limit_and_the_spindle_value_can_come_from_the_settings_file() {
    let cfg = tmp("limit.cfg");
    std::fs::write(
        &cfg,
        "[machine]\nmax_power_percent = 30\ns_max = 255\n[defaults]\npower = 20\n",
    )
    .unwrap();
    let base = [
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--config",
        cfg.as_str(),
    ];
    let mut over = base.to_vec();
    over.extend(["--dry-run", "--power", "40"]);
    let (ok, _, err) = run(&over);
    assert!(!ok);
    assert!(err.contains("max_power_percent"), "{err}");
    // 20 % of 255 is 51
    let out = tmp("limit.nc");
    let mut at = base.to_vec();
    at.extend([out.as_str(), "--power", "20"]);
    let (ok, _, err) = run(&at);
    assert!(ok, "{err}");
    assert!(std::fs::read_to_string(&out).unwrap().contains("S51"));
    // the option wins over the file
    let mut wins = base.to_vec();
    wins.extend(["--dry-run", "--power", "40", "--max-power", "60"]);
    let (ok, _, err) = run(&wins);
    assert!(ok, "{err}");
}

#[test]
fn the_output_is_never_written_over_the_settings_file() {
    let cfg = tmp("same.cfg");
    std::fs::write(&cfg, "[machine]\ns_max = 1000\n").unwrap();
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        &cfg,
        "--width-mm",
        "16",
        "--config",
        &cfg,
    ]);
    assert!(!ok);
    assert!(err.contains("settings"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&cfg).unwrap(),
        "[machine]\ns_max = 1000\n"
    );
}

const MATERIALS: &str = "examples/materials.cfg";

#[test]
fn the_library_of_the_examples_is_listed() {
    let (ok, out, err) = run(&["materials", MATERIALS]);
    assert!(ok, "{err}");
    for name in [
        "birch 3 mm cut",
        "birch 3 mm engrave",
        "birch photo",
        "slate photo",
    ] {
        assert!(out.contains(name), "{out}");
    }
    assert!(out.contains("4 material(s)"), "{out}");
}

#[test]
fn a_material_gives_the_same_engraving_as_its_values_typed_in() {
    let (ok, with_material, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dry-run",
        "--materials",
        MATERIALS,
        "--material",
        "Birch Photo",
    ]);
    assert!(ok, "{err}");
    let (ok, typed, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dry-run",
        "--power",
        "30",
        "--feed",
        "3000",
        "--dot-mm",
        "0.1",
        "--overscan",
        "2",
        "--dither",
        "atkinson",
        "--gamma",
        "1.4",
        "--contrast",
        "10",
        "--auto-level",
    ]);
    assert!(ok, "{err}");
    assert_eq!(with_material, typed);
}

#[test]
fn an_option_of_the_command_line_wins_over_the_material() {
    let (ok, from_option, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dry-run",
        "--materials",
        MATERIALS,
        "--material",
        "slate photo",
        "--dither",
        "fs",
        "--dot-mm",
        "0.5",
    ]);
    assert!(ok, "{err}");
    let (ok, plain, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dry-run",
        "--power",
        "45",
        "--feed",
        "2000",
        "--dot-mm",
        "0.5",
        "--brightness",
        "5",
    ]);
    assert!(ok, "{err}");
    assert_eq!(from_option, plain);
}

#[test]
fn a_material_sets_the_layers_of_a_drawing_and_the_limit_of_power_holds_over_it() {
    let out = tmp("material-cut.nc");
    let (ok, _, err) = run(&[
        "cut",
        "examples/plate.svg",
        &out,
        "--materials",
        MATERIALS,
        "--material",
        "birch 3 mm cut",
    ]);
    assert!(ok, "{err}");
    // 90 % of 1000
    assert!(std::fs::read_to_string(&out).unwrap().contains("S900"));
    let (ok, _, err) = run(&[
        "cut",
        "examples/plate.svg",
        "--dry-run",
        "--materials",
        MATERIALS,
        "--material",
        "birch 3 mm cut",
        "--max-power",
        "50",
    ]);
    assert!(!ok);
    assert!(err.contains("max_power_percent"), "{err}");
}

#[test]
fn a_material_that_does_not_exist_names_the_ones_that_do() {
    let (ok, _, err) = run(&[
        "layers",
        "examples/plate.svg",
        "--materials",
        MATERIALS,
        "--material",
        "granite",
    ]);
    assert!(!ok);
    assert!(
        err.contains("granite") && err.contains("birch photo"),
        "{err}"
    );
}

#[test]
fn a_material_needs_its_library_and_a_library_needs_its_material() {
    let (ok, _, err) = run(&["layers", "examples/plate.svg", "--materials", MATERIALS]);
    assert!(!ok);
    assert!(err.contains("--material NAME"), "{err}");
    let (ok, _, err) = run(&["layers", "examples/plate.svg", "--material", "birch photo"]);
    assert!(!ok);
    assert!(err.contains("--materials"), "{err}");
}

#[test]
fn a_broken_library_is_an_error_that_names_the_line_and_the_output_never_lands_on_it() {
    let lib = tmp("broken.cfg");
    std::fs::write(&lib, "[material a]\npower = 500\n").unwrap();
    let (ok, _, err) = run(&["materials", &lib]);
    assert!(!ok);
    assert!(err.contains("line 2") && err.contains("power"), "{err}");
    std::fs::write(&lib, "[material a]\npower = 20\nspeed = 1000\n").unwrap();
    let (ok, _, err) = run(&[
        "cut",
        "examples/plate.svg",
        &lib,
        "--materials",
        &lib,
        "--material",
        "a",
    ]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    assert!(std::fs::read_to_string(&lib)
        .unwrap()
        .starts_with("[material a]"));
}
