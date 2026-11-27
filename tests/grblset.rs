//! The saved listing of the settings of a controller compared with a program, and the meaning of its alarms and errors. No machine and no port: the listings are written here.

use eh3d_phoenix_laser::grblcodes::{explain, read_answer, Answer, ALARMS, ERRORS};
use eh3d_phoenix_laser::grblset::{check, parse, Expect, Kind, MAX_DUMP_BYTES};
use eh3d_phoenix_laser::lang::Lang;
use eh3d_phoenix_laser::verify::{verify, Limits};
use std::process::Command;

const GOOD: &str =
    "$13=0\n$30=1000\n$31=0\n$32=1\n$110=6000.000\n$111=6000.000\n$130=400\n$131=300\nok\n";

fn expect() -> Expect {
    Expect {
        s_max: 1000,
        bed_mm: Some((400.0, 300.0)),
        max_feed_mm_min: Some(3000.0),
    }
}

fn kinds(listing: &str) -> Vec<Kind> {
    check(&parse(listing).unwrap(), &expect())
        .iter()
        .map(|f| f.kind)
        .collect()
}

#[test]
fn a_listing_is_read_with_its_comments_and_everything_else_is_left_alone() {
    let d = parse(
        "\u{feff}[VER:1.1h]\r\n$N0=G54\r\n$32=1 (laser mode)\r\n  $30 = 255  \r\nok\r\n$32=1\r\n",
    );
    let d = d.unwrap();
    assert_eq!(d.len(), 2);
    assert_eq!(
        (d.get(32), d.get(30), d.get(0)),
        (Some(1.0), Some(255.0), None)
    );
}

#[test]
fn a_listing_that_cannot_be_believed_is_an_error() {
    for (text, words) in [
        ("$32=on\n", "line 1"),
        ("$30=1000\n$32=\n", "line 2"),
        ("$32=1\n$32=0\n", "twice"),
        ("$30=inf\n", "not a number"),
        ("ok\n", "no setting"),
        ("", "no setting"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(e.contains(words), "{text:?}: {e}");
    }
    assert!(parse(&"x".repeat(MAX_DUMP_BYTES + 1)).is_err());
}

#[test]
fn a_listing_that_agrees_finds_nothing() {
    assert!(kinds(GOOD).is_empty(), "{:?}", kinds(GOOD));
}

#[test]
fn each_setting_that_disagrees_is_found_and_the_unknown_is_serious() {
    let swap = |from: &str, to: &str| kinds(&GOOD.replace(from, to));
    assert_eq!(swap("$32=1", "$32=0"), vec![Kind::LaserModeOff]);
    assert_eq!(swap("$32=1", "$32=2"), vec![Kind::LaserModeOff]);
    assert_eq!(swap("$32=1\n", ""), vec![Kind::LaserModeUnknown]);
    assert_eq!(swap("$30=1000", "$30=255"), vec![Kind::SMaxDiffers]);
    assert_eq!(swap("$30=1000\n", ""), vec![Kind::SMaxUnknown]);
    assert_eq!(swap("$31=0", "$31=10"), vec![Kind::MinPower]);
    assert_eq!(swap("$13=0", "$13=1"), vec![Kind::Inches]);
    assert_eq!(swap("$130=400", "$130=300"), vec![Kind::BedSmaller]);
    assert_eq!(swap("$111=6000.000", "$111=2000"), vec![Kind::SpeedLower]);
    for kind in Kind::ALL {
        assert_eq!(
            kind.serious(),
            matches!(
                kind,
                Kind::LaserModeOff | Kind::LaserModeUnknown | Kind::SMaxDiffers | Kind::SMaxUnknown
            )
        );
    }
}

#[test]
fn every_finding_is_said_in_the_eight_languages_with_the_same_places() {
    for kind in Kind::ALL {
        let s = kind.sentences();
        for (i, text) in s.iter().enumerate() {
            assert!(!text.trim().is_empty(), "{kind:?} {i}");
            for place in ["{0}", "{1}"] {
                assert_eq!(
                    text.contains(place),
                    s[0].contains(place),
                    "{kind:?} {i} {place}"
                );
            }
            // a line of the report, not a page
            assert!(text.chars().count() <= 200, "{kind:?} {i}: {text}");
            assert!(!s[..i].contains(text), "{kind:?} {i} repeats a language");
        }
    }
}

#[test]
fn the_verdict_of_a_program_takes_the_listing_into_account() {
    let program = b"G21\nG90\nF3000\nM4 S100\nG1 X10\nM5\nM2\n";
    let limits = Limits::default();
    let mut v = verify(program, &limits).unwrap();
    v.add_controller(&parse(GOOD).unwrap(), &limits, false);
    assert!(v.passes(), "{:?}", v.problems);
    assert!(v.guards.contains(&"controller_settings"));
    let mut v = verify(program, &limits).unwrap();
    v.add_controller(
        &parse(&GOOD.replace("$32=1", "$32=0").replace("=6000.000", "=1000")).unwrap(),
        &limits,
        false,
    );
    assert!(!v.passes());
    assert!(
        v.problems.iter().any(|p| p.contains("$32=0")),
        "{:?}",
        v.problems
    );
    assert!(v.notes.iter().any(|n| n.contains("3000")), "{:?}", v.notes);
}

#[test]
fn every_alarm_and_error_of_the_table_is_said_in_the_eight_languages() {
    for (table, prefix) in [(ALARMS, "ALARM:"), (ERRORS, "error:")] {
        for (k, (number, sentences)) in table.iter().enumerate() {
            assert!(
                table[..k].iter().all(|(n, _)| n < number),
                "{prefix}{number}"
            );
            for (i, lang) in Lang::ALL.into_iter().enumerate() {
                let got = explain(&format!("{prefix}{number}"), lang).unwrap();
                assert_eq!(got, sentences[i]);
                assert!(
                    !got.trim().is_empty() && got.chars().count() <= 200,
                    "{got}"
                );
                assert!(!sentences[..i].contains(&sentences[i]), "{prefix}{number}");
            }
        }
    }
    assert_eq!(ALARMS.len(), 9);
}

#[test]
fn an_answer_is_read_in_its_ways_and_an_unknown_number_is_never_guessed() {
    assert_eq!(read_answer("ALARM:2"), Some((Answer::Alarm, 2)));
    assert_eq!(read_answer(" alarm 9 "), Some((Answer::Alarm, 9)));
    assert_eq!(read_answer("Error:11"), Some((Answer::Error, 11)));
    for bad in [
        "",
        "ALARM",
        "error:",
        "error:x",
        "ok",
        "alarm:-1",
        "error:99999999999",
    ] {
        assert_eq!(read_answer(bad), None, "{bad}");
        assert!(explain(bad, Lang::En).is_err(), "{bad}");
    }
    assert!(explain("error:200", Lang::En)
        .unwrap_err()
        .contains("not in the table"));
}

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

fn file(name: &str, text: &str) -> String {
    let path = format!("{}/controller-{name}", env!("CARGO_TARGET_TMPDIR"));
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn the_command_line_compares_a_listing_and_ties_it_to_the_check_of_a_program() {
    let good = file("good.txt", GOOD);
    let off = file("off.txt", &GOOD.replace("$32=1", "$32=0"));
    let (ok, out, err) = run(&["controller", &good, "--lang", "es"]);
    assert!(ok, "{err}");
    assert!(out.contains("0 finding(s)"), "{out}");
    let (ok, out, err) = run(&["controller", &off, "--lang", "es"]);
    assert!(!ok);
    assert!(out.contains("modo láser"), "{out}");
    assert!(err.contains("serious"), "{err}");
    let (ok, out, _) = run(&["controller", &good, "--smax", "255", "--lang", "en"]);
    assert!(
        !ok && out.contains("S1000") && out.contains("S255"),
        "{out}"
    );
    let program = file("p.nc", "G21\nG90\nF600\nM4 S100\nG1 X10\nM5\nM2\n");
    let (ok, out, err) = run(&["verify", &program, "--controller", &good]);
    assert!(ok, "{out}{err}");
    let (ok, out, _) = run(&["verify", &program, "--controller", &off, "--json"]);
    assert!(!ok);
    assert!(
        out.contains("controller_settings") && out.contains("$32=0"),
        "{out}"
    );
    let (ok, _, err) = run(&["verify", &program, "--controller", "no-such-file"]);
    assert!(!ok && err.contains("no-such-file"), "{err}");
}

#[test]
fn the_command_line_explains_an_answer() {
    let (ok, out, err) = run(&["explain", "ALARM:1", "--lang", "de"]);
    assert!(ok, "{err}");
    assert!(out.contains("Endschalter"), "{out}");
    let (ok, out, _) = run(&["explain", "error", "11", "--lang", "en"]);
    assert!(ok && out.contains("longer"), "{out}");
    let (ok, _, err) = run(&["explain", "error:200"]);
    assert!(!ok && err.contains("not in the table"), "{err}");
    assert!(!run(&["explain"]).0);
}
