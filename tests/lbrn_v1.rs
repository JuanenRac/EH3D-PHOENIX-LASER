//! Projects of the older format (`.lbrn`), whose paths are written as one element for each vertex and one for each piece, on projects written here (nothing is a copy of a project made by
//! somebody else): the paths, the curves, the file with the extension `.lbrn` in the command line, damaged paths and a fuzz with a fixed seed.

use eh3d_phoenix_laser::lbrn2::{inventory, read_project};
use eh3d_phoenix_laser::vector::{bounds_of, ReadOptions};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

const CUT: &str = "    <CutSetting type=\"Cut\">\n        <index Value=\"0\"/>\n        <name Value=\"C00\"/>\n        <maxPower Value=\"40\"/>\n        <speed Value=\"10\"/>\n    </CutSetting>\n";

fn project(shapes: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LightBurnProject AppVersion=\"0.9.20\" FormatVersion=\"0\" MirrorX=\"True\" MirrorY=\"True\">\n{CUT}{shapes}    <Notes ShowOnLoad=\"0\" Notes=\"\"/>\n</LightBurnProject>\n"
    )
}

/// A closed triangle of 30 by 20 mm moved by (10, 5): three vertices and three lines.
const TRIANGLE: &str = "    <Shape Type=\"Path\" CutIndex=\"0\">\n        <XForm>1 0 0 1 10 5</XForm>\n        <V vx=\"0\" vy=\"0\"/>\n        <V vx=\"30\" vy=\"0\"/>\n        <V vx=\"0\" vy=\"20\"/>\n        <P T=\"L\" p0=\"0\" p1=\"1\"/>\n        <P T=\"L\" p0=\"1\" p1=\"2\"/>\n        <P T=\"L\" p0=\"2\" p1=\"0\"/>\n    </Shape>\n";

fn read(text: &str) -> eh3d_phoenix_laser::lbrn2::Project {
    read_project(text.as_bytes(), &ReadOptions::default()).unwrap()
}

#[test]
fn a_path_written_as_elements_is_drawn_with_its_transform_and_its_layer() {
    let p = read(&project(TRIANGLE));
    assert_eq!(p.drawing.shapes.len(), 1, "{:?}", p.drawing.warnings);
    let s = &p.drawing.shapes[0];
    assert!(s.closed);
    let b = bounds_of(s.pts.iter().copied()).unwrap();
    assert!((b.0 .0 - 10.0).abs() < 1e-9 && (b.1 .0 - 40.0).abs() < 1e-9);
    assert!((b.1 .1 - b.0 .1 - 20.0).abs() < 1e-9);
    assert!(p
        .drawing
        .warnings
        .iter()
        .all(|w| !w.contains("no earlier path")));
}

#[test]
fn a_curve_of_the_older_format_gives_a_smooth_outline_inside_its_handles() {
    // a half circle of radius 10 by one curve with its handles at the usual distance
    let curve = "    <Shape Type=\"Path\" CutIndex=\"0\">\n        <XForm>1 0 0 1 0 0</XForm>\n        <V vx=\"-10\" vy=\"0\" c0x=\"-10\" c0y=\"13.3\"/>\n        <V vx=\"10\" vy=\"0\" c1x=\"10\" c1y=\"13.3\"/>\n        <P T=\"B\" p0=\"0\" p1=\"1\"/>\n        <P T=\"L\" p0=\"1\" p1=\"0\"/>\n    </Shape>\n";
    let p = read(&project(curve));
    assert_eq!(p.drawing.shapes.len(), 1, "{:?}", p.drawing.warnings);
    let s = &p.drawing.shapes[0];
    assert!(s.pts.len() > 8, "{}", s.pts.len());
    let b = bounds_of(s.pts.iter().copied()).unwrap();
    // the top of the curve is at three quarters of the handle height, 10 mm
    assert!((b.1 .1 - b.0 .1 - 10.0).abs() < 0.1, "{b:?}");
}

#[test]
fn damaged_paths_of_the_older_format_are_skipped_with_a_note() {
    for (name, shape) in [
        (
            "no vx",
            "<Shape Type=\"Path\" CutIndex=\"0\"><V vy=\"1\"/><V vx=\"1\" vy=\"1\"/><P T=\"L\" p0=\"0\" p1=\"1\"/></Shape>",
        ),
        (
            "no pieces",
            "<Shape Type=\"Path\" CutIndex=\"0\"><V vx=\"0\" vy=\"1\"/><V vx=\"1\" vy=\"1\"/></Shape>",
        ),
        (
            "a piece outside",
            "<Shape Type=\"Path\" CutIndex=\"0\"><V vx=\"0\" vy=\"1\"/><V vx=\"1\" vy=\"1\"/><P T=\"L\" p0=\"0\" p1=\"7\"/></Shape>",
        ),
        (
            "a piece of another kind",
            "<Shape Type=\"Path\" CutIndex=\"0\"><V vx=\"0\" vy=\"1\"/><V vx=\"1\" vy=\"1\"/><P T=\"Q\" p0=\"0\" p1=\"1\"/></Shape>",
        ),
    ] {
        let p = read(&project(shape));
        assert!(p.drawing.shapes.is_empty(), "{name}");
        assert!(!p.drawing.warnings.is_empty(), "{name}");
    }
}

#[test]
fn the_inventory_counts_the_elements_of_the_older_format_as_read() {
    let inv = inventory(project(TRIANGLE).as_bytes(), &ReadOptions::default()).unwrap();
    let unread: Vec<&str> = inv
        .unread_elements()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert!(
        !unread.contains(&"V") && !unread.contains(&"P"),
        "{unread:?}"
    );
    assert!(inv.attributes.iter().any(|(k, n)| k == "V.vx" && *n == 3));
    assert!(inv
        .unread_attributes()
        .iter()
        .all(|(k, _)| k.starts_with("Notes.")));
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-lbrnv1-{}-{tag}", std::process::id()));
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

#[test]
fn a_file_with_the_extension_lbrn_is_read_by_the_command_line() {
    let s = Scratch::new("cli");
    let file = s.write("plate.lbrn", &project(TRIANGLE));
    let (code, out, err) = run(&["layers", &file]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("1 shapes (1 closed)"), "{out}");
    let out_file = s.write("plate.nc", "");
    let (code, _, err) = run(&["cut", &file, &out_file, "--bed", "300,200"]);
    assert_eq!(code, Some(0), "{err}");
    let nc = fs::read_to_string(&out_file).unwrap();
    assert!(nc.contains("G1") || nc.contains("G01"), "{nc}");
    // an extension that is not a drawing is still refused, and says which ones are read
    let other = s.write("plate.xyz", &project(TRIANGLE));
    let (code, _, err) = run(&["layers", &other]);
    assert_ne!(code, Some(0));
    assert!(err.contains("LBRN"), "{err}");
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
fn damaged_projects_of_the_older_format_never_panic_with_a_fixed_seed() {
    let base = project(TRIANGLE);
    let bytes = base.as_bytes();
    let mut rng = Rng(0x1b2d_0000_5eed_0001);
    for _ in 0..500 {
        let mut b = bytes.to_vec();
        for _ in 0..1 + rng.below(5) {
            if b.is_empty() {
                break;
            }
            let i = rng.below(b.len());
            match rng.below(4) {
                0 => b[i] = (rng.next() & 0xff) as u8,
                1 => {
                    b.remove(i);
                }
                2 => b.insert(i, b"<>\"/= 0123456789"[rng.below(16)]),
                _ => b.truncate(i),
            }
        }
        if let Ok(p) = read_project(&b, &ReadOptions::default()) {
            assert!(p
                .drawing
                .shapes
                .iter()
                .all(|s| s.pts.iter().all(|q| q.0.is_finite() && q.1.is_finite())));
        }
    }
}

#[test]
fn the_layers_of_a_project_give_their_values_to_the_command_line_when_asked() {
    let s = Scratch::new("layers");
    let file = s.write("plate.lbrn", &project(TRIANGLE));
    // without the option the layer has the defaults of the settings
    let (code, out, _) = run(&["layers", &file]);
    assert_eq!(code, Some(0));
    assert!(out.contains("at 50 % and 600 mm/min"), "{out}");
    // with it the layer has the power (40 %) and the speed (10 mm/s, 600 mm/min) of the project
    let (code, out, err) = run(&["layers", &file, "--project-layers"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("at 40 % and 600 mm/min"), "{out}");
    assert!(err.contains("1 layer(s) of the project"), "{err}");
    // an option of the command line wins over the project
    let (_, out, _) = run(&["layers", &file, "--project-layers", "--power", "15"]);
    assert!(out.contains("at 15 % and 600 mm/min"), "{out}");
    // a layer that the settings file names keeps what the settings file says
    let cfg = s.write(
        "laser.cfg",
        "[layer c00]\nmode = cut\npower = 22\nspeed = 900\n",
    );
    let (code, out, err) = run(&["layers", &file, "--project-layers", "--config", &cfg]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("at 22 % and 900 mm/min"), "{out}");
    assert!(err.contains("0 layer(s) of the project"), "{err}");
    // the option needs a project, and a drawing that is not one is refused
    let svg = s.write("a.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
    let (code, _, err) = run(&["layers", &svg, "--project-layers"]);
    assert_ne!(code, Some(0));
    assert!(!err.is_empty());
}

fn text_shape(attrs: &str, at: &str) -> String {
    format!("    <Shape Type=\"Text\" CutIndex=\"0\" {attrs}>\n        <XForm>1 0 0 1 {at}</XForm>\n    </Shape>\n")
}

#[test]
fn a_text_that_has_only_its_letters_is_drawn_with_the_stroke_font_where_the_project_puts_it() {
    // centred across and up (Ah 1, Av 1) on (100, 50): the box of the letters is centred there, 7.5 mm high
    let p = read(&project(&text_shape(
        "Str=\"kerf\" H=\"7.5\" Ah=\"1\" Av=\"1\"",
        "100 50",
    )));
    assert!(!p.drawing.shapes.is_empty(), "{:?}", p.drawing.warnings);
    assert!(p.drawing.shapes.iter().all(|s| !s.closed));
    let b = bounds_of(p.drawing.shapes.iter().flat_map(|s| s.pts.iter().copied())).unwrap();
    assert!(((b.0 .0 + b.1 .0) / 2.0 - 100.0).abs() < 1e-6, "{b:?}");
    assert!(((b.0 .1 + b.1 .1) / 2.0 - 50.0).abs() < 1e-6, "{b:?}");
    assert!((b.1 .1 - b.0 .1 - 7.5).abs() < 1e-6, "{b:?}");
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("stroke font of the program")));
    // left and bottom (Ah 0, Av 2): the box starts at the point and stands on it
    let p = read(&project(&text_shape(
        "Str=\"ab\" H=\"10\" Ah=\"0\" Av=\"2\"",
        "20 30",
    )));
    let b = bounds_of(p.drawing.shapes.iter().flat_map(|s| s.pts.iter().copied())).unwrap();
    assert!(
        (b.0 .0 - 20.0).abs() < 1e-6 && (b.1 .1 - 30.0).abs() < 1e-6,
        "{b:?}"
    );
    // the same project twice gives the same drawing
    assert_eq!(
        read(&project(&text_shape("Str=\"ab\" H=\"10\"", "1 2"))).drawing,
        read(&project(&text_shape("Str=\"ab\" H=\"10\"", "1 2"))).drawing
    );
}

#[test]
fn a_text_that_cannot_be_drawn_is_left_out_with_a_count_and_a_blank_is_said() {
    // a character that the font has not is left blank, and the rest is drawn
    let p = read(&project(&text_shape("Str=\"10%\" H=\"5\"", "0 0")));
    assert!(!p.drawing.shapes.is_empty());
    assert!(p.drawing.warnings.iter().any(|w| w.contains("left blank")));
    // nothing that can be drawn, a height over the limit of the font and a text with no height
    for attrs in [
        "Str=\"%%\" H=\"5\"",
        "Str=\"ab\" H=\"5000\"",
        "Str=\"ab\"",
        "H=\"5\"",
    ] {
        let p = read(&project(&text_shape(attrs, "0 0")));
        assert!(p.drawing.shapes.is_empty(), "{attrs}");
        assert!(
            p.stats.skipped.iter().any(|(k, _)| k.starts_with("text")),
            "{attrs}: {:?}",
            p.stats.skipped
        );
    }
}
