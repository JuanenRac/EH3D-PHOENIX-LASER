//! The perforation and the lines cut once, the results and libraries put into a library of materials, the controller in the report and in the window, the extra passes of the window and the
//! ceiling of a material over them. Everything runs on a computer: no port is opened and nothing is burnt.

use eh3d_phoenix_laser::cut::{cut_once, perforate, plan};
use eh3d_phoenix_laser::lang::Lang;
use eh3d_phoenix_laser::libedit::{merge, record, Measured, OnClash};
use eh3d_phoenix_laser::materials;
use eh3d_phoenix_laser::report;
use eh3d_phoenix_laser::server::{vector_job, vector_request};
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::vconfig;
use eh3d_phoenix_laser::vector::{dist, Drawing, ReadOptions};
use eh3d_phoenix_laser::verify::{verify, Limits};
use eh3d_phoenix_laser::window::{LayerField, Status, Window};
use std::path::PathBuf;
use std::process::Command;

const ONE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40mm\" height=\"30mm\" viewBox=\"0 0 40 30\"><rect x=\"1\" y=\"1\" width=\"38\" height=\"28\" stroke=\"#000000\" fill=\"none\"/></svg>";
const TWICE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40mm\" height=\"30mm\" viewBox=\"0 0 40 30\"><rect x=\"1\" y=\"1\" width=\"38\" height=\"28\" stroke=\"#000000\" fill=\"none\"/><rect x=\"1\" y=\"1\" width=\"38\" height=\"28\" stroke=\"#000000\" fill=\"none\"/></svg>";
const PROGRAM: &str = "G21\nG90\nM4 S0\nF600\nG0 X5 Y5\nG1 X45 Y5 S250\nS0\nM5\nM2\n";
const LIBRARY: &str =
    "# mine\n[material birch]\nverified = on\npower = 40\nspeed = 600\nmax_power = 60\n";

/// A folder of its own for each test, removed at the end.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-shopfloor-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, text).unwrap();
        p
    }

    fn run(&self, args: &[&str]) -> (bool, String, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .expect("the program runs");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn drawing(svg: &str) -> Drawing {
    read_svg(svg.as_bytes(), &ReadOptions::default()).unwrap()
}

#[test]
fn the_dashes_of_a_perforation_leave_every_bridge_at_least_as_long_as_asked() {
    let square = [
        (0.0, 0.0),
        (10.5, 0.0),
        (10.5, 10.0),
        (0.0, 10.0),
        (0.0, 0.0),
    ];
    let dashes = perforate(&square, true, 4.0, 1.0, 100).unwrap().unwrap();
    // 41 mm round: 8 dashes, spread so that each period is 5.125 mm
    assert_eq!(dashes.len(), 8);
    for d in &dashes {
        let length: f64 = d.windows(2).map(|w| dist(w[0], w[1])).sum();
        assert!((length - 4.0).abs() < 1e-6, "{length}");
    }
    // the bridge that closes the ring is there too
    let end = *dashes.last().unwrap().last().unwrap();
    assert!(dist(end, dashes[0][0]) >= 1.0, "{end:?}");
    // an open line: the pattern starts at its start
    let line = [(0.0, 0.0), (12.0, 0.0)];
    let open = perforate(&line, false, 4.0, 1.0, 100).unwrap().unwrap();
    assert_eq!(open.len(), 3);
    assert_eq!(open[2], vec![(10.0, 0.0), (12.0, 0.0)]);
    // too short for a dash and a bridge, and too many dashes
    assert_eq!(perforate(&square, true, 50.0, 1.0, 100).unwrap(), None);
    assert!(perforate(&square, true, 4.0, 1.0, 7)
        .unwrap_err()
        .contains("dashes"));
}

#[test]
fn a_perforated_cut_burns_dashes_and_passes_the_check_with_either_power_mode() {
    for mode in ["m4", "m3"] {
        let s = vconfig::parse(&format!(
            "[machine]\nlaser_mode = {mode}\noverscan_mm = 1\n[defaults]\nperf_cut = 5\nperf_gap = 1\n"
        ))
        .unwrap();
        let made = plan(&drawing(ONE), &s).unwrap();
        // 132 mm round: 22 dashes of 5 mm
        assert!(
            (made.stats.cut_mm - 110.0).abs() < 1e-6,
            "{}",
            made.stats.cut_mm
        );
        let verdict = verify(made.gcode.as_bytes(), &Limits::default()).unwrap();
        assert!(verdict.passes(), "{mode}: {:?}", verdict.problems);
    }
    for half in [
        "perf_cut = 2",
        "perf_gap = 2",
        "perf_cut = 0.01\nperf_gap = 1",
    ] {
        let e = vconfig::parse(&format!("[defaults]\n{half}\n")).unwrap_err();
        assert!(e.contains("needs both"), "{half}: {e}");
    }
}

#[test]
fn a_line_drawn_twice_is_cut_once_only_when_it_is_asked_for_and_never_through_a_bridge() {
    let twice = drawing(TWICE);
    let off = plan(&twice, &vconfig::parse("").unwrap()).unwrap();
    assert!(
        (off.stats.cut_mm - 264.0).abs() < 1e-6,
        "{}",
        off.stats.cut_mm
    );
    assert!(off.warnings.iter().any(|w| w.contains("drawn twice")));
    let on = plan(
        &twice,
        &vconfig::parse("[order]\nmerge_doubled = on\n").unwrap(),
    )
    .unwrap();
    assert!(
        (on.stats.cut_mm - 132.0).abs() < 1e-6,
        "{}",
        on.stats.cut_mm
    );
    assert!(on.warnings.iter().any(|w| w.contains("cut once")));
    let tabbed = plan(
        &twice,
        &vconfig::parse("[order]\nmerge_doubled = on\n[defaults]\ntabs = 2\ntab_width = 2\n")
            .unwrap(),
    )
    .unwrap();
    assert!(
        (tabbed.stats.cut_mm - 256.0).abs() < 1e-6,
        "{}",
        tabbed.stats.cut_mm
    );
    assert!(tabbed.warnings.iter().any(|w| w.contains("not merged")));
    // a piece that loses a segment in its middle is two open pieces
    let a = vec![(0.0, 0.0), (10.0, 0.0)];
    let b = vec![(0.0, 5.0), (0.0, 0.0), (10.0, 0.0), (10.0, 5.0)];
    let (kept, left_out) = cut_once(vec![(a, false), (b, true)]);
    assert_eq!(left_out, 1);
    assert_eq!(kept.len(), 3);
    assert_eq!(kept[1], (vec![(0.0, 5.0), (0.0, 0.0)], false));
    assert_eq!(kept[2], (vec![(10.0, 0.0), (10.0, 5.0)], false));
}

#[test]
fn a_vector_job_carries_the_power_mode_and_says_that_the_server_does_not_apply_it() {
    for (mode, noted) in [("m4", false), ("m3", true)] {
        let s = vconfig::parse(&format!("[machine]\nlaser_mode = {mode}\n")).unwrap();
        let (job, notes) = vector_job(&drawing(ONE), &s).unwrap();
        let body = vector_request("laser-1", "plate", &job);
        assert!(
            body.contains(&format!("\"laser_mode\":\"{mode}\"")),
            "{body}"
        );
        assert_eq!(
            notes.iter().any(|n| n.contains("does not apply it")),
            noted,
            "{notes:?}"
        );
    }
    let s = vconfig::parse("[defaults]\nperf_cut = 5\nperf_gap = 1\n").unwrap();
    let (_, notes) = vector_job(&drawing(ONE), &s).unwrap();
    assert!(notes.iter().any(|n| n.contains("cut whole")), "{notes:?}");
}

#[test]
fn a_result_is_written_as_not_verified_and_never_over_another_value_without_being_asked() {
    let found = Measured {
        power: Some(45.0),
        kerf: Some(0.15),
        ..Measured::default()
    };
    let asked = record(LIBRARY, "Birch", &found, false).unwrap();
    assert_eq!(asked.text, LIBRARY);
    assert_eq!(asked.clashes.len(), 1);
    assert_eq!(
        (asked.clashes[0].key, asked.clashes[0].old.as_str()),
        ("power", "40")
    );
    let made = record(LIBRARY, "Birch", &found, true).unwrap();
    assert_eq!(made.changed, vec!["power", "kerf"]);
    assert!(!made.added && made.text.starts_with("# mine\n"));
    let lib = materials::parse(&made.text).unwrap();
    let m = lib.find("birch").unwrap();
    assert_eq!(
        (m.power, m.speed, m.kerf),
        (Some(45.0), Some(600.0), Some(0.15))
    );
    assert_eq!((m.verified, m.max_power), (Some(false), Some(60.0)));
    // the same again changes nothing, and a new material is added
    let again = record(&made.text, "birch", &found, false).unwrap();
    assert!(again.changed.is_empty() && again.clashes.is_empty());
    assert_eq!(again.text, made.text);
    let new = record(LIBRARY, "card", &found, false).unwrap();
    assert!(new.added);
    assert_eq!(
        materials::parse(&new.text)
            .unwrap()
            .find("card")
            .unwrap()
            .verified,
        Some(false)
    );
    // over the ceiling of the material, nothing to write and a width of nothing
    let over = Measured {
        power: Some(80.0),
        ..Measured::default()
    };
    assert!(record(LIBRARY, "birch", &over, true)
        .unwrap_err()
        .contains("max_power"));
    assert!(record(LIBRARY, "birch", &Measured::default(), true).is_err());
    let none = Measured {
        kerf: Some(0.0),
        ..Measured::default()
    };
    assert!(record(LIBRARY, "birch", &none, true).is_err());
}

#[test]
fn another_library_is_brought_in_without_writing_over_anything_unasked_or_raising_a_ceiling() {
    let theirs = "[material BIRCH]\nverified = on\npower = 50\nspeed = 600\nmax_power = 90\n[material card]\nverified = on\npower = 10\nspeed = 900\n";
    let stopped = merge(LIBRARY, theirs, OnClash::Stop).unwrap();
    assert!(stopped.stopped);
    assert_eq!(stopped.text, LIBRARY);
    assert_eq!(stopped.clashed, vec!["birch"]);
    let kept = merge(LIBRARY, theirs, OnClash::KeepMine).unwrap();
    let lib = materials::parse(&kept.text).unwrap();
    assert_eq!(lib.find("birch").unwrap().power, Some(40.0));
    assert_eq!(lib.find("birch").unwrap().verified, Some(true));
    assert_eq!(lib.find("card").unwrap().verified, Some(false));
    let taken = merge(LIBRARY, theirs, OnClash::TakeTheirs).unwrap();
    let lib = materials::parse(&taken.text).unwrap();
    let birch = lib.find("birch").unwrap();
    assert_eq!(birch.name, "birch");
    assert_eq!((birch.power, birch.max_power), (Some(50.0), Some(60.0)));
    assert_eq!(birch.verified, Some(false));
    // a value over the ceiling the person set is refused
    let hot = "[material birch]\npower = 70\n";
    assert!(merge(LIBRARY, hot, OnClash::TakeTheirs)
        .unwrap_err()
        .contains("max_power"));
    // the same library again has nothing to say
    let same = merge(LIBRARY, LIBRARY, OnClash::Stop).unwrap();
    assert_eq!((same.stopped, same.same.len()), (false, 1));
    assert_eq!(same.text, LIBRARY);
}

#[test]
fn the_commands_record_merge_and_report_the_controller() {
    let s = Scratch::new("cli");
    s.write("lib.cfg", LIBRARY);
    s.write(
        "other.cfg",
        "[material birch]\npower = 50\n[material card]\npower = 10\n",
    );
    let (ok, _, e) = s.run(&["record", "lib.cfg", "--material", "birch", "--power", "45"]);
    assert!(!ok && e.contains("--replace"), "{e}");
    let (ok, out, e) = s.run(&[
        "record",
        "lib.cfg",
        "--material",
        "birch",
        "--gap",
        "1.65",
        "--strips",
        "10",
    ]);
    assert!(ok && out.contains("kerf"), "{out}{e}");
    let text = std::fs::read_to_string(s.0.join("lib.cfg")).unwrap();
    assert!(
        text.contains("kerf = 0.15") && text.contains("verified = off"),
        "{text}"
    );
    let (ok, _, e) = s.run(&["merge", "lib.cfg", "other.cfg"]);
    assert!(!ok && e.contains("--keep-mine"), "{e}");
    assert_eq!(std::fs::read_to_string(s.0.join("lib.cfg")).unwrap(), text);
    let (ok, out, e) = s.run(&["merge", "lib.cfg", "other.cfg", "--keep-mine"]);
    assert!(ok && out.contains("1 added"), "{out}{e}");

    s.write("p.nc", PROGRAM);
    s.write("off.txt", "$30=1000\n$32=0\n");
    let (ok, out, e) = s.run(&["report", "p.nc", "--lang", "en"]);
    assert!(ok && out.contains("was not read"), "{out}{e}");
    let (ok, out, _) = s.run(&["report", "p.nc", "--lang", "en", "--controller", "off.txt"]);
    assert!(
        !ok && out.contains("does not agree") && out.contains("$32=0"),
        "{out}"
    );
}

#[test]
fn the_ceiling_of_a_material_holds_over_every_extra_pass() {
    let s = Scratch::new("ceiling");
    s.write("d.svg", ONE);
    s.write("m.cfg", "[material birch]\npower = 20\nmax_power = 30\n");
    s.write(
        "s.cfg",
        "[layer #000000]\npower = 20\n[sublayer #000000/deep]\npower = 60\n",
    );
    let (ok, _, e) = s.run(&[
        "cut",
        "d.svg",
        "out.nc",
        "--config",
        "s.cfg",
        "--material",
        "birch",
        "--materials",
        "m.cfg",
    ]);
    assert!(
        !ok && e.contains("sublayer #000000/deep") && e.contains("max_power"),
        "{e}"
    );
    assert!(!s.0.join("out.nc").exists());
}

/// A window with a drawing open.
fn window(s: &Scratch) -> Window {
    let d = s.write("d.svg", ONE);
    let mut w = Window::new(Lang::En);
    assert!(w.open_dropped(&d), "{:?}", w.status);
    w
}

#[test]
fn the_window_holds_the_ceiling_of_the_material_it_applied() {
    let s = Scratch::new("window-ceiling");
    let mut w = window(&s);
    let lib = s.write("m.cfg", "[material birch]\npower = 20\nmax_power = 30\n");
    assert!(w.load_materials(&lib));
    assert!(w.apply_material("birch"));
    assert!(w.compute(), "{:?}", w.status);
    w.set_layer_field(0, LayerField::Power, "50".to_string());
    assert!(!w.compute());
    assert!(
        matches!(&w.status, Status::Failed(e) if e.contains("max_power")),
        "{:?}",
        w.status
    );
}

#[test]
fn a_controller_that_does_not_agree_stops_the_export_and_is_in_the_report() {
    let s = Scratch::new("window-controller");
    let mut w = window(&s);
    assert!(w.compute(), "{:?}", w.status);
    assert!(w.controller_findings().is_none());
    assert!(w.make_report());
    let not_read = report::Key::ControllerNotRead.text(Lang::En);
    assert!(w.report_text.as_ref().unwrap().contains(not_read));
    let listing = s.write("grbl.txt", "$30=1000\n$32=0\n");
    assert!(w.load_controller(&listing), "{:?}", w.status);
    assert!(w
        .controller_findings()
        .unwrap()
        .iter()
        .any(|f| f.kind.serious()));
    let out = s.0.join("out.nc");
    assert!(!w.export(&out));
    assert!(!out.exists());
    assert!(w.compute() && w.make_report());
    assert!(w.report_text.as_ref().unwrap().contains("$32=0"));
    // a file that is not a listing leaves the one that was read
    let junk = s.write("junk.txt", "hello\n");
    assert!(!w.load_controller(&junk));
    assert!(w.shop.controller.is_some());
    w.forget_controller();
    assert!(w.export(&out), "{:?}", w.status);
    w.shop.alarm = "ALARM:2".to_string();
    assert!(w.alarm_answer().unwrap().is_ok());
    w.shop.alarm = "ALARM:999".to_string();
    assert!(w.alarm_answer().unwrap().is_err());
}

#[test]
fn the_window_adds_and_removes_extra_passes_and_keeps_those_of_other_drawings_aside() {
    let s = Scratch::new("window-passes");
    let mut w = window(&s);
    // a pass of a layer the drawing does not have does not stop the work
    let cfg = s.write(
        "s.cfg",
        "[order]\nmerge_doubled = on\n[layer #ff0000]\npower = 20\n[sublayer #ff0000/extra]\npower = 30\n",
    );
    assert!(w.load_config(&cfg), "{:?}", w.status);
    assert!(w.shop.merge_doubled && w.shop.passes.is_empty());
    assert!(w.compute(), "{:?}", w.status);
    w.shop.new_label = "Fill".to_string();
    assert!(w.add_pass(), "{:?}", w.status);
    w.shop.new_label = "fill".to_string();
    assert!(!w.add_pass());
    w.set_pass_field(0, 0, "25".to_string());
    assert!(w.compute(), "{:?}", w.status);
    let text = w.done.as_ref().unwrap().gcode.clone();
    assert!(text.contains("; layer #000000/fill: cut, 25 %"), "{text}");
    w.set_pass_field(0, 2, "x".to_string());
    assert!(!w.compute());
    w.remove_pass(0);
    assert!(w.compute(), "{:?}", w.status);
    assert!(!w.done.as_ref().unwrap().gcode.contains("/fill"));
}

#[test]
fn the_window_writes_a_result_and_merges_a_library_asking_first() {
    let s = Scratch::new("window-library");
    let mut w = window(&s);
    let lib = s.write("m.cfg", LIBRARY);
    assert!(w.load_materials(&lib));
    w.shop.result.name = "birch".to_string();
    w.shop.result.power = "45".to_string();
    assert!(!w.record_result());
    assert!(matches!(w.status, Status::RecordAsk(..)), "{:?}", w.status);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), LIBRARY);
    assert!(w.record_result(), "{:?}", w.status);
    assert!(std::fs::read_to_string(&lib)
        .unwrap()
        .contains("power = 45"));

    let other = s.write("other.cfg", "[material birch]\npower = 50\n");
    let before = std::fs::read_to_string(&lib).unwrap();
    assert!(w.merge_library(&other), "{:?}", w.status);
    assert!(w.shop.merge_pending.is_some());
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
    assert!(w.merge_resolve(true), "{:?}", w.status);
    assert!(w.shop.merge_pending.is_none());
    assert!(std::fs::read_to_string(&lib)
        .unwrap()
        .contains("power = 50"));
    for lang in Lang::ALL {
        assert!(!w.status.text(lang).is_empty());
    }
}

#[test]
fn a_different_full_power_is_serious_unless_the_person_turned_the_switch_on() {
    let s = Scratch::new("smax");
    s.write("p.nc", PROGRAM);
    s.write("other.txt", "$30=255\n$32=1\n");
    s.write("warn.cfg", "[verify]\ns_max_mismatch_warns = on\n");
    let (ok, out, _) = s.run(&["verify", "p.nc", "--controller", "other.txt"]);
    assert!(!ok && out.contains("S255"), "{out}");
    let (ok, out, e) = s.run(&[
        "verify",
        "p.nc",
        "--config",
        "warn.cfg",
        "--controller",
        "other.txt",
    ]);
    assert!(ok && out.contains("only a warning"), "{out}{e}");
    let (ok, out, _) = s.run(&["controller", "other.txt", "--lang", "en"]);
    assert!(!ok && out.contains("!!"), "{out}");
    let (ok, out, e) = s.run(&[
        "controller",
        "other.txt",
        "--config",
        "warn.cfg",
        "--lang",
        "en",
    ]);
    assert!(ok && out.contains("wrongly scaled"), "{out}{e}");
    // a full power that is not given stays serious with the switch on
    s.write("none.txt", "$32=1\n");
    let (ok, _, _) = s.run(&["controller", "none.txt", "--config", "warn.cfg"]);
    assert!(!ok);

    assert!(!vconfig::Settings::default().verify.s_max_mismatch_warns);
    let mut w = window(&s);
    assert!(w.load_controller(&s.0.join("other.txt")), "{:?}", w.status);
    assert!(!w.shop.s_max_warns);
    let out = s.0.join("out.nc");
    assert!(!w.export(&out) && !out.exists());
    w.set_s_max_warns(true);
    assert!(w
        .controller_findings()
        .unwrap()
        .iter()
        .all(|f| !f.kind.serious()));
    assert!(w.export(&out), "{:?}", w.status);
    // a project never turns it on for the person
    let theirs = vconfig::parse("[verify]\ns_max_mismatch_warns = on\noverscan_mm = 20\n").unwrap();
    let kept = eh3d_phoenix_laser::suitewin::keep_my_limits(&vconfig::Settings::default(), theirs);
    assert!(!kept.verify.s_max_mismatch_warns);
    assert_eq!(kept.verify.overscan_mm, 0.0);
}
