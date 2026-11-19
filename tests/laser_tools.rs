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
fn the_tone_curve_can_come_from_the_settings_file_and_the_options_win_over_it() {
    let cfg = tmp("tone.cfg");
    std::fs::write(&cfg, "[tone]\nbrightness = 100\n").unwrap();
    let base = [
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dither",
        "threshold",
        "--dry-run",
        "--config",
        cfg.as_str(),
    ];
    // a brightness of 100 in the file leaves nothing to burn
    let (ok, out, err) = run(&base);
    assert!(ok, "{err}");
    assert!(out.contains("0 lines"), "{out}");
    // the option wins over the file
    let mut over = base.to_vec();
    over.extend(["--brightness", "0"]);
    let (ok, out, err) = run(&over);
    assert!(ok, "{err}");
    assert!(!out.contains(" 0 lines"), "{out}");
    // a tone that is out of range in the file is an error that names the file
    std::fs::write(&cfg, "[tone]\ngamma = 0\n").unwrap();
    let (ok, _, err) = run(&base);
    assert!(!ok);
    assert!(err.contains("gamma"), "{err}");
}

const GOOD_PROGRAM: &str = "G21\nG90\nF1000\nG0 X0 Y0\nM3 S200\nG1 X10 Y0\nM5\nM2\n";

#[test]
fn verify_gives_the_hash_of_the_file_and_a_verdict_and_the_exit_code_follows_it() {
    let nc = tmp("verify-good.nc");
    std::fs::write(&nc, GOOD_PROGRAM).unwrap();
    let (ok, out, err) = run(&["verify", &nc]);
    assert!(ok, "{err}");
    assert!(out.contains("verdict: pass"), "{out}");
    // the same hash as the one the program writes in its own headers
    let hash = out
        .lines()
        .find_map(|l| l.strip_prefix("toolpath sha256: "))
        .unwrap()
        .to_string();
    assert_eq!(hash.len(), 64);
    assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
    // the JSON form
    let (ok, out, err) = run(&["verify", &nc, "--json"]);
    assert!(ok, "{err}");
    assert!(
        out.contains(&format!("\"toolpath_sha256\": \"{hash}\"")),
        "{out}"
    );
    assert!(out.contains("\"verdict\": \"pass\""), "{out}");
    // a limit that the program is over fails it, with the problems on the standard output and the exit code 1
    let (ok, out, err) = run(&["verify", &nc, "--max-power", "10"]);
    assert!(!ok);
    assert!(
        out.contains("problem:") && out.contains("verdict: FAIL"),
        "{out}"
    );
    assert!(err.contains("fails 1 check"), "{err}");
    let (ok, out, _) = run(&["verify", &nc, "--max-speed", "500", "--json"]);
    assert!(!ok);
    assert!(
        out.contains("\"verdict\": \"fail\"") && out.contains("limit of speed"),
        "{out}"
    );
    let (ok, _, _) = run(&["verify", &nc, "--bed", "5,5"]);
    assert!(!ok);
    let (ok, _, _) = run(&["verify", &nc, "--smax", "100"]);
    assert!(!ok);
    // the limits can come from a profile of the settings file
    let cfg = profiles_file(
        "verify-profile.cfg",
        "[defaults]\npower = 5\nspeed = 1000\n",
    );
    let (ok, _, err) = run(&["verify", &nc, "--config", &cfg, "--profile", "small diode"]);
    assert!(!ok, "S200 of S255 is more than 5 %");
    assert!(err.contains("fails"), "{err}");
    let (ok, out, err) = run(&[
        "verify",
        &nc,
        "--config",
        &cfg,
        "--profile",
        "big",
        "--bed",
        "400,300",
    ]);
    assert!(ok && out.contains("verdict: pass"), "{out} {err}");
}

#[test]
fn the_json_of_verify_says_the_profile_and_the_hash_of_the_settings_it_used() {
    let nc = tmp("verify-record.nc");
    std::fs::write(&nc, GOOD_PROGRAM).unwrap();
    // with no settings the machine of the record is empty and the checks are named
    let (ok, out, err) = run(&["verify", &nc, "--json"]);
    assert!(ok, "{err}");
    assert!(
        out.contains("\"machine\": {\"profile\": null, \"config_sha256\": null}"),
        "{out}"
    );
    assert!(out.contains("\"validations\": [\"has_moves\""), "{out}");
    // with a profile of a settings file, the record has its name and the SHA-256 of the file
    let cfg = profiles_file("verify-record.cfg", "[defaults]\npower = 5\n");
    let hash = eh3d_phoenix_laser::gcode_out::sha256_hex(&std::fs::read(&cfg).unwrap());
    let (ok, out, err) = run(&[
        "verify",
        &nc,
        "--config",
        &cfg,
        "--profile",
        "BIG",
        "--json",
    ]);
    assert!(ok, "{out} {err}");
    assert!(
        out.contains(&format!(
            "\"machine\": {{\"profile\": \"big\", \"config_sha256\": \"{hash}\"}}"
        )),
        "{out}"
    );
    // a change to the file changes the hash of the record
    std::fs::write(
        &cfg,
        format!(
            "[defaults]\npower = 6\n{}",
            "[profile big]\nmax_power_percent = 70\n"
        ),
    )
    .unwrap();
    let (ok, other, err) = run(&[
        "verify",
        &nc,
        "--config",
        &cfg,
        "--profile",
        "big",
        "--json",
    ]);
    assert!(ok, "{other} {err}");
    assert!(!other.contains(&hash), "{other}");
    // the text form has no record to keep
    let (ok, text, _) = run(&["verify", &nc, "--config", &cfg]);
    assert!(ok && !text.contains("config_sha256"));
}

#[test]
fn verify_refuses_what_it_cannot_read_and_what_is_not_a_program() {
    let (ok, _, err) = run(&["verify"]);
    assert!(!ok);
    assert!(err.contains("G-code file"), "{err}");
    let (ok, _, err) = run(&["verify", "/no/such/folder/a.nc"]);
    assert!(!ok);
    assert!(err.contains("a.nc"), "{err}");
    let junk = tmp("verify-junk.nc");
    std::fs::write(&junk, "this is not a program").unwrap();
    let (ok, out, _) = run(&["verify", &junk]);
    assert!(!ok);
    assert!(out.contains("no move in the plane"), "{out}");
    let (ok, _, err) = run(&["verify", &junk, "--max-power", "0"]);
    assert!(!ok);
    assert!(err.contains("max_power_percent"), "{err}");
    let (ok, _, err) = run(&["verify", &junk, "extra", "words"]);
    assert!(!ok);
    assert!(err.contains("unexpected"), "{err}");
}

#[test]
fn the_program_the_engraving_writes_passes_its_own_check() {
    let nc = tmp("verify-own.nc");
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        &nc,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--power",
        "40",
    ]);
    assert!(ok, "{err}");
    let (ok, out, err) = run(&["verify", &nc, "--max-power", "40"]);
    assert!(ok, "{out} {err}");
    let (ok, _, _) = run(&["verify", &nc, "--max-power", "39"]);
    assert!(!ok);
    // the same for a drawing that is cut
    let cut = tmp("verify-cut.nc");
    let (ok, _, err) = run(&[
        "cut",
        "examples/plate.svg",
        &cut,
        "--config",
        "examples/plate.cfg",
    ]);
    assert!(ok, "{err}");
    let (ok, out, err) = run(&["verify", &cut, "--config", "examples/plate.cfg"]);
    assert!(ok, "{out} {err}");
}

#[test]
fn verify_allows_the_overscan_of_another_program_only_as_far_as_it_is_told() {
    // the head runs 2 mm past a bed of 10 x 10 mm on both sides with the laser off, and burns inside
    let nc = tmp("verify-overscan.nc");
    std::fs::write(
        &nc,
        "G21\nG90\nF1000\nG0 X-2 Y5\nM3 S0\nG1 X0 Y5\nG1 X10 Y5 S300\nG1 X12 Y5 S0\nM5\nM2\n",
    )
    .unwrap();
    let (ok, out, _) = run(&["verify", &nc, "--smax", "1000", "--bed", "10,10"]);
    assert!(!ok);
    assert!(out.contains("goes outside of the bed"), "{out}");
    let (ok, out, err) = run(&[
        "verify",
        &nc,
        "--bed",
        "10,10",
        "--overscan-mm",
        "2",
        "--json",
    ]);
    assert!(ok, "{out} {err}");
    assert!(out.contains("\"overscan_mm\": 2}"), "{out}");
    let (ok, _, _) = run(&["verify", &nc, "--bed", "10,10", "--overscan-mm", "1.5"]);
    assert!(!ok);
    // the settings file says it too, and the option wins over it
    let cfg = tmp("verify-overscan.cfg");
    std::fs::write(
        &cfg,
        "[machine]\nbed_x_mm = 10\nbed_y_mm = 10\n[verify]\noverscan_mm = 2\n",
    )
    .unwrap();
    let (ok, out, err) = run(&["verify", &nc, "--config", &cfg]);
    assert!(ok, "{out} {err}");
    let (ok, _, _) = run(&["verify", &nc, "--config", &cfg, "--overscan-mm", "0"]);
    assert!(!ok);
    // burning outside the bed fails whatever the allowance
    let burns = tmp("verify-burns.nc");
    std::fs::write(
        &burns,
        "G21\nG90\nF1000\nG0 X0 Y5\nM3 S300\nG1 X11 Y5\nM5\nM2\n",
    )
    .unwrap();
    let (ok, out, _) = run(&["verify", &burns, "--bed", "10,10", "--overscan-mm", "100"]);
    assert!(!ok);
    assert!(out.contains("burns outside of the bed"), "{out}");
    // an allowance that is not possible is an error
    for bad in ["-1", "101", "x"] {
        let (ok, _, err) = run(&["verify", &nc, "--overscan-mm", bad]);
        assert!(!ok, "{bad}");
        assert!(!err.contains("panicked"), "{err}");
    }
}

#[test]
fn calibrate_spreads_the_powers_and_the_feeds_around_a_material_and_burns_the_figures() {
    let lib = tmp("calibrate-lib.cfg");
    std::fs::write(
        &lib,
        "[material test wood]\npower = 50\nspeed = 1000\n[material only power]\npower = 30\n",
    )
    .unwrap();
    let out_file = tmp("calibrate-material.nc");
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "test wood",
        "--materials",
        &lib,
    ]);
    assert!(ok, "{err}");
    // 5 powers from 30 to 70 % and 5 feeds from 600 to 1400 mm/min
    assert!(out.contains("25 squares written"), "{out}");
    assert!(
        out.contains("powers 30, 40, 50, 60, 70 %, feeds 600, 800, 1000, 1200, 1400 mm/min"),
        "{out}"
    );
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert!(
        text.contains("(figures at 50% feed 1000)"),
        "a grid made from a material has its figures by default"
    );
    // --no-labels takes them away
    let (ok, _, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "test wood",
        "--materials",
        &lib,
        "--no-labels",
    ]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert!(!text.contains("(figures"), "{text}");
    // a grid from lists, with no material, has none unless they are asked for
    let (ok, _, err) = run(&[
        "calibrate",
        &out_file,
        "--powers",
        "10,20",
        "--feeds",
        "1000",
    ]);
    assert!(ok, "{err}");
    assert!(!std::fs::read_to_string(&out_file)
        .unwrap()
        .contains("(figures"));
    // the figures, with the option, and the check of the program passes
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "test wood",
        "--materials",
        &lib,
        "--labels",
        "--steps",
        "3",
        "--spread",
        "20",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("9 squares written"), "{out}");
    assert!(out.contains("powers 40, 50, 60 %"), "{out}");
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert!(text.contains("(figures at 50% feed 1000)"), "{text}");
    assert!(text.contains("grid of power against feed with figures"));
    let (ok, out, err) = run(&["verify", &out_file]);
    assert!(ok, "{out} {err}");
    // the limits hold over the ladder: it is held at them and never goes over
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "test wood",
        "--materials",
        &lib,
        "--max-power",
        "55",
        "--max-speed",
        "1100",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("powers 30, 40, 50, 55 %, feeds 600, 800, 1000, 1100 mm/min"),
        "{out}"
    );
    // a material over the limit is refused, never lowered
    let (ok, _, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "test wood",
        "--materials",
        &lib,
        "--max-power",
        "40",
    ]);
    assert!(!ok);
    assert!(err.contains("the power of the material"), "{err}");
    // the lists that are given win over the material, and then the ladder options have nothing to do
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "test wood",
        "--materials",
        &lib,
        "--powers",
        "10,20",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("10 squares written"), "{out}");
    let (ok, _, err) = run(&[
        "calibrate",
        &out_file,
        "--powers",
        "10,20",
        "--feeds",
        "1000",
        "--steps",
        "3",
    ]);
    assert!(!ok);
    assert!(
        err.contains("--steps and --spread need --material"),
        "{err}"
    );
    // a material with no speed needs the feeds
    let (ok, _, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "only power",
        "--materials",
        &lib,
    ]);
    assert!(!ok);
    assert!(err.contains("has no speed"), "{err}");
    // the library is never written over
    let (ok, _, err) = run(&[
        "calibrate",
        &lib,
        "--material",
        "test wood",
        "--materials",
        &lib,
    ]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    let (ok, _, err) = run(&[
        "calibrate",
        &out_file,
        "--powers",
        "10",
        "--feeds",
        "1000",
        "--labels",
        "--no-labels",
    ]);
    assert!(!ok);
    assert!(err.contains("cannot be given together"), "{err}");
}

#[test]
fn the_figures_and_the_ladder_can_come_from_the_settings_file() {
    let lib = tmp("calibrate-cfg-lib.cfg");
    std::fs::write(&lib, "[material m]\npower = 50\nspeed = 1000\n").unwrap();
    let cfg = tmp("calibrate-cfg.cfg");
    std::fs::write(
        &cfg,
        "[calibrate]\nsteps = 3\nspread_percent = 20\nlabels = on\nlabel_mm = 4\nmin_power_percent = 5\nmin_feed_mm_min = 100\n",
    )
    .unwrap();
    let out_file = tmp("calibrate-cfg.nc");
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "m",
        "--materials",
        &lib,
        "--config",
        &cfg,
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("9 squares written"), "{out}");
    assert!(std::fs::read_to_string(&out_file)
        .unwrap()
        .contains("(figures at 50% feed 1000)"));
    // the options win over the file
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "m",
        "--materials",
        &lib,
        "--config",
        &cfg,
        "--no-labels",
        "--steps",
        "2",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("4 squares written"), "{out}");
    assert!(!std::fs::read_to_string(&out_file)
        .unwrap()
        .contains("(figures"));
    // a value out of range is an error that names it
    for (key, bad) in [
        ("steps", "13"),
        ("spread_percent", "91"),
        ("label_mm", "0.5"),
        ("min_power_percent", "0"),
        ("min_feed_mm_min", "30001"),
    ] {
        std::fs::write(&cfg, format!("[calibrate]\n{key} = {bad}\n")).unwrap();
        let (ok, _, err) = run(&[
            "calibrate",
            &out_file,
            "--powers",
            "10",
            "--feeds",
            "1000",
            "--config",
            &cfg,
        ]);
        assert!(!ok, "{key}");
        assert!(err.contains(key), "{err}");
    }
}

#[test]
fn the_lowest_power_and_feed_of_the_ladder_come_from_the_settings() {
    let lib = tmp("ladder-min-lib.cfg");
    std::fs::write(&lib, "[material m]\npower = 10\nspeed = 200\n").unwrap();
    let out_file = tmp("ladder-min.nc");
    // by default the ladder goes down to 1 % and 10 mm/min: 10 % with a spread of 90 % is 1 %, 200 with the same is 20
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "m",
        "--materials",
        &lib,
        "--steps",
        "3",
        "--spread",
        "90",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("powers 1, 10, 19 %, feeds 20, 200, 380 mm/min"),
        "{out}"
    );
    // the keys of the file hold the ladder higher
    let cfg = tmp("ladder-min.cfg");
    std::fs::write(
        &cfg,
        "[calibrate]\nmin_power_percent = 5\nmin_feed_mm_min = 100\n",
    )
    .unwrap();
    let (ok, out, err) = run(&[
        "calibrate",
        &out_file,
        "--material",
        "m",
        "--materials",
        &lib,
        "--steps",
        "3",
        "--spread",
        "90",
        "--config",
        &cfg,
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("powers 5, 10, 19 %, feeds 100, 200, 380 mm/min"),
        "{out}"
    );
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

const PROFILES: &str = "[profile small diode]\ntravel_x_mm = 40\ntravel_y_mm = 30\nmax_power_percent = 5\nmax_speed_mm_min = 1500\ns_max = 255\n\
    [profile big]\ntravel_x_mm = 400\ntravel_y_mm = 300\nmax_power_percent = 70\nmax_speed_mm_min = 9000\ns_max = 1000\nlaser_watts = 10\n";

/// A settings file with the two profiles of [`PROFILES`] and the name of the test in it.
fn profiles_file(name: &str, extra: &str) -> String {
    let cfg = tmp(name);
    std::fs::write(&cfg, format!("{extra}{PROFILES}")).unwrap();
    cfg
}

/// A grey picture of `size` x `size` pixels as a PGM file: a dark disc of radius `r` on white, with a white hole of radius `hole` in the middle (0 for none).
fn disc_picture(name: &str, size: usize, r: f64, hole: f64) -> String {
    let path = tmp(name);
    let c = size as f64 / 2.0;
    let mut text = format!("P2\n{size} {size}\n255\n");
    for y in 0..size {
        for x in 0..size {
            let d = (x as f64 - c).hypot(y as f64 - c);
            text.push_str(if d <= r && (hole == 0.0 || d > hole) {
                "0 "
            } else {
                "255 "
            });
        }
        text.push('\n');
    }
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn the_contours_of_a_picture_are_cut_like_the_shapes_of_a_drawing() {
    let pic = disc_picture("trace-ring.pgm", 60, 20.0, 8.0);
    let nc = tmp("trace-ring.nc");
    let (ok, out, err) = run(&[
        "trace",
        &pic,
        &nc,
        "--width-mm",
        "30",
        "--power",
        "60",
        "--speed",
        "300",
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("2 contours") && out.contains("60 x 60 pixels"),
        "{out}"
    );
    assert!(out.contains("1 layers, 2 paths"), "{out}");
    let text = written(&nc);
    assert!(
        text.contains("; layer trace: cut, 60 %, 300 mm/min"),
        "{text}"
    );
    assert!(
        text.contains("trace-ring.pgm") && text.contains("S600"),
        "{text}"
    );
    assert!(text.trim_end().ends_with("M5\nM2"), "{text}");
    // the work starts at the origin
    let (ok, out, err) = run(&["preview", &nc]);
    assert!(ok, "{err}");
    assert!(out.contains(" mm at 0.0,0.0"), "{out}");
}

#[test]
fn a_picture_can_be_engraved_by_its_shape_and_counted_with_a_dry_run() {
    let pic = disc_picture("trace-disc.pgm", 60, 20.0, 0.0);
    let nc = tmp("trace-disc.nc");
    let (ok, out, err) = run(&[
        "trace",
        &pic,
        &nc,
        "--width-mm",
        "30",
        "--engrave",
        "--spacing",
        "0.2",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("1 contours"), "{out}");
    let text = written(&nc);
    assert!(text.contains("; layer trace: engrave"), "{text}");
    // a dry run counts and writes nothing
    let nothing = tmp("trace-dry.nc");
    let _ = std::fs::remove_file(&nothing);
    let (ok, out, err) = run(&["trace", &pic, "--width-mm", "30", "--dry-run"]);
    assert!(ok, "{err}");
    assert!(out.contains("dry run, nothing written"), "{out}");
    assert!(!std::path::Path::new(&nothing).exists());
}

#[test]
fn the_trace_options_pick_what_is_followed_and_a_picture_with_nothing_to_follow_is_an_error() {
    let pic = disc_picture("trace-opts.pgm", 40, 10.0, 0.0);
    let ok_with = |extra: &[&str]| {
        let mut a = vec!["trace", pic.as_str(), "--width-mm", "20", "--dry-run"];
        a.extend(extra);
        run(&a)
    };
    assert!(ok_with(&[]).0);
    // nothing is ink under a threshold of 0
    let (ok, _, err) = ok_with(&["--threshold", "0"]);
    assert!(!ok);
    assert!(err.contains("no contour"), "{err}");
    // the dark disc is the paper when inverted, and then the picture itself is the ink: a contour round the whole picture and the hole of the disc
    let (ok, out, err) = ok_with(&["--invert"]);
    assert!(ok, "{err}");
    assert!(out.contains("2 contours"), "{out}");
    // a speck limit above the size of the disc drops everything
    let (ok, _, err) = ok_with(&["--min-area", "100000"]);
    assert!(!ok);
    assert!(err.contains("no contour"), "{err}");
    for (bad, word) in [
        (["--threshold", "256"], "--threshold"),
        (["--threshold", "1.5"], "--threshold"),
        (["--simplify", "-1"], "simplify_mm"),
        (["--simplify", "9"], "simplify_mm"),
    ] {
        let (ok, _, err) = ok_with(&bad);
        assert!(!ok, "{bad:?}");
        assert!(err.contains(word), "{err}");
    }
    // the width is needed, and must be possible
    let (ok, _, err) = run(&["trace", &pic, "--dry-run"]);
    assert!(!ok);
    assert!(err.contains("--width-mm"), "{err}");
    let (ok, _, err) = run(&["trace", &pic, "--width-mm", "0", "--dry-run"]);
    assert!(!ok);
    assert!(err.contains("width"), "{err}");
}

#[test]
fn the_limits_of_the_trace_come_from_the_settings_file_and_the_output_never_lands_on_its_sources() {
    let pic = disc_picture("trace-limits.pgm", 40, 10.0, 0.0);
    let cfg = tmp("trace-limits.cfg");
    std::fs::write(&cfg, "[trace]\nmax_segments = 10\n").unwrap();
    let (ok, _, err) = run(&[
        "trace",
        &pic,
        "--width-mm",
        "20",
        "--dry-run",
        "--config",
        &cfg,
    ]);
    assert!(!ok);
    assert!(err.contains("max_segments"), "{err}");
    std::fs::write(&cfg, "[trace]\nthreshold = 0\n").unwrap();
    let (ok, _, err) = run(&[
        "trace",
        &pic,
        "--width-mm",
        "20",
        "--dry-run",
        "--config",
        &cfg,
    ]);
    assert!(!ok);
    assert!(err.contains("no contour"), "{err}");
    // the output is never the picture or the settings
    let (ok, _, err) = run(&["trace", &pic, &pic, "--width-mm", "20"]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    std::fs::write(&cfg, "[trace]\nthreshold = 128\n").unwrap();
    let (ok, _, err) = run(&["trace", &pic, &cfg, "--width-mm", "20", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    // a file that is not a picture is an error that names it
    let junk = tmp("trace-junk.pgm");
    std::fs::write(&junk, "not a picture").unwrap();
    let (ok, _, err) = run(&["trace", &junk, "--width-mm", "20", "--dry-run"]);
    assert!(!ok);
    assert!(err.contains("trace-junk.pgm"), "{err}");
}

#[test]
fn a_profile_is_chosen_with_the_option_and_its_limits_hold_over_the_work() {
    let cfg = profiles_file("profiles.cfg", "");
    let drawing = [
        "cut",
        "examples/plate.svg",
        "--dry-run",
        "--config",
        cfg.as_str(),
    ];
    // no profile chosen: the default power of the layers (50 %) is allowed
    let (ok, _, err) = run(&drawing);
    assert!(ok, "{err}");
    // the small diode allows 5 % only, and the default layers ask for 50 %
    let mut small = drawing.to_vec();
    small.extend(["--profile", "Small Diode"]);
    let (ok, _, err) = run(&small);
    assert!(!ok);
    assert!(
        err.contains("max_power_percent") && err.contains("[defaults]"),
        "{err}"
    );
    // the big one allows 70 %
    let mut big = drawing.to_vec();
    big.extend(["--profile", "big"]);
    let (ok, _, err) = run(&big);
    assert!(ok, "{err}");
    // a profile that is not in the file is an error that lists the ones there are
    let mut none = drawing.to_vec();
    none.extend(["--profile", "plasma"]);
    let (ok, _, err) = run(&none);
    assert!(!ok);
    assert!(
        err.contains("plasma") && err.contains("small diode, big"),
        "{err}"
    );
    // the profiles live in the settings file
    let (ok, _, err) = run(&["cut", "examples/plate.svg", "--dry-run", "--profile", "big"]);
    assert!(!ok);
    assert!(err.contains("--config"), "{err}");
}

#[test]
fn a_profile_chosen_in_the_file_is_the_machine_and_the_option_still_wins_over_it() {
    let cfg = profiles_file(
        "profile-in-file.cfg",
        "[machine]\nprofile = small diode\n[defaults]\npower = 5\nspeed = 1000\n",
    );
    let nc = tmp("profile-s.nc");
    // the S of the profile is the full power of the program: 255, not the 1000 of the default
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        &nc,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--power",
        "5",
        "--feed",
        "1500",
        "--config",
        &cfg,
    ]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&nc).unwrap();
    assert!(text.contains("S13") && !text.contains("S50"), "{text}");
    assert!(text.contains("S 0-255"), "{text}");
    // the big profile chosen on the command line replaces it (S1000, up to 70 %)
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        &nc,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--power",
        "60",
        "--config",
        &cfg,
        "--profile",
        "big",
    ]);
    assert!(ok, "{err}");
    assert!(std::fs::read_to_string(&nc).unwrap().contains("S600"));
    // the option of the command line wins over the profile
    let (ok, _, err) = run(&[
        "engrave",
        GRADIENT,
        &nc,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--power",
        "5",
        "--feed",
        "1500",
        "--config",
        &cfg,
        "--smax",
        "100",
    ]);
    assert!(ok, "{err}");
    assert!(std::fs::read_to_string(&nc).unwrap().contains("S 0-100"));
}

#[test]
fn a_feed_over_the_limit_of_speed_is_refused_and_never_lowered() {
    let base = [
        "engrave",
        GRADIENT,
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
        "--dry-run",
    ];
    let mut over = base.to_vec();
    over.extend(["--feed", "4000", "--max-speed", "3000"]);
    let (ok, _, err) = run(&over);
    assert!(!ok);
    assert!(
        err.contains("--feed") && err.contains("max_speed_mm_min"),
        "{err}"
    );
    let mut at = base.to_vec();
    at.extend(["--feed", "3000", "--max-speed", "3000"]);
    let (ok, _, err) = run(&at);
    assert!(ok, "{err}");
    // the speeds of the grid, of the frame and of the focus test obey it too
    let nc = tmp("speed.nc");
    let (ok, _, err) = run(&[
        "calibrate",
        &nc,
        "--powers",
        "20",
        "--feeds",
        "1000,4000",
        "--max-speed",
        "3000",
    ]);
    assert!(!ok);
    assert!(err.contains("--feeds"), "{err}");
    let (ok, _, err) = run(&["frame", &nc, "--size", "10,10", "--max-speed", "2000"]);
    assert!(!ok);
    assert!(
        err.contains("frame") && err.contains("max_speed_mm_min"),
        "{err}"
    );
    let (ok, _, err) = run(&[
        "focus",
        &nc,
        "--z-step",
        "0.5",
        "--feed",
        "2000",
        "--max-speed",
        "1000",
    ]);
    assert!(!ok);
    assert!(
        err.contains("focus") && err.contains("max_speed_mm_min"),
        "{err}"
    );
    // a limit out of range is an error
    let (ok, _, err) = run(&["frame", &nc, "--size", "10,10", "--max-speed", "0"]);
    assert!(!ok);
    assert!(err.contains("max_speed_mm_min"), "{err}");
}

#[test]
fn the_travel_of_a_profile_is_the_bed_of_the_frame_and_of_the_preview() {
    let cfg = profiles_file("profile-bed.cfg", "[defaults]\npower = 5\nspeed = 1000\n");
    let nc = tmp("profile-bed.nc");
    // 50 x 50 does not fit the 40 x 30 of the small diode, and fits the big one
    let (ok, _, err) = run(&[
        "frame",
        &nc,
        "--size",
        "50,50",
        "--feed",
        "1000",
        "--config",
        &cfg,
        "--profile",
        "small diode",
    ]);
    assert!(!ok);
    assert!(err.contains("outside of the bed"), "{err}");
    // the frame at its default speed is over the limit of speed of the small diode
    let (ok, _, err) = run(&[
        "frame",
        &nc,
        "--size",
        "10,10",
        "--config",
        &cfg,
        "--profile",
        "small diode",
    ]);
    assert!(!ok);
    assert!(err.contains("max_speed_mm_min"), "{err}");
    let (ok, out, err) = run(&[
        "frame",
        &nc,
        "--size",
        "50,50",
        "--config",
        &cfg,
        "--profile",
        "big",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("50.0 x 50.0"), "{out}");
    // the preview warns of work outside the travel of the profile
    let work = tmp("profile-work.nc");
    std::fs::write(
        &work,
        "G21\nG90\nF1000\nG0 X0 Y0\nM3 S100\nG1 X80 Y0\nM5\nM2\n",
    )
    .unwrap();
    let (ok, _, err) = run(&[
        "preview",
        &work,
        "--config",
        &cfg,
        "--profile",
        "small diode",
    ]);
    assert!(ok, "{err}");
    assert!(err.contains("outside"), "{err}");
    let (ok, _, err) = run(&["preview", &work, "--config", &cfg, "--profile", "big"]);
    assert!(ok, "{err}");
    assert!(!err.contains("outside"), "{err}");
}

#[test]
fn the_cap_of_the_pilot_power_is_in_the_settings_file_and_the_limit_of_power_still_holds() {
    let cfg = tmp("frame-cap.cfg");
    let nc = tmp("frame-cap.nc");
    let ask = |pilot: &str, cfg: &str| {
        run(&[
            "frame",
            &nc,
            "--size",
            "10,10",
            "--pilot",
            pilot,
            "--confirm-laser-on",
            "--config",
            cfg,
        ])
    };
    // the default cap is 5 %
    std::fs::write(&cfg, "[frame]\nfeed_mm_min = 3000\n").unwrap();
    assert!(!ask("8", &cfg).0);
    // a higher cap in the file lets 8 % through, still with the confirmation
    std::fs::write(&cfg, "[frame]\nmax_pilot_percent = 10\n").unwrap();
    let (ok, out, err) = ask("8", &cfg);
    assert!(ok, "{err}");
    assert!(out.contains("pilot power S80"), "{out}");
    // the limit of power of the machine is still over the cap
    std::fs::write(
        &cfg,
        "[machine]\nmax_power_percent = 6\n[frame]\nmax_pilot_percent = 10\n",
    )
    .unwrap();
    let (ok, _, err) = ask("8", &cfg);
    assert!(!ok);
    assert!(err.contains("max_power_percent"), "{err}");
    // a cap of 0 allows no pilot power
    std::fs::write(&cfg, "[frame]\nmax_pilot_percent = 0\n").unwrap();
    let (ok, _, err) = ask("1", &cfg);
    assert!(!ok);
    assert!(err.contains("max_pilot_percent"), "{err}");
    // a pilot power in the file above the cap of the file is an error of the file
    std::fs::write(&cfg, "[frame]\npilot_percent = 4\nmax_pilot_percent = 2\n").unwrap();
    let (ok, _, err) = run(&["frame", &nc, "--size", "10,10", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("pilot_percent"), "{err}");
    // a cap out of range is an error
    std::fs::write(&cfg, "[frame]\nmax_pilot_percent = 101\n").unwrap();
    let (ok, _, err) = run(&["frame", &nc, "--size", "10,10", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("max_pilot_percent"), "{err}");
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

#[test]
fn the_focus_test_burns_a_ladder_of_lines_at_different_heights() {
    let nc = tmp("focus.nc");
    let (ok, out, err) = run(&[
        "focus", &nc, "--z-step", "-0.5", "--steps", "4", "--length", "8", "--power", "20",
    ]);
    assert!(ok, "{err}");
    assert!(err.contains("moves Z"), "{err}");
    assert!(
        out.contains("4 lines of 8 mm from Z0 to Z-1.5, at 20 %"),
        "{out}"
    );
    let text = written(&nc);
    assert!(
        text.contains("G0 Z-1.5\n") && text.contains("G1 X8 S200\n"),
        "{text}"
    );
    let (ok, seen, err) = run(&["preview", &nc]);
    assert!(ok, "{err}");
    assert!(seen.contains("32 mm burned"), "{seen}");
}

#[test]
fn the_focus_test_does_not_guess_which_way_is_down_and_obeys_the_limit_of_power() {
    let nc = tmp("focus-bad.nc");
    let (ok, _, err) = run(&["focus", &nc]);
    assert!(!ok);
    assert!(err.contains("--z-step"), "{err}");
    let (ok, _, err) = run(&["focus", &nc, "--z-step", "0.5", "--max-power", "20"]);
    assert!(!ok);
    assert!(err.contains("max_power_percent"), "{err}");
    let (ok, _, err) = run(&["focus", &nc, "--z-step", "0.5", "--steps", "1"]);
    assert!(!ok);
    assert!(err.contains("steps"), "{err}");
}

#[test]
fn the_focus_test_reads_the_settings_file_and_never_lands_on_it() {
    let cfg = tmp("focus.cfg");
    std::fs::write(
        &cfg,
        "[machine]\ns_max = 255\n[focus]\nz_start_mm = 2\nz_step_mm = 0.5\nsteps = 3\npower_percent = 40\n",
    )
    .unwrap();
    let nc = tmp("focus-cfg.nc");
    let (ok, out, err) = run(&["focus", &nc, "--config", &cfg]);
    assert!(ok, "{err}");
    assert!(out.contains("3 lines of 10 mm from Z2 to Z3"), "{out}");
    // 40 % of 255
    assert!(written(&nc).contains("S102"));
    let before = written(&cfg);
    let (ok, _, err) = run(&["focus", &cfg, "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
    assert_eq!(written(&cfg), before);
}

#[test]
fn text_is_burnt_with_the_font_of_strokes_inside_the_limits() {
    let out_file = tmp("text.nc");
    let (ok, out, err) = run(&[
        "text",
        &out_file,
        "--text",
        "Phoenix 42",
        "--height-mm",
        "8",
        "--power",
        "25",
        "--feed",
        "1500",
        "--origin",
        "5,5",
        "--bed",
        "200,100",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("10 characters"), "{out}");
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert!(text.contains("text with a font of strokes"), "{text}");
    assert!(text.contains("S250 F1500"), "{text}");
    // the program passes the check, inside the bed
    let (ok, out, err) = run(&["verify", &out_file, "--bed", "200,100"]);
    assert!(ok, "{out} {err}");
    // lower case is the same as capitals
    let upper = tmp("text-upper.nc");
    let (ok, _, err) = run(&[
        "text",
        &upper,
        "--text",
        "PHOENIX 42",
        "--height-mm",
        "8",
        "--power",
        "25",
        "--feed",
        "1500",
        "--origin",
        "5,5",
        "--bed",
        "200,100",
    ]);
    assert!(ok, "{err}");
    let body = |f: &str| {
        std::fs::read_to_string(f)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with(';'))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(body(&out_file), body(&upper));
    // what is not allowed is refused and nothing is written
    let refused = tmp("text-refused.nc");
    let _ = std::fs::remove_file(&refused);
    for (args, word) in [
        (vec!["--text", "caf\u{e9}"], "not in the font"),
        (vec!["--text", "   "], "nothing to burn"),
        (
            vec!["--text", "A", "--max-power", "10"],
            "the power of the text",
        ),
        (
            vec!["--text", "A", "--max-speed", "500"],
            "the speed of the text",
        ),
        (vec!["--text", "A", "--height-mm", "150"], "height_mm"),
        (vec!["--text", "A", "--passes", "0"], "passes"),
        (
            vec!["--text", "AAAAAAAAAA", "--bed", "20,20"],
            "outside of the bed",
        ),
        (vec!["--text", "A", "--origin", "1"], "--origin needs X,Y"),
        (vec![], "--text"),
    ] {
        let mut a = vec!["text", refused.as_str()];
        a.extend(args.iter().copied());
        let (ok, _, err) = run(&a);
        assert!(!ok, "{args:?}");
        assert!(err.contains(word), "{args:?}: {err}");
        assert!(!err.contains("panicked"), "{err}");
    }
    assert!(!std::path::Path::new(&refused).exists());
    // the settings file holds the usual values, and an option wins over it
    let cfg = tmp("text.cfg");
    std::fs::write(
        &cfg,
        "[text]\nheight_mm = 6\nmax_height_mm = 20\npower_percent = 15\nspeed_mm_min = 800\npasses = 2\nmax_chars = 4\n",
    )
    .unwrap();
    let (ok, out, err) = run(&["text", &out_file, "--text", "ABCD", "--config", &cfg]);
    assert!(ok, "{err}");
    assert!(out.contains("at 15 % and 800 mm/min"), "{out}");
    let (ok, _, err) = run(&["text", &out_file, "--text", "ABCDE", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("max_chars"), "{err}");
    let (ok, out, err) = run(&[
        "text", &out_file, "--text", "AB", "--config", &cfg, "--power", "30",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("at 30 %"), "{out}");
    // the settings are never written over
    let (ok, _, err) = run(&["text", &cfg, "--text", "A", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
}

#[test]
fn a_qr_code_is_burnt_by_filling_its_modules_inside_the_limits() {
    let out_file = tmp("qr.nc");
    let args = [
        "qr",
        out_file.as_str(),
        "--text",
        "EH3D 42",
        "--ecc",
        "Q",
        "--module-mm",
        "0.5",
        "--line-mm",
        "0.1",
        "--power",
        "25",
        "--feed",
        "1500",
        "--origin",
        "5,5",
        "--bed",
        "100,100",
    ];
    let (ok, out, err) = run(&args);
    assert!(ok, "{err}");
    assert!(
        out.contains("7 bytes, version 1 (21 x 21 modules, level Q)"),
        "{out}"
    );
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert!(text.contains("QR code filled with lines"), "{text}");
    assert!(text.contains("S250 F1500"), "{text}");
    // the program passes the check, inside the bed, and the same data gives the same program
    let (ok, out, err) = run(&["verify", &out_file, "--bed", "100,100"]);
    assert!(ok, "{out} {err}");
    let again = tmp("qr-again.nc");
    let mut second = args.to_vec();
    second[1] = again.as_str();
    let (ok, _, err) = run(&second);
    assert!(ok, "{err}");
    assert_eq!(text, std::fs::read_to_string(&again).unwrap());
    // --print shows the symbol: 21 rows of 42 characters that start with the finder pattern
    let mut shown = args.to_vec();
    shown.push("--print");
    let (ok, out, err) = run(&shown);
    assert!(ok, "{err}");
    let rows: Vec<&str> = out.lines().filter(|l| l.contains("##")).collect();
    assert_eq!(rows.len(), 21, "{out}");
    assert!(rows[0].starts_with("##############  "), "{}", rows[0]);
    // the data is never written into a comment of the program
    let tricky = tmp("qr-tricky.nc");
    let (ok, _, err) = run(&["qr", &tricky, "--text", "A) (M3"]);
    assert!(ok, "{err}");
    for line in std::fs::read_to_string(&tricky).unwrap().lines() {
        if line.starts_with('(') {
            assert!(line.starts_with("(qr code of ") && line.matches(')').count() == 1);
        }
        assert_ne!(line, "M3");
    }
    // what is not allowed is refused and nothing is written
    let refused = tmp("qr-refused.nc");
    let _ = std::fs::remove_file(&refused);
    let long = "a".repeat(63);
    let long40 = "a".repeat(41);
    for (args, word) in [
        (
            vec!["--text", long.as_str(), "--ecc", "M", "--max-power", "100"],
            "max_bytes",
        ),
        (vec!["--text", long40.as_str()], "max_bytes"),
        (vec!["--text", "A", "--ecc", "X"], "--ecc"),
        (
            vec!["--text", "A", "--max-power", "10"],
            "the power of the QR code",
        ),
        (
            vec!["--text", "A", "--max-speed", "500"],
            "the speed of the QR code",
        ),
        (vec!["--text", "A", "--module-mm", "10"], "max_size_mm"),
        (vec!["--text", "A", "--module-mm", "11"], "module_mm"),
        (vec!["--text", "A", "--line-mm", "2"], "line_mm"),
        (vec!["--text", "A", "--passes", "0"], "passes"),
        (vec!["--text", "A", "--bed", "10,10"], "outside of the bed"),
        (vec!["--text", "A", "--origin", "1"], "--origin needs X,Y"),
        (vec![], "--text"),
    ] {
        let mut a = vec!["qr", refused.as_str()];
        a.extend(args.iter().copied());
        let (ok, _, err) = run(&a);
        assert!(!ok, "{args:?}");
        assert!(err.contains(word), "{args:?}: {err}");
        assert!(!err.contains("panicked"), "{err}");
    }
    assert!(!std::path::Path::new(&refused).exists());
    // data that needs a bigger version than the settings allow is refused and says which one
    let cfg = tmp("qr.cfg");
    std::fs::write(
        &cfg,
        "[qr]\nmax_version = 1\nmax_bytes = 20\npower_percent = 15\nspeed_mm_min = 800\necc = L\n",
    )
    .unwrap();
    let (ok, out, err) = run(&[
        "qr",
        &out_file,
        "--text",
        "ABCDEFGHIJKLMNOPQ",
        "--config",
        &cfg,
    ]);
    assert!(ok, "{err}");
    assert!(
        out.contains("level L") && out.contains("at 15 % and 800 mm/min"),
        "{out}"
    );
    let (ok, _, err) = run(&[
        "qr",
        &out_file,
        "--text",
        "ABCDEFGHIJKLMNOPQR",
        "--config",
        &cfg,
    ]);
    assert!(!ok);
    assert!(
        err.contains("max_version") && err.contains("version 2"),
        "{err}"
    );
    let (ok, out, err) = run(&[
        "qr", &out_file, "--text", "AB", "--config", &cfg, "--power", "30",
    ]);
    assert!(ok, "{err}");
    assert!(out.contains("at 30 %"), "{out}");
    // the settings are never written over
    let (ok, _, err) = run(&["qr", &cfg, "--text", "A", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("written over"), "{err}");
}

#[test]
fn a_qr_code_can_burn_its_light_modules_from_an_option_or_from_the_settings() {
    let plain = tmp("qr-plain.nc");
    let flipped = tmp("qr-flipped.nc");
    let (ok, _, err) = run(&["qr", &plain, "--text", "EH3D 42", "--line-mm", "0.5"]);
    assert!(ok, "{err}");
    let (ok, _, err) = run(&[
        "qr",
        &flipped,
        "--text",
        "EH3D 42",
        "--line-mm",
        "0.5",
        "--invert",
    ]);
    assert!(ok, "{err}");
    let (a, b) = (
        std::fs::read_to_string(&plain).unwrap(),
        std::fs::read_to_string(&flipped).unwrap(),
    );
    assert!(!a.contains("inverted") && b.contains(", inverted)"), "{b}");
    assert_ne!(a, b);
    // both pass the check, and between them they burn the whole square (441 modules of 1 mm, one line each 0.5 mm)
    let burnt = |file: &str| -> f64 {
        let (ok, out, err) = run(&["verify", file, "--bed", "100,100", "--json"]);
        assert!(ok, "{out} {err}");
        let at = out.find("\"burn_mm\":").expect(&out) + 10;
        let rest = out[at..].trim_start();
        let end = rest
            .find(|c: char| c != '.' && !c.is_ascii_digit())
            .unwrap();
        rest[..end].parse().unwrap()
    };
    let total = burnt(&plain) + burnt(&flipped);
    assert!((total - 441.0 * 2.0).abs() < 0.5, "{total}");
    // the settings can turn it on and the option can turn it off again
    let cfg = tmp("qr-invert.cfg");
    std::fs::write(&cfg, "[qr]\ninvert = on\n").unwrap();
    let from_file = tmp("qr-from-file.nc");
    let (ok, _, err) = run(&[
        "qr",
        &from_file,
        "--text",
        "EH3D 42",
        "--line-mm",
        "0.5",
        "--config",
        &cfg,
    ]);
    assert!(ok, "{err}");
    assert_eq!(b, std::fs::read_to_string(&from_file).unwrap());
    let (ok, _, err) = run(&[
        "qr",
        &from_file,
        "--text",
        "EH3D 42",
        "--line-mm",
        "0.5",
        "--config",
        &cfg,
        "--no-invert",
    ]);
    assert!(ok, "{err}");
    assert_eq!(a, std::fs::read_to_string(&from_file).unwrap());
    // the two together are an error, and a bad value in the file names the key
    let (ok, _, err) = run(&["qr", &from_file, "--text", "A", "--invert", "--no-invert"]);
    assert!(!ok);
    assert!(err.contains("cannot be given together"), "{err}");
    std::fs::write(&cfg, "[qr]\ninvert = maybe\n").unwrap();
    let (ok, _, err) = run(&["qr", &from_file, "--text", "A", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("on or off"), "{err}");
}

#[test]
fn a_qr_code_can_run_the_head_up_to_each_line_with_the_laser_off_and_never_out_of_the_bed() {
    let plain = tmp("qr-run-plain.nc");
    let up = tmp("qr-run-up.nc");
    let base = ["--text", "EH3D 42", "--line-mm", "0.5", "--origin", "5,5"];
    let mut a = vec!["qr", plain.as_str()];
    a.extend(base);
    let (ok, _, err) = run(&a);
    assert!(ok, "{err}");
    let mut b = vec!["qr", up.as_str()];
    b.extend(base);
    b.extend(["--overscan-mm", "2", "--bed", "100,100"]);
    let (ok, _, err) = run(&b);
    assert!(ok, "{err}");
    let (p, u) = (
        std::fs::read_to_string(&plain).unwrap(),
        std::fs::read_to_string(&up).unwrap(),
    );
    // the same lines are burnt, each with a run-up and a run-out that have the power at zero
    let burnt = |g: &str| g.lines().filter(|l| l.contains(" S200 ")).count();
    assert!(burnt(&p) > 0 && burnt(&p) == burnt(&u));
    assert!(
        u.lines()
            .filter(|l| l.starts_with("G1") && l.contains(" S0 "))
            .count()
            == 2 * burnt(&u)
    );
    assert!(!p.lines().any(|l| l.starts_with("G1") && l.contains(" S0 ")));
    let (ok, out, err) = run(&["verify", &up, "--bed", "100,100"]);
    assert!(ok, "{out} {err}");
    // a run-up that leaves the bed, or one out of its range, is refused and nothing is written
    let refused = tmp("qr-run-refused.nc");
    let _ = std::fs::remove_file(&refused);
    let (ok, _, err) = run(&[
        "qr",
        &refused,
        "--text",
        "A",
        "--overscan-mm",
        "2",
        "--bed",
        "21,21",
    ]);
    assert!(!ok);
    assert!(
        err.contains("overscan_mm") && err.contains("outside of the bed"),
        "{err}"
    );
    for bad in ["21", "-1", "x"] {
        let (ok, _, err) = run(&["qr", &refused, "--text", "A", "--overscan-mm", bad]);
        assert!(!ok, "{bad}");
        assert!(!err.contains("panicked"), "{err}");
    }
    assert!(!std::path::Path::new(&refused).exists());
    // the settings file can ask for it, and the option wins
    let cfg = tmp("qr-run.cfg");
    std::fs::write(&cfg, "[qr]\noverscan_mm = 3\n").unwrap();
    let from_file = tmp("qr-run-file.nc");
    let (ok, _, err) = run(&["qr", &from_file, "--text", "A", "--config", &cfg]);
    assert!(ok, "{err}");
    assert!(std::fs::read_to_string(&from_file)
        .unwrap()
        .contains(" S0 F"));
    let (ok, _, err) = run(&[
        "qr",
        &from_file,
        "--text",
        "A",
        "--config",
        &cfg,
        "--overscan-mm",
        "0",
    ]);
    assert!(ok, "{err}");
    assert!(!std::fs::read_to_string(&from_file)
        .unwrap()
        .contains(" S0 F"));
}

#[test]
fn the_text_and_the_qr_code_can_be_burnt_as_mirror_images_from_an_option_or_from_the_settings() {
    for (command, data) in [("text", "EH3D 42"), ("qr", "EH3D 42")] {
        let plain = tmp(&format!("{command}-plain.nc"));
        let mirror = tmp(&format!("{command}-mirror.nc"));
        let (ok, _, err) = run(&[command, &plain, "--text", data, "--origin", "4,4"]);
        assert!(ok, "{err}");
        let (ok, _, err) = run(&[
            command, &mirror, "--text", data, "--origin", "4,4", "--mirror",
        ]);
        assert!(ok, "{err}");
        let (a, b) = (
            std::fs::read_to_string(&plain).unwrap(),
            std::fs::read_to_string(&mirror).unwrap(),
        );
        assert_ne!(a, b, "{command}");
        // both pass the check inside the same box
        for file in [&plain, &mirror] {
            let (ok, out, err) = run(&["verify", file, "--bed", "100,100"]);
            assert!(ok, "{command}: {out} {err}");
        }
        // the settings can ask for it and the option can turn it off
        let cfg = tmp(&format!("{command}-mirror.cfg"));
        std::fs::write(&cfg, format!("[{command}]\nmirror = on\n")).unwrap();
        let from_file = tmp(&format!("{command}-from-file.nc"));
        let (ok, _, err) = run(&[
            command, &from_file, "--text", data, "--origin", "4,4", "--config", &cfg,
        ]);
        assert!(ok, "{err}");
        assert_eq!(b, std::fs::read_to_string(&from_file).unwrap(), "{command}");
        let (ok, _, err) = run(&[
            command,
            &from_file,
            "--text",
            data,
            "--origin",
            "4,4",
            "--config",
            &cfg,
            "--no-mirror",
        ]);
        assert!(ok, "{err}");
        assert_eq!(a, std::fs::read_to_string(&from_file).unwrap(), "{command}");
        // the two together are an error
        let (ok, _, err) = run(&[
            command,
            &from_file,
            "--text",
            "A",
            "--mirror",
            "--no-mirror",
        ]);
        assert!(!ok);
        assert!(err.contains("cannot be given together"), "{err}");
    }
}

#[test]
fn the_rows_of_a_text_can_start_at_the_left_in_the_middle_or_end_at_the_right() {
    let words = "EHEHEH\nE";
    let make = |name: &str, extra: &[&str]| -> String {
        let out = tmp(name);
        let mut args = vec!["text", out.as_str(), "--text", words, "--origin", "3,3"];
        args.extend_from_slice(extra);
        let (ok, _, err) = run(&args);
        assert!(ok, "{err}");
        std::fs::read_to_string(&out).unwrap()
    };
    let left = make("align-left.nc", &[]);
    assert_eq!(left, make("align-left2.nc", &["--align", "left"]));
    let centre = make("align-centre.nc", &["--align", "center"]);
    assert_eq!(centre, make("align-centre2.nc", &["--align", "CENTRE"]));
    let right = make("align-right.nc", &["--align", "right"]);
    assert!(left != centre && centre != right && left != right);
    // the same strokes are burnt in the same box
    for g in [&left, &centre, &right] {
        assert_eq!(g.lines().count(), left.lines().count());
    }
    let file = tmp("align-right-check.nc");
    std::fs::write(&file, &right).unwrap();
    let (ok, out, err) = run(&["verify", &file, "--bed", "100,100"]);
    assert!(ok, "{out} {err}");
    // the settings can ask for it and the option wins; a word that is not an alignment is refused
    let cfg = tmp("align.cfg");
    std::fs::write(&cfg, "[text]\nalign = right\n").unwrap();
    assert_eq!(right, make("align-file.nc", &["--config", cfg.as_str()]));
    assert_eq!(
        left,
        make(
            "align-file2.nc",
            &["--config", cfg.as_str(), "--align", "left"]
        )
    );
    let refused = tmp("align-refused.nc");
    let _ = std::fs::remove_file(&refused);
    let (ok, _, err) = run(&["text", &refused, "--text", "A", "--align", "middle"]);
    assert!(!ok);
    assert!(err.contains("left, center or right"), "{err}");
    std::fs::write(&cfg, "[text]\nalign = middle\n").unwrap();
    let (ok, _, err) = run(&["text", &refused, "--text", "A", "--config", &cfg]);
    assert!(!ok);
    assert!(err.contains("left, center or right"), "{err}");
    assert!(!std::path::Path::new(&refused).exists());
}

#[test]
fn a_text_or_a_qr_code_that_would_burn_for_too_long_is_refused_and_nothing_is_written() {
    // (a text of one pass takes minutes: five passes at 2 mm/min take hours, as a QR code does at 5 mm/min)
    for (command, slow) in [
        ("text", ["--feed", "2", "--passes", "5"]),
        ("qr", ["--feed", "5", "--passes", "1"]),
    ] {
        let out = tmp(&format!("{command}-long.nc"));
        let _ = std::fs::remove_file(&out);
        // at 5 mm/min the burning takes hours
        let mut args = vec![command, out.as_str(), "--text", "EH3D 42"];
        args.extend(slow);
        let (ok, _, err) = run(&args);
        assert!(!ok, "{command}");
        assert!(
            err.contains("max_minutes") && err.contains("minutes"),
            "{command}: {err}"
        );
        assert!(!std::path::Path::new(&out).exists());
        // a higher limit lets it through, and the settings can raise it too
        let mut args = vec![command, out.as_str(), "--text", "EH3D 42"];
        args.extend(slow);
        args.extend(["--max-minutes", "1440"]);
        let (ok, _, err) = run(&args);
        assert!(ok, "{command}: {err}");
        let cfg = tmp(&format!("{command}-long.cfg"));
        std::fs::write(&cfg, format!("[{command}]\nmax_minutes = 1440\n")).unwrap();
        let mut args = vec![command, out.as_str(), "--text", "EH3D 42"];
        args.extend(slow);
        args.extend(["--config", cfg.as_str()]);
        let (ok, _, err) = run(&args);
        assert!(ok, "{command}: {err}");
        // the option wins over the file, and a limit out of range is an error
        let mut args = vec![command, out.as_str(), "--text", "EH3D 42"];
        args.extend(slow);
        args.extend(["--config", cfg.as_str(), "--max-minutes", "10"]);
        let (ok, _, err) = run(&args);
        assert!(!ok && err.contains("max_minutes"), "{command}: {err}");
        for bad in ["0", "1441", "x"] {
            let (ok, _, err) = run(&[command, &out, "--text", "A", "--max-minutes", bad]);
            assert!(!ok, "{command} {bad}");
            assert!(!err.contains("panicked"), "{err}");
        }
    }
}

#[test]
fn an_inverted_qr_code_can_have_a_burnt_border_that_counts_for_the_size_and_the_bed() {
    let out = tmp("qr-border.nc");
    let base = ["qr", out.as_str(), "--text", "EH3D 42", "--origin", "5,5"];
    let mut a = base.to_vec();
    a.extend(["--invert", "--border-modules", "2", "--bed", "100,100"]);
    let (ok, stdout, err) = run(&a);
    assert!(ok, "{err}");
    // 21 modules and two on each side of them, of 1 mm
    assert!(stdout.contains("25.0 x 25.0 mm"), "{stdout}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains(", inverted, border of 2 modules)"), "{text}");
    let (ok, o, err) = run(&["verify", &out, "--bed", "100,100"]);
    assert!(ok, "{o} {err}");
    // the square with its border has to fit in the bed
    let refused = tmp("qr-border-refused.nc");
    let _ = std::fs::remove_file(&refused);
    let mut b = vec![
        "qr",
        refused.as_str(),
        "--text",
        "EH3D 42",
        "--origin",
        "5,5",
    ];
    b.extend(["--invert", "--border-modules", "2", "--bed", "29,29"]);
    let (ok, _, err) = run(&b);
    assert!(!ok);
    assert!(err.contains("outside of the bed"), "{err}");
    // a border needs the inversion, and its width has a range
    let mut c = vec!["qr", refused.as_str(), "--text", "A"];
    c.extend(["--border-modules", "2"]);
    let (ok, _, err) = run(&c);
    assert!(!ok);
    assert!(
        err.contains("border_modules") && err.contains("invert"),
        "{err}"
    );
    for bad in ["9", "-1", "1.5", "x"] {
        let (ok, _, err) = run(&[
            "qr",
            &refused,
            "--text",
            "A",
            "--invert",
            "--border-modules",
            bad,
        ]);
        assert!(!ok, "{bad}");
        assert!(!err.contains("panicked"), "{err}");
    }
    assert!(!std::path::Path::new(&refused).exists());
    // the settings file can ask for both
    let cfg = tmp("qr-border.cfg");
    std::fs::write(&cfg, "[qr]\ninvert = on\nborder_modules = 1\n").unwrap();
    let from_file = tmp("qr-border-file.nc");
    let (ok, stdout, err) = run(&["qr", &from_file, "--text", "EH3D 42", "--config", &cfg]);
    assert!(ok, "{err}");
    assert!(stdout.contains("23.0 x 23.0 mm"), "{stdout}");
}

#[test]
fn the_marks_command_burns_the_corners_of_the_rectangle_and_refuses_what_does_not_fit() {
    let out = tmp("marks.nc");
    let _ = std::fs::remove_file(&out);
    let (ok, said, err) = run(&["marks", &out, "--size", "40,20", "--origin", "10,5"]);
    assert!(ok, "{err}");
    assert!(said.contains("marks round 40.0 x 20.0 mm"), "{said}");
    let program = std::fs::read_to_string(&out).unwrap();
    assert!(
        program.contains("positioning marks, 4 strokes, 1 passes"),
        "{program}"
    );
    let (pass, report, _) = run(&["verify", &out, "--bed", "100,100"]);
    assert!(pass, "{report}");
    // the options and a cross in the middle
    let (ok, said, err) = run(&[
        "marks",
        &out,
        "--size",
        "40,20",
        "--centre-cross",
        "--arm-mm",
        "8",
        "--margin-mm",
        "2",
        "--power",
        "15",
        "--passes",
        "2",
        "--bed",
        "60,40",
        "--origin",
        "5,5",
    ]);
    assert!(ok, "{err}");
    assert!(
        said.contains("and a cross") && said.contains("at 15 %"),
        "{said}"
    );
    assert!(std::fs::read_to_string(&out)
        .unwrap()
        .contains("6 strokes, 2 passes"));
    // what is refused: nothing is written
    for (args, word) in [
        (vec!["--size", "40,20", "--arm-mm", "11"], "half a side"),
        (
            vec!["--size", "40,20", "--power", "60", "--max-power", "50"],
            "max_power_percent",
        ),
        (
            vec!["--size", "40,20", "--feed", "5000", "--max-speed", "2000"],
            "max_speed_mm_min",
        ),
        (
            vec!["--size", "40,20", "--bed", "30,30"],
            "outside of the bed",
        ),
        (
            vec![
                "--size",
                "40,20",
                "--origin",
                "10,10",
                "--margin-mm",
                "1",
                "--bed",
                "50,30",
            ],
            "outside of the bed",
        ),
        (vec!["--size", "0.5,20"], "each side must be from 1 to 1000"),
        (vec!["--size", "40"], "--size needs W,H"),
        (
            vec!["--size", "40,20", "--centre-cross", "--no-centre-cross"],
            "cannot be given together",
        ),
        (vec!["--size", "40,20", "--passes", "0"], "passes"),
        (vec![], "--size"),
    ] {
        let refused = tmp("marks-refused.nc");
        let _ = std::fs::remove_file(&refused);
        let mut full = vec!["marks", refused.as_str()];
        full.extend(args.iter().copied());
        let (ok, _, err) = run(&full);
        assert!(!ok, "{args:?}");
        assert!(err.contains(word), "{args:?}: {err}");
        assert!(!err.contains("panicked"), "{err}");
        assert!(!std::path::Path::new(&refused).exists(), "{args:?}");
    }
    // the settings say the usual values, the options win, and the output is never the settings
    let cfg = tmp("marks.cfg");
    std::fs::write(&cfg, "[marks]\narm_mm = 3\ncentre_cross = on\n").unwrap();
    let (ok, said, err) = run(&["marks", &out, "--size", "40,20", "--config", &cfg]);
    assert!(
        ok && said.contains("arms of 3 mm and a cross"),
        "{said} {err}"
    );
    let (ok, said, _) = run(&[
        "marks",
        &out,
        "--size",
        "40,20",
        "--config",
        &cfg,
        "--no-centre-cross",
        "--arm-mm",
        "4",
    ]);
    assert!(ok && said.contains("arms of 4 mm,"), "{said}");
    let (ok, _, err) = run(&["marks", &cfg, "--size", "40,20", "--config", &cfg]);
    assert!(!ok && err.contains("written over the settings"), "{err}");
}

#[test]
fn the_barcode_command_burns_a_barcode_that_reads_back_and_refuses_what_does_not_fit() {
    let out = tmp("barcode.nc");
    let _ = std::fs::remove_file(&out);
    let (ok, said, err) = run(&["barcode", &out, "--text", "EH3D-42"]);
    assert!(ok, "{err}");
    assert!(said.contains("7 characters, set B"), "{said}");
    let program = std::fs::read_to_string(&out).unwrap();
    assert!(program.contains("code 128 barcode in set B"), "{program}");
    assert!(
        !program.contains("EH3D-42"),
        "the data is not written into the program"
    );
    let (pass, report, _) = run(&["verify", &out]);
    assert!(pass, "{report}");
    // figures only are written in set C, which is narrower
    let narrow = tmp("barcode-c.nc");
    let (ok, said, err) = run(&["barcode", &narrow, "--text", "20261119"]);
    assert!(ok && said.contains("set C"), "{said} {err}");
    let code = eh3d_phoenix_laser::barcode::encode(b"20261119").unwrap();
    assert!(
        said.contains(&format!("{:.1}", code.modules.len() as f64 * 0.4)),
        "{said}"
    );
    // what cannot be written is refused and nothing is written
    for (args, word) in [
        (vec!["--text", "caf\u{e9}"], "from space to ~"),
        (vec!["--text", "A234567890123456789012"], "max_chars"),
        (
            vec!["--text", "EH3D", "--module-mm", "5", "--height-mm", "5"],
            "max_width_mm",
        ),
        (
            vec!["--text", "EH3D", "--feed", "1", "--passes", "20"],
            "max_minutes",
        ),
        (
            vec!["--text", "EH3D", "--power", "60", "--max-power", "50"],
            "max_power_percent",
        ),
        (
            vec!["--text", "EH3D", "--feed", "5000", "--max-speed", "2000"],
            "max_speed_mm_min",
        ),
        (
            vec!["--text", "EH3D", "--bed", "20,20"],
            "outside of the bed",
        ),
        (
            vec!["--text", "EH3D", "--bed", "100,100", "--origin", "-1,0"],
            "outside of the bed",
        ),
        (vec!["--text", "EH3D", "--line-mm", "5"], "line_mm"),
        (vec!["--text", ""], "empty"),
        (
            vec!["--text", "EH3D", "--mirror", "--no-mirror"],
            "cannot be given together",
        ),
        (vec![], "--text"),
    ] {
        let refused = tmp("barcode-refused.nc");
        let _ = std::fs::remove_file(&refused);
        let mut full = vec!["barcode", refused.as_str()];
        full.extend(args.iter().copied());
        let (ok, _, err) = run(&full);
        assert!(!ok, "{args:?}");
        assert!(err.contains(word), "{args:?}: {err}");
        assert!(!err.contains("panicked"), "{err}");
        assert!(!std::path::Path::new(&refused).exists(), "{args:?}");
    }
    // the settings say the usual values, the options win and the output is never the settings
    let cfg = tmp("barcode.cfg");
    std::fs::write(
        &cfg,
        "[barcode]\nmodule_mm = 0.5\nheight_mm = 8\nmirror = on\n",
    )
    .unwrap();
    let (ok, said, err) = run(&["barcode", &out, "--text", "AB", "--config", &cfg]);
    assert!(ok, "{err}");
    assert!(said.contains("x 8.0 mm"), "{said}");
    assert!(std::fs::read_to_string(&out).unwrap().contains("mirrored"));
    let (ok, _, _) = run(&[
        "barcode",
        &out,
        "--text",
        "AB",
        "--config",
        &cfg,
        "--no-mirror",
    ]);
    assert!(ok);
    assert!(!std::fs::read_to_string(&out).unwrap().contains("mirrored"));
    let (ok, _, err) = run(&["barcode", &cfg, "--text", "AB", "--config", &cfg]);
    assert!(!ok && err.contains("written over the settings"), "{err}");
    assert!(std::fs::read_to_string(&cfg)
        .unwrap()
        .contains("module_mm = 0.5"));
}

#[test]
fn the_cross_option_fills_an_engraved_layer_a_second_time() {
    let engraved = |out: &str| -> f64 {
        let end = out.find(" mm engraved").unwrap_or_else(|| panic!("{out}"));
        out[..end].rsplit(' ').next().unwrap().parse().unwrap()
    };
    let nc = tmp("cross.nc");
    let base = [
        "vector",
        "examples/plate.svg",
        nc.as_str(),
        "--config",
        "examples/plate.cfg",
    ];
    let (ok, once, err) = run(&base);
    assert!(ok, "{err}");
    let mut with = base.to_vec();
    with.push("--cross");
    let (ok, twice, err) = run(&with);
    assert!(ok, "{err}");
    let (a, b) = (engraved(&once), engraved(&twice));
    assert!(a > 100.0 && (b - 2.0 * a).abs() < 0.02 * a, "{a} {b}");
    // the program is still a clean one inside the bed
    let (ok, out, err) = run(&["verify", &nc]);
    assert!(ok, "{out} {err}");
    // the settings file can ask for it, and --no-cross takes it away
    let cfg = tmp("cross.cfg");
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/plate.cfg"))
        .unwrap();
    std::fs::write(&cfg, format!("{text}\n[layer #0000ff]\nhatch_cross = on\n")).unwrap();
    let from_file = [
        "vector",
        "examples/plate.svg",
        nc.as_str(),
        "--config",
        cfg.as_str(),
    ];
    let (ok, out, err) = run(&from_file);
    assert!(ok, "{err}");
    assert!((engraved(&out) - b).abs() < 0.02 * b, "{out}");
    let mut off = from_file.to_vec();
    off.push("--no-cross");
    let (ok, out, err) = run(&off);
    assert!(ok, "{err}");
    assert!((engraved(&out) - a).abs() < 0.02 * a, "{out}");
    let mut both = base.to_vec();
    both.extend(["--cross", "--no-cross"]);
    let (ok, _, err) = run(&both);
    assert!(!ok);
    assert!(err.contains("cannot be given together"), "{err}");
}

#[test]
fn verify_fails_a_program_that_is_not_the_one_that_was_authorised() {
    let nc = tmp("expect.nc");
    let (ok, _, err) = run(&["text", &nc, "--text", "OK"]);
    assert!(ok, "{err}");
    let (ok, out, err) = run(&["verify", &nc]);
    assert!(ok, "{out} {err}");
    let hash = out
        .lines()
        .find_map(|l| l.strip_prefix("toolpath sha256: "))
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(hash.len(), 64);
    // the hash that was authorised, in any case, passes
    let (ok, out, err) = run(&["verify", &nc, "--expect-sha256", &hash.to_uppercase()]);
    assert!(ok, "{out} {err}");
    assert!(!out.contains("authorised"), "{out}");
    // another hash fails, says which one the program has, and shows in the JSON report
    let other = "0".repeat(64);
    let (ok, out, err) = run(&["verify", &nc, "--expect-sha256", &other]);
    assert!(!ok);
    assert!(out.contains("not the one that was authorised"), "{out}");
    assert!(out.contains(&hash), "{out}");
    assert!(err.contains("fails 1 check"), "{err}");
    let (ok, out, _) = run(&["verify", &nc, "--expect-sha256", &other, "--json"]);
    assert!(!ok);
    assert!(out.contains("\"verdict\": \"fail\""), "{out}");
    assert!(out.contains("not the one that was authorised"), "{out}");
    // a value that is not a SHA-256 is refused before the program is read
    let (g64, long) = ("g".repeat(64), "0".repeat(65));
    for bad in ["", "abc", g64.as_str(), long.as_str()] {
        let (ok, _, err) = run(&["verify", &nc, "--expect-sha256", bad]);
        assert!(!ok, "{bad}");
        assert!(err.contains("64 hex digits"), "{err}");
    }
}
