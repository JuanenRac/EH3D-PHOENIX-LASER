//! The book of presets kept as text: a book is written and read back as it was (with the links of its layers), the same book gives the same text, a damaged text names its line and keeps
//! nothing, and the command that lists a book speaks the language it is asked. A series of random books with a fixed seed is written, read back and damaged.

use eh3d_phoenix_laser::commonmat::What;
use eh3d_phoenix_laser::presets::{Binding, Book, Preset, Target};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

fn preset(name: &str, power: f64, verified: bool) -> Preset {
    Preset {
        name: name.to_string(),
        material: "wood/birch-plywood".to_string(),
        thickness_mm: Some(3.0),
        what: What::Cut,
        power_percent: power,
        speed_mm_min: 600.0,
        passes: 2,
        air_assist: Some(true),
        verified,
        alternative: false,
    }
}

fn sample() -> Book {
    let mut b = Book::new();
    b.add(preset("birch cut", 70.0, false)).unwrap();
    let mut e = preset("birch mark", 15.5, true);
    e.what = What::Engrave;
    e.thickness_mm = None;
    e.air_assist = None;
    b.add(e).unwrap();
    b.add(preset("birch cut b", 80.0, false).as_alternative())
        .unwrap();
    b.bindings.push(Binding {
        target: Target::layer("c00"),
        preset: "birch cut".into(),
        linked: true,
    });
    b.bindings.push(Binding {
        target: Target::sublayer("c01", "second pass"),
        preset: "birch mark".into(),
        linked: false,
    });
    b
}

#[test]
fn a_book_is_written_and_read_back_as_it_was() {
    let book = sample();
    let text = book.to_text().unwrap();
    assert_eq!(Book::from_text(&text).unwrap(), book);
    // the same book gives the same text, and the text of what was read is the text it was read from
    assert_eq!(text, sample().to_text().unwrap());
    assert_eq!(Book::from_text(&text).unwrap().to_text().unwrap(), text);
    assert!(text.contains("[bind c01/second pass]") && text.contains("linked = on"));
    // an empty book is a book
    assert_eq!(
        Book::from_text(&Book::new().to_text().unwrap()).unwrap(),
        Book::new()
    );
}

#[test]
fn a_name_with_a_bracket_cannot_be_kept() {
    let mut b = Book::new();
    b.add(preset("a[b]", 10.0, true)).unwrap();
    assert!(b.to_text().is_err());
}

#[test]
fn a_damaged_text_names_its_line_and_keeps_nothing() {
    let good = sample().to_text().unwrap();
    let cases: Vec<(String, &str)> = vec![
        (good.replace("power = 70", "power = lots"), "power"),
        (good.replace("what = cut", "what = burn"), "what"),
        (good.replace("passes = 2", "passes = -1"), "passes"),
        (good.replace("power = 70", "power = 700"), "power"),
        (
            good.replace("preset = birch cut\n", "preset = nothing\n"),
            "nothing",
        ),
        (good.replace("linked = on", "linked = maybe"), "linked"),
        (
            good.replace("[preset birch mark]", "[preset BIRCH CUT]"),
            "already",
        ),
        (format!("{good}\n[bind c00]\npreset = birch cut\n"), "twice"),
        (format!("{good}\n[stuff x]\n"), "not [preset"),
        (format!("{good}\n[preset open\n"), "ends with ]"),
        (format!("{good}\npreset = birch cut\n"), "twice"),
        ("power = 5\n".to_string(), "outside"),
        (format!("{good}\nwhat 5\n"), "key = value"),
        (
            good.replace("material = wood/birch-plywood\n", ""),
            "material",
        ),
        (
            "[preset p]\nmaterial = m\nwhat = cut\npower = 5\nspeed = 5\npasses = 1\nflavour = 2\n"
                .to_string(),
            "flavour",
        ),
    ];
    for (text, word) in cases {
        let e = Book::from_text(&text).unwrap_err();
        assert!(e.contains(word), "{word}: {e}");
    }
    let big = "#".repeat((1 << 20) + 1);
    assert!(Book::from_text(&big).unwrap_err().contains("1 MiB"));
}

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

#[test]
fn random_books_are_read_back_exactly_and_damaged_ones_never_panic_with_a_fixed_seed() {
    let mut rng = Rng(0xb00c_5eed_0000_0110);
    for round in 0..120 {
        let mut b = Book::new();
        for k in 0..1 + rng.below(8) {
            let mut p = preset(
                &format!("preset {round}-{k}"),
                1.0 + rng.below(99) as f64,
                rng.below(2) == 0,
            );
            p.speed_mm_min = 10.0 + rng.below(20_000) as f64 + 0.25;
            p.passes = 1 + rng.below(9) as u32;
            p.thickness_mm = (rng.below(2) == 0).then(|| rng.below(20) as f64 / 2.0);
            p.air_assist = [None, Some(true), Some(false)][rng.below(3)];
            p.what = if rng.below(2) == 0 {
                What::Cut
            } else {
                What::Engrave
            };
            p.alternative = rng.below(5) == 0;
            b.add(p).unwrap();
        }
        for k in 0..rng.below(6) {
            let name = b.presets[rng.below(b.presets.len())].name.to_lowercase();
            let target = if rng.below(2) == 0 {
                Target::layer(&format!("c{k:02}"))
            } else {
                Target::sublayer(&format!("c{k:02}"), &format!("pass {k}"))
            };
            b.bindings.push(Binding {
                target,
                preset: name,
                linked: rng.below(2) == 0,
            });
        }
        let text = b.to_text().unwrap();
        assert_eq!(Book::from_text(&text).unwrap(), b, "{text}");
        // a damaged copy is a book or an error, never a panic
        let bytes: Vec<char> = text.chars().collect();
        for _ in 0..10 {
            let mut c = bytes.clone();
            for _ in 0..1 + rng.below(4) {
                if c.is_empty() {
                    break;
                }
                let i = rng.below(c.len());
                match rng.below(3) {
                    0 => c[i] = ['=', '[', ']', '\n', 'x', '9', ' '][rng.below(7)],
                    1 => {
                        c.remove(i);
                    }
                    _ => c.truncate(i),
                }
            }
            let _ = Book::from_text(&c.into_iter().collect::<String>());
        }
    }
}

#[test]
fn the_command_lists_a_book_in_the_language_asked() {
    let dir = std::env::temp_dir().join(format!("eh3d-laser-presetbook-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("book.cfg");
    std::fs::write(&file, sample().to_text().unwrap()).unwrap();
    let run = |lang: &str| {
        let o = Command::new(BIN)
            .args(["presets", &file.to_string_lossy(), "--lang", lang])
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    };
    let en = run("en");
    assert!(en.starts_with("3 preset(s), 2 binding(s)"), "{en}");
    assert!(en.contains("c00 <- birch cut"));
    assert!(en.contains("(second figure)"));
    let es = run("es");
    assert_ne!(en, es);
    assert!(es.starts_with("3 preset(s)"));
    for lang in ["de", "fr", "it", "ja", "zh", "pt"] {
        assert_ne!(run(lang), en, "{lang}");
    }
    // a damaged book is refused with the file and the line
    std::fs::write(&file, "[preset p]\nwhat = burn\n").unwrap();
    let o = Command::new(BIN)
        .args(["presets", &file.to_string_lossy()])
        .output()
        .unwrap();
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("book.cfg") && err.contains("line"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
