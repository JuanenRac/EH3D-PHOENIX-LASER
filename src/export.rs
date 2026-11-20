// EH3D-PHOENIX-LASER - writing a drawing as SVG or DXF.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A drawing the program has read (an SVG or a DXF, with its curves already cut into straight pieces) can be written again as a plain SVG or a plain DXF, so that another program (a CAD
//! program, a cutter, a viewer) can open what this one understood. The SVG has one group for each layer (named as Inkscape names its layers), a `polyline` for an open shape and a
//! `polygon` for a closed one, in millimetres with the y axis down and the lower left corner of the drawing at the lower left of the page. The DXF is the old ASCII form of release 12
//! (`POLYLINE`, `VERTEX` and `SEQEND`, which every reader of DXF knows), in millimetres with the y axis up, with a table of layers; a layer name is written with letters, figures, `_`, `-`
//! and `$`, any other character becoming `_`, because that is what the old form allows. The colour of a shape is kept: as the stroke of the SVG, and in the DXF as the AutoCAD index when it is one of the nine
//! basic colours and as a true colour (group 420) always.

use crate::vector::{bounds_of, Drawing, Pt, Shape, Source};

/// The numbers are written with at most this many decimals (a ten-thousandth of a millimetre).
const DECIMALS: usize = 4;

/// The kind of file a drawing is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Svg,
    Dxf,
}

impl Format {
    /// The format an extension names (`svg` or `dxf`, in any case).
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.trim().to_ascii_lowercase().as_str() {
            "svg" => Some(Format::Svg),
            "dxf" => Some(Format::Dxf),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Svg => "SVG",
            Format::Dxf => "DXF",
        }
    }
}

/// The drawing as text in the format asked for. A drawing with nothing in it is an error: there is no file to write.
pub fn write(d: &Drawing, format: Format) -> Result<String, String> {
    if d.shapes.iter().all(|s| s.pts.len() < 2) {
        return Err("the drawing has no shape to write".to_string());
    }
    Ok(match format {
        Format::Svg => to_svg(d),
        Format::Dxf => to_dxf(d),
    })
}

/// A number as short text that reads back as the same number to a ten-thousandth, with no exponent and no `-0`.
fn num(v: f64) -> String {
    let text = format!("{v:.DECIMALS$}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-0" {
        "0".to_string()
    } else {
        text.to_string()
    }
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// The colour of a shape as `#rrggbb`, when it is one (black when it is not).
fn hex_of(shape: &Shape) -> String {
    match shape.color.as_deref() {
        Some(c)
            if c.len() == 7
                && c.starts_with('#')
                && c[1..].bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            c.to_ascii_lowercase()
        }
        _ => "#000000".to_string(),
    }
}

/// The name of the layer of a shape: the one it has, or `0` when it has none.
fn layer_of(shape: &Shape) -> &str {
    let l = shape.layer.trim();
    if l.is_empty() {
        "0"
    } else {
        l
    }
}

/// A point as the machine has it, with the y axis up: an SVG keeps the y axis of the page, which points down, so its points are turned over (the cutting does the same).
fn up(d: &Drawing, p: Pt) -> Pt {
    match d.source {
        Source::Svg => (p.0, -p.1),
        Source::Dxf => p,
    }
}

/// The layers in the order they first appear.
fn layers(d: &Drawing) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for s in d.shapes.iter().filter(|s| s.pts.len() >= 2) {
        let l = layer_of(s);
        if !out.contains(&l) {
            out.push(l);
        }
    }
    out
}

/// The drawing as an SVG: millimetres, the y axis down, the lower left corner of the drawing at the lower left of the page.
pub fn to_svg(d: &Drawing) -> String {
    let (lo, hi) = bounds_of(
        d.shapes
            .iter()
            .filter(|s| s.pts.len() >= 2)
            .flat_map(|s| s.pts.iter().map(|p| up(d, *p))),
    )
    .unwrap_or(((0.0, 0.0), (0.0, 0.0)));
    let (w, h) = (hi.0 - lo.0, hi.1 - lo.1);
    let at = |p: &Pt| format!("{},{}", num(p.0 - lo.0), num(hi.1 - p.1));
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n",
        w = num(w),
        h = num(h)
    ));
    for (n, layer) in layers(d).into_iter().enumerate() {
        let label = xml_escape(layer);
        out.push_str(&format!(
            "  <g id=\"layer{}\" inkscape:groupmode=\"layer\" inkscape:label=\"{label}\">\n",
            n + 1
        ));
        for s in d
            .shapes
            .iter()
            .filter(|s| s.pts.len() >= 2 && layer_of(s) == layer)
        {
            let points: Vec<String> = s.pts.iter().map(|p| at(&up(d, *p))).collect();
            out.push_str(&format!(
                "    <{tag} points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.1\"/>\n",
                points.join(" "),
                hex_of(s),
                tag = if s.closed { "polygon" } else { "polyline" }
            ));
        }
        out.push_str("  </g>\n");
    }
    out.push_str("</svg>\n");
    out
}

/// A layer name as the old form of DXF writes it.
fn dxf_layer(name: &str) -> String {
    let clean: String = name
        .chars()
        .take(31)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '$') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.is_empty() {
        "0".to_string()
    } else {
        clean
    }
}

/// The AutoCAD index of the nine basic colours.
fn aci_of(hex: &str) -> Option<u8> {
    match hex {
        "#ff0000" => Some(1),
        "#ffff00" => Some(2),
        "#00ff00" => Some(3),
        "#00ffff" => Some(4),
        "#0000ff" => Some(5),
        "#ff00ff" => Some(6),
        "#ffffff" => Some(7),
        "#808080" => Some(8),
        "#c0c0c0" => Some(9),
        _ => None,
    }
}

/// The drawing as a DXF of release 12 (ASCII): millimetres, the y axis up.
pub fn to_dxf(d: &Drawing) -> String {
    let mut names: Vec<String> = Vec::new();
    for l in layers(d) {
        let n = dxf_layer(l);
        if !names.contains(&n) {
            names.push(n);
        }
    }
    let mut out = String::new();
    let mut group = |code: i32, value: &str| {
        out.push_str(&format!("{code}\n{value}\n"));
    };
    group(0, "SECTION");
    group(2, "HEADER");
    group(9, "$ACADVER");
    group(1, "AC1009");
    group(9, "$INSUNITS");
    group(70, "4");
    group(0, "ENDSEC");
    group(0, "SECTION");
    group(2, "TABLES");
    group(0, "TABLE");
    group(2, "LTYPE");
    group(70, "1");
    group(0, "LTYPE");
    group(2, "CONTINUOUS");
    group(70, "0");
    group(3, "Solid line");
    group(72, "65");
    group(73, "0");
    group(40, "0.0");
    group(0, "ENDTAB");
    group(0, "TABLE");
    group(2, "LAYER");
    group(70, &names.len().to_string());
    for n in &names {
        group(0, "LAYER");
        group(2, n);
        group(70, "0");
        group(62, "7");
        group(6, "CONTINUOUS");
    }
    group(0, "ENDTAB");
    group(0, "ENDSEC");
    group(0, "SECTION");
    group(2, "ENTITIES");
    for s in d.shapes.iter().filter(|s| s.pts.len() >= 2) {
        let layer = dxf_layer(layer_of(s));
        let hex = hex_of(s);
        group(0, "POLYLINE");
        group(8, &layer);
        if s.color.is_some() {
            if let Some(i) = aci_of(&hex) {
                group(62, &i.to_string());
            }
            let rgb = u32::from_str_radix(&hex[1..], 16).unwrap_or(0);
            group(420, &rgb.to_string());
        }
        group(66, "1");
        group(10, "0.0");
        group(20, "0.0");
        group(30, "0.0");
        group(70, if s.closed { "1" } else { "0" });
        for p in &s.pts {
            let p = up(d, *p);
            group(0, "VERTEX");
            group(8, &layer);
            group(10, &num(p.0));
            group(20, &num(p.1));
            group(30, "0.0");
        }
        group(0, "SEQEND");
        group(8, &layer);
    }
    group(0, "ENDSEC");
    group(0, "EOF");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxf::read_dxf;
    use crate::svg::read_svg;
    use crate::vector::{ReadOptions, Source};

    fn shape(layer: &str, color: Option<&str>, pts: &[Pt], closed: bool) -> Shape {
        Shape {
            layer: layer.to_string(),
            color: color.map(str::to_string),
            pts: pts.to_vec(),
            closed,
        }
    }

    fn drawing(shapes: Vec<Shape>) -> Drawing {
        Drawing {
            source: Source::Dxf,
            shapes,
            warnings: Vec::new(),
            page_height_mm: None,
        }
    }

    fn sample() -> Drawing {
        drawing(vec![
            shape(
                "Cut",
                Some("#ff0000"),
                &[(10.0, 5.0), (30.0, 5.0), (30.0, 25.0), (10.0, 25.0)],
                true,
            ),
            shape("Cut", None, &[(0.0, 0.0), (1.0, 1.0)], false),
            shape(
                "Mark",
                Some("#12ab34"),
                &[(12.5, 7.25), (20.0, 20.0)],
                false,
            ),
        ])
    }

    fn close_pts(a: &[Pt], b: &[Pt], eps: f64) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(p, q)| (p.0 - q.0).abs() < eps && (p.1 - q.1).abs() < eps)
    }

    #[test]
    fn numbers_are_short_and_have_no_exponent_or_negative_zero() {
        assert_eq!(num(0.0), "0");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(-0.00001), "0");
        assert_eq!(num(12.5), "12.5");
        assert_eq!(num(3.0), "3");
        assert_eq!(num(0.12345), "0.1235");
        assert_eq!(num(1e-7), "0");
        assert_eq!(num(-2.5), "-2.5");
        assert_eq!(num(100000.0), "100000");
    }

    #[test]
    fn the_formats_are_chosen_by_the_extension() {
        assert_eq!(Format::from_extension("SVG"), Some(Format::Svg));
        assert_eq!(Format::from_extension(" dxf "), Some(Format::Dxf));
        assert_eq!(Format::from_extension("nc"), None);
        assert_eq!(Format::from_extension(""), None);
        assert_eq!((Format::Svg.name(), Format::Dxf.name()), ("SVG", "DXF"));
    }

    #[test]
    fn a_drawing_with_nothing_in_it_is_not_written() {
        assert!(write(&drawing(Vec::new()), Format::Svg).is_err());
        assert!(write(
            &drawing(vec![shape("a", None, &[(0.0, 0.0)], false)]),
            Format::Dxf
        )
        .is_err());
        assert!(write(&sample(), Format::Svg).is_ok());
    }

    #[test]
    fn the_svg_has_a_group_for_each_layer_and_a_polygon_for_each_closed_shape() {
        let svg = to_svg(&sample());
        assert!(svg.starts_with("<?xml"), "{svg}");
        assert!(svg.contains("inkscape:label=\"Cut\"") && svg.contains("inkscape:label=\"Mark\""));
        assert_eq!(svg.matches("<g ").count(), 2);
        assert_eq!(svg.matches("<polygon").count(), 1);
        assert_eq!(svg.matches("<polyline").count(), 2);
        // the box of the drawing is 30 by 25 mm and the page is that size
        assert!(
            svg.contains("width=\"30mm\" height=\"25mm\" viewBox=\"0 0 30 25\""),
            "{svg}"
        );
        // the y axis points down: the point 10, 25 (the top left of the square) is at 10, 0
        assert!(svg.contains("points=\"10,20 30,20 30,0 10,0\""), "{svg}");
        assert!(svg.contains("stroke=\"#ff0000\"") && svg.contains("stroke=\"#12ab34\""));
        // a shape with no colour is black
        assert!(svg.contains("stroke=\"#000000\""));
    }

    #[test]
    fn the_names_and_the_text_of_an_svg_are_escaped() {
        let d = drawing(vec![shape(
            "a<b>&\"c'\u{7}",
            Some("not a colour"),
            &[(0.0, 0.0), (1.0, 0.0)],
            false,
        )]);
        let svg = to_svg(&d);
        assert!(
            svg.contains("inkscape:label=\"a&lt;b&gt;&amp;&quot;c&apos; \""),
            "{svg}"
        );
        assert!(svg.contains("stroke=\"#000000\""));
        // it is a file this program reads: the layer comes back as it was written
        let back = read_svg(svg.as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(back.shapes.len(), 1);
    }

    #[test]
    fn an_svg_written_by_the_program_reads_back_as_the_same_shapes_in_the_same_layers() {
        let d = sample();
        let svg = to_svg(&d);
        let back = read_svg(svg.as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(back.shapes.len(), d.shapes.len(), "{:?}", back.warnings);
        // the reader keeps the y axis of the page, which points down; the box of the drawing starts at the origin
        let (lo, hi) = d.bounds().unwrap();
        for (a, b) in d.shapes.iter().zip(&back.shapes) {
            let moved: Vec<Pt> = a.pts.iter().map(|p| (p.0 - lo.0, hi.1 - p.1)).collect();
            assert!(close_pts(&moved, &b.pts, 1e-3), "{:?} {:?}", moved, b.pts);
            assert_eq!(a.closed, b.closed);
            assert_eq!(a.layer, b.layer);
        }
    }

    #[test]
    fn the_dxf_has_the_old_form_with_a_table_of_layers_and_ends_where_it_should() {
        let dxf = to_dxf(&sample());
        assert!(dxf.starts_with("0\nSECTION\n2\nHEADER\n"));
        assert!(dxf.contains("9\n$ACADVER\n1\nAC1009\n") && dxf.contains("9\n$INSUNITS\n70\n4\n"));
        assert!(dxf.contains("2\nLAYER\n70\n2\n"), "{dxf}");
        assert!(dxf.ends_with("0\nENDSEC\n0\nEOF\n"));
        assert_eq!(dxf.matches("0\nPOLYLINE\n").count(), 3);
        assert_eq!(dxf.matches("0\nSEQEND\n").count(), 3);
        assert_eq!(dxf.matches("0\nVERTEX\n").count(), 8);
        // a closed shape has the flag 1, a colour of the basic nine its index and the others a true colour only
        assert!(dxf.contains("62\n1\n420\n16711680\n"));
        assert!(dxf.contains("420\n1223476\n") && !dxf.contains("62\n5\n"));
        // every line is a group: an even number of lines
        assert_eq!(dxf.lines().count() % 2, 0);
    }

    #[test]
    fn a_dxf_written_by_the_program_reads_back_as_the_same_shapes_layers_and_colours() {
        let d = sample();
        let back = read_dxf(to_dxf(&d).as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(back.shapes.len(), d.shapes.len(), "{:?}", back.warnings);
        for (a, b) in d.shapes.iter().zip(&back.shapes) {
            assert!(close_pts(&a.pts, &b.pts, 1e-3), "{:?} {:?}", a.pts, b.pts);
            assert_eq!(a.closed, b.closed);
            assert_eq!(a.layer, b.layer);
            assert_eq!(a.color, b.color);
        }
    }

    #[test]
    fn a_layer_name_the_old_form_does_not_allow_is_written_with_underscores() {
        assert_eq!(dxf_layer("Cut"), "Cut");
        assert_eq!(dxf_layer("a b/c:d"), "a_b_c_d");
        assert_eq!(dxf_layer("Ünï"), "_n_");
        assert_eq!(dxf_layer(""), "0");
        assert_eq!(dxf_layer(&"x".repeat(100)).len(), 31);
        let d = drawing(vec![shape("a b", None, &[(0.0, 0.0), (1.0, 0.0)], false)]);
        let dxf = to_dxf(&d);
        assert!(dxf.contains("2\na_b\n") && dxf.contains("8\na_b\n"));
        // two names that become the same name are one layer in the table
        let two = drawing(vec![
            shape("a b", None, &[(0.0, 0.0), (1.0, 0.0)], false),
            shape("a/b", None, &[(0.0, 1.0), (1.0, 1.0)], false),
        ]);
        assert!(to_dxf(&two).contains("2\nLAYER\n70\n1\n"));
    }

    #[test]
    fn an_svg_keeps_its_shape_when_it_is_written_as_dxf_because_its_y_axis_is_turned_over() {
        // the page has the y axis down: the rectangle is 10 wide and 4 high, and its top edge is at y = 2
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40mm\" height=\"30mm\" viewBox=\"0 0 40 30\"><rect x=\"5\" y=\"2\" width=\"10\" height=\"4\" fill=\"none\" stroke=\"#ff0000\"/><path d=\"M5,20 L25,20\" fill=\"none\" stroke=\"#0000ff\"/></svg>";
        let d = read_svg(svg.as_bytes(), &ReadOptions::default()).unwrap();
        let dxf = to_dxf(&d);
        let back = read_dxf(dxf.as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(back.shapes.len(), 2);
        // in the machine the line, which was lower on the page, is lower than the rectangle: the line is at y = -20 and the rectangle spans -6 to -2
        let line = back.shapes.iter().find(|s| s.pts.len() == 2).unwrap();
        assert!(
            close_pts(&line.pts, &[(5.0, -20.0), (25.0, -20.0)], 1e-3),
            "{:?}",
            line.pts
        );
        let rect = back.shapes.iter().find(|s| s.closed).unwrap();
        let (lo, hi) = bounds_of(rect.pts.iter().copied()).unwrap();
        assert!(
            (lo.1 + 6.0).abs() < 1e-3 && (hi.1 + 2.0).abs() < 1e-3,
            "{lo:?} {hi:?}"
        );
        // and an SVG written from an SVG is the same page, moved to the origin
        let again = read_svg(to_svg(&d).as_bytes(), &ReadOptions::default()).unwrap();
        let (a, b) = (d.bounds().unwrap(), again.bounds().unwrap());
        assert!(((a.1 .1 - a.0 .1) - (b.1 .1 - b.0 .1)).abs() < 1e-3);
        assert!(close_pts(&[b.0], &[(0.0, 0.0)], 1e-3), "{b:?}");
    }

    #[test]
    fn a_drawing_goes_from_dxf_to_svg_and_back_without_losing_a_shape() {
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\nHole\n10\n5\n20\n5\n40\n2\n0\nLINE\n8\nCut\n10\n0\n20\n0\n11\n20\n21\n0\n0\nENDSEC\n0\nEOF\n";
        let d = read_dxf(dxf.as_bytes(), &ReadOptions::default()).unwrap();
        let svg = write(&d, Format::Svg).unwrap();
        let again = read_svg(svg.as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(again.shapes.len(), 2);
        let (lo, hi) = (d.bounds().unwrap(), again.bounds().unwrap());
        let size = |b: (Pt, Pt)| (b.1 .0 - b.0 .0, b.1 .1 - b.0 .1);
        assert!((size(lo).0 - size(hi).0).abs() < 1e-3 && (size(lo).1 - size(hi).1).abs() < 1e-3);
        let dxf2 = write(&again, Format::Dxf).unwrap();
        let third = read_dxf(dxf2.as_bytes(), &ReadOptions::default()).unwrap();
        assert_eq!(third.shapes.len(), 2);
    }
}
