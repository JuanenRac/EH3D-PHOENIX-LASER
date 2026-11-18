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

/// Writes an engraving of the gradient and gives its path.
fn engraved(name: &str) -> String {
    let out = tmp(name);
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        &out,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
    ]);
    assert!(ok, "{err}");
    out
}

#[test]
fn the_preview_of_an_engraving_counts_what_the_engraving_counted() {
    let nc = engraved("preview.nc");
    let (ok, dry, err) = run(&[
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dry-run",
    ]);
    assert!(ok, "{err}");
    let (ok, out, err) = run(&["preview", &nc]);
    assert!(ok, "{err}");
    // "... , N mm burned, about T s" in the engraving; the same burnt length in the preview
    let burned = |text: &str| -> String {
        let end = text.find(" mm burned").unwrap();
        text[..end].rsplit(' ').next().unwrap().to_string()
    };
    assert_eq!(burned(&dry), burned(&out), "{dry} / {out}");
    assert!(err.is_empty(), "{err}");
}

#[test]
fn the_preview_draws_the_path_as_an_svg_and_never_over_its_program() {
    let nc = engraved("draw.nc");
    let svg = tmp("draw.svg");
    let (ok, out, err) = run(&["preview", &nc, &svg, "--bed", "40,40"]);
    assert!(ok, "{err}");
    assert!(out.contains("picture written"), "{out}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.starts_with("<svg ") && text.trim_end().ends_with("</svg>"));
    assert!(text.contains("<path"), "{text}");
    let before = std::fs::read_to_string(&nc).unwrap();
    let (ok, _, err) = run(&["preview", &nc, &nc]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    assert_eq!(std::fs::read_to_string(&nc).unwrap(), before);
}

#[test]
fn the_preview_tells_what_looks_dangerous() {
    let nc = tmp("danger.nc");
    std::fs::write(&nc, "G21\nG90\nM4 S1500\nG0 X10 Y10\nG1 X300 Y10\n").unwrap();
    let (ok, _, err) = run(&["preview", &nc, "--smax", "1000", "--bed", "100,100"]);
    assert!(ok, "{err}");
    for word in [
        "rapid move",
        "no feed",
        "above the full power",
        "outside of the bed",
        "does not end",
    ] {
        assert!(err.contains(word), "{word} in {err}");
    }
}

#[test]
fn a_program_without_a_move_and_rubbish_are_errors_not_pictures() {
    let nc = tmp("empty.nc");
    std::fs::write(&nc, "G21\nM5\nM2\n").unwrap();
    let (ok, _, err) = run(&["preview", &nc]);
    assert!(!ok);
    assert!(err.contains("no move"), "{err}");
    let (ok, _, err) = run(&["preview", &tmp("missing.nc")]);
    assert!(!ok);
    assert!(!err.is_empty());
    let (ok, _, err) = run(&["preview", &nc, "--accel", "0"]);
    assert!(!ok);
    assert!(err.contains("accel"), "{err}");
    let (ok, _, err) = run(&["preview", &nc, "--bed", "5"]);
    assert!(!ok);
    assert!(err.contains("--bed needs X,Y"), "{err}");
}

fn written(path: &str) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_frame_of_a_size_traces_the_rectangle_with_the_laser_off() {
    let nc = tmp("frame-size.nc");
    let (ok, out, err) = run(&[
        "frame", &nc, "--size", "40,20", "--origin", "5,10", "--margin", "1",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("42.0 x 22.0 mm at 4.0,9.0") && out.contains("laser off"),
        "{out}"
    );
    let text = written(&nc);
    assert!(text.contains("G0 X4 Y9"), "{text}");
    assert!(text.contains("G1 X46 Y31"), "{text}");
    assert!(!text.contains("M3") && !text.contains("M4"), "{text}");
    assert!(text.contains("laser OFF"), "{text}");
}

#[test]
fn the_frame_of_a_program_is_where_the_work_burns() {
    let work = engraved("frame-work.nc");
    let nc = tmp("frame-of-work.nc");
    let (ok, out, err) = run(&["frame", &nc, "--gcode", &work]);
    assert!(ok, "{err}");
    // the gradient burns 15 x 7.5 mm from the corner, not the run past the work
    assert!(out.contains("15.0 x 7.5 mm at 0.0,0.0"), "{out}");
    let (ok, _, err) = run(&["frame", &nc, "--gcode", &work, "--origin", "1,1"]);
    assert!(!ok);
    assert!(err.contains("--origin"), "{err}");
}

#[test]
fn a_pilot_power_needs_the_confirmation_and_stays_small() {
    let nc = tmp("frame-pilot.nc");
    let base = ["frame", nc.as_str(), "--size", "10,10"];
    let mut ask = base.to_vec();
    ask.extend(["--pilot", "1"]);
    let (ok, _, err) = run(&ask);
    assert!(!ok);
    assert!(err.contains("--confirm-laser-on"), "{err}");
    let mut sure = ask.clone();
    sure.push("--confirm-laser-on");
    let (ok, out, err) = run(&sure);
    assert!(ok, "{err}");
    assert!(out.contains("laser is ON"), "{out}");
    let text = written(&nc);
    assert!(
        text.contains("M3 S10\n") && text.contains("M5\nM2\n"),
        "{text}"
    );
    for (pilot, limit) in [("6", "100"), ("3", "2")] {
        let (ok, _, err) = run(&[
            "frame",
            &nc,
            "--size",
            "10,10",
            "--pilot",
            pilot,
            "--confirm-laser-on",
            "--max-power",
            limit,
        ]);
        assert!(!ok, "{pilot} {limit}");
        assert!(err.contains("power"), "{err}");
    }
}

#[test]
fn the_frame_takes_its_values_from_the_settings_file_and_the_bed_limits_it() {
    let cfg = tmp("frame.cfg");
    std::fs::write(
        &cfg,
        "[machine]\nbed_x_mm = 100\nbed_y_mm = 50\n[frame]\nfeed_mm_min = 1200\nlaps = 2\nmargin_mm = 2\n",
    )
    .unwrap();
    let nc = tmp("frame-cfg.nc");
    let (ok, out, err) = run(&[
        "frame", &nc, "--size", "20,10", "--origin", "10,10", "--config", &cfg,
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("2 lap(s)") && out.contains("24.0 x 14.0 mm"),
        "{out}"
    );
    assert!(written(&nc).contains("F1200"));
    // the same frame on a bed too small for it
    let (ok, _, err) = run(&[
        "frame", &nc, "--size", "20,10", "--origin", "90,10", "--config", &cfg,
    ]);
    assert!(!ok);
    assert!(err.contains("outside of the bed"), "{err}");
    // a pilot power in the file still asks for the confirmation on the command line
    std::fs::write(&cfg, "[frame]\npilot_percent = 1\n").unwrap();
    let (ok, _, err) = run(&["frame", &nc, "--size", "20,10", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("--confirm-laser-on"), "{err}");
}

#[test]
fn the_frame_needs_one_source_and_never_lands_on_what_it_is_made_from() {
    let nc = tmp("frame-bad.nc");
    for args in [
        vec!["frame", nc.as_str()],
        vec!["frame", nc.as_str(), "--size", "10"],
        vec!["frame", nc.as_str(), "--size", "0,5"],
        vec![
            "frame",
            nc.as_str(),
            "--size",
            "10,10",
            "--gcode",
            nc.as_str(),
        ],
        vec!["frame", nc.as_str(), "--size", "10,10", "--laps", "0"],
        vec!["frame", nc.as_str(), "--size", "10,10", "--feed", "0"],
    ] {
        let (ok, _, err) = run(&args);
        assert!(!ok, "{args:?}");
        assert!(!err.is_empty());
    }
    let work = engraved("frame-same.nc");
    let before = written(&work);
    let (ok, _, err) = run(&["frame", &work, "--gcode", &work]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    assert_eq!(written(&work), before);
}
