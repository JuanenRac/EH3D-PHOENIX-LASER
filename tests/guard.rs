//! The forbidden zones, the interlocks and the ceilings of a material as the command line shows them: what is refused, what passes and what is said.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d_laser_guard_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, name: &str, text: &str) -> String {
        let p = self.0.join(name);
        fs::write(&p, text).unwrap();
        p.to_string_lossy().into_owned()
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A program that burns a line from 0, 0 to 40, 0 and then travels to 40, 30.
const PROGRAM: &str = "G21\nG90\nM4 S0\nF600\nG0 X0 Y0\nG1 X40 Y0 S300\nS0\nG0 X40 Y30\nM5\nM2\n";

#[test]
fn verify_fails_a_program_that_burns_in_a_no_burn_zone_and_passes_one_that_stays_away() {
    let s = Scratch::new("zones");
    let program = s.write("p.nc", PROGRAM);
    let across = s.write(
        "across.cfg",
        "[zone clamp]\nx_mm = 10\ny_mm = -2\nwidth_mm = 5\nheight_mm = 4\nkind = no_burn\n",
    );
    let (code, out, _) = run(&["verify", &program, "--config", &across]);
    assert_eq!(code, Some(1), "{out}");
    assert!(
        out.contains("forbidden zone clamp") && out.contains("line 6"),
        "{out}"
    );
    assert!(out.contains("verdict: FAIL"), "{out}");
    let (_, json, _) = run(&["verify", &program, "--config", &across, "--json"]);
    assert!(
        json.contains("\"forbidden_zones\"") && json.contains("\"verdict\": \"fail\""),
        "{json}"
    );
    // the same zone placed where the program does not burn: it passes and says it was checked
    let away = s.write(
        "away.cfg",
        "[zone clamp]\nx_mm = 100\ny_mm = 100\nwidth_mm = 5\nheight_mm = 4\nkind = no_burn\n",
    );
    let (code, out, _) = run(&["verify", &program, "--config", &away]);
    assert_eq!(code, Some(0), "{out}");
    // the travel of the head across a no-burn zone is allowed and across a no-go zone is not
    let travel = s.write(
        "travel.cfg",
        "[zone gate]\nx_mm = 38\ny_mm = 10\nwidth_mm = 4\nheight_mm = 5\nkind = no_burn\n",
    );
    assert_eq!(run(&["verify", &program, "--config", &travel]).0, Some(0));
    let no_go = s.write(
        "nogo.cfg",
        "[zone gate]\nx_mm = 38\ny_mm = 10\nwidth_mm = 4\nheight_mm = 5\n",
    );
    let (code, out, _) = run(&["verify", &program, "--config", &no_go]);
    assert_eq!(code, Some(1));
    assert!(out.contains("no-go zone gate"), "{out}");
}

#[test]
fn verify_fails_every_program_while_the_emergency_stop_is_pressed() {
    let s = Scratch::new("estop");
    let program = s.write("p.nc", PROGRAM);
    let pressed = s.write("pressed.cfg", "[interlocks]\nestop = pressed\n");
    let (code, out, _) = run(&["verify", &program, "--config", &pressed]);
    assert_eq!(code, Some(1));
    assert!(out.contains("emergency stop is pressed"), "{out}");
    let (_, json, _) = run(&["verify", &program, "--config", &pressed, "--json"]);
    assert!(json.contains("\"interlocks\""), "{json}");
    let released = s.write("released.cfg", "[interlocks]\nestop = released\n");
    assert_eq!(run(&["verify", &program, "--config", &released]).0, Some(0));
    // a required enclosure that is not said to be closed stops it, and a closed one lets it pass
    let lid = s.write("lid.cfg", "[interlocks]\nrequire_enclosure = on\n");
    let (code, out, _) = run(&["verify", &program, "--config", &lid]);
    assert_eq!(code, Some(1));
    assert!(out.contains("enclosure must be closed"), "{out}");
    let shut = s.write(
        "shut.cfg",
        "[interlocks]\nrequire_enclosure = on\nenclosure = closed\n",
    );
    assert_eq!(run(&["verify", &program, "--config", &shut]).0, Some(0));
}

#[test]
fn a_file_with_a_zone_that_forbids_nothing_is_refused_with_its_reason() {
    let s = Scratch::new("badzone");
    let program = s.write("p.nc", PROGRAM);
    let bad = s.write("bad.cfg", "[zone empty]\nx_mm = 1\n");
    let (code, _, err) = run(&["verify", &program, "--config", &bad]);
    assert_eq!(code, Some(1));
    assert!(err.contains("width_mm"), "{err}");
}

#[test]
fn the_example_of_the_zones_runs_against_a_program_of_the_bed() {
    let s = Scratch::new("example");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = root.join("examples/safety.cfg");
    let config = config.to_string_lossy().into_owned();
    // a line burnt in the middle of the bed of the example passes
    let mid = s.write(
        "mid.nc",
        "G21\nG90\nM4 S0\nF600\nG0 X100 Y100\nG1 X150 Y100 S300\nS0\nM5\nM2\n",
    );
    let (code, out, _) = run(&["verify", &mid, "--config", &config]);
    assert_eq!(code, Some(0), "{out}");
    // one that burns in the corner of the clamp does not
    let corner = s.write(
        "corner.nc",
        "G21\nG90\nM4 S0\nF600\nG0 X0 Y10\nG1 X20 Y10 S300\nS0\nM5\nM2\n",
    );
    let (code, out, _) = run(&["verify", &corner, "--config", &config]);
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn a_material_ceiling_holds_over_the_options_of_the_command_line() {
    let s = Scratch::new("ceiling");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let plate = root
        .join("examples/plate.svg")
        .to_string_lossy()
        .into_owned();
    let image = root
        .join("examples/gradient.pgm")
        .to_string_lossy()
        .into_owned();
    let lib = s.write(
        "lib.cfg",
        "[material soft]\nverified = off\npower = 20\nspeed = 1000\nmax_power = 30\nmax_speed = 2000\n",
    );
    let out = s.path("out.nc");
    // inside the ceiling
    let (code, _, err) = run(&[
        "cut",
        &plate,
        &out,
        "--materials",
        &lib,
        "--material",
        "soft",
    ]);
    assert_eq!(code, Some(0), "{err}");
    // the note about a material that was not verified goes to the error output
    assert!(err.contains("not verified"), "{err}");
    // over it
    let (code, _, err) = run(&[
        "cut",
        &plate,
        &out,
        "--materials",
        &lib,
        "--material",
        "soft",
        "--power",
        "35",
    ]);
    assert_eq!(code, Some(1));
    assert!(err.contains("max_power") && err.contains("soft"), "{err}");
    let (code, _, err) = run(&[
        "cut",
        &plate,
        &out,
        "--materials",
        &lib,
        "--material",
        "soft",
        "--speed",
        "2500",
    ]);
    assert_eq!(code, Some(1));
    assert!(err.contains("max_speed"), "{err}");
    // a picture too
    let (code, _, err) = run(&[
        "engrave",
        &image,
        &out,
        "--width-mm",
        "20",
        "--materials",
        &lib,
        "--material",
        "soft",
        "--power",
        "31",
    ]);
    assert_eq!(code, Some(1));
    assert!(err.contains("max_power"), "{err}");
    let (code, _, err) = run(&[
        "engrave",
        &image,
        &out,
        "--width-mm",
        "20",
        "--materials",
        &lib,
        "--material",
        "soft",
    ]);
    assert_eq!(code, Some(0), "{err}");
}

#[test]
fn the_list_of_a_library_marks_what_was_not_verified_and_the_starter_library_is_all_of_it() {
    let s = Scratch::new("list");
    let lib = s.write(
        "lib.cfg",
        "[material a]\nverified = on\npower = 10\n[material b]\nverified = off\npower = 10\n[material c]\npower = 10\n",
    );
    let (code, out, _) = run(&["materials", &lib]);
    assert_eq!(code, Some(0));
    assert!(out.contains("a: power 10 % [verified]"), "{out}");
    assert!(out.contains("b: power 10 % [not verified]"), "{out}");
    assert!(out.contains("c: power 10 %\n"), "{out}");
    let starter = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/materials-starter.cfg");
    let (code, out, _) = run(&["materials", &starter.to_string_lossy()]);
    assert_eq!(code, Some(0), "{out}");
    let lines: Vec<&str> = out.lines().filter(|l| l.contains(": ")).collect();
    assert!(lines.len() >= 7);
    assert!(
        lines
            .iter()
            .filter(|l| !l.contains("material(s) in"))
            .all(|l| l.contains("[not verified]")),
        "{out}"
    );
}
