//! Damaged drawings and pictures with a fixed seed: every cut of the example drawings and many copies of them with bytes changed, inserted or cut are read without a panic, and a drawing
//! that is read holds no shape with a number that is not finite. The data of the text, the QR code and the barcode, the numbers of their settings, the positioning marks, the focus grid, the settings file and the
//! G-code that is read back are fuzzed the same way. The seed is fixed, so a failure is the same failure every time.

use eh3d_phoenix_laser::barcode;
use eh3d_phoenix_laser::calibrate::focus_grid;
use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::frame::marks_program;
use eh3d_phoenix_laser::preview;
use eh3d_phoenix_laser::qr;
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::text as laser_text;
use eh3d_phoenix_laser::vconfig::{
    self, BarcodeSettings, FocusSettings, MarksSettings, QrSettings, TextSettings,
};
use eh3d_phoenix_laser::vector::{Drawing, ReadOptions};
use eh3d_phoenix_laser::verify::{verify, Limits};

/// A small generator of numbers (xorshift64): the same sequence on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

impl Rng {
    /// One of the values of a list.
    fn pick<T: Copy>(&mut self, list: &[T]) -> T {
        list[self.below(list.len())]
    }
}

fn example(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn finite(drawing: &Drawing) {
    for shape in &drawing.shapes {
        let text = format!("{shape:?}");
        assert!(!text.contains("NaN") && !text.contains("inf"), "{text}");
    }
}

fn read(kind: &str, bytes: &[u8]) {
    let opts = ReadOptions::default();
    let drawing = if kind == "dxf" {
        read_dxf(bytes, &opts)
    } else {
        read_svg(bytes, &opts)
    };
    if let Ok(d) = drawing {
        finite(&d);
    }
}

#[test]
fn the_examples_are_read_and_every_cut_of_them_is_read_without_a_panic() {
    for (name, kind) in [("plate.dxf", "dxf"), ("plate.svg", "svg")] {
        let bytes = example(name);
        let whole = if kind == "dxf" {
            read_dxf(&bytes, &ReadOptions::default())
        } else {
            read_svg(&bytes, &ReadOptions::default())
        };
        assert!(!whole.unwrap().shapes.is_empty(), "{name}");
        for end in (0..bytes.len()).step_by(7) {
            read(kind, &bytes[..end]);
        }
    }
}

const PIECES: &[u8] = b"0123456789.-eE\n \t<>/=\"ZMLCQ";

#[test]
fn copies_with_bytes_changed_inserted_or_cut_are_read_without_a_panic() {
    let mut rng = Rng(0xA076_1D64_78BD_642F);
    for (name, kind) in [("plate.dxf", "dxf"), ("plate.svg", "svg")] {
        let bytes = example(name);
        for round in 0..300 {
            let mut copy = bytes.clone();
            for _ in 0..1 + round % 5 {
                let at = rng.below(copy.len());
                match rng.below(4) {
                    0 => copy[at] = PIECES[rng.below(PIECES.len())],
                    1 => copy.insert(at, rng.next() as u8),
                    2 => {
                        copy.remove(at);
                    }
                    _ => copy.truncate(at.max(1)),
                }
                if copy.is_empty() {
                    copy.push(b'0');
                }
            }
            read(kind, &copy);
        }
    }
}

#[test]
fn bytes_that_are_not_a_drawing_are_read_without_a_panic() {
    let mut rng = Rng(3);
    for len in [0usize, 1, 5, 64, 1000, 8192] {
        let junk: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        read("dxf", &junk);
        read("svg", &junk);
    }
    read(
        "svg",
        b"<svg width=\"1e999\" height=\"-1\"><path d=\"M 0 0 L 1e308 1e308 L -1e308 5 Z\"/></svg>",
    );
    read("svg", b"<svg><circle cx=\"nan\" cy=\"1\" r=\"inf\"/></svg>");
}

/// Data of every shape: only figures, printable characters, or a mix with controls, accents and characters of more than one byte.
fn random_data(rng: &mut Rng) -> String {
    const POOL: &[char] = &[
        ' ',
        '!',
        'A',
        'z',
        '0',
        '9',
        '-',
        '/',
        '~',
        '\n',
        '\t',
        '\0',
        '\u{7f}',
        '\u{e9}',
        '\u{20ac}',
        '\u{65e5}',
        '\u{1f600}',
        '(',
        ')',
        ';',
        '%',
    ];
    let len = rng.below(31);
    match rng.below(3) {
        0 => (0..len)
            .map(|_| char::from(b'0' + rng.below(10) as u8))
            .collect(),
        1 => (0..len)
            .map(|_| char::from(b' ' + rng.below(95) as u8))
            .collect(),
        _ => (0..len).map(|_| rng.pick(POOL)).collect(),
    }
}

fn the_limits_of_a_bed(bx: f64, by: f64) -> Limits {
    Limits {
        bed_mm: Some((bx, by)),
        ..Limits::default()
    }
}

#[test]
fn random_data_is_refused_or_burnt_and_what_is_burnt_passes_the_check_and_does_not_hold_the_data() {
    let mut rng = Rng(0x5EED_0128);
    let limits = the_limits_of_a_bed(200.0, 200.0);
    let machine = qr::Machine {
        s_max: 1000,
        max_power_percent: 100.0,
        max_speed_mm_min: 30_000.0,
        bed_mm: Some((200.0, 200.0)),
    };
    let text_machine = laser_text::Machine {
        s_max: 1000,
        max_power_percent: 100.0,
        max_speed_mm_min: 30_000.0,
        bed_mm: Some((200.0, 200.0)),
    };
    let (mut written, mut refused, mut sets_c) = (0, 0, 0);
    for _ in 0..120 {
        let data = random_data(&mut rng);
        let origin = (rng.below(40) as f64, rng.below(40) as f64);
        let mut outputs: Vec<(&str, String)> = Vec::new();
        match laser_text::program(&data, origin, &TextSettings::default(), &text_machine) {
            Ok((g, _)) => outputs.push(("text", g)),
            Err(_) => refused += 1,
        }
        match qr::program(&data, origin, &QrSettings::default(), &machine) {
            Ok((g, _)) => outputs.push(("qr", g)),
            Err(_) => refused += 1,
        }
        match barcode::program(&data, origin, &BarcodeSettings::default(), &machine) {
            Ok((g, code)) => {
                // what is burnt is what was asked: the barcode reads back into the data
                assert_eq!(
                    barcode::decode(&code.modules).unwrap(),
                    data.as_bytes(),
                    "{data:?}"
                );
                sets_c += usize::from(code.set_c);
                outputs.push(("barcode", g));
            }
            Err(_) => refused += 1,
        }
        for (kind, g) in outputs {
            let v = verify(g.as_bytes(), &limits).unwrap();
            assert!(v.passes(), "{kind} {data:?}: {:?}", v.problems);
            assert!(
                v.burn_mm.is_finite() && v.seconds.is_finite(),
                "{kind} {data:?}"
            );
            // the data is never written into a comment of the program (a word of six letters cannot come by chance)
            if data.len() >= 6 && data.chars().all(|c| c.is_ascii_alphabetic()) {
                assert!(!g.contains(&data), "{kind}: {data}");
            }
            written += 1;
        }
    }
    // both ways were taken often, and the figures went in set C now and then
    assert!(
        written > 60 && refused > 20 && sets_c > 2,
        "{written} {refused} {sets_c}"
    );
}

impl Rng {
    /// A usual value, and now and then one that is out of range or not a number.
    fn mostly<T: Copy>(&mut self, good: &[T], bad: &[T]) -> T {
        if self.below(5) == 0 {
            self.pick(bad)
        } else {
            self.pick(good)
        }
    }
}

const BAD_REALS: &[f64] = &[
    0.0,
    0.01,
    -1.0,
    1e-9,
    1e12,
    5000.0,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
];
const BAD_WHOLES: &[u32] = &[0, 21, 81, 1_000_001, u32::MAX];

#[test]
fn random_numbers_in_the_settings_of_the_barcode_and_the_marks_are_refused_or_burn_inside_the_limits(
) {
    let mut rng = Rng(0xBA2C_0DE1);
    let (mut burnt, mut refused) = (0, 0);
    for _ in 0..800 {
        let machine = qr::Machine {
            s_max: rng.mostly(&[255, 1000, 10_000], &[0, 100_001]),
            max_power_percent: rng.mostly(&[50.0, 100.0], BAD_REALS),
            max_speed_mm_min: rng.mostly(&[5000.0, 30_000.0], BAD_REALS),
            bed_mm: rng.pick(&[None, Some((200.0, 200.0)), Some((30.0, 30.0))]),
        };
        let limits = Limits {
            s_max: machine.s_max,
            max_power_percent: machine.max_power_percent,
            max_speed_mm_min: machine.max_speed_mm_min,
            bed_mm: machine.bed_mm,
            ..Limits::default()
        };
        let origin = (
            rng.mostly(&[0.0, 5.0, 10.0], BAD_REALS),
            rng.mostly(&[0.0, 5.0, 10.0], BAD_REALS),
        );
        let b = BarcodeSettings {
            module_mm: rng.mostly(&[0.2, 0.4, 1.0], BAD_REALS),
            height_mm: rng.mostly(&[5.0, 10.0, 20.0], BAD_REALS),
            line_mm: rng.mostly(&[0.1, 0.2], BAD_REALS),
            power_percent: rng.mostly(&[5.0, 10.0, 20.0], BAD_REALS),
            speed_mm_min: rng.mostly(&[500.0, 1000.0, 3000.0], BAD_REALS),
            passes: rng.mostly(&[1, 2, 3], BAD_WHOLES),
            max_chars: rng.mostly(&[20, 40], BAD_WHOLES),
            max_width_mm: rng.mostly(&[150.0, 300.0], BAD_REALS),
            max_lines: rng.mostly(&[100_000], BAD_WHOLES),
            max_minutes: rng.mostly(&[60.0, 240.0], BAD_REALS),
            mirror: rng.below(2) == 0,
        };
        let k = MarksSettings {
            arm_mm: rng.mostly(&[2.0, 5.0, 10.0], BAD_REALS),
            margin_mm: rng.mostly(&[0.0, 1.0, 2.0], BAD_REALS),
            power_percent: rng.mostly(&[5.0, 10.0, 20.0], BAD_REALS),
            speed_mm_min: rng.mostly(&[500.0, 1000.0, 3000.0], BAD_REALS),
            passes: rng.mostly(&[1, 2, 3], BAD_WHOLES),
            centre_cross: rng.below(2) == 0,
        };
        let size = (
            rng.mostly(&[20.0, 40.0, 100.0], BAD_REALS),
            rng.mostly(&[20.0, 40.0, 100.0], BAD_REALS),
        );
        let mut programs: Vec<String> = Vec::new();
        if let Ok((g, _)) = barcode::program("EH3D-42", origin, &b, &machine) {
            // what is let through is what the settings allow
            assert!(b.validate().is_ok());
            programs.push(g);
        } else {
            refused += 1;
        }
        if let Ok(g) = marks_program(size, origin, &k, &machine) {
            assert!(k.validate().is_ok());
            programs.push(g);
        } else {
            refused += 1;
        }
        for g in programs {
            let v = verify(g.as_bytes(), &limits).unwrap();
            assert!(v.passes(), "{:?}\n{machine:?}\n{g}", v.problems);
            burnt += 1;
        }
    }
    assert!(burnt > 50 && refused > 100, "{burnt} {refused}");
}

#[test]
fn random_powers_and_steps_in_the_focus_grid_are_refused_or_stay_under_the_limit() {
    let mut rng = Rng(0xF0C5_0001);
    let (mut made, mut refused) = (0, 0);
    for _ in 0..600 {
        let f = FocusSettings {
            z_start_mm: rng.mostly(&[0.0, 5.0, -3.0], &[101.0, f64::NAN]),
            z_step_mm: rng.mostly(&[-0.5, 0.25, -1.0], &[0.0, -20.0, 21.0, f64::NAN]),
            steps: rng.mostly(&[2, 9, 30], &[0, 1, 31]),
            length_mm: rng.mostly(&[1.0, 10.0, 100.0], &[0.5, 101.0]),
            spacing_mm: rng.mostly(&[0.5, 2.0, 20.0], &[0.1, 21.0]),
            feed_mm_min: rng.mostly(&[1000.0, 30_000.0], &[0.5, 30_001.0]),
            power_percent: 30.0,
        };
        let count = if rng.below(6) == 0 {
            rng.pick(&[0, 11, 12])
        } else {
            1 + rng.below(10)
        };
        let powers: Vec<f64> = (0..count)
            .map(|_| {
                rng.mostly(
                    &[1.0, 10.0, 30.0, 50.0, 99.9, 100.0],
                    &[0.0, 0.001, 100.5, -5.0, f64::NAN, f64::INFINITY],
                )
            })
            .collect();
        let s_max = rng.mostly(&[1000u32, 255], &[0, 100_001]);
        let limit = rng.mostly(&[50.0, 100.0], &[5.0, 0.0]);
        match focus_grid(&f, &powers, s_max, limit) {
            Ok(g) => {
                made += 1;
                assert!((1..=10).contains(&powers.len()), "{powers:?}");
                assert!(
                    powers.iter().all(|p| *p > 0.0 && *p <= limit),
                    "{powers:?} {limit}"
                );
                let l = Limits {
                    s_max,
                    max_power_percent: limit,
                    ..Limits::default()
                };
                let v = verify(g.as_bytes(), &l).unwrap();
                assert!(v.passes(), "{:?}", v.problems);
                assert_eq!(g.matches("G1 X").count(), f.steps as usize * powers.len());
            }
            Err(_) => refused += 1,
        }
    }
    assert!(made > 10 && refused > 100, "{made} {refused}");
}

#[test]
fn g_code_with_bytes_changed_is_read_by_the_preview_and_the_check_without_a_panic() {
    let machine = qr::Machine {
        s_max: 1000,
        max_power_percent: 100.0,
        max_speed_mm_min: 30_000.0,
        bed_mm: None,
    };
    let programs = [
        barcode::program("EH3D-42", (1.0, 2.0), &BarcodeSettings::default(), &machine)
            .unwrap()
            .0,
        marks_program(
            (40.0, 20.0),
            (5.0, 5.0),
            &MarksSettings {
                centre_cross: true,
                ..MarksSettings::default()
            },
            &machine,
        )
        .unwrap(),
        focus_grid(
            &FocusSettings {
                z_step_mm: -0.5,
                steps: 4,
                ..FocusSettings::default()
            },
            &[10.0, 30.0],
            1000,
            100.0,
        )
        .unwrap(),
        qr::program("EH3D 42", (0.0, 0.0), &QrSettings::default(), &machine)
            .unwrap()
            .0,
    ];
    const TOKENS: &[&str] = &[
        "G1 X1e999 ",
        "S-5 ",
        "F0 ",
        "M30\n",
        "G2 X5 Y5 I1 J1\n",
        "(",
        ")",
        ";",
        "*",
        "\n",
        "N5 ",
        "G20\n",
        "G91\n",
        "X-1000000000 ",
        "S99999999999999999999 ",
        "F0.0000001 ",
        "M3 ",
        "M5\n",
        "G0 ",
        "G1 ",
    ];
    let mut rng = Rng(0x9C0D_E5ED);
    let options = [
        preview::Options::default(),
        preview::Options {
            s_max: Some(1000),
            laser_watts: Some(10.0),
            bed_mm: Some((50.0, 50.0)),
            ..preview::Options::default()
        },
    ];
    let mut read_ok = 0;
    for program in &programs {
        for round in 0..100 {
            let mut copy = program.clone().into_bytes();
            for _ in 0..1 + round % 6 {
                let at = rng.below(copy.len());
                match rng.below(5) {
                    0 => copy[at] = rng.next() as u8,
                    1 => {
                        let token = rng.pick(TOKENS).as_bytes().to_vec();
                        copy.splice(at..at, token);
                    }
                    2 => {
                        copy.remove(at);
                    }
                    3 => copy.truncate(at.max(1)),
                    _ => copy.insert(at, b' '),
                }
                if copy.is_empty() {
                    copy.push(b'0');
                }
            }
            let text = String::from_utf8_lossy(&copy).into_owned();
            let parsed = preview::read(&text);
            for o in &options {
                if let Ok(r) = preview::analyse(&parsed, o) {
                    read_ok += 1;
                    assert!(
                        r.seconds.is_finite()
                            && r.burn_mm.is_finite()
                            && r.travel_mm.is_finite()
                            && r.burn_seconds.is_finite(),
                        "{text}"
                    );
                    assert!(
                        r.energy_j.is_none_or(f64::is_finite)
                            && r.mean_power_w.is_none_or(f64::is_finite),
                        "{text}"
                    );
                }
            }
            // the check says yes or no, or that it cannot read it, and never panics
            let _ = verify(&copy, &Limits::default());
        }
    }
    assert!(read_ok > 400, "{read_ok}");
}

#[test]
fn a_settings_file_with_bytes_changed_is_read_or_refused_and_what_is_read_is_inside_the_ranges() {
    let original = String::from_utf8(example("plate.cfg")).unwrap();
    let words = [
        "[barcode]\n",
        "[marks]\n",
        "[qr]\n",
        "max_minutes = 1e9\n",
        "arm_mm = -4\n",
        "passes = 99\n",
        "mirror = maybe\n",
        "= \n",
        "[",
        "]",
        "\n",
        "centre_cross = on\n",
        "line_mm = 0\n",
        "max_chars = 4294967296\n",
    ];
    let mut rng = Rng(0xC0F1_6000);
    let (mut good, mut bad) = (0, 0);
    for round in 0..600 {
        let mut text = original.clone();
        for _ in 0..1 + round % 4 {
            let at = (0..=text.len())
                .rev()
                .find(|i| text.is_char_boundary(*i) && *i <= rng.below(text.len() + 1))
                .unwrap_or(0);
            match rng.below(3) {
                0 => text.insert_str(at, rng.pick(&words)),
                1 => text.truncate(at),
                _ => {
                    if let Some(c) = text[at..].chars().next() {
                        text.replace_range(at..at + c.len_utf8(), "9");
                    }
                }
            }
        }
        match vconfig::parse(&text) {
            Ok(s) => {
                good += 1;
                assert!(
                    s.barcode.validate().is_ok()
                        && s.marks.validate().is_ok()
                        && s.qr.validate().is_ok()
                        && s.text.validate().is_ok()
                );
            }
            Err(e) => {
                bad += 1;
                assert!(!e.is_empty());
            }
        }
    }
    assert!(good > 20 && bad > 100, "{good} {bad}");
}

#[test]
fn the_same_work_is_written_byte_for_byte_every_time() {
    let machine = qr::Machine {
        s_max: 1000,
        max_power_percent: 100.0,
        max_speed_mm_min: 30_000.0,
        bed_mm: None,
    };
    for _ in 0..3 {
        let a = barcode::program("EH3D-42", (1.0, 2.0), &BarcodeSettings::default(), &machine)
            .unwrap()
            .0;
        let b = barcode::program("EH3D-42", (1.0, 2.0), &BarcodeSettings::default(), &machine)
            .unwrap()
            .0;
        assert_eq!(a, b);
        let k = MarksSettings::default();
        assert_eq!(
            marks_program((40.0, 20.0), (0.0, 0.0), &k, &machine).unwrap(),
            marks_program((40.0, 20.0), (0.0, 0.0), &k, &machine).unwrap()
        );
    }
}

/// A name of a layer made of whatever the generator brings: quotes, backslashes, controls, accents and characters of several bytes.
fn wild_name(rng: &mut Rng) -> String {
    const PIECES: [&str; 14] = [
        "a", "Z", "7", "\"", "\\", "/", "\n", "\t", "\u{1}", "é", "ß", "日本", "😀", " ",
    ];
    (0..1 + rng.below(12))
        .map(|_| PIECES[rng.below(PIECES.len())])
        .collect()
}

fn wild_number(rng: &mut Rng) -> f64 {
    match rng.below(6) {
        0 => 0.0,
        1 => -(rng.below(100_000) as f64) / 7.0,
        2 => rng.below(1_000_000) as f64 / 1000.0,
        3 => 1e-9 * rng.below(1000) as f64,
        4 => 1e9 * rng.below(1000) as f64,
        _ => rng.below(500) as f64,
    }
}

#[test]
fn a_vector_job_with_wild_names_and_numbers_is_written_as_json_that_reads_back_into_it() {
    use eh3d_phoenix_laser::server::{vector_request, VectorJob, VectorOp, VectorPath, APP};
    use eh3d_serverclient::Json;
    let mut rng = Rng(0x5eed_1a5e_0000_0009);
    for _ in 0..300 {
        let mut job = VectorJob::default();
        for _ in 0..1 + rng.below(4) {
            let layer = wild_name(&mut rng);
            let points = (0..2 + rng.below(8))
                .map(|_| (wild_number(&mut rng), wild_number(&mut rng)))
                .collect();
            job.paths.push(VectorPath {
                layer: layer.clone(),
                points,
                closed: rng.below(2) == 0,
            });
            job.ops.push(VectorOp {
                layer,
                mode: rng.pick(&["cut", "engrave"]),
                power_pct: wild_number(&mut rng),
                speed_mm_min: wild_number(&mut rng),
                passes: rng.below(100) as u32,
            });
        }
        let name = wild_name(&mut rng);
        let body = vector_request("laser-1", &name, &job);
        let v = Json::parse(&body).unwrap_or_else(|e| panic!("{e}: {body}"));
        assert_eq!(v.get("name").and_then(Json::as_str), Some(name.as_str()));
        assert_eq!(v.get("app").and_then(Json::as_str), Some(APP));
        let r = v.get("request").unwrap();
        let paths = r.get("paths").and_then(Json::as_array).unwrap();
        let ops = r.get("operations").and_then(Json::as_array).unwrap();
        assert_eq!((paths.len(), ops.len()), (job.paths.len(), job.ops.len()));
        for (p, back) in job.paths.iter().zip(paths) {
            assert_eq!(
                back.get("layer").and_then(Json::as_str),
                Some(p.layer.as_str())
            );
            assert_eq!(back.get("closed").and_then(Json::as_bool), Some(p.closed));
            let pts = back.get("points").and_then(Json::as_array).unwrap();
            assert_eq!(pts.len(), p.points.len());
            for (q, b) in p.points.iter().zip(pts) {
                let xy = b.as_array().unwrap();
                assert_eq!(xy, [Json::Num(q.0), Json::Num(q.1)], "{q:?}");
            }
        }
        for (o, back) in job.ops.iter().zip(ops) {
            assert_eq!(back.get("power_pct"), Some(&Json::Num(o.power_pct)));
            assert_eq!(back.get("speed"), Some(&Json::Num(o.speed_mm_min)));
            assert_eq!(
                back.get("passes").and_then(Json::as_u64),
                Some(u64::from(o.passes))
            );
        }
    }
}

#[test]
fn a_vector_job_with_wild_numbers_is_refused_or_passes_the_limits_and_never_panics() {
    use eh3d_phoenix_laser::server::{check_vector_before_send, VectorJob, VectorOp, VectorPath};
    use eh3d_phoenix_laser::vconfig::Settings;
    let mut rng = Rng(0x5eed_1a5e_0000_000a);
    for _ in 0..400 {
        let mut s = Settings::default();
        s.machine.max_power_percent = 1.0 + rng.below(100) as f64;
        s.send.max_power_percent = 1.0 + rng.below(100) as f64;
        s.machine.max_speed_mm_min = 1.0 + rng.below(20_000) as f64;
        s.machine.bed_mm = if rng.below(2) == 0 {
            None
        } else {
            Some((1.0 + rng.below(800) as f64, 1.0 + rng.below(800) as f64))
        };
        let mut job = VectorJob::default();
        for _ in 0..1 + rng.below(3) {
            let layer = wild_name(&mut rng);
            job.paths.push(VectorPath {
                layer: layer.clone(),
                points: (0..2 + rng.below(6))
                    .map(|_| {
                        (
                            wild_number(&mut rng) % 1000.0,
                            wild_number(&mut rng) % 1000.0,
                        )
                    })
                    .collect(),
                closed: false,
            });
            job.ops.push(VectorOp {
                layer,
                mode: "cut",
                power_pct: wild_number(&mut rng) % 120.0,
                speed_mm_min: wild_number(&mut rng) % 30_000.0,
                passes: 1,
            });
        }
        let verdict = check_vector_before_send(&job, &s);
        // the lower of the limit of the machine and the limit of what is sent
        let cap = s.machine.max_power_percent.min(s.send.max_power_percent);
        let over_power = job.ops.iter().any(|o| o.power_pct > cap);
        let over_speed = job
            .ops
            .iter()
            .any(|o| o.speed_mm_min > s.machine.max_speed_mm_min);
        let outside = s.machine.bed_mm.is_some_and(|(bx, by)| {
            job.paths
                .iter()
                .flat_map(|p| p.points.iter())
                .any(|q| q.0 < 0.0 || q.1 < 0.0 || q.0 > bx || q.1 > by)
        });
        // a power below 0 and a speed of 0 or less are refused as well, whatever the limits say
        let bad_number = job
            .ops
            .iter()
            .any(|o| o.power_pct < 0.0 || o.speed_mm_min <= 0.0);
        assert_eq!(
            verdict.is_ok(),
            !(bad_number || over_power || over_speed || outside),
            "{verdict:?}"
        );
    }
}

#[test]
fn a_test_fire_with_wild_numbers_is_refused_or_sent_as_numbers_that_read_back() {
    use eh3d_phoenix_laser::server::{check_test_fire, test_fire_request};
    use eh3d_phoenix_laser::vconfig::Settings;
    use eh3d_serverclient::Json;
    let mut rng = Rng(0x5eed_1a5e_0000_000b);
    let s = Settings::default();
    for _ in 0..500 {
        let power = match rng.below(8) {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => -wild_number(&mut rng),
            3 | 4 => 40.0 * ((rng.below(1000) as f64) / 1000.0),
            _ => wild_number(&mut rng) % 150.0,
        };
        let seconds = match rng.below(8) {
            0 => f64::NAN,
            1 => f64::NEG_INFINITY,
            _ => wild_number(&mut rng) % 20.0,
        };
        let verdict = check_test_fire(power, seconds, &s);
        let fine = power.is_finite()
            && power > 0.0
            && power <= s.machine.max_power_percent.min(s.send.max_power_percent)
            && seconds.is_finite()
            && (0.01..=10.0).contains(&seconds);
        assert_eq!(verdict.is_ok(), fine, "{power} {seconds}");
        if fine {
            let v = Json::parse(&test_fire_request(power, seconds)).unwrap();
            assert_eq!(v.get("power_pct"), Some(&Json::Num(power)));
            assert_eq!(v.get("seconds"), Some(&Json::Num(seconds)));
        }
    }
}

#[test]
fn a_damaged_answer_of_the_server_is_read_without_a_panic() {
    use eh3d_phoenix_laser::server::{FireInfo, JobInfo, MachineInfo};
    use eh3d_serverclient::Json;
    let whole = "{\"id\":\"w-1\",\"name\":\"L\",\"state\":\"running\",\"done\":30,\"total\":120,\"message\":\"x\",\"fired\":true,\"power_pct\":1,\"seconds\":0.2,\"detail\":{\"estop_latched\":true,\"beam_on\":false}}";
    let mut rng = Rng(0x5eed_1a5e_0000_000c);
    let read = |text: &str| {
        if let Ok(v) = Json::parse(text) {
            let j = JobInfo::from_json(&v);
            let _ = (j.percent(), j.finished(), j.active(), j.held());
            let m = MachineInfo::from_json(&v);
            let _ = (m.latched, m.beam_on);
            let f = FireInfo::from_json(&v);
            assert!(f.power_pct.is_finite() && f.seconds.is_finite());
        }
    };
    assert!(Json::parse(whole).is_ok());
    for end in 0..whole.len() {
        if whole.is_char_boundary(end) {
            read(&whole[..end]);
        }
    }
    let bytes = whole.as_bytes();
    for _ in 0..2000 {
        let mut copy = bytes.to_vec();
        for _ in 0..1 + rng.below(4) {
            let at = rng.below(copy.len());
            match rng.below(3) {
                0 => copy[at] = rng.next() as u8,
                1 => copy.insert(at, rng.next() as u8),
                _ => {
                    copy.remove(at);
                }
            }
        }
        read(&String::from_utf8_lossy(&copy));
    }
    // numbers that are not whole or are huge give zeros and a percent that holds
    let j = JobInfo::from_json(&Json::parse("{\"done\":1e300,\"total\":-5}").unwrap());
    assert_eq!((j.done, j.total, j.percent()), (0, 0, 0));
}
