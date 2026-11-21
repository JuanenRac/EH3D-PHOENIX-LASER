//! The reader of LBRN2 project files on synthetic projects written here by hand with the structure real projects have (an XML root with layers and shapes, transforms of six numbers,
//! lists of vertices with handles, lists of pieces that are shared, groups, text with an outline, pictures), plus hostile files and a fuzz with a fixed seed. Nothing here is a copy of a
//! file made by somebody else.

use eh3d_phoenix_laser::cut::{plan, verify_gcode};
use eh3d_phoenix_laser::lbrn2::{
    inventory, read_lbrn2, read_project, CutKind, READ_ATTRIBUTES, READ_ELEMENTS,
};
use eh3d_phoenix_laser::vconfig::{LayerMode, Settings};
use eh3d_phoenix_laser::vector::{bounds_of, Drawing, Limits, Pt, ReadOptions, Source};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

/// A project with the layers and the shapes given.
fn project(cuts: &str, shapes: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n<LightBurnProject AppVersion=\"9.9.00\" DeviceName=\"Test bench\" FormatVersion=\"1\" MaterialHeight=\"0\" MirrorX=\"False\" MirrorY=\"True\">\r\n    <Thumbnail Source=\"AAAA\"/>\r\n    <VariableText>\r\n        <Start Value=\"0\"/>\r\n    </VariableText>\r\n{cuts}{shapes}    <Notes ShowOnLoad=\"0\" Notes=\"\"/>\r\n</LightBurnProject>\r\n"
    )
}

/// A layer (`CutSetting`) with the values given as `name=value` pairs.
fn cut(kind: &str, values: &[(&str, &str)]) -> String {
    let mut s = format!("    <CutSetting type=\"{kind}\">\n");
    for (k, v) in values {
        s.push_str(&format!("        <{k} Value=\"{v}\"/>\n"));
    }
    s.push_str("    </CutSetting>\n");
    s
}

fn rect(cut_index: u32, w: f64, h: f64, xform: &str) -> String {
    format!("    <Shape Type=\"Rect\" CutIndex=\"{cut_index}\" W=\"{w}\" H=\"{h}\" Cr=\"0\">\n        <XForm>{xform}</XForm>\n    </Shape>\n")
}

fn path(
    cut_index: u32,
    ids: Option<(u32, u32)>,
    verts: Option<&str>,
    prims: Option<&str>,
    xform: &str,
) -> String {
    let mut s = format!("    <Shape Type=\"Path\" CutIndex=\"{cut_index}\"");
    if let Some((v, p)) = ids {
        s.push_str(&format!(" VertID=\"{v}\" PrimID=\"{p}\""));
    }
    s.push_str(">\n");
    s.push_str(&format!("        <XForm>{xform}</XForm>\n"));
    if let Some(v) = verts {
        s.push_str(&format!("        <VertList>{v}</VertList>\n"));
    }
    if let Some(p) = prims {
        s.push_str(&format!("        <PrimList>{p}</PrimList>\n"));
    }
    s.push_str("    </Shape>\n");
    s
}

const IDENT: &str = "1 0 0 1 0 0";
const OPTS: fn() -> ReadOptions = ReadOptions::default;

fn read(text: &str) -> Drawing {
    read_lbrn2(text.as_bytes(), &OPTS()).unwrap()
}

fn bbox(d: &Drawing) -> (Pt, Pt) {
    bounds_of(d.shapes.iter().flat_map(|s| s.pts.iter().copied())).expect("points")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn dist_to_polyline(p: Pt, pts: &[Pt]) -> f64 {
    pts.windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 {
                0.0
            } else {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
            };
            (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
        })
        .fold(f64::INFINITY, f64::min)
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-lbrn2-{}-{tag}", std::process::id()));
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

// ---------------------------------------------------------------- the root and the layers

#[test]
fn the_root_gives_the_program_the_device_the_form_and_the_mirror_flags() {
    let p = read_project(project("", &rect(0, 10.0, 10.0, IDENT)).as_bytes(), &OPTS()).unwrap();
    assert_eq!(p.app_version, "9.9.00");
    assert_eq!(p.device_name, "Test bench");
    assert_eq!(p.format_version, "1");
    assert_eq!(p.mirror, (false, true));
    assert_eq!(p.drawing.source, Source::Svg);
}

#[test]
fn the_layers_are_read_with_their_type_number_name_and_every_value_as_written() {
    let cuts = format!(
        "{}{}",
        cut(
            "Cut",
            &[
                ("index", "1"),
                ("name", "outline"),
                ("maxPower", "85"),
                ("minPower", "60"),
                ("speed", "8"),
                ("numPasses", "2"),
                ("priority", "3"),
                ("tabCount", "1")
            ]
        ),
        cut(
            "Scan",
            &[
                ("index", "0"),
                ("name", ""),
                ("maxPower", "20"),
                ("speed", "600"),
                ("interval", "0.04"),
                ("priority", "0")
            ]
        ),
    );
    let p = read_project(project(&cuts, "").as_bytes(), &OPTS()).unwrap();
    assert_eq!(p.cut_settings.len(), 2);
    let c = &p.cut_settings[0];
    assert_eq!(
        (c.kind.clone(), c.index, c.name.as_str()),
        (CutKind::Cut, 1, "outline")
    );
    assert_eq!(c.max_power(), Some(85.0));
    assert_eq!(c.min_power(), Some(60.0));
    assert_eq!(c.speed_mm_s(), Some(8.0));
    assert_eq!(c.passes(), 2);
    assert_eq!(c.priority(), Some(3));
    assert!(c.output());
    // a value this reader does not use is kept as written
    assert!(c.values.iter().any(|(k, v)| k == "tabCount" && v == "1"));
    let s = &p.cut_settings[1];
    assert_eq!(s.kind, CutKind::Scan);
    assert_eq!(s.interval_mm(), Some(0.04));
    // a layer with no name is called C and its number with two digits
    assert_eq!(s.label(), "C00");
    assert_eq!(c.label(), "outline");
}

#[test]
fn a_layer_becomes_a_hint_for_the_table_with_the_speed_in_mm_per_minute() {
    let cuts = format!(
        "{}{}{}",
        cut(
            "Cut",
            &[
                ("index", "1"),
                ("name", "C01"),
                ("maxPower", "85"),
                ("speed", "20"),
                ("numPasses", "3"),
                ("priority", "3")
            ]
        ),
        cut(
            "Scan",
            &[
                ("index", "0"),
                ("name", "C00"),
                ("maxPower", "20"),
                ("speed", "600"),
                ("interval", "0.04"),
                ("priority", "1")
            ]
        ),
        cut(
            "Cut",
            &[
                ("index", "2"),
                ("name", "C02"),
                ("maxPower", "50"),
                ("speed", "10"),
                ("doOutput", "0")
            ]
        ),
    );
    let p = read_project(project(&cuts, "").as_bytes(), &OPTS()).unwrap();
    let hints = p.hints();
    let get = |k: &str| hints.iter().find(|(n, _)| n == k).unwrap().1;
    let c1 = get("c01");
    assert_eq!(c1.mode, LayerMode::Cut);
    assert!(close(c1.power_percent, 85.0));
    // the file gives mm/s, the table mm/min
    assert!(close(c1.speed_mm_min, 1200.0));
    assert_eq!(c1.passes, 3);
    assert_eq!(c1.order, 3);
    let c0 = get("c00");
    assert_eq!(c0.mode, LayerMode::Engrave);
    assert!(close(c0.speed_mm_min, 36000.0));
    assert!(close(c0.hatch_spacing_mm, 0.04));
    assert_eq!(c0.order, 1);
    // a layer that is not sent to the laser is skipped
    assert_eq!(get("c02").mode, LayerMode::Skip);
    // the drawing says the values are the project's
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("check them for this machine")));
}

#[test]
fn a_layer_without_a_number_is_left_out_with_a_note_and_one_without_values_is_not_a_problem() {
    let cuts = "    <CutSetting type=\"Cut\">\n        <name Value=\"x\"/>\n    </CutSetting>\n    <CutSetting type=\"Weird\">\n        <index Value=\"4\"/>\n    </CutSetting>\n";
    let p = read_project(project(cuts, "").as_bytes(), &OPTS()).unwrap();
    assert_eq!(p.cut_settings.len(), 1);
    assert_eq!(p.cut_settings[0].kind, CutKind::Other("Weird".to_string()));
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("without a number")));
    // the hint of a layer with no values is the default of the table
    let h = p.cut_settings[0].hint();
    assert_eq!(h.mode, LayerMode::Engrave);
    assert!(close(h.speed_mm_min, 600.0));
}

#[test]
fn the_shapes_name_their_layer_by_number_and_a_number_with_no_layer_is_called_by_its_number() {
    let cuts = cut("Cut", &[("index", "1"), ("name", "outline")]);
    let shapes = format!(
        "{}{}{}",
        rect(1, 4.0, 4.0, IDENT),
        rect(7, 4.0, 4.0, "1 0 0 1 20 0"),
        "    <Shape Type=\"Rect\" W=\"2\" H=\"2\"><XForm>1 0 0 1 40 0</XForm></Shape>\n"
    );
    let d = read(&project(&cuts, &shapes));
    let layers: Vec<&str> = d.shapes.iter().map(|s| s.layer.as_str()).collect();
    assert_eq!(layers, ["outline", "C07", "C00"]);
    assert!(d.shapes.iter().all(|s| s.color.is_none()));
}

// ---------------------------------------------------------------- the shapes

#[test]
fn a_rectangle_is_centred_on_its_origin_and_moved_by_its_transform() {
    let d = read(&project("", &rect(0, 20.0, 10.0, "1 0 0 1 100 50")));
    assert_eq!(d.shapes.len(), 1);
    assert!(d.shapes[0].closed && d.shapes[0].pts.len() == 4);
    let (lo, hi) = bbox(&d);
    assert!(
        close(lo.0, 90.0) && close(lo.1, 45.0) && close(hi.0, 110.0) && close(hi.1, 55.0),
        "{lo:?} {hi:?}"
    );
}

#[test]
fn a_rounded_rectangle_keeps_its_size_and_gains_points_and_a_radius_too_big_is_cut_to_fit() {
    let rounded = "    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"20\" H=\"10\" Cr=\"3\"><XForm>1 0 0 1 0 0</XForm></Shape>\n";
    let d = read(&project("", rounded));
    let (lo, hi) = bbox(&d);
    assert!(
        close(hi.0 - lo.0, 20.0) && close(hi.1 - lo.1, 10.0),
        "{lo:?} {hi:?}"
    );
    assert!(d.shapes[0].pts.len() > 8);
    // the corner is cut: the point (10, 5) is not on the outline
    assert!(dist_to_polyline((10.0, 5.0), &d.shapes[0].pts) > 0.5);
    let big = "    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"20\" H=\"10\" Cr=\"99\"><XForm>1 0 0 1 0 0</XForm></Shape>\n";
    let b = read(&project("", big));
    let (lo, hi) = bbox(&b);
    assert!(close(hi.0 - lo.0, 20.0) && close(hi.1 - lo.1, 10.0));
}

#[test]
fn a_rectangle_without_a_size_is_left_out_and_counted() {
    let shapes = "    <Shape Type=\"Rect\" CutIndex=\"0\"><XForm>1 0 0 1 0 0</XForm></Shape>\n    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"0\" H=\"5\"><XForm>1 0 0 1 0 0</XForm></Shape>\n";
    let p = read_project(project("", shapes).as_bytes(), &OPTS()).unwrap();
    assert!(p.drawing.shapes.is_empty());
    assert!(
        p.stats
            .skipped
            .iter()
            .any(|(k, n)| k == "rectangle without a size" && *n == 2),
        "{:?}",
        p.stats.skipped
    );
}

#[test]
fn line_closed_is_a_polygon_through_every_vertex() {
    let tri = path(
        0,
        None,
        Some("V0 0V10 0V0 10"),
        Some("LineClosed"),
        "2 0 0 2 5 5",
    );
    let d = read(&project("", &tri));
    assert_eq!(d.shapes.len(), 1);
    let s = &d.shapes[0];
    assert!(s.closed);
    assert_eq!(s.pts.len(), 3);
    assert!(
        close(s.pts[1].0, 25.0) && close(s.pts[1].1, 5.0),
        "{:?}",
        s.pts
    );
    assert!(close(s.pts[2].0, 5.0) && close(s.pts[2].1, 25.0));
}

#[test]
fn pieces_that_return_to_the_first_vertex_close_the_outline_and_others_leave_it_open() {
    let closed = path(
        0,
        None,
        Some("V0 0V10 0V10 10V0 10"),
        Some("L0 1L1 2L2 3L3 0"),
        IDENT,
    );
    let open = path(0, None, Some("V0 0V10 0V10 10"), Some("L0 1L1 2"), IDENT);
    let d = read(&project("", &format!("{closed}{open}")));
    assert_eq!(d.shapes.len(), 2);
    assert!(d.shapes[0].closed && d.shapes[0].pts.len() == 4);
    assert!(!d.shapes[1].closed && d.shapes[1].pts.len() == 3);
}

#[test]
fn a_chain_that_breaks_starts_another_outline() {
    let two = path(
        0,
        None,
        Some("V0 0V10 0V10 10V20 20V30 20V30 30"),
        Some("L0 1L1 2L2 0L3 4L4 5L5 3"),
        IDENT,
    );
    let d = read(&project("", &two));
    assert_eq!(d.shapes.len(), 2);
    assert!(d.shapes.iter().all(|s| s.closed && s.pts.len() == 3));
}

#[test]
fn a_bezier_leaves_the_first_vertex_by_its_c0_and_arrives_by_the_c1_of_the_second() {
    // from (0,0) with the handle c0 at (0,10) to (10,10) with the handle c1 at (0,10): the curve bulges to the upper left, and its middle is at (1.25, 8.75)
    let verts = "V0 0c0x0c0y10c1x5c1y5V10 10c0x15c0y15c1x0c1y10";
    let p = path(0, None, Some(verts), Some("B0 1"), IDENT);
    let d = read(&project("", &p));
    assert_eq!(d.shapes.len(), 1);
    let pts = &d.shapes[0].pts;
    assert!(pts.len() > 4, "the curve is cut into pieces: {pts:?}");
    assert!(dist_to_polyline((1.25, 8.75), pts) < 0.01, "{pts:?}");
    assert!(close(pts[0].0, 0.0) && close(pts[0].1, 0.0));
    let last = pts[pts.len() - 1];
    assert!(close(last.0, 10.0) && close(last.1, 10.0));
    // if the handles were taken the other way round (c1 of the first, c0 of the second) the curve would bulge to the lower right instead
    assert!(dist_to_polyline((8.75, 1.25), pts) > 1.0);
}

#[test]
fn a_handle_with_only_its_x_stands_for_no_handle_and_the_piece_is_straight() {
    let p = path(
        0,
        None,
        Some("V0 0c0x1c1x1V10 0c0x1c1x1"),
        Some("B0 1"),
        IDENT,
    );
    let d = read(&project("", &p));
    let pts = &d.shapes[0].pts;
    assert!(pts.iter().all(|q| q.1.abs() < 1e-9), "{pts:?}");
    assert!(close(pts[pts.len() - 1].0, 10.0));
}

#[test]
fn the_curves_are_cut_finer_when_the_tolerance_is_finer() {
    let verts = "V0 0c0x0c0y10c1x5c1y5V10 10c0x15c0y15c1x0c1y10";
    let text = project("", &path(0, None, Some(verts), Some("B0 1"), IDENT));
    let coarse = read_lbrn2(
        text.as_bytes(),
        &ReadOptions {
            tolerance_mm: 0.5,
            ..ReadOptions::default()
        },
    )
    .unwrap();
    let fine = read_lbrn2(
        text.as_bytes(),
        &ReadOptions {
            tolerance_mm: 0.002,
            ..ReadOptions::default()
        },
    )
    .unwrap();
    assert!(fine.shapes[0].pts.len() > coarse.shapes[0].pts.len() * 2);
}

#[test]
fn the_lists_a_path_names_can_be_left_out_by_the_ones_after_it_and_are_counted_as_shared() {
    let first = path(
        0,
        Some((5, 9)),
        Some("V0 0V10 0V10 10"),
        Some("LineClosed"),
        IDENT,
    );
    let second = path(0, Some((5, 9)), None, None, "1 0 0 1 100 0");
    let third = path(0, Some((5, 9)), None, None, "1 0 0 1 0 100");
    let p = read_project(
        project("", &format!("{first}{second}{third}")).as_bytes(),
        &OPTS(),
    )
    .unwrap();
    assert_eq!(p.drawing.shapes.len(), 3);
    assert_eq!(p.stats.shared_lists, 4);
    let xs: Vec<f64> = p.drawing.shapes.iter().map(|s| s.pts[0].0).collect();
    assert!(close(xs[0], 0.0) && close(xs[1], 100.0) && close(xs[2], 0.0));
    assert!(close(p.drawing.shapes[2].pts[0].1, 100.0));
}

#[test]
fn a_path_that_names_lists_nobody_wrote_is_skipped_with_a_note_and_the_rest_is_read() {
    let lost = path(0, Some((77, 78)), None, None, IDENT);
    let fine = rect(0, 4.0, 4.0, IDENT);
    let p = read_project(project("", &format!("{lost}{fine}")).as_bytes(), &OPTS()).unwrap();
    assert_eq!(p.drawing.shapes.len(), 1);
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("no earlier path wrote")));
    assert!(p
        .stats
        .skipped
        .iter()
        .any(|(k, _)| k == "path without readable lists"));
}

#[test]
fn groups_place_their_children_and_the_matrix_of_the_child_acts_first() {
    // the group scales by 0.5 and moves by (100, 0); the child rectangle of 8 x 8 sits at (10, 0) inside the group
    let inner = rect(0, 8.0, 8.0, "1 0 0 1 10 0");
    let group = format!("    <Shape Type=\"Group\" CutIndex=\"0\">\n        <XForm>0.5 0 0 0.5 100 0</XForm>\n        <Children>\n{inner}        </Children>\n    </Shape>\n");
    let d = read(&project("", &group));
    let (lo, hi) = bbox(&d);
    // centre x = 100 + 0.5 * 10 = 105, half width 0.5 * 4 = 2
    assert!(close(lo.0, 103.0) && close(hi.0, 107.0), "{lo:?} {hi:?}");
    assert!(close(lo.1, -2.0) && close(hi.1, 2.0));
}

#[test]
fn groups_inside_groups_compose_and_a_turn_of_a_quarter_moves_x_to_y() {
    // a point at (1, 0) turned by the matrix 0 1 -1 0 goes to (0, 1)
    let tri = path(
        0,
        None,
        Some("V1 0V2 0V1 1"),
        Some("LineClosed"),
        "0 1 -1 0 0 0",
    );
    let d = read(&project("", &tri));
    let p = &d.shapes[0].pts;
    assert!(close(p[0].0, 0.0) && close(p[0].1, 1.0), "{p:?}");
    assert!(close(p[1].0, 0.0) && close(p[1].1, 2.0));
    assert!(close(p[2].0, -1.0) && close(p[2].1, 1.0));
    let inner = path(
        0,
        None,
        Some("V0 0V1 0V0 1"),
        Some("LineClosed"),
        "1 0 0 1 3 0",
    );
    let inner_group = format!("    <Shape Type=\"Group\" CutIndex=\"0\"><XForm>2 0 0 2 0 0</XForm><Children>\n{inner}</Children></Shape>\n");
    let outer = format!("    <Shape Type=\"Group\" CutIndex=\"0\"><XForm>1 0 0 1 100 0</XForm><Children>\n{inner_group}</Children></Shape>\n");
    let n = read(&project("", &outer));
    // 100 + 2 * (3 + 0) = 106, 100 + 2 * (3 + 1) = 108
    let (lo, hi) = bbox(&n);
    assert!(close(lo.0, 106.0) && close(hi.0, 108.0), "{lo:?} {hi:?}");
}

#[test]
fn a_text_is_drawn_from_its_backup_path_with_the_transform_of_that_path_only() {
    let text = "    <Shape Type=\"Text\" CutIndex=\"0\" Font=\"Sans,-1\" Str=\"ab\" H=\"5\" HasBackupPath=\"1\">\n        <BackupPath Type=\"Path\" CutIndex=\"0\">\n            <XForm>1 0 0 1 10 20</XForm>\n            <VertList>V0 0V4 0V4 4</VertList>\n            <PrimList>LineClosed</PrimList>\n        </BackupPath>\n        <XForm>1 0 0 1 10 20</XForm>\n    </Shape>\n";
    let d = read(&project("", text));
    let (lo, hi) = bbox(&d);
    // the text and its backup path carry the same transform: it is applied once, not twice
    assert!(
        close(lo.0, 10.0) && close(lo.1, 20.0) && close(hi.0, 14.0) && close(hi.1, 24.0),
        "{lo:?} {hi:?}"
    );
    // a text with no outline is left out and counted
    let bare =
        "    <Shape Type=\"Text\" CutIndex=\"0\" Str=\"x\"><XForm>1 0 0 1 0 0</XForm></Shape>\n";
    let p = read_project(project("", bare).as_bytes(), &OPTS()).unwrap();
    assert!(p.drawing.shapes.is_empty());
    assert!(p
        .stats
        .skipped
        .iter()
        .any(|(k, _)| k == "text without an outline"));
}

#[test]
fn a_picture_that_cannot_be_decoded_and_a_shape_of_a_type_not_seen_are_left_out_with_a_note_and_the_rest_is_read(
) {
    let bitmap = "    <Shape Type=\"Bitmap\" CutIndex=\"0\" W=\"10\" H=\"10\" File=\"x.png\" Data=\"iVBORw0KGgo=\"><XForm>1 0 0 1 0 0</XForm></Shape>\n";
    let odd = "    <Shape Type=\"Spiral\" CutIndex=\"0\" Turns=\"3\"><XForm>1 0 0 1 0 0</XForm></Shape>\n";
    let ok = rect(0, 6.0, 6.0, IDENT);
    let p = read_project(
        project("", &format!("{bitmap}{odd}{ok}")).as_bytes(),
        &OPTS(),
    )
    .unwrap();
    assert_eq!(p.drawing.shapes.len(), 1);
    // eight bytes that begin like a PNG are no picture
    assert!(p
        .stats
        .skipped
        .iter()
        .any(|(k, n)| k == "picture that could not be decoded" && *n == 1));
    assert!(p
        .stats
        .skipped
        .iter()
        .any(|(k, n)| k == "shape of type Spiral" && *n == 1));
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("1 picture that could not be decoded not read")));
    assert!(p.pictures.is_empty());
    let types: Vec<&str> = p
        .stats
        .shape_types
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert_eq!(types, ["Bitmap", "Spiral", "Rect"]);
}

#[test]
fn a_picture_with_a_very_long_data_attribute_is_not_kept_in_memory_but_is_counted() {
    let big = "A".repeat(200_000);
    let bitmap = format!("    <Shape Type=\"Bitmap\" CutIndex=\"0\" W=\"10\" H=\"10\" Data=\"{big}\"><XForm>1 0 0 1 0 0</XForm></Shape>\n");
    let p = read_project(project("", &bitmap).as_bytes(), &OPTS()).unwrap();
    // it is text of zeros once decoded: not a picture in a format that is read
    assert!(p
        .stats
        .skipped
        .iter()
        .any(|(k, _)| k.starts_with("picture")));
    assert!(p.pictures.is_empty());
    let inv = inventory(project("", &bitmap).as_bytes(), &OPTS()).unwrap();
    assert!(inv
        .attributes
        .iter()
        .any(|(k, n)| k == "Shape.Data" && *n == 1));
}

#[test]
fn the_parts_of_a_shape_that_the_reader_does_not_know_are_counted() {
    let shape = "    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"5\" H=\"5\"><XForm>1 0 0 1 0 0</XForm><Mystery Value=\"1\"/></Shape>\n";
    let p = read_project(project("", shape).as_bytes(), &OPTS()).unwrap();
    assert_eq!(p.stats.unknown_parts, vec![("Mystery".to_string(), 1)]);
    assert_eq!(p.drawing.shapes.len(), 1);
}

#[test]
fn the_statistics_count_what_was_followed() {
    let p1 = path(
        0,
        None,
        Some("V0 0V10 0V10 10c0x12c0y12c1x9c1y9"),
        Some("L0 1B1 2L2 0"),
        IDENT,
    );
    let p = read_project(
        project("", &format!("{p1}{}", rect(0, 2.0, 2.0, IDENT))).as_bytes(),
        &OPTS(),
    )
    .unwrap();
    assert_eq!((p.stats.lines, p.stats.beziers), (2, 1));
    assert_eq!(p.stats.vertices, 3);
    assert_eq!(p.stats.read_shapes, 2);
    assert!(p.stats.elements >= 10);
}

// ---------------------------------------------------------------- hostile and broken files

fn refused(text: &str) -> String {
    read_lbrn2(text.as_bytes(), &OPTS()).unwrap_err()
}

#[test]
fn files_that_are_not_projects_are_refused_with_the_reason() {
    assert!(refused("").contains("no element"));
    assert!(refused("hello").contains("no element"));
    assert!(refused("<svg xmlns=\"http://www.w3.org/2000/svg\"/>").contains("no LightBurnProject"));
    assert!(refused("<LightBurnProject><Shape></LightBurnProject>").contains("is closed as"));
    assert!(refused("<LightBurnProject>").contains("not closed"));
    assert!(refused("<LightBurnProject a=1/>").contains("not quoted"));
    assert!(refused("<LightBurnProject a=\"1/>").contains("not closed"));
    assert!(refused("</LightBurnProject>").contains("no start tag"));
    assert!(refused("<LightBurnProject><!-- never ends</LightBurnProject>").contains("comment"));
    assert!(refused("<LightBurnProject").contains("tag is not closed"));
}

#[test]
fn a_declared_entity_is_never_expanded() {
    let t = "<?xml version=\"1.0\"?>\n<!DOCTYPE LightBurnProject [<!ENTITY a \"aaaa\">]>\n<LightBurnProject AppVersion=\"&a;\"/>";
    assert!(refused(t).contains("entities"));
    // the predefined entities and numeric references are read
    let ok = "<LightBurnProject DeviceName=\"a &amp; b &#65;\"/>";
    let p = read_project(ok.as_bytes(), &OPTS()).unwrap();
    assert_eq!(p.device_name, "a & b A");
}

#[test]
fn the_limits_hold_for_the_size_the_depth_and_the_number_of_elements() {
    let text = project("", &rect(0, 4.0, 4.0, IDENT));
    let small = ReadOptions {
        limits: Limits {
            max_file_bytes: 50,
            ..Limits::default()
        },
        ..ReadOptions::default()
    };
    assert!(read_lbrn2(text.as_bytes(), &small)
        .unwrap_err()
        .contains("max_file_bytes"));
    let flat = ReadOptions {
        limits: Limits {
            max_depth: 2,
            ..Limits::default()
        },
        ..ReadOptions::default()
    };
    assert!(read_lbrn2(text.as_bytes(), &flat)
        .unwrap_err()
        .contains("max_depth"));
    let few = ReadOptions {
        limits: Limits {
            max_nodes: 3,
            ..Limits::default()
        },
        ..ReadOptions::default()
    };
    assert!(read_lbrn2(text.as_bytes(), &few)
        .unwrap_err()
        .contains("max_nodes"));
    // a bomb of nested groups stops at the depth
    let mut bomb = String::from("<LightBurnProject>");
    for _ in 0..200 {
        bomb.push_str("<Shape Type=\"Group\"><Children>");
    }
    assert!(refused(&bomb).contains("max_depth"));
    // a project of many shapes stops at the number of shapes
    let many = ReadOptions {
        limits: Limits {
            max_shapes: 2,
            ..Limits::default()
        },
        ..ReadOptions::default()
    };
    let shapes: String = (0..5)
        .map(|k| rect(0, 2.0, 2.0, &format!("1 0 0 1 {} 0", k * 10)))
        .collect();
    assert!(read_lbrn2(project("", &shapes).as_bytes(), &many)
        .unwrap_err()
        .contains("max_shapes"));
    // many paths that name one long list of vertices do not read it over and over without end
    let few_points = ReadOptions {
        limits: Limits {
            max_points: 10,
            ..Limits::default()
        },
        ..ReadOptions::default()
    };
    let first = path(
        0,
        Some((1, 1)),
        Some("V0 0V1 0V1 1V0 1V2 2V3 3V4 4"),
        Some("LineClosed"),
        IDENT,
    );
    let again = path(0, Some((1, 1)), None, None, IDENT);
    let text = project("", &format!("{first}{again}{again}"));
    assert!(read_lbrn2(text.as_bytes(), &few_points).is_err());
}

#[test]
fn numbers_that_are_not_numbers_and_points_too_far_away_never_get_into_the_drawing() {
    let nan = path(
        0,
        None,
        Some("V0 0V10 0V0 10"),
        Some("LineClosed"),
        "NaN 0 0 1 0 0",
    );
    let p = read_project(project("", &nan).as_bytes(), &OPTS()).unwrap();
    assert!(p.drawing.shapes.is_empty());
    assert!(p
        .drawing
        .warnings
        .iter()
        .any(|w| w.contains("transform that is not six numbers")));
    // five numbers are not a transform: the shape has no place, so it is left out and counted
    let five = rect(0, 4.0, 4.0, "1 0 0 1 0");
    let q = read_project(project("", &five).as_bytes(), &OPTS()).unwrap();
    assert!(q.drawing.shapes.is_empty());
    assert!(q
        .stats
        .skipped
        .iter()
        .any(|(k, _)| k == "shape with a transform that is not six numbers"));
    let far = path(0, None, Some("V0 0V1e9 0V0 10"), Some("LineClosed"), IDENT);
    let f = read(&project("", &far));
    assert!(f.shapes.is_empty());
    assert!(f.warnings.iter().any(|w| w.contains("too far away")));
    let bad_vert = path(0, None, Some("V0 zzz"), Some("LineClosed"), IDENT);
    let b = read(&project("", &bad_vert));
    assert!(b.shapes.is_empty());
    assert!(
        b.warnings.iter().any(|w| w.contains("no y")),
        "{:?}",
        b.warnings
    );
}

#[test]
fn a_piece_that_joins_a_vertex_that_is_not_there_skips_the_path_and_a_bad_list_of_pieces_too() {
    let out = path(0, None, Some("V0 0V1 0"), Some("L0 5"), IDENT);
    let bad = path(0, None, Some("V0 0V1 0"), Some("X0 1"), IDENT);
    let huge = path(0, None, Some("V0 0V1 0"), Some("L0 99999999999"), IDENT);
    let d = read(&project("", &format!("{out}{bad}{huge}")));
    assert!(d.shapes.is_empty());
    assert!(
        d.warnings
            .iter()
            .any(|w| w.contains("joins vertices 0 and 5")),
        "{:?}",
        d.warnings
    );
    assert!(d.warnings.iter().any(|w| w.contains("L or B was expected")));
}

#[test]
fn an_unknown_handle_name_skips_the_path() {
    let odd = path(0, None, Some("V0 0c2x1V1 0"), Some("L0 1"), IDENT);
    let d = read(&project("", &odd));
    assert!(d.shapes.is_empty());
    assert!(d
        .warnings
        .iter()
        .any(|w| w.contains("handle this reader does not know")));
}

#[test]
fn a_file_with_a_byte_order_mark_and_other_line_endings_reads_the_same() {
    let text = project("", &rect(0, 4.0, 4.0, "1 0 0 1 1 1"));
    let a = read(&text);
    let b = read(&format!("\u{feff}{}", text.replace("\r\n", "\n")));
    let c = read(&text.replace("\r\n", "\r"));
    assert_eq!(a.shapes, b.shapes);
    assert_eq!(a.shapes, c.shapes);
}

#[test]
fn reading_twice_gives_the_same_drawing() {
    let verts = "V0 0c0x0c0y10c1x5c1y5V10 10c0x15c0y15c1x0c1y10";
    let text = project(
        "",
        &format!(
            "{}{}",
            path(0, None, Some(verts), Some("B0 1"), IDENT),
            rect(1, 7.0, 3.0, "0.7 0.7 -0.7 0.7 5 5")
        ),
    );
    assert_eq!(read(&text), read(&text));
}

// ---------------------------------------------------------------- what a file holds and what the reader leaves out

#[test]
fn the_inventory_counts_every_element_and_attribute_and_names_the_ones_the_reader_leaves_out() {
    let cuts = cut(
        "Cut",
        &[
            ("index", "0"),
            ("maxPower", "10"),
            ("speed", "5"),
            ("dotTime", "1"),
        ],
    );
    let text = project(&cuts, &rect(0, 3.0, 3.0, IDENT));
    let inv = inventory(text.as_bytes(), &OPTS()).unwrap();
    let count = |k: &str| inv.elements.iter().find(|(n, _)| n == k).map(|(_, n)| *n);
    assert_eq!(count("Shape"), Some(1));
    assert_eq!(count("CutSetting"), Some(1));
    assert_eq!(count("LightBurnProject"), Some(1));
    let unread: Vec<&str> = inv
        .unread_elements()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    for left_out in ["Thumbnail", "Notes", "dotTime"] {
        assert!(unread.contains(&left_out), "{left_out} in {unread:?}");
    }
    // the counter of the variable text is read now
    assert!(!unread.contains(&"VariableText") && !unread.contains(&"Start"));
    assert!(!unread.contains(&"maxPower") && !unread.contains(&"Shape"));
    let attrs: Vec<&str> = inv
        .unread_attributes()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect();
    assert!(attrs.contains(&"LightBurnProject.MaterialHeight"));
    assert!(attrs.contains(&"Thumbnail.Source"));
    assert!(!attrs.contains(&"Shape.W"));
}

#[test]
fn what_the_reader_uses_is_listed_without_repeats_and_in_the_form_the_inventory_writes() {
    for list in [READ_ELEMENTS.as_slice(), READ_ATTRIBUTES.as_slice()] {
        let mut sorted = list.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), list.len());
    }
    for a in READ_ATTRIBUTES {
        let (element, attribute) = a.split_once('.').expect("element.attribute");
        assert!(!element.is_empty() && !attribute.is_empty());
        assert!(READ_ELEMENTS.contains(&element), "{a}");
    }
}

// ---------------------------------------------------------------- from the project to a program and to the command line

#[test]
fn a_project_is_cut_into_a_program_that_passes_the_check() {
    let cuts = cut("Cut", &[("index", "0"), ("name", "C00")]);
    let shapes = format!(
        "{}{}",
        rect(0, 40.0, 20.0, "1 0 0 1 30 20"),
        path(
            0,
            None,
            Some("V0 0V10 0V10 10"),
            Some("LineClosed"),
            "1 0 0 1 20 10"
        )
    );
    let d = read(&project(&cuts, &shapes));
    let mut s = Settings::default();
    s.machine.bed_mm = Some((400.0, 400.0));
    let planned = plan(&d, &s).unwrap();
    verify_gcode(&planned.gcode, s.machine.s_max, Some((400.0, 400.0))).unwrap();
    assert!(planned.gcode.contains("G1"));
    // the project is turned over like an SVG: the first vertex (y = 10 in the file, the lowest in the picture) is on the line of the highest y of the bed
    assert_eq!(d.source, Source::Svg);
}

#[test]
fn the_command_line_lists_converts_and_cuts_a_project() {
    let s = Scratch::new("cli");
    let cuts = cut(
        "Cut",
        &[
            ("index", "1"),
            ("name", "outline"),
            ("maxPower", "30"),
            ("speed", "10"),
        ],
    );
    let file = s.write(
        "plate.lbrn2",
        &project(&cuts, &rect(1, 30.0, 20.0, "1 0 0 1 50 50")),
    );
    let (code, listing, err) = run(&["layers", &file]);
    assert_eq!(code, Some(0), "{err}");
    assert!(listing.contains("outline"), "{listing}");
    let svg = s.path("plate.svg");
    let (code, out, err) = run(&["convert", &file, &svg]);
    assert_eq!(code, Some(0), "{err}");
    assert!(out.contains("1 shapes in 1 layer(s) written to"), "{out}");
    let out_nc = s.path("plate.nc");
    let (code, _, err) = run(&["cut", &file, &out_nc]);
    assert_eq!(code, Some(0), "{err}");
    let g = fs::read_to_string(&out_nc).unwrap();
    assert!(g.contains("G1"), "{g}");
    // a file that is not a project is refused with the extension it has
    let bad = s.write("bad.lbrn2", "this is not xml");
    let (code, _, err) = run(&["layers", &bad]);
    assert_ne!(code, Some(0));
    assert!(err.contains("bad.lbrn2"), "{err}");
}

// ---------------------------------------------------------------- a fuzz with a fixed seed

/// A small generator of numbers with a fixed seed, so a failure can be run again.
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

#[test]
fn damaged_projects_are_read_or_refused_and_never_stop_the_program() {
    let cuts = cut(
        "Cut",
        &[("index", "0"), ("maxPower", "40"), ("speed", "12")],
    );
    let verts = "V0 0c0x0c0y10c1x5c1y5V10 10c0x15c0y15c1x0c1y10V20 0";
    let shapes = format!(
        "{}{}{}",
        path(0, Some((1, 1)), Some(verts), Some("B0 1L1 2L2 0"), IDENT),
        path(0, Some((1, 1)), None, None, "1 0 0 1 30 0"),
        rect(0, 8.0, 8.0, "1 0 0 1 60 0")
    );
    let original = project(&cuts, &shapes).into_bytes();
    assert!(read_lbrn2(&original, &OPTS()).is_ok());
    let mut rng = Lcg(0x5eed_1b2a);
    let mut survived_ok = 0usize;
    for _ in 0..600 {
        let mut b = original.clone();
        for _ in 0..(1 + rng.next() % 4) {
            let at = (rng.next() as usize) % b.len();
            match rng.next() % 4 {
                0 => b[at] = (rng.next() % 256) as u8,
                1 => {
                    b.remove(at);
                }
                2 => b.insert(at, b"<>\"/=&-0123456789cVLB "[(rng.next() % 22) as usize]),
                _ => b.truncate(at.max(1)),
            }
            if b.is_empty() {
                b.push(b'<');
            }
        }
        if read_lbrn2(&b, &OPTS()).is_ok() {
            survived_ok += 1;
        }
        let _ = inventory(&b, &OPTS());
    }
    // some of the damaged files still read (the damage fell in a place that does not matter), and the rest were refused without a panic
    assert!(survived_ok > 0 && survived_ok < 600, "{survived_ok}");
}
