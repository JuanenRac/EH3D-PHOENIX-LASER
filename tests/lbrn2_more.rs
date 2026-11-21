//! The pictures, the direction of the y axis, the ellipse and the polygon, the tabs and the counter of the variable text of an LBRN2 project, on projects and pictures written here (a PNG
//! made with the encoder of the tests, put in base64 by a small function below), with hostile pictures and a fuzz with a fixed seed. Nothing here is a copy of a file made by somebody
//! else, and the real projects the reader was measured on are not used.

use eh3d_phoenix_laser::cut::plan;
use eh3d_phoenix_laser::lbrn2::{inventory, read_project, Project, READ_ATTRIBUTES};
use eh3d_phoenix_laser::lbrn2_pic::{
    decode_base64, decode_picture, orient, CODE_COUNT, CODE_DATA, CODE_FORMAT, CODE_PIXELS,
    CODE_ROTATED, CODE_SIZE,
};
use eh3d_phoenix_laser::vconfig::{LayerMode, Settings};
use eh3d_phoenix_laser::vector::{bounds_of, Limits, ReadOptions};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");
const IDENT: &str = "1 0 0 1 0 0";

// ---------------------------------------------------------------- a small base64 encoder and a PNG for the tests

fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// A grey PNG of `w` x `h` with the levels given.
fn png_of(w: u32, h: u32, levels: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        writer.write_image_data(levels).unwrap();
    }
    out
}

/// A 3 x 2 picture whose six levels are all different, so that a flip or a turn can be told.
const LEVELS: [u8; 6] = [10, 20, 30, 40, 50, 60];

fn bitmap(cut_index: u32, w: f64, h: f64, xform: &str, data: &str) -> String {
    format!("    <Shape Type=\"Bitmap\" CutIndex=\"{cut_index}\" W=\"{w}\" H=\"{h}\" Gamma=\"1\" File=\"a.png\" Data=\"{data}\">\n        <XForm>{xform}</XForm>\n    </Shape>\n")
}

fn project(cuts: &str, shapes: &str, tail: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LightBurnProject AppVersion=\"9.9.00\" FormatVersion=\"1\">\n{cuts}{shapes}{tail}</LightBurnProject>\n"
    )
}

fn layer(kind: &str, values: &[(&str, &str)]) -> String {
    let mut s = format!("    <CutSetting type=\"{kind}\">\n");
    for (k, v) in values {
        s.push_str(&format!("        <{k} Value=\"{v}\"/>\n"));
    }
    s.push_str("    </CutSetting>\n");
    s
}

fn read(text: &str) -> Project {
    read_project(text.as_bytes(), &ReadOptions::default()).unwrap()
}

fn read_with(text: &str, opts: &ReadOptions) -> Project {
    read_project(text.as_bytes(), opts).unwrap()
}

fn levels(p: &Project, i: usize) -> Vec<u8> {
    let g = &p.pictures[i].gray;
    (0..g.h)
        .flat_map(|y| (0..g.w).map(move |x| g.at(x, y)))
        .collect()
}

fn one_picture(xform: &str) -> String {
    project(
        "",
        &bitmap(0, 30.0, 20.0, xform, &base64(&png_of(3, 2, &LEVELS))),
        "",
    )
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-lbrn2m-{}-{tag}", std::process::id()));
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

// ---------------------------------------------------------------- base64

#[test]
fn base64_round_trips_any_length_with_spaces_and_without_padding() {
    let mut rng = Lcg(0x6a09_e667);
    for len in 0..64usize {
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next() % 256) as u8).collect();
        let text = base64(&bytes);
        assert_eq!(decode_base64(&text, 1 << 20).unwrap(), bytes, "{len}");
        let spaced: String = text
            .chars()
            .enumerate()
            .flat_map(|(i, c)| if i % 7 == 6 { vec![c, '\n'] } else { vec![c] })
            .collect();
        assert_eq!(decode_base64(&spaced, 1 << 20).unwrap(), bytes);
        let bare = text.trim_end_matches('=');
        assert_eq!(decode_base64(bare, 1 << 20).unwrap(), bytes);
    }
}

#[test]
fn base64_refuses_what_is_not_base64_text_and_a_text_that_decodes_too_long() {
    for bad in ["ab$d", "abcd=abcd", "a", "abcde"] {
        let e = decode_base64(bad, 1 << 20).unwrap_err();
        assert!(e.starts_with(CODE_DATA), "{bad}: {e}");
    }
    let long = base64(&[7u8; 3000]);
    let e = decode_base64(&long, 1000).unwrap_err();
    assert!(e.starts_with(CODE_SIZE), "{e}");
    assert!(decode_base64(&long, 3000).is_ok());
}

// ---------------------------------------------------------------- the picture

#[test]
fn a_picture_is_decoded_and_kept_with_its_layer_and_the_size_the_project_gives_it() {
    let cuts = layer("Image", &[("index", "4"), ("name", "photo")]);
    let shapes = bitmap(4, 30.0, 20.0, IDENT, &base64(&png_of(3, 2, &LEVELS)));
    let p = read(&project(&cuts, &shapes, ""));
    assert_eq!(p.stats.pictures, 1);
    assert_eq!(p.pictures.len(), 1);
    let pic = &p.pictures[0];
    assert_eq!(pic.layer, "photo");
    assert!((pic.width_mm - 30.0).abs() < 1e-9 && (pic.height_mm - 20.0).abs() < 1e-9);
    assert_eq!((pic.gray.w, pic.gray.h), (3, 2));
    assert_eq!(levels(&p, 0), LEVELS);
    // a picture is not part of the drawing that is cut
    assert!(p.drawing.shapes.is_empty());
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("1 picture(s) of the project are kept for engraving")));
}

#[test]
fn the_transform_of_a_picture_scales_flips_and_turns_by_quarters() {
    // scaled: the size is the one after the matrix
    let p = read(&one_picture("2 0 0 0.5 5 5"));
    assert!((p.pictures[0].width_mm - 60.0).abs() < 1e-9);
    assert!((p.pictures[0].height_mm - 10.0).abs() < 1e-9);
    assert_eq!(levels(&p, 0), LEVELS);
    // mirrored in x: each row is reversed
    let p = read(&one_picture("-1 0 0 1 0 0"));
    assert_eq!(levels(&p, 0), [30, 20, 10, 60, 50, 40]);
    // mirrored in y: the rows change places
    let p = read(&one_picture("1 0 0 -1 0 0"));
    assert_eq!(levels(&p, 0), [40, 50, 60, 10, 20, 30]);
    // a half turn is both
    let p = read(&one_picture("-1 0 0 -1 0 0"));
    assert_eq!(levels(&p, 0), [60, 50, 40, 30, 20, 10]);
    // a quarter turn (x' = -y, y' = x): the picture is 2 wide and 3 tall, and its size follows
    let p = read(&one_picture("0 1 -1 0 0 0"));
    let pic = &p.pictures[0];
    assert_eq!((pic.gray.w, pic.gray.h), (2, 3));
    assert!((pic.width_mm - 20.0).abs() < 1e-9 && (pic.height_mm - 30.0).abs() < 1e-9);
    assert_eq!(levels(&p, 0), [40, 10, 50, 20, 60, 30]);
    // the other quarter turn (x' = y, y' = -x)
    let p = read(&one_picture("0 -1 1 0 0 0"));
    assert_eq!(levels(&p, 0), [30, 60, 20, 50, 10, 40]);
}

#[test]
fn a_picture_turned_by_something_else_than_a_quarter_is_refused_with_its_code() {
    let p = read(&one_picture("0.866 0.5 -0.5 0.866 0 0"));
    assert!(p.pictures.is_empty());
    assert!(p
        .stats
        .skipped
        .iter()
        .any(|(k, _)| k == "picture turned by something other than a quarter turn"));
    assert!(p.drawing.warnings.iter().any(|w| w.contains(CODE_ROTATED)));
    let g = decode_picture(&base64(&png_of(3, 2, &LEVELS)), &Limits::default()).unwrap();
    assert!(orient(&g, &[0.0; 6], 1.0, 1.0).is_err());
}

#[test]
fn the_limits_of_a_picture_refuse_it_before_it_is_unpacked() {
    let data = base64(&png_of(3, 2, &LEVELS));
    let text = project("", &bitmap(0, 3.0, 2.0, IDENT, &data), "");
    // pixels
    let mut o = ReadOptions::default();
    o.limits.max_bitmap_pixels = 5;
    let p = read_with(&text, &o);
    assert!(p.pictures.is_empty());
    assert!(p.drawing.warnings.iter().any(|w| w.contains(CODE_PIXELS)));
    o.limits.max_bitmap_pixels = 6;
    assert_eq!(read_with(&text, &o).pictures.len(), 1);
    // the text
    let mut o = ReadOptions::default();
    o.limits.max_bitmap_bytes = data.len() - 1;
    let p = read_with(&text, &o);
    assert!(p.pictures.is_empty());
    assert!(p.drawing.warnings.iter().any(|w| w.contains(CODE_SIZE)));
    o.limits.max_bitmap_bytes = data.len();
    assert_eq!(read_with(&text, &o).pictures.len(), 1);
    // the number of pictures
    let two = project(
        "",
        &format!(
            "{}{}",
            bitmap(0, 3.0, 2.0, IDENT, &data),
            bitmap(0, 3.0, 2.0, IDENT, &data)
        ),
        "",
    );
    let mut o = ReadOptions::default();
    o.limits.max_bitmaps = 1;
    let p = read_with(&two, &o);
    assert_eq!(p.pictures.len(), 1);
    assert!(p.drawing.warnings.iter().any(|w| w.contains(CODE_COUNT)));
    // a picture that claims a huge size in its header is refused from the header, with nothing allocated
    let mut huge = png_of(3, 2, &LEVELS);
    huge[16..20].copy_from_slice(&60_000u32.to_be_bytes());
    huge[20..24].copy_from_slice(&60_000u32.to_be_bytes());
    let e = decode_picture(&base64(&huge), &Limits::default()).unwrap_err();
    assert!(e.starts_with(CODE_PIXELS), "{e}");
}

#[test]
fn a_picture_in_another_format_or_damaged_is_left_out_with_a_code_and_the_rest_is_read() {
    let jpeg = base64(&[
        0xff, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1,
    ]);
    let png = png_of(3, 2, &LEVELS);
    let cut_short = base64(&png[..png.len() / 2]);
    let not_text = "AB$D";
    for (data, code) in [
        (jpeg.as_str(), CODE_FORMAT),
        (cut_short.as_str(), CODE_DATA),
        (not_text, CODE_DATA),
    ] {
        let shapes = format!(
            "{}    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"4\" H=\"4\"><XForm>{IDENT}</XForm></Shape>\n",
            bitmap(0, 3.0, 2.0, IDENT, data)
        );
        let p = read(&project("", &shapes, ""));
        assert!(p.pictures.is_empty(), "{code}");
        assert_eq!(p.drawing.shapes.len(), 1);
        assert!(
            p.drawing.warnings.iter().any(|w| w.contains(code)),
            "{code}: {:?}",
            p.drawing.warnings
        );
    }
    // a PGM in the picture is read too
    let mut pgm = b"P5\n3 2\n255\n".to_vec();
    pgm.extend_from_slice(&LEVELS);
    let p = read(&project("", &bitmap(0, 3.0, 2.0, IDENT, &base64(&pgm)), ""));
    assert_eq!(levels(&p, 0), LEVELS);
}

// ---------------------------------------------------------------- the direction of the y axis

fn square_at(y: f64) -> String {
    format!("    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"10\" H=\"10\"><XForm>1 0 0 1 20 {y}</XForm></Shape>\n")
}

#[test]
fn the_y_axis_follows_the_setting_and_the_default_is_the_one_before() {
    let text = project("", &square_at(50.0), "");
    let on = read(&text);
    let b = bounds_of(on.drawing.shapes.iter().flat_map(|s| s.pts.iter().copied())).unwrap();
    assert!((b.0 .1 - 45.0).abs() < 1e-9 && (b.1 .1 - 55.0).abs() < 1e-9);
    assert!(ReadOptions::default().lbrn2_flip_y);
    let off_opts = ReadOptions {
        lbrn2_flip_y: false,
        ..ReadOptions::default()
    };
    let off = read_with(&text, &off_opts);
    let b = bounds_of(
        off.drawing
            .shapes
            .iter()
            .flat_map(|s| s.pts.iter().copied()),
    )
    .unwrap();
    assert!((b.0 .1 + 55.0).abs() < 1e-9 && (b.1 .1 + 45.0).abs() < 1e-9);
    // in the program the two are mirror images: the same square, the other way up
    let cuts = layer("Cut", &[("index", "0")]);
    let tall = project(
        &cuts,
        "    <Shape Type=\"Path\" CutIndex=\"0\"><XForm>1 0 0 1 0 0</XForm><VertList>V0 0V10 0V10 30</VertList><PrimList>LineClosed</PrimList></Shape>\n",
        "",
    );
    let s = Settings::default();
    let g_on = plan(&read(&tall).drawing, &s).unwrap().gcode;
    let g_off = plan(&read_with(&tall, &off_opts).drawing, &s)
        .unwrap()
        .gcode;
    assert_ne!(g_on, g_off);
}

#[test]
fn the_picture_follows_the_same_setting_and_every_reading_says_the_direction_is_not_confirmed() {
    let text = one_picture(IDENT);
    let off_opts = ReadOptions {
        lbrn2_flip_y: false,
        ..ReadOptions::default()
    };
    let on = read(&text);
    let off = read_with(&text, &off_opts);
    assert_eq!(levels(&on, 0), LEVELS);
    assert_eq!(levels(&off, 0), [40, 50, 60, 10, 20, 30]);
    for (p, word, flag) in [
        (&on, "down, like an SVG", "on"),
        (&off, "up, like the machine", "off"),
    ] {
        let note = p
            .drawing
            .warnings
            .iter()
            .find(|w| w.contains("not confirmed with a real part"))
            .unwrap_or_else(|| panic!("{:?}", p.drawing.warnings));
        assert!(note.contains(word) && note.contains(&format!("lbrn2_flip_y = {flag}")));
    }
    // the note reaches the program that is made from a drawing, so the report before a job carries it
    let cuts = layer("Cut", &[("index", "0")]);
    let text = project(&cuts, &square_at(10.0), "");
    let planned = plan(&read(&text).drawing, &Settings::default()).unwrap();
    assert!(planned
        .warnings
        .iter()
        .any(|w| w.contains("not confirmed with a real part")));
}

#[test]
fn the_setting_is_a_key_of_the_settings_file_and_the_contract_lists_it() {
    let s = Scratch::new("flip");
    let cfg = s.write("p.cfg", "[read]\nlbrn2_flip_y = off\n[limits]\nmax_bitmaps = 2\nmax_bitmap_bytes = 1000\nmax_bitmap_pixels = 50\nmax_csv_bytes = 100\nmax_csv_rows = 5\nmax_csv_columns = 3\n");
    let loaded = eh3d_phoenix_laser::vconfig::load(std::path::Path::new(&cfg)).unwrap();
    assert!(!loaded.read.lbrn2_flip_y);
    assert_eq!(loaded.read.limits.max_bitmaps, 2);
    assert_eq!(loaded.read.limits.max_bitmap_pixels, 50);
    assert_eq!(loaded.read.limits.max_csv_rows, 5);
    let bad = s.write("b.cfg", "[limits]\nmax_bitmaps = 0\n");
    assert!(eh3d_phoenix_laser::vconfig::load(std::path::Path::new(&bad)).is_err());
    let (_, listing, _) = run(&["settings"]);
    for key in [
        "lbrn2_flip_y",
        "max_bitmaps",
        "max_bitmap_bytes",
        "max_bitmap_pixels",
        "max_csv_bytes",
        "max_csv_rows",
        "max_csv_columns",
    ] {
        assert!(listing.contains(key), "{key}");
    }
}

// ---------------------------------------------------------------- the ellipse and the polygon

fn round(kind: &str, attrs: &str) -> String {
    format!(
        "    <Shape Type=\"{kind}\" CutIndex=\"0\" {attrs}><XForm>1 0 0 1 100 50</XForm></Shape>\n"
    )
}

#[test]
fn an_ellipse_is_drawn_with_its_two_radii_and_stays_within_the_tolerance() {
    let p = read(&project("", &round("Ellipse", "Rx=\"10\" Ry=\"5\""), ""));
    assert_eq!(p.drawing.shapes.len(), 1);
    let s = &p.drawing.shapes[0];
    assert!(s.closed);
    let b = bounds_of(s.pts.iter().copied()).unwrap();
    // the pieces are within the tolerance (0.01 mm) of the ellipse, so its box is too
    assert!((b.0 .0 - 90.0).abs() < 0.05 && (b.1 .0 - 110.0).abs() < 0.05);
    assert!((b.0 .1 - 45.0).abs() < 0.05 && (b.1 .1 - 55.0).abs() < 0.05);
    for &(x, y) in &s.pts {
        let r = ((x - 100.0) / 10.0).powi(2) + ((y - 50.0) / 5.0).powi(2);
        assert!((r - 1.0).abs() < 1e-9, "{x} {y}");
    }
    // a circle with one radius
    let c = read(&project("", &round("Ellipse", "R=\"4\""), ""));
    assert_eq!(c.drawing.shapes.len(), 1);
    // without a size it is left out
    let none = read(&project("", &round("Ellipse", "Rx=\"0\" Ry=\"5\""), ""));
    assert!(none.drawing.shapes.is_empty());
    assert!(none
        .stats
        .skipped
        .iter()
        .any(|(k, _)| k == "ellipse without a size"));
}

#[test]
fn a_polygon_has_its_sides_on_the_ellipse_with_a_vertex_pointing_up() {
    let p = read(&project(
        "",
        &round("Polygon", "N=\"6\" Rx=\"10\" Ry=\"10\""),
        "",
    ));
    let s = &p.drawing.shapes[0];
    assert_eq!(s.pts.len(), 6);
    assert!(s.closed);
    // the first vertex is straight up in a y axis that points down
    assert!((s.pts[0].0 - 100.0).abs() < 1e-9 && (s.pts[0].1 - 40.0).abs() < 1e-9);
    for &(x, y) in &s.pts {
        assert!((((x - 100.0).powi(2) + (y - 50.0).powi(2)).sqrt() - 10.0).abs() < 1e-9);
    }
    for bad in [
        "N=\"2\" Rx=\"10\" Ry=\"10\"",
        "Rx=\"10\" Ry=\"10\"",
        "N=\"5000\" Rx=\"10\" Ry=\"10\"",
    ] {
        let p = read(&project("", &round("Polygon", bad), ""));
        assert!(p.drawing.shapes.is_empty(), "{bad}");
        assert!(!p.stats.skipped.is_empty());
    }
}

// ---------------------------------------------------------------- the tabs

#[test]
fn the_tabs_of_a_layer_become_hints_for_a_cut_and_for_nothing_else() {
    let cuts = format!(
        "{}{}{}{}{}",
        layer(
            "Cut",
            &[
                ("index", "0"),
                ("tabsEnabled", "1"),
                ("tabCount", "3"),
                ("tabSize", "0.8")
            ]
        ),
        layer(
            "Cut",
            &[
                ("index", "1"),
                ("tabsEnabled", "0"),
                ("tabCount", "3"),
                ("tabSize", "0.8")
            ]
        ),
        layer(
            "Scan",
            &[("index", "2"), ("tabsEnabled", "1"), ("tabCount", "3")]
        ),
        layer(
            "Cut",
            &[
                ("index", "3"),
                ("tabsEnabled", "1"),
                ("tabCount", "500"),
                ("tabSize", "0.8")
            ]
        ),
        layer(
            "Cut",
            &[("index", "4"), ("tabsEnabled", "1"), ("tabCount", "2")]
        ),
    );
    let p = read(&project(&cuts, "", ""));
    let hints: Vec<_> = p.cut_settings.iter().map(|c| c.hint()).collect();
    assert_eq!(hints[0].mode, LayerMode::Cut);
    assert_eq!(hints[0].tabs, 3);
    assert!((hints[0].tab_width_mm - 0.8).abs() < 1e-9);
    assert_eq!(hints[1].tabs, 0);
    assert_eq!(hints[2].tabs, 0);
    assert_eq!(hints[3].tabs, 0, "a count over 100 is no count");
    // a tab without a size uses the default width of the program
    assert_eq!(hints[4].tabs, 2);
    assert!(
        (hints[4].tab_width_mm - eh3d_phoenix_laser::vconfig::LayerParams::default().tab_width_mm)
            .abs()
            < 1e-9
    );
    assert!(p.cut_settings[0].tabs_enabled() && !p.cut_settings[1].tabs_enabled());
    // the hint reaches the program: a cut with tabs lifts the beam where the tabs are
    let cut_text = project(
        &layer(
            "Cut",
            &[
                ("index", "0"),
                ("tabsEnabled", "1"),
                ("tabCount", "2"),
                ("tabSize", "1"),
            ],
        ),
        &square_at(10.0),
        "",
    );
    let q = read(&cut_text);
    let mut s = Settings::default();
    s.layers.push(q.hints().remove(0));
    let with_tabs = plan(&q.drawing, &s).unwrap().gcode;
    let without = plan(&q.drawing, &Settings::default()).unwrap().gcode;
    assert_ne!(with_tabs, without);
}

// ---------------------------------------------------------------- the counter of the variable text

#[test]
fn the_variable_text_of_a_project_gives_a_counter_for_the_series() {
    let tail = "    <VariableText>\n        <Start Value=\"0\"/>\n        <End Value=\"9\"/>\n        <Current Value=\"3\"/>\n        <Increment Value=\"2\"/>\n        <AutoAdvance Value=\"1\"/>\n    </VariableText>\n";
    let p = read(&project("", "", tail));
    let vt = p.variable_text.unwrap();
    assert_eq!((vt.start, vt.end, vt.current, vt.increment), (0, 9, 3, 2));
    assert!(vt.auto_advance);
    let counter = vt.counter().unwrap();
    let values: Vec<i64> = (0..counter.count)
        .map(|i| counter.value(i).unwrap())
        .collect();
    assert_eq!(values, [3, 5, 7, 9]);
    // a block with a value missing, an increment of 0 and an end before the start are no series
    assert!(read(&project(
        "",
        "",
        "    <VariableText><Start Value=\"0\"/></VariableText>\n"
    ))
    .variable_text
    .is_none());
    let zero = tail.replace("Increment Value=\"2\"", "Increment Value=\"0\"");
    assert!(read(&project("", "", &zero))
        .variable_text
        .unwrap()
        .counter()
        .is_err());
    let back = tail.replace("Current Value=\"3\"", "Current Value=\"30\"");
    assert!(read(&project("", "", &back))
        .variable_text
        .unwrap()
        .counter()
        .is_err());
    assert!(read(&project("", "", "")).variable_text.is_none());
}

#[test]
fn what_the_new_parts_use_is_listed_in_the_attributes_the_reader_reads() {
    let text = project(
        &layer("Cut", &[("index", "0"), ("tabsEnabled", "1"), ("tabCount", "2"), ("tabSize", "1")]),
        &format!(
            "{}{}{}",
            round("Ellipse", "Rx=\"3\" Ry=\"2\""),
            round("Polygon", "N=\"5\" Rx=\"3\" Ry=\"3\""),
            bitmap(0, 3.0, 2.0, IDENT, &base64(&png_of(3, 2, &LEVELS)))
        ),
        "    <VariableText><Start Value=\"0\"/><End Value=\"9\"/><Current Value=\"0\"/><Increment Value=\"1\"/><AutoAdvance Value=\"0\"/></VariableText>\n",
    );
    let inv = inventory(text.as_bytes(), &ReadOptions::default()).unwrap();
    assert!(
        inv.unread_elements().is_empty(),
        "{:?}",
        inv.unread_elements()
    );
    assert!(
        inv.unread_attributes()
            .iter()
            .all(|(k, _)| !k.contains("Data")),
        "{:?}",
        inv.unread_attributes()
    );
    for a in [
        "Shape.Data",
        "Shape.Rx",
        "Shape.N",
        "tabCount.Value",
        "Current.Value",
    ] {
        assert!(READ_ATTRIBUTES.contains(&a), "{a}");
        assert!(inv.attributes.iter().any(|(k, _)| k == a), "{a}");
    }
}

// ---------------------------------------------------------------- from the project to the engraving

fn engraving_project(extra_layer: &str) -> String {
    let cuts = format!(
        "{}{}",
        layer(
            "Image",
            &[
                ("index", "0"),
                ("name", "photo"),
                ("maxPower", "35"),
                ("speed", "50")
            ]
        ),
        extra_layer
    );
    let mut levels = Vec::new();
    for y in 0..8u32 {
        for x in 0..8u32 {
            levels.push(((x * 32 + y * 4) % 256) as u8);
        }
    }
    project(
        &cuts,
        &bitmap(0, 8.0, 8.0, IDENT, &base64(&png_of(8, 8, &levels))),
        "",
    )
}

#[test]
fn a_project_with_a_picture_is_engraved_like_the_picture_alone_with_the_values_of_its_layer() {
    let s = Scratch::new("engrave");
    let file = s.write("photo.lbrn2", &engraving_project(""));
    let out = s.path("photo.nc");
    let (code, stdout, err) = run(&["engrave", &file, &out, "--dot-mm", "1"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(stdout.contains("8 x 8 dots"), "{stdout}");
    assert!(
        err.contains("the power and the speed of the layer photo"),
        "{err}"
    );
    assert!(err.contains("not confirmed with a real part"), "{err}");
    let g = fs::read_to_string(&out).unwrap();
    // the power of the layer (35 % of S1000) and its speed (50 mm/s = 3000 mm/min) are used
    assert!(g.contains("S350"), "{g}");
    assert!(g.contains("F3000"), "{g}");
    // the same picture as a file of its own gives the same dots
    let img = s.path("p.png");
    let mut levels = Vec::new();
    for y in 0..8u32 {
        for x in 0..8u32 {
            levels.push(((x * 32 + y * 4) % 256) as u8);
        }
    }
    fs::write(&img, png_of(8, 8, &levels)).unwrap();
    let alone = s.path("alone.nc");
    let (code, _, err) = run(&[
        "engrave",
        &img,
        &alone,
        "--width-mm",
        "8",
        "--dot-mm",
        "1",
        "--power",
        "35",
        "--feed",
        "3000",
    ]);
    assert_eq!(code, Some(0), "{err}");
    let strip = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| !l.starts_with(';') && !l.starts_with('('))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(strip(&g), strip(&fs::read_to_string(&alone).unwrap()));
}

#[test]
fn the_values_of_the_layer_never_pass_the_limits_and_the_options_win() {
    let s = Scratch::new("limits");
    let file = s.write("photo.lbrn2", &engraving_project(""));
    let out = s.path("photo.nc");
    // 35 % is over a limit of 20 %: refused, never lowered
    let (code, _, err) = run(&["engrave", &file, &out, "--dot-mm", "1", "--max-power", "20"]);
    assert_ne!(code, Some(0));
    assert!(err.to_lowercase().contains("power"), "{err}");
    assert!(!std::path::Path::new(&out).exists());
    // the option wins over the layer
    let (code, _, err) = run(&[
        "engrave",
        &file,
        &out,
        "--dot-mm",
        "1",
        "--max-power",
        "20",
        "--power",
        "10",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(fs::read_to_string(&out).unwrap().contains("S100"));
    // a picture that is not there
    let (code, _, err) = run(&["engrave", &file, &out, "--picture", "2", "--dry-run"]);
    assert_ne!(code, Some(0));
    assert!(err.contains("--picture 2"), "{err}");
    // a project with no picture, and the width is needed for an image only
    let none = s.write("none.lbrn2", &project("", &square_at(1.0), ""));
    let (code, _, err) = run(&["engrave", &none, "--dry-run"]);
    assert_ne!(code, Some(0));
    assert!(err.contains("0 picture"), "{err}");
    // the output is never written over the project
    let (code, _, err) = run(&["engrave", &file, &file, "--dot-mm", "1"]);
    assert_ne!(code, Some(0));
    assert!(err.contains("written over"), "{err}");
}

#[test]
fn the_settings_file_changes_the_direction_of_the_picture_in_the_program() {
    let s = Scratch::new("dir");
    let file = s.write("photo.lbrn2", &engraving_project(""));
    let cfg = s.write("off.cfg", "[read]\nlbrn2_flip_y = off\n");
    let on = s.path("on.nc");
    let off = s.path("off.nc");
    assert_eq!(
        run(&["engrave", &file, &on, "--dot-mm", "1", "--power", "35", "--feed", "3000"]).0,
        Some(0)
    );
    let (code, _, err) = run(&[
        "engrave", &file, &off, "--dot-mm", "1", "--power", "35", "--feed", "3000", "--config",
        &cfg,
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(err.contains("lbrn2_flip_y = off"), "{err}");
    assert_ne!(
        fs::read_to_string(&on).unwrap(),
        fs::read_to_string(&off).unwrap()
    );
}

// ---------------------------------------------------------------- a fuzz with a fixed seed

#[test]
fn damaged_pictures_and_new_parts_are_read_or_refused_and_never_stop_the_program() {
    let cuts = layer(
        "Cut",
        &[
            ("index", "0"),
            ("tabsEnabled", "1"),
            ("tabCount", "2"),
            ("tabSize", "1"),
        ],
    );
    let shapes = format!(
        "{}{}{}{}",
        bitmap(0, 3.0, 2.0, "0 1 -1 0 4 4", &base64(&png_of(3, 2, &LEVELS))),
        round("Ellipse", "Rx=\"3\" Ry=\"2\""),
        round("Polygon", "N=\"5\" Rx=\"3\" Ry=\"3\""),
        square_at(5.0)
    );
    let tail = "    <VariableText><Start Value=\"0\"/><End Value=\"9\"/><Current Value=\"0\"/><Increment Value=\"1\"/></VariableText>\n";
    let original = project(&cuts, &shapes, tail).into_bytes();
    assert_eq!(
        read_project(&original, &ReadOptions::default())
            .unwrap()
            .pictures
            .len(),
        1
    );
    let mut rng = Lcg(0x3c6e_f372);
    let (mut with_picture, mut total_ok) = (0usize, 0usize);
    for _ in 0..800 {
        let mut b = original.clone();
        for _ in 0..(1 + rng.next() % 5) {
            let at = (rng.next() as usize) % b.len();
            match rng.next() % 5 {
                0 => b[at] = (rng.next() % 256) as u8,
                1 => {
                    b.remove(at);
                }
                2 => b.insert(
                    at,
                    b"<>\"/=&-0123456789cVLB AZaz+"[(rng.next() % 27) as usize],
                ),
                3 => b.truncate(at.max(1)),
                _ => {
                    let end = (at + 1 + (rng.next() % 6) as usize).min(b.len());
                    b.drain(at..end);
                }
            }
            if b.is_empty() {
                b.push(b'<');
            }
        }
        if let Ok(p) = read_project(&b, &ReadOptions::default()) {
            total_ok += 1;
            with_picture += p.pictures.len().min(1);
            // whatever was read is bounded by the limits
            assert!(p.pictures.len() <= Limits::default().max_bitmaps);
            for pic in &p.pictures {
                assert!(pic.gray.w * pic.gray.h <= Limits::default().max_bitmap_pixels);
            }
        }
        let _ = inventory(&b, &ReadOptions::default());
    }
    assert!(total_ok > 0 && total_ok < 800, "{total_ok}");
    assert!(with_picture > 0, "no damaged file kept its picture");
}

#[test]
fn damaged_pictures_are_refused_by_the_decoder_and_never_stop_the_program() {
    let png = png_of(5, 4, &[7u8; 20]);
    let mut rng = Lcg(0xbb67_ae85);
    let mut decoded = 0usize;
    for _ in 0..1500 {
        let mut b = png.clone();
        for _ in 0..(1 + rng.next() % 4) {
            let at = (rng.next() as usize) % b.len();
            match rng.next() % 3 {
                0 => b[at] = (rng.next() % 256) as u8,
                1 => b.truncate(at.max(1)),
                _ => b.insert(at, (rng.next() % 256) as u8),
            }
        }
        match decode_picture(&base64(&b), &Limits::default()) {
            Ok(g) => {
                decoded += 1;
                assert!(g.w * g.h <= Limits::default().max_bitmap_pixels);
            }
            Err(e) => assert!(e.starts_with("PHOENIX-LASER-LBRN2-BITMAP-"), "{e}"),
        }
    }
    assert!(decoded > 0 && decoded < 1500, "{decoded}");
    // text that is not base64 at all
    for _ in 0..500 {
        let n = (rng.next() % 60) as usize;
        let s: String = (0..n)
            .map(|_| (32 + (rng.next() % 95) as u8) as char)
            .collect();
        let _ = decode_base64(&s, 1 << 16);
        let _ = decode_picture(&s, &Limits::default());
    }
}

// ---------------------------------------------------------------- the window

#[test]
fn the_window_shows_the_warning_about_the_y_axis_and_follows_the_setting() {
    use eh3d_phoenix_laser::lang::Lang;
    use eh3d_phoenix_laser::window::Window;
    use std::path::Path;
    let s = Scratch::new("window");
    let cuts = layer("Cut", &[("index", "0"), ("name", "outline")]);
    let file = s.write("plate.lbrn2", &project(&cuts, &square_at(10.0), ""));
    let mut w = Window::new(Lang::En);
    assert!(w.open_dropped(Path::new(&file)));
    assert!(w
        .warnings
        .iter()
        .any(|n| n.contains("not confirmed with a real part") && n.contains("down")));
    let cfg = s.write("off.cfg", "[read]\nlbrn2_flip_y = off\n");
    let mut w = Window::new(Lang::En);
    assert!(w.load_config(Path::new(&cfg)));
    assert!(w.open_dropped(Path::new(&file)));
    assert!(w.warnings.iter().any(
        |n| n.contains("not confirmed with a real part") && n.contains("up, like the machine")
    ));
}
