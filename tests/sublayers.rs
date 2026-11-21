//! Several passes over one layer (`[sublayer LAYER/LABEL]`): each pass has its own power, speed and passes, the work goes in the order engraving first and cutting last, every pass is held by the
//! limits of the machine and named when it is not, and what is wrong in the file is refused with its line. The drawings are written here and the damaged files are made with a fixed seed.

use eh3d_phoenix_laser::cut::{comment_text, layer_times, plan, verify_gcode};
use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::server::vector_job;
use eh3d_phoenix_laser::sublayers::{execution_order, MAX_LABEL_CHARS, MAX_SUBLAYERS_PER_LAYER};
use eh3d_phoenix_laser::vconfig::{parse, LayerMode, Settings};
use eh3d_phoenix_laser::vector::{Drawing, ReadOptions};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

/// A square of 10 mm as a light polyline in a DXF file, on the layer `l`.
const SQUARE_DXF: &str = "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\nl\n90\n4\n70\n1\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n0\nENDSEC\n0\nEOF\n";

/// The base of the files: a layer `l` that is cut, with the two passes of a fill and a mark over it.
const BASE: &str = "[layer l]\nmode = cut\npower = 40\nspeed = 300\n";

fn square() -> Drawing {
    read_dxf(SQUARE_DXF.as_bytes(), &ReadOptions::default()).unwrap()
}

fn settings(extra: &str) -> Settings {
    parse(&format!("{BASE}{extra}")).unwrap_or_else(|e| panic!("{e}"))
}

/// The program cut at the comment that opens each operation: the name and the lines under it.
fn sections(g: &str) -> Vec<(String, Vec<&str>)> {
    let mut out: Vec<(String, Vec<&str>)> = Vec::new();
    for line in g.lines() {
        if let Some(rest) = line.strip_prefix("; layer ") {
            out.push((rest.split(": ").next().unwrap().to_string(), Vec::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push(line);
        }
    }
    out
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-sublayers-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, name: &str, text: &str) -> String {
        let p = self.0.join(name);
        fs::write(&p, text).unwrap();
        p.to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_fill_and_the_mark_come_before_the_cut_whatever_the_order_of_the_file() {
    let s = settings(
        "[sublayer l/mark]\nmode = engrave\npower = 10\nspeed = 2000\nhatch_spacing = 1\n\
         [sublayer l/fill]\nmode = engrave\npower = 20\nspeed = 1500\nhatch_spacing = 0.5\n",
    );
    let d = square();
    let order: Vec<String> = execution_order(&d, &s).iter().map(|o| o.name()).collect();
    assert_eq!(order, ["l/mark", "l/fill", "l"]);
    let g = plan(&d, &s).unwrap().gcode;
    let names: Vec<String> = sections(&g).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["l/mark", "l/fill", "l"], "{g}");
    // each pass burns with its own power: S100 is 10 %, S200 is 20 % and S400 is 40 % of S1000
    let by_name = |n: &str| sections(&g).into_iter().find(|(k, _)| k == n).unwrap().1;
    assert!(by_name("l/mark").iter().any(|l| l.contains("S100")));
    assert!(by_name("l/fill").iter().any(|l| l.contains("S200")));
    assert!(by_name("l").iter().any(|l| l.contains("S400")));
    assert!(!by_name("l").iter().any(|l| l.contains("S200")));
    // and the program passes the check of the G-code
    verify_gcode(&g, s.machine.s_max, s.machine.bed_mm).unwrap();
}

#[test]
fn the_order_of_a_pass_moves_it_among_the_passes_of_its_kind() {
    let s = settings(
        "[sublayer l/late]\nmode = engrave\norder = 5\n[sublayer l/early]\nmode = engrave\norder = -5\n\
         [sublayer l/also]\nmode = cut\norder = -9\n",
    );
    let d = square();
    let order: Vec<String> = execution_order(&d, &s).iter().map(|o| o.name()).collect();
    // engraving first (lowest order first), then cutting: the layer (order 0) and the pass with order -9
    assert_eq!(order, ["l/early", "l/late", "l/also", "l"]);
    // the program writes them in the same order
    let g = plan(&d, &s).unwrap().gcode;
    let names: Vec<String> = sections(&g).into_iter().map(|(n, _)| n).collect();
    assert_eq!(names, order);
}

#[test]
fn a_pass_with_equal_order_never_goes_before_its_own_layer() {
    let s = settings("[sublayer l/second]\npower = 30\n[sublayer l/third]\npower = 20\n");
    let order: Vec<String> = execution_order(&square(), &s)
        .iter()
        .map(|o| o.name())
        .collect();
    assert_eq!(order, ["l", "l/second", "l/third"]);
}

#[test]
fn each_pass_has_its_own_number_of_passes_and_speed() {
    let s = settings("[sublayer l/deep]\npower = 60\nspeed = 120\npasses = 3\nfocus_step = -0.5\n");
    let g = plan(&square(), &s).unwrap().gcode;
    let secs = sections(&g);
    let passes = |n: &str| {
        secs.iter()
            .find(|(k, _)| k == n)
            .unwrap()
            .1
            .iter()
            .filter(|l| l.starts_with("; pass "))
            .count()
    };
    assert_eq!((passes("l"), passes("l/deep")), (1, 3));
    let deep = &secs.iter().find(|(k, _)| k == "l/deep").unwrap().1;
    assert!(deep.contains(&"F120"), "{deep:?}");
    // the focus of the deep pass moves on at each of its three passes and goes back once at the end; the layer's does not move
    assert_eq!(deep.iter().filter(|l| l.starts_with("G0 Z")).count(), 4);
    let base = &secs.iter().find(|(k, _)| k == "l").unwrap().1;
    assert!(!base.iter().any(|l| l.starts_with("G0 Z")));
}

#[test]
fn a_pass_is_held_by_the_limits_of_the_machine_and_the_error_names_the_layer_and_the_label() {
    let e = parse(
        "[machine]\nmax_power_percent = 30\n[defaults]\npower = 10\n[layer l]\npower = 20\n[sublayer l/deep]\npower = 50\n",
    )
    .unwrap_err();
    assert!(
        e.contains("sublayer l/deep") && e.contains("max_power_percent"),
        "{e}"
    );
    let e = parse("[machine]\nmax_speed_mm_min = 500\n[defaults]\nspeed = 100\n[layer l]\nspeed = 400\n[sublayer l/fast]\nspeed = 900\n")
        .unwrap_err();
    assert!(
        e.contains("sublayer l/fast") && e.contains("max_speed_mm_min"),
        "{e}"
    );
    let e =
        parse("[machine]\nmax_passes = 3\n[layer l]\n[sublayer l/many]\npasses = 4\n").unwrap_err();
    assert!(e.contains("sublayer l/many") && e.contains("passes"), "{e}");
    // a value outside a range is refused too, and nothing is lowered in silence
    let e = parse("[layer l]\n[sublayer l/x]\npower = 150\n").unwrap_err();
    assert!(e.contains("sublayer l/x"), "{e}");
    // a pass that is skipped is not held by a limit it would not use
    assert!(parse("[machine]\nmax_power_percent = 30\n[defaults]\npower = 10\n[layer l]\npower = 20\n[sublayer l/off]\nmode = skip\npower = 50\n").is_ok());
}

#[test]
fn what_is_wrong_in_a_sublayer_section_is_refused_with_its_line() {
    for (text, want) in [
        ("[sublayer l/a]\n", "has no [layer l] section"),
        ("[layer l]\n[sublayer l]\n", "LAYER/LABEL"),
        ("[layer l]\n[sublayer l/]\n", "label"),
        ("[layer l]\n[sublayer /a]\n", "no layer"),
        ("[layer l]\n[sublayer l/a b]\n", "not a letter"),
        ("[layer l]\n[sublayer l/a]\nbogus = 1\n", "line 3"),
        ("[layer l]\n[sublayer l/a]\nmode = weld\n", "line 3"),
        ("[layer l]\n[sublayer l/a]\npower = x\n", "line 3"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
    // the label is limited and so is the number of passes of a layer
    let long = "x".repeat(MAX_LABEL_CHARS + 1);
    assert!(parse(&format!("[layer l]\n[sublayer l/{long}]\n")).is_err());
    let mut many = String::from("[layer l]\n");
    for i in 0..=MAX_SUBLAYERS_PER_LAYER {
        many.push_str(&format!("[sublayer l/p{i}]\n"));
    }
    let e = parse(&many).unwrap_err();
    assert!(e.contains("more than"), "{e}");
}

#[test]
fn a_skipped_layer_takes_its_sublayers_with_it_and_a_skipped_pass_writes_nothing() {
    let s = settings("[sublayer l/a]\nmode = skip\n");
    let g = plan(&square(), &s).unwrap().gcode;
    assert_eq!(
        sections(&g).into_iter().map(|(n, _)| n).collect::<Vec<_>>(),
        ["l"]
    );
    let off = parse("[layer l]\nmode = skip\n[sublayer l/a]\nmode = engrave\n").unwrap();
    assert!(execution_order(&square(), &off).is_empty());
    let e = plan(&square(), &off).unwrap_err();
    assert!(e.contains("nothing to cut or engrave"), "{e}");
}

#[test]
fn the_same_settings_give_the_same_program_and_a_layer_without_passes_is_unchanged() {
    let with = settings("[sublayer l/fill]\nmode = engrave\nhatch_spacing = 0.5\n");
    let d = square();
    assert_eq!(
        plan(&d, &with).unwrap().gcode,
        plan(&d, &with).unwrap().gcode
    );
    // the layer alone writes what it always wrote: one section and no mention of a pass of another name
    let alone = plan(&d, &settings("")).unwrap();
    assert_eq!(alone.stats.layers, 1);
    assert_eq!(plan(&d, &with).unwrap().stats.layers, 2);
    assert!(sections(&alone.gcode).len() == 1);
}

#[test]
fn the_time_of_each_pass_is_given_under_the_name_the_program_writes() {
    let s = settings("[sublayer l/fill]\nmode = engrave\nhatch_spacing = 0.5\n");
    let times = layer_times(&square(), &s, 1000.0).unwrap();
    let names: Vec<&str> = times.iter().map(|t| t.key.as_str()).collect();
    assert_eq!(names, [comment_text("l/fill"), comment_text("l")]);
    assert!(times.iter().all(|t| t.seconds > 0.0));
}

#[test]
fn the_layers_command_lists_the_passes_with_their_time() {
    let sc = Scratch::new("cli");
    let cfg = sc.write(
        "p.cfg",
        &format!("{BASE}[sublayer l/fill]\nmode = engrave\npower = 20\nhatch_spacing = 0.5\n"),
    );
    let dxf = sc.write("sq.dxf", SQUARE_DXF);
    let out = Command::new(BIN)
        .args(["layers", "--config", &cfg, "--accel", "1000", &dxf])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("  + l/fill: engrave at 20 % and 300 mm/min, 1 pass(es)"),
        "{text}"
    );
    assert_eq!(text.matches("time: about").count(), 2, "{text}");
}

#[test]
fn the_server_is_told_that_the_passes_are_not_sent_as_operations() {
    let s = settings("[sublayer l/fill]\nmode = engrave\n");
    let (job, notes) = vector_job(&square(), &s).unwrap();
    assert_eq!(job.ops.len(), 1);
    assert!(
        notes.iter().any(|n| n.contains("sub-layers of the layer")),
        "{notes:?}"
    );
}

/// A small generator of numbers that gives the same series every time.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

#[test]
fn damaged_files_with_sublayers_never_panic_and_what_is_read_is_inside_the_limits() {
    let seed_text = format!(
        "[machine]\nmax_power_percent = 60\nmax_speed_mm_min = 3000\n{BASE}[sublayer l/fill]\nmode = engrave\npower = 20\nspeed = 1500\n[sublayer l/deep]\npower = 50\npasses = 2\n"
    );
    let d = square();
    let bytes: Vec<char> = seed_text.chars().collect();
    let mut rng = Lcg(0x5EED_0001);
    let alphabet: Vec<char> = "[]/=0123456789. \nabcl-_#".chars().collect();
    let (mut accepted, mut refused) = (0, 0);
    for _ in 0..600 {
        let mut text = bytes.clone();
        for _ in 0..(1 + rng.next() % 4) {
            let at = (rng.next() as usize) % text.len();
            match rng.next() % 3 {
                0 => {
                    text.remove(at);
                }
                1 => text[at] = alphabet[(rng.next() as usize) % alphabet.len()],
                _ => text.insert(at, alphabet[(rng.next() as usize) % alphabet.len()]),
            }
        }
        let text: String = text.into_iter().collect();
        match parse(&text) {
            Ok(s) => {
                accepted += 1;
                for sub in s
                    .sublayers
                    .iter()
                    .filter(|x| x.params.mode != LayerMode::Skip)
                {
                    assert!(sub.params.power_percent <= s.machine.max_power_percent);
                    assert!(sub.params.speed_mm_min <= s.machine.max_speed_mm_min);
                    assert!(sub.params.passes <= s.machine.max_passes);
                }
                // what is read can be planned or refused, never a panic
                let _ = plan(&d, &s);
            }
            Err(_) => refused += 1,
        }
    }
    assert!(accepted > 10 && refused > 10, "{accepted} {refused}");
}
