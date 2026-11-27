//! Dynamic power (M4) and constant power (M3): every program the crate writes, in both modes, what the check says about them, the comments that are taken and the program that is sent to
//! a machine. The programs are only read as text.

use eh3d_phoenix_laser::barcode;
use eh3d_phoenix_laser::calibrate::{
    focus_grid, focus_test, grid, interval_test, kerf_test, Grid, Kerf,
};
use eh3d_phoenix_laser::cut::plan;
use eh3d_phoenix_laser::frame;
use eh3d_phoenix_laser::lasermode::LaserMode;
use eh3d_phoenix_laser::preview::{clean_comments, read, Move};
use eh3d_phoenix_laser::qr;
use eh3d_phoenix_laser::server::job_request;
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::text as laser_text;
use eh3d_phoenix_laser::vconfig::{self, Settings};
use eh3d_phoenix_laser::verify::{verify, Limits};
use std::path::Path;
use std::process::Command;

/// The programs of the crate as they are written with dynamic power.
fn programs() -> Vec<(&'static str, String)> {
    let settings = Settings::default();
    let machine = qr::Machine {
        s_max: 1000,
        max_power_percent: 100.0,
        max_speed_mm_min: 30_000.0,
        bed_mm: None,
    };
    let text_machine = laser_text::Machine {
        s_max: 1000,
        max_power_percent: 100.0,
        max_speed_mm_min: 30_000.0,
        bed_mm: None,
    };
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="20mm" viewBox="0 0 40 20"><rect x="2" y="2" width="12" height="9" fill="none" stroke="red"/><path d="M 20 3 L 33 3 L 33 14" fill="none" stroke="red"/></svg>"#;
    let drawing = read_svg(svg, &settings.read).unwrap();
    let filled = vconfig::parse("[defaults]\nmode = engrave\nhatch_spacing = 1\n").unwrap();
    let kerf = Kerf {
        power_percent: 50.0,
        feed_mm_min: 600.0,
        ..Kerf::default()
    };
    let focus = vconfig::FocusSettings {
        z_step_mm: -0.5,
        ..vconfig::FocusSettings::default()
    };
    let g = Grid::default();
    vec![
        ("cut", plan(&drawing, &settings).unwrap().gcode),
        ("vector", plan(&drawing, &filled).unwrap().gcode),
        ("grid", grid(&[20.0, 40.0], &[600.0, 1200.0], &g).unwrap()),
        (
            "interval",
            interval_test(&[1.0, 2.0], 30.0, 2000.0, &g, Some(2.0)).unwrap(),
        ),
        ("kerf", kerf_test(&kerf, 1000).unwrap()),
        ("focus", focus_test(&focus, 1000, 100.0).unwrap()),
        (
            "focus grid",
            focus_grid(&focus, &[10.0, 20.0], 1000, 100.0).unwrap(),
        ),
        (
            "text",
            laser_text::program("EH3D", (5.0, 5.0), &Default::default(), &text_machine)
                .unwrap()
                .0,
        ),
        (
            "qr",
            qr::program("EH3D", (5.0, 5.0), &Default::default(), &machine)
                .unwrap()
                .0,
        ),
        (
            "barcode",
            barcode::program("EH3D", (5.0, 5.0), &Default::default(), &machine)
                .unwrap()
                .0,
        ),
        (
            "marks",
            frame::marks_program((40.0, 20.0), (10.0, 10.0), &Default::default(), &machine)
                .unwrap(),
        ),
    ]
}

fn burnt_mm(program: &str) -> f64 {
    read(program)
        .moves
        .iter()
        .filter(|m| m.power > 0.0)
        .map(|m| (m.to.0 - m.from.0).hypot(m.to.1 - m.from.1))
        .sum()
}

/// How far the head runs straight on, in the direction of `burn`, over the moves given (the nearest first).
fn straight<'a>(burn: &Move, moves: impl Iterator<Item = &'a Move>) -> f64 {
    let unit = |m: &Move| {
        let (x, y) = (m.to.0 - m.from.0, m.to.1 - m.from.1);
        (x / x.hypot(y), y / x.hypot(y))
    };
    let d = unit(burn);
    moves
        .take_while(|m| {
            let u = unit(m);
            !m.rapid && u.0 * d.0 + u.1 * d.1 > 0.999_999
        })
        .map(|m| (m.to.0 - m.from.0).hypot(m.to.1 - m.from.1))
        .sum()
}

#[test]
fn every_program_has_both_modes_and_constant_power_burns_the_same_with_the_head_already_moving() {
    for (name, m4) in programs() {
        assert!(m4.contains("M4 S0"), "{name}:\n{m4}");
        let m3 = LaserMode::Constant.apply(&m4, 2.0).unwrap().gcode;
        assert!(m3.contains("M3 S0") && !m3.contains("M4"), "{name}:\n{m3}");
        let p = read(&m3);
        assert!(
            p.ended && p.faults.is_empty(),
            "{name}: {:?}\n{m3}",
            p.faults
        );
        assert!(
            (burnt_mm(&m3) - burnt_mm(&m4)).abs() < 0.05,
            "{name}: {} against {}",
            burnt_mm(&m3),
            burnt_mm(&m4)
        );
        // no burn begins or ends with the head at rest
        for (k, m) in p.moves.iter().enumerate() {
            if m.power <= 0.0 {
                continue;
            }
            if k == 0 || p.moves[k - 1].power <= 0.0 {
                assert!(
                    straight(m, p.moves[..k].iter().rev()) > 1.99,
                    "{name}: line {} has no run-up\n{m3}",
                    m.line
                );
            }
            if p.moves.get(k + 1).is_none_or(|n| n.power <= 0.0) {
                assert!(
                    straight(m, p.moves[k + 1..].iter()) > 1.99,
                    "{name}: line {} has no run-out\n{m3}",
                    m.line
                );
            }
        }
        let v = verify(m3.as_bytes(), &Limits::default()).unwrap();
        assert!(v.passes(), "{name}: {:?}", v.problems);
        assert!(
            v.notes.iter().any(|n| n.contains("constant power (M3)")),
            "{name}: {:?}",
            v.notes
        );
        let dynamic = verify(m4.as_bytes(), &Limits::default()).unwrap();
        assert!(
            !dynamic.notes.iter().any(|n| n.contains("(M3)")),
            "{name}: {:?}",
            dynamic.notes
        );
    }
}

#[test]
fn the_check_says_where_constant_power_is_the_more_dangerous() {
    let notes = |program: &str| {
        verify(program.as_bytes(), &Limits::default())
            .unwrap()
            .notes
    };
    // from rest, round a corner, and switched on while the head stands
    let n = notes("G21\nG90\nF600\nG0 X0 Y0\nM3 S100\nG1 X10\nG1 Y10\nM5\nM2\n");
    assert!(
        n.iter()
            .any(|t| t.contains("line 6") && t.contains("2 burn(s)")),
        "{n:?}"
    );
    assert!(
        n.iter()
            .any(|t| t.contains("line 5") && t.contains("head stands")),
        "{n:?}"
    );
    // with a run-up, the laser turned on by the move itself: only the general advice
    let n = notes("G21\nG90\nF600\nM3 S0\nG0 X0 Y0\nG1 X2\nG1 X12 S100\nG1 X14 S0\nM5\nM2\n");
    assert!(n.iter().any(|t| t.contains("constant power (M3)")), "{n:?}");
    assert!(
        !n.iter()
            .any(|t| t.contains("burn(s)") || t.contains("head stands")),
        "{n:?}"
    );
}

#[test]
fn the_settings_and_a_profile_choose_the_mode_and_dynamic_is_the_default() {
    assert_eq!(Settings::default().machine.laser_mode, LaserMode::Dynamic);
    let s = vconfig::parse("[machine]\nlaser_mode = m3\n").unwrap();
    assert_eq!(s.machine.laser_mode, LaserMode::Constant);
    let s = vconfig::parse("[machine]\nprofile = a\n[profile a]\nlaser_mode = M3\n").unwrap();
    assert_eq!(s.machine.laser_mode, LaserMode::Constant);
    let e = vconfig::parse("[machine]\nlaser_mode = m5\n").unwrap_err();
    assert!(e.contains("m4") && e.contains("m3"), "{e}");
}

#[test]
fn the_commands_write_the_mode_of_the_settings_file() {
    let dir = std::env::temp_dir().join(format!("lasermode-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
            .args(args)
            .current_dir(Path::new(&dir))
            .output()
            .expect("the program starts")
    };
    std::fs::write(dir.join("m3.cfg"), "[machine]\nlaser_mode = m3\n").unwrap();
    std::fs::write(
        dir.join("none.cfg"),
        "[machine]\nlaser_mode = m3\noverscan_mm = 0\n",
    )
    .unwrap();
    let out = run(&[
        "text", "--text", "A", "--origin", "5,5", "--config", "m3.cfg", "a.nc",
    ]);
    assert!(out.status.success(), "{out:?}");
    let program = std::fs::read_to_string(dir.join("a.nc")).unwrap();
    assert!(
        program.contains("M3 S0") && !program.contains("M4"),
        "{program}"
    );
    // with no run-up there is no constant power, and nothing is written
    let out = run(&["text", "--text", "A", "--config", "none.cfg", "b.nc"]);
    assert!(
        !out.status.success() && !dir.join("b.nc").exists(),
        "{out:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn accents_are_taken_in_a_comment_and_a_carriage_return_ends_it() {
    let check = |line: &str| {
        let program = format!("G21\nG90\nF600\nM4 S100\nG1 X10\n{line}\nM5\nM2\n");
        verify(program.as_bytes(), &Limits::default()).unwrap()
    };
    for line in [
        "(dise\u{f1}o a mano)",
        "; fabricaci\u{f3}n \u{c4}\u{d6} \u{65e5}\u{672c}\u{8a9e}",
        "(25 \u{b0}C)",
    ] {
        let v = check(line);
        assert!(v.passes() && v.notes.is_empty(), "{line}: {v:?}");
    }
    for line in ["; done?", "(go!)", "(a~b)", "(a\u{18}b)", "; a\u{85}b"] {
        let v = check(line);
        assert!(
            v.passes() && v.notes.iter().any(|n| n.contains("obeys at once")),
            "{line:?}: {v:?}"
        );
    }
    // outside of a comment nothing changed
    for line in ["G1 X5 \u{f1}", "G1 X5 \u{c4} (ok)"] {
        assert!(!check(line).passes(), "{line}");
    }
    // what follows a carriage return alone is not part of the comment
    let p = read("F600\nM4 S100\n; note\rG1 X10\nM5\nM2\n");
    assert_eq!(p.moves.len(), 1, "{:?}", p.notes);
}

#[test]
fn what_is_sent_to_a_machine_has_its_comments_cleaned_and_its_code_as_it_is() {
    let program = "\u{feff}; a\u{f1}o ready? go!\nG1 X5 (\u{65e5} ~) Y2 ; \u{c4}\r\nM5\n";
    assert_eq!(
        clean_comments(program),
        "; a_o ready_ go_\nG1 X5 (_ _) Y2 ; _\r\nM5\n"
    );
    let body = job_request("laser-1", "plate", program);
    assert!(
        body.contains("a_o ready_ go_") && body.contains("G1 X5 (_ _) Y2"),
        "{body}"
    );
    // code is not changed: what is wrong in it is for the check to refuse
    assert_eq!(clean_comments("G1 X5 !\n"), "G1 X5 !\n");
}
