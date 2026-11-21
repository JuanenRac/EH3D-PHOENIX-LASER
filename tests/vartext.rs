//! The variable text: a counter and the columns of a CSV file become one text for each row, and `phoenix-laser text` writes one program for each. CSV files are written here; nothing in
//! them is ever run, a row that cannot be burnt stops the whole series before a file is written, and the reader is fuzzed with a fixed seed.

use eh3d_phoenix_laser::vartext::{
    parse_csv, plan, render, Counter, Csv, CODE_COUNTER, CODE_CSV, CODE_FIELD, CODE_SERIES,
    MAX_TEXT_CHARS,
};
use eh3d_phoenix_laser::vector::Limits;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-vartext-{}-{tag}", std::process::id()));
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

    fn count(&self, prefix: &str) -> usize {
        fs::read_dir(&self.0)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
            .count()
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

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

fn lim() -> Limits {
    Limits::default()
}

const THREE: &str = "name,part\nANA,A1\nBOB,B2\nCAL,C3\n";

// ---------------------------------------------------------------- the CSV file

#[test]
fn a_csv_file_is_read_with_its_header_and_rows() {
    let c = parse_csv(THREE, &lim()).unwrap();
    assert_eq!(c.header, ["name", "part"]);
    assert_eq!(c.rows.len(), 3);
    assert_eq!(c.rows[1], ["BOB", "B2"]);
    assert_eq!(c.column("part"), Some(1));
    assert_eq!(c.column("nothing"), None);
}

#[test]
fn quotes_separators_new_lines_blank_lines_and_the_byte_order_mark_are_read() {
    let text = "\u{feff}name;note\r\n\"A;B\";\"say \"\"hi\"\"\"\r\n\r\n\"two\nlines\";x\r\n";
    let c = parse_csv(text, &lim()).unwrap();
    assert_eq!(c.header, ["name", "note"]);
    assert_eq!(c.rows.len(), 2);
    assert_eq!(c.rows[0], ["A;B", "say \"hi\""]);
    assert_eq!(c.rows[1], ["two\nlines", "x"]);
    // the last row needs no new line, and an empty field is an empty text
    let c = parse_csv("a,b\n1,\n,2", &lim()).unwrap();
    assert_eq!(c.rows, [["1", ""], ["", "2"]]);
}

#[test]
fn a_csv_file_that_is_wrong_is_refused_with_its_code_and_the_row() {
    for (text, want) in [
        ("", "the file is empty"),
        ("a,b\n1\n", "row 1 has 1 fields"),
        ("a,b\n1,2,3\n", "row 1 has 3 fields"),
        ("a,a\n1,2\n", "named twice"),
        ("a,\n1,2\n", "column 2 has no name"),
        ("a,b\n\"1,2\n", "not closed"),
    ] {
        let e = parse_csv(text, &lim()).unwrap_err();
        assert!(e.starts_with(CODE_CSV) && e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn the_limits_of_the_csv_file_hold() {
    let mut l = lim();
    l.max_csv_bytes = 20;
    let e = parse_csv(THREE, &l).unwrap_err();
    assert!(e.contains("max_csv_bytes"), "{e}");
    let mut l = lim();
    l.max_csv_rows = 2;
    let e = parse_csv(THREE, &l).unwrap_err();
    assert!(e.contains("max_csv_rows"), "{e}");
    l.max_csv_rows = 3;
    assert!(parse_csv(THREE, &l).is_ok());
    let mut l = lim();
    l.max_csv_columns = 1;
    let e = parse_csv(THREE, &l).unwrap_err();
    assert!(e.contains("max_csv_columns"), "{e}");
    let long = format!("a\n{}\n", "x".repeat(2000));
    assert!(parse_csv(&long, &lim())
        .unwrap_err()
        .contains("longer than"));
}

// ---------------------------------------------------------------- the places of the text

fn csv() -> Csv {
    parse_csv(THREE, &lim()).unwrap()
}

#[test]
fn the_places_are_filled_from_the_counter_the_row_and_the_columns() {
    let c = csv();
    let fields = Some((&c, c.rows[1].as_slice()));
    assert_eq!(render("{name}-{part}", fields, None, 2).unwrap(), "BOB-B2");
    assert_eq!(render("N{row}", fields, None, 2).unwrap(), "N2");
    assert_eq!(render("{counter}", None, Some(7), 1).unwrap(), "7");
    assert_eq!(render("{counter:4}", None, Some(7), 1).unwrap(), "0007");
    assert_eq!(render("{counter:4}", None, Some(-7), 1).unwrap(), "-0007");
    assert_eq!(
        render("{counter:2}", None, Some(12345), 1).unwrap(),
        "12345"
    );
    assert_eq!(render("{ counter : 3 }", None, Some(5), 1).unwrap(), "005");
    // braces written twice are the braces themselves, and a text without places is the same text
    assert_eq!(render("{{x}} }}", None, None, 1).unwrap(), "{x} }");
    assert_eq!(render("PLAIN 1", None, None, 1).unwrap(), "PLAIN 1");
}

#[test]
fn a_place_that_names_nothing_is_an_error_and_nothing_is_guessed() {
    let c = csv();
    let fields = Some((&c, c.rows[0].as_slice()));
    for (text, with_csv, with_counter) in [
        ("{nothing}", true, false),
        ("{name}", false, false),
        ("{counter}", true, false),
        ("{counter:0}", false, true),
        ("{counter:13}", false, true),
        ("{counter:x}", false, true),
        ("{row:3}", true, false),
        ("{name:3}", true, false),
        ("{unclosed", true, false),
        ("lonely }", true, false),
    ] {
        let e = render(
            text,
            if with_csv { fields } else { None },
            if with_counter { Some(1) } else { None },
            1,
        )
        .unwrap_err();
        assert!(e.starts_with(CODE_FIELD), "{text}: {e}");
    }
    let long = "x".repeat(MAX_TEXT_CHARS + 1);
    assert!(render(&long, None, None, 1).is_err());
}

// ---------------------------------------------------------------- the series

#[test]
fn a_csv_of_three_rows_gives_three_different_texts_and_the_counter_goes_with_it() {
    let c = csv();
    let k = Counter::new(100, 5, 99).unwrap();
    let texts = plan("{name} {counter:4}", Some(&c), Some(&k), &lim()).unwrap();
    assert_eq!(texts, ["ANA 0100", "BOB 0105", "CAL 0110"]);
    let only_counter = plan(
        "S{counter:3}",
        None,
        Some(&Counter::new(1, 1, 4).unwrap()),
        &lim(),
    )
    .unwrap();
    assert_eq!(only_counter, ["S001", "S002", "S003", "S004"]);
    // nothing to make a series from, an empty series and one that is too long
    assert!(plan("x", None, None, &lim())
        .unwrap_err()
        .starts_with(CODE_SERIES));
    assert!(
        plan("x", None, Some(&Counter::new(1, 1, 0).unwrap()), &lim())
            .unwrap_err()
            .starts_with(CODE_SERIES)
    );
    let mut l = lim();
    l.max_csv_rows = 3;
    assert!(plan("x", None, Some(&Counter::new(1, 1, 4).unwrap()), &l)
        .unwrap_err()
        .contains("max_csv_rows"));
    // the row that is wrong is named
    let e = plan("{part}", Some(&c), None, &lim()).unwrap();
    assert_eq!(e, ["A1", "B2", "C3"]);
    let e = plan("{counter}", Some(&c), None, &lim()).unwrap_err();
    assert!(e.starts_with("row 1:"), "{e}");
}

#[test]
fn a_counter_is_made_from_numbers_that_make_sense() {
    assert!(Counter::new(0, 0, 3).unwrap_err().starts_with(CODE_COUNTER));
    assert!(Counter::new(i64::MAX, 1, 3).is_err());
    let k = Counter::new(i64::from(i32::MAX), 1_000_000_000_000, 5).unwrap();
    assert!(k.value(usize::MAX).is_err());
    assert_eq!(Counter::from_range(10, 0, -5).unwrap().count, 3);
    assert_eq!(Counter::from_range(0, 9, 2).unwrap().count, 5);
    assert!(Counter::from_range(5, 0, 1).is_err());
    assert!(Counter::from_range(0, 5, 0).is_err());
    assert!(Counter::from_range(i64::MIN, i64::MAX, 1).is_err());
}

// ---------------------------------------------------------------- the command line

#[test]
fn text_writes_one_program_for_each_row_and_they_differ() {
    let s = Scratch::new("series");
    let file = s.write("people.csv", THREE);
    let out = s.path("plate.nc");
    let (code, stdout, err) = run(&[
        "text",
        &out,
        "--text",
        "{name} {counter:2}",
        "--csv",
        &file,
        "--counter",
        "7",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(stdout.contains("3 programs written"), "{stdout}");
    let first = fs::read_to_string(s.path("plate_001.nc")).unwrap();
    let second = fs::read_to_string(s.path("plate_002.nc")).unwrap();
    let third = fs::read_to_string(s.path("plate_003.nc")).unwrap();
    assert!(first != second && second != third && first != third);
    assert!(!std::path::Path::new(&out).exists());
    // the header names the CSV file and its hash, the programs end with the laser off, and they pass the check
    assert!(first.contains("people.csv"), "{first}");
    for g in [&first, &second, &third] {
        let (code, _, err) = run(&["verify", &s.write("one.nc", g)]);
        assert_eq!(code, Some(0), "{err}");
    }
    // the same call again gives the same files
    let (code, _, _) = run(&[
        "text",
        &out,
        "--text",
        "{name} {counter:2}",
        "--csv",
        &file,
        "--counter",
        "7",
    ]);
    assert_eq!(code, Some(0));
    assert_eq!(fs::read_to_string(s.path("plate_002.nc")).unwrap(), second);
}

#[test]
fn the_preview_shows_the_text_of_each_row_and_writes_nothing() {
    let s = Scratch::new("preview");
    let file = s.write("people.csv", THREE);
    let (code, stdout, err) = run(&[
        "text",
        "--text",
        "{name}-{row}",
        "--csv",
        &file,
        "--preview",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        stdout.contains("row 1: ANA-1") && stdout.contains("row 3: CAL-3"),
        "{stdout}"
    );
    assert!(stdout.contains("3 rows, nothing written"));
    assert_eq!(s.count("plate"), 0);
    let (code, _, err) = run(&["text", "--text", "X", "--preview"]);
    assert_ne!(code, Some(0));
    assert!(err.contains("--preview"), "{err}");
}

#[test]
fn one_row_that_cannot_be_burnt_stops_the_series_before_any_file_is_written() {
    let s = Scratch::new("refused");
    // a field that looks like a command is only text; here it holds characters the font has not, so the row is refused
    let file = s.write("odd.csv", "name\nANA\n$(rm -rf)\nCAL\n");
    let out = s.path("plate.nc");
    let (code, _, err) = run(&["text", &out, "--text", "{name}", "--csv", &file]);
    assert_ne!(code, Some(0));
    assert!(err.contains("row 2"), "{err}");
    assert_eq!(s.count("plate"), 0);
    // a power over the limit stops it too
    let ok = s.write("ok.csv", THREE);
    let (code, _, _) = run(&[
        "text",
        &out,
        "--text",
        "{name}",
        "--csv",
        &ok,
        "--power",
        "90",
        "--max-power",
        "50",
    ]);
    assert_ne!(code, Some(0));
    assert_eq!(s.count("plate"), 0);
    // the series never writes over the CSV file
    let (code, _, err) = run(&["text", &ok, "--text", "{name}", "--csv", &ok]);
    assert_ne!(code, Some(0));
    assert!(err.contains("written over"), "{err}");
    assert_eq!(fs::read_to_string(&ok).unwrap(), THREE);
}

#[test]
fn the_limits_of_the_settings_file_hold_for_the_series() {
    let s = Scratch::new("limits");
    let file = s.write("people.csv", THREE);
    let cfg = s.write("p.cfg", "[limits]\nmax_csv_rows = 2\n");
    let out = s.path("plate.nc");
    let (code, _, err) = run(&[
        "text", &out, "--text", "{name}", "--csv", &file, "--config", &cfg,
    ]);
    assert_ne!(code, Some(0));
    assert!(err.contains("max_csv_rows"), "{err}");
    assert_eq!(s.count("plate"), 0);
    let (code, _, err) = run(&[
        "text",
        &out,
        "--text",
        "{counter}",
        "--counter",
        "1,1,3",
        "--config",
        &cfg,
    ]);
    assert_ne!(code, Some(0));
    assert!(err.contains("max_csv_rows"), "{err}");
}

#[test]
fn the_counter_of_a_project_gives_the_series_and_a_bad_counter_is_refused() {
    let s = Scratch::new("project");
    let project = s.write(
        "serial.lbrn2",
        "<LightBurnProject><VariableText><Start Value=\"0\"/><End Value=\"6\"/><Current Value=\"2\"/><Increment Value=\"2\"/></VariableText></LightBurnProject>",
    );
    let (code, stdout, err) = run(&[
        "text",
        "--text",
        "SN{counter:3}",
        "--counter-project",
        &project,
        "--preview",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        stdout.contains("row 1: SN002") && stdout.contains("row 3: SN006"),
        "{stdout}"
    );
    assert!(stdout.contains("3 rows"));
    let none = s.write("none.lbrn2", "<LightBurnProject></LightBurnProject>");
    let (code, _, err) = run(&[
        "text",
        "--text",
        "{counter}",
        "--counter-project",
        &none,
        "--preview",
    ]);
    assert_ne!(code, Some(0));
    assert!(err.contains("no variable text"), "{err}");
    for bad in ["x", "1,0,3", "1,1", "1,1,-2", "1,2,3,4"] {
        let (code, _, _) = run(&["text", "--text", "{counter}", "--counter", bad, "--preview"]);
        assert_ne!(code, Some(0), "{bad}");
    }
    let (code, _, err) = run(&[
        "text",
        "--text",
        "{counter}",
        "--counter",
        "1",
        "--counter-project",
        &project,
        "--preview",
    ]);
    assert_ne!(code, Some(0));
    assert!(err.contains("cannot be given together"), "{err}");
}

// ---------------------------------------------------------------- a fuzz with a fixed seed

#[test]
fn damaged_csv_files_and_texts_are_read_or_refused_and_never_stop_the_program() {
    let mut rng = Lcg(0xa54f_f53a);
    let alphabet: Vec<char> = "ab,;\"\n\r{}:0123 xyz\u{e9}\u{feff}".chars().collect();
    let (mut csv_ok, mut render_ok) = (0usize, 0usize);
    let names = ["a", "b", "x"];
    for _ in 0..3000 {
        let n = (rng.next() % 80) as usize;
        let text: String = (0..n)
            .map(|_| alphabet[(rng.next() as usize) % alphabet.len()])
            .collect();
        if let Ok(c) = parse_csv(&text, &lim()) {
            csv_ok += 1;
            assert!(c.rows.iter().all(|r| r.len() == c.header.len()));
            assert!(c.rows.len() <= lim().max_csv_rows);
        }
        let table = Csv {
            header: names.iter().map(ToString::to_string).collect(),
            rows: vec![vec!["1".into(), "2".into(), "3".into()]],
        };
        if let Ok(t) = render(&text, Some((&table, table.rows[0].as_slice())), Some(5), 1) {
            render_ok += 1;
            assert!(t.chars().count() <= MAX_TEXT_CHARS);
        }
    }
    assert!(csv_ok > 0 && render_ok > 0, "{csv_ok} {render_ok}");
}
