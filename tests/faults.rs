//! What the check of a program refuses because it is dangerous or cannot be judged, tried on small programs written here. Nothing is run: the programs are only read as text.

use eh3d_phoenix_laser::preview::{read, MAX_LINES};
use eh3d_phoenix_laser::verify::{verify, Limits, Verdict};

fn check(program: &str) -> Verdict {
    verify(program.as_bytes(), &Limits::default()).unwrap()
}

/// A program that passes, with `extra` lines before its first burning move.
fn with(extra: &str) -> Verdict {
    check(&format!(
        "G21\nG90\nF600\nM4 S100\n{extra}\nG1 X10 Y0\nM5\nM2\n"
    ))
}

fn failed_with(v: &Verdict, words: &str) -> bool {
    !v.passes() && v.problems.iter().any(|p| p.contains(words))
}

#[test]
fn the_program_the_others_are_made_from_passes() {
    let v = with("");
    assert!(v.passes(), "{:?}", v.problems);
}

#[test]
fn spaces_inside_a_number_count_for_nothing_as_for_the_controller() {
    // the controller reads S1000: full power, over a limit of 20 %
    let limits = Limits {
        max_power_percent: 20.0,
        bed_mm: Some((100.0, 100.0)),
        ..Limits::default()
    };
    let v = verify(b"G21\nG90\nF600\nM4 S1 000\nG1 X10\nM5\nM2\n", &limits).unwrap();
    assert_eq!(v.max_s, 1000.0);
    assert!(failed_with(&v, "limit of power"), "{:?}", v.problems);
    let v = verify(b"G21\nG90\nF600\nM4 S100\nG1 X1 000\nM5\nM2\n", &limits).unwrap();
    assert!(failed_with(&v, "outside of the bed"), "{:?}", v.problems);
}

#[test]
fn a_danger_is_not_lost_behind_a_full_list_of_notes() {
    let notes: String = (0..40).map(|_| "G28\n").collect();
    let program = format!("G21\nG90\n{notes}M4 S100\nF600\nG0 X10\nM5\nM2\n");
    let v = check(&program);
    assert!(failed_with(&v, "during a rapid move"), "{:?}", v.problems);
    let p = read(&program);
    assert!(p.faults.iter().any(|f| f.contains("rapid move")));
    assert!(p.notes[0].contains("rapid move"), "{:?}", p.notes);
}

#[test]
fn a_program_too_long_to_be_read_whole_fails() {
    let mut program = String::from("G21\nG90\nF600\nM4 S100\nG1 X1\nM5\n");
    program.push_str(&"\n".repeat(MAX_LINES));
    program.push_str("M4 S1000\nG0 X999\n");
    let v = check(&program);
    assert!(failed_with(&v, "was not checked"), "{:?}", v.problems);
}

#[test]
fn constant_power_left_on_while_the_head_stands_fails() {
    for (body, words) in [
        ("M3 S100\nG1 X10\nG4 P2\nM5\nM2\n", "waits (G4)"),
        ("M3 S100\nG1 X10\nM0\nM5\nM2\n", "pauses"),
        ("M3 S100\nG1 X10\nM1\nM5\nM2\n", "pauses"),
        ("M3 S100\nG1 X10\nG0 Z5\nM5\nM2\n", "only the height"),
        ("M3 S100\nG1 X10\n", "ends with the laser on"),
    ] {
        let v = check(&format!("G21\nG90\nF600\n{body}"));
        assert!(failed_with(&v, words), "{body}: {:?}", v.problems);
        // with the power that follows the movement (M4), or with S at zero, the beam is out while the head stands
        let dynamic = body.replace("M3", "M4").replace("G0 Z5", "S0\nG0 Z5");
        let v = check(&format!("G21\nG90\nF600\n{dynamic}"));
        assert!(
            !v.problems.iter().any(|p| p.contains(words)),
            "{dynamic}: {:?}",
            v.problems
        );
    }
    let off = check("G21\nG90\nF600\nM3 S100\nG1 X10\nS0\nG4 P2\nM0\nM5\nM2\n");
    assert!(off.passes(), "{:?}", off.problems);
}

#[test]
fn a_command_of_the_controller_inside_a_program_fails() {
    for line in [
        "$32=0",
        "$30=255",
        "$H",
        "$X",
        "$J=G91 X10 F100",
        "$RST=*",
        "G0 X0 $X",
    ] {
        let v = with(line);
        assert!(
            failed_with(&v, "command of the controller"),
            "{line}: {:?}",
            v.problems
        );
    }
    // in a comment it is only a comment
    assert!(with("; costs $5").passes());
    assert!(with("(was $32=0)").passes());
}

#[test]
fn a_feed_in_another_meaning_and_the_outputs_of_other_controllers_fail() {
    for line in ["G93", "G95"] {
        assert!(failed_with(&with(line), "what F means"), "{line}");
    }
    for line in ["M106 S255", "M107", "M67 E0 Q50", "M62 P1", "M42 P4 S255"] {
        assert!(failed_with(&with(line), "cannot be judged"), "{line}");
    }
    assert!(with("G94\nM8\nM9\nM7").passes());
    let v = with("M6");
    assert!(v.passes(), "{:?}", v.problems);
    assert!(v.notes.iter().any(|n| n.contains("M6")), "{:?}", v.notes);
}

#[test]
fn what_cannot_be_read_fails_instead_of_being_left_out() {
    for line in [
        "G1 X#1",
        "G1 X[5+5]",
        "G1 X1.2.3",
        "G1 X--5",
        "G1 X9999999999999",
        "/G1 X500",
        "G1 X",
        "5",
        "M117 Hello",
    ] {
        let v = with(line);
        assert!(
            failed_with(&v, "cannot be judged"),
            "{line}: {:?}",
            v.problems
        );
    }
    for line in [
        "N10 G1 X+.5 Y-0. ; *57",
        "g1x5y5",
        "(a note) G1 (another) X5",
    ] {
        let v = with(line);
        assert!(v.passes(), "{line}: {:?}", v.problems);
    }
}

#[test]
fn a_power_or_a_feed_below_zero_fails() {
    assert!(failed_with(&with("S-5"), "S is below zero"));
    assert!(failed_with(&with("F-100"), "F is below zero"));
}

#[test]
fn what_the_controller_obeys_at_once_fails_in_the_code_and_is_told_in_a_comment() {
    for line in ["G1 X5 !", "~", "?", "G1 X5 \u{18}", "G1 X5 \u{e9}"] {
        assert!(
            failed_with(&with(line), "obeys at once"),
            "{line:?}: {:?}",
            with(line).problems
        );
    }
    for line in ["; done?", "(go!)"] {
        let v = with(line);
        assert!(v.passes(), "{line}: {:?}", v.problems);
        assert!(
            v.notes.iter().any(|n| n.contains("obeys at once")),
            "{line}: {:?}",
            v.notes
        );
    }
    // a byte order mark at the start of the file is thrown away
    let v = check("\u{feff}G21\nG90\nF600\nM4 S100\nG1 X10\r\nM5\nM2\n");
    assert!(v.passes() && v.notes.is_empty(), "{:?}", v);
}

#[test]
fn a_line_longer_than_the_controller_takes_fails() {
    let long = format!("G1 X1.{}", "0".repeat(90));
    let v = with(&long);
    assert!(failed_with(&v, "longer than"), "{:?}", v.problems);
    let spaced = format!("G1{}X5", " ".repeat(200));
    assert!(with(&spaced).passes(), "{:?}", with(&spaced).problems);
    let commented = format!("G1 X5 ({})", "a".repeat(200));
    assert!(with(&commented).passes());
}

#[test]
fn a_comment_in_brackets_ends_at_the_first_bracket_as_for_the_controller() {
    // the controller obeys what follows the first `)`: full power, outside of the bed
    let limits = Limits {
        max_power_percent: 20.0,
        bed_mm: Some((100.0, 100.0)),
        ..Limits::default()
    };
    let v = verify(
        b"G21\nG90\nF600\nM4 S100\n(a (b) S1000 G1 X500\nM5\nM2\n",
        &limits,
    )
    .unwrap();
    assert_eq!(v.max_s, 1000.0);
    assert!(failed_with(&v, "limit of power"), "{:?}", v.problems);
    assert!(failed_with(&v, "outside of the bed"), "{:?}", v.problems);
    // a `;` inside brackets is part of the comment
    assert!(with("(a ; b) G1 X5").passes());
    assert_eq!(read("(a ; b) G1 X5\n").moves.len(), 1);
    assert_eq!(read("G0 (a ; b) X5\n").moves.len(), 1);
    // outside of brackets it starts a comment that goes to the end
    assert_eq!(read("; (a) G1 X5\n").moves.len(), 0);
}

#[test]
fn a_line_the_controller_refuses_fails() {
    for (line, words) in [
        ("(a (b)) M5", "no comment open"),
        ("M5 )", "no comment open"),
        ("M5 *12", "checksum"),
        ("S0 E5", "does not know E5"),
        ("M5 A1", "does not know A1"),
        ("M5 G41", "does not know G41"),
        ("S0 M6", "does not know M6"),
        ("G64 G1 X5", "does not know G64"),
    ] {
        let v = with(line);
        assert!(failed_with(&v, words), "{line}: {:?}", v.problems);
    }
    // a line with only a word the controller does not know changes nothing: it is told
    for line in ["G64", "G41", "E5", "M6", "T1"] {
        let v = with(line);
        assert!(v.passes(), "{line}: {:?}", v.problems);
    }
}

#[test]
fn words_that_cannot_be_on_one_line_fail() {
    for line in [
        "M3 M5",
        "M5 M4",
        "M2 M30",
        "G0 G1 X5",
        "G90 G91",
        "G20 G21",
        "G54 G55",
        "S100 S0",
        "G1 X1 X2",
        "F100 F200",
        "G4 P1 P2",
    ] {
        let v = with(line);
        assert!(
            failed_with(&v, "cannot be on one line"),
            "{line}: {:?}",
            v.problems
        );
    }
    for line in [
        "G21 G90 G17 G94",
        "G53 G0 X0",
        "M8 M9",
        "M4 S100 G1 X5 F600",
    ] {
        let v = with(line);
        assert!(
            !v.problems
                .iter()
                .any(|p| p.contains("cannot be on one line")),
            "{line}: {:?}",
            v.problems
        );
    }
}

#[test]
fn after_the_end_of_a_program_nothing_may_move_and_the_modes_are_the_usual_ones() {
    for tail in ["G0 X5", "X5", "M3 S100", "M4", "G0 Z3"] {
        let v = check(&format!(
            "G21\nG90\nF600\nM4 S100\nG1 X10\nM5\nM2\n{tail}\nM5\n"
        ));
        assert!(
            failed_with(&v, "after the end of the program"),
            "{tail}: {:?}",
            v.problems
        );
    }
    let v = check("G21\nG90\nF600\nM4 S100\nG1 X10\nM5\nM2\nM5\nM9\n");
    assert!(v.passes(), "{:?}", v.problems);
    // the controller forgets G91 at M2, so the head goes to 5 and not 15
    let p = read("G91\nG0 X10\nM2\nX5\n");
    assert_eq!(p.moves.last().unwrap().to, (5.0, 0.0));
    assert!(!p.moves.last().unwrap().rapid, "G1 is the mode after M2");
}

#[test]
fn a_move_with_the_laser_on_before_any_way_of_moving_fails() {
    let v = check("G21\nG90\nF600\nM3 S100\nX10\nG1 X20\nM5\nM2\n");
    assert!(failed_with(&v, "before any G0 or G1"), "{:?}", v.problems);
    let v = check("G21\nG90\nF600\nX10\nM4 S100\nG1 X20\nM5\nM2\n");
    assert!(v.passes(), "{:?}", v.problems);
}

#[test]
fn a_program_that_writes_what_the_controller_keeps_fails() {
    for (line, words) in [
        ("G10 L2 P1 X0 Y0", "G10 writes"),
        ("G10 L20 P1 X0", "G10 writes"),
        ("G28.1", "G28.1 and G30.1"),
        ("G30.1", "G28.1 and G30.1"),
    ] {
        let v = with(line);
        assert!(failed_with(&v, words), "{line}: {:?}", v.problems);
    }
    let v = with("G92 X0 Y0");
    assert!(v.passes(), "{:?}", v.problems);
}

#[test]
fn random_bytes_never_stop_the_reader_and_a_fault_always_fails_the_program() {
    let alphabet: Vec<char> = "GMXYSFPE$0123456789.-+ ();*!?#\n\u{e9}".chars().collect();
    let mut seed = 0x1234_5678_9abc_def0u64;
    for _ in 0..400 {
        let text: String = (0..120)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                alphabet[(seed % alphabet.len() as u64) as usize]
            })
            .collect();
        let p = read(&text);
        let v = check(&text);
        for f in &p.faults {
            assert!(v.problems.contains(f), "{text:?}: {f}");
        }
    }
}
