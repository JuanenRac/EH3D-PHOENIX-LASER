//! The reader of LBRN2 project files (`.lbrn2`, the project format of a laser program of another maker; its names are trademarks of their owners): the shapes of a project become a drawing in millimetres, and the settings of its
//! layers (`CutSetting`) are kept as hints for the table of layers.
//!
//! What is read comes from the behaviour of real project files, not from a published specification: the file is XML with a `LightBurnProject` root; the layers are `CutSetting`
//! elements (a `type` of `Cut`, `Scan` or `Image`, and one child per value with a `Value` attribute); the shapes are `Shape` elements with a `Type` and an `XForm` of six numbers
//! (`a b c d e f`, the matrix of the plane: `x' = a x + c y + e`, `y' = b x + d y + f`) that places the shape in the space of its parent. A `Path` has a `VertList` (vertices written
//! `V x y`, each followed by the optional handles `c0x c0y c1x c1y`; a handle with only its x and the value 1 stands for no handle) and a `PrimList` of `L a b` (a line) and `B a b` (a cubic
//! Bezier from vertex `a` to vertex `b` that leaves `a` through its `c0` and arrives at `b` through its `c1`); the word `LineClosed` is a closed polygon through all the vertices. A path
//! may name the lists it uses (`VertID`, `PrimID`) and leave them out when an earlier path wrote them. A `Rect` is centred on its origin, a `Text` carries its letters as a `BackupPath`,
//! a `Group` has `Children`.
//!
//! What is not read is said in the notes of the drawing and counted in [`Stats`]: pictures (`Bitmap`), shapes of a type that was never seen in the files this was measured on, and text
//! that has no outline. The y axis of the file is taken as the one of an SVG (it points down), so the drawing is turned over like an SVG when it is cut; that was not confirmed against a
//! machine.

use crate::flatten;
use crate::vconfig::{LayerMode, LayerParams};
use crate::vector::{Collector, Drawing, Pt, ReadOptions, Source};
use std::collections::HashMap;
use std::rc::Rc;

/// The most vertices one list may hold (a larger one is a mistake of the file).
const MAX_LIST_ITEMS: usize = 5_000_000;
/// The most attributes one element may have.
const MAX_ATTRS: usize = 512;
/// The most layers a project may have.
pub const MAX_CUT_SETTINGS: usize = 256;
/// An attribute longer than this is a picture or a preview, which this reader does not use: it is not kept.
const MAX_KEPT_ATTR: usize = 4096;

type Mat = [f64; 6];
const IDENTITY: Mat = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// The matrix of a child placed in the space of its parent.
fn mul(p: &Mat, q: &Mat) -> Mat {
    [
        p[0] * q[0] + p[2] * q[1],
        p[1] * q[0] + p[3] * q[1],
        p[0] * q[2] + p[2] * q[3],
        p[1] * q[2] + p[3] * q[3],
        p[0] * q[4] + p[2] * q[5] + p[4],
        p[1] * q[4] + p[3] * q[5] + p[5],
    ]
}

fn apply(m: &Mat, p: Pt) -> Pt {
    (
        m[0] * p.0 + m[2] * p.1 + m[4],
        m[1] * p.0 + m[3] * p.1 + m[5],
    )
}

// ---------------------------------------------------------------- the text of the file as elements

#[derive(Debug, Default)]
struct Elem {
    name: String,
    attrs: Vec<(String, String)>,
    /// The text of the elements that hold numbers (`XForm`, `VertList`, `PrimList`); empty for the others.
    text: String,
    kids: Vec<Elem>,
}

impl Elem {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn kid(&self, name: &str) -> Option<&Elem> {
        self.kids.iter().find(|k| k.name == name)
    }

    fn number(&self, name: &str) -> Option<f64> {
        self.attr(name)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
    }
}

fn keeps_text(name: &str) -> bool {
    matches!(name, "XForm" | "VertList" | "PrimList")
}

/// Reads the text of the file into elements. The depth and the number of elements are held to the limits, no entity is expanded and an element is never read from a name that is not closed.
fn parse_elements(text: &str, max_depth: usize, max_nodes: usize) -> Result<Elem, String> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut stack: Vec<Elem> = Vec::new();
    let mut root: Option<Elem> = None;
    let mut nodes = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            let end = text[i..].find('<').map_or(bytes.len(), |o| i + o);
            if let Some(top) = stack.last_mut() {
                if keeps_text(&top.name) {
                    top.text.push_str(&text[i..end]);
                }
            }
            i = end;
            continue;
        }
        let rest = &text[i..];
        if let Some(inside) = rest.strip_prefix("<!--") {
            let end = inside.find("-->").ok_or("a comment is not closed")?;
            i += 4 + end + 3;
        } else if rest.starts_with("<![CDATA[") {
            let end = rest.find("]]>").ok_or("a CDATA section is not closed")?;
            if let Some(top) = stack.last_mut() {
                if keeps_text(&top.name) {
                    top.text.push_str(&rest[9..end]);
                }
            }
            i += end + 3;
        } else if rest.starts_with("<?") {
            let end = rest
                .find("?>")
                .ok_or("a processing instruction is not closed")?;
            i += end + 2;
        } else if rest.starts_with("<!") {
            let end = rest.find('>').ok_or("a declaration is not closed")?;
            if rest[..end].to_ascii_uppercase().contains("<!ENTITY") {
                return Err(
                    "the file declares entities, which this reader does not expand".to_string(),
                );
            }
            i += end + 1;
        } else if let Some(body) = rest.strip_prefix("</") {
            let end = body.find('>').ok_or("an end tag is not closed")?;
            let name = body[..end].trim();
            let done = stack.pop().ok_or("an end tag has no start tag")?;
            if done.name != name {
                return Err(format!("the element {} is closed as {name}", done.name));
            }
            i += 2 + end + 1;
            match stack.last_mut() {
                Some(parent) => parent.kids.push(done),
                None => {
                    root = Some(done);
                    break;
                }
            }
        } else {
            let (elem, empty, used) = start_tag(rest)?;
            i += used;
            nodes += 1;
            if nodes > max_nodes {
                return Err(format!(
                    "the file has more than {max_nodes} elements (max_nodes)"
                ));
            }
            if empty {
                match stack.last_mut() {
                    Some(parent) => parent.kids.push(elem),
                    None => {
                        root = Some(elem);
                        break;
                    }
                }
            } else {
                if stack.len() >= max_depth {
                    return Err(format!(
                        "the elements are nested more than {max_depth} deep (max_depth)"
                    ));
                }
                stack.push(elem);
            }
        }
    }
    if !stack.is_empty() {
        return Err(format!("the element {} is not closed", stack[0].name));
    }
    root.ok_or_else(|| "this is not a project file: there is no element".to_string())
}

/// Reads a start tag at the start of `rest`: the element, whether it is empty (`<a/>`) and the bytes it takes.
fn start_tag(rest: &str) -> Result<(Elem, bool, usize), String> {
    let bytes = rest.as_bytes();
    let mut j = 1;
    while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b'/' && bytes[j] != b'>'
    {
        j += 1;
    }
    let name = rest[1..j].to_string();
    if name.is_empty() {
        return Err("a tag has no name".to_string());
    }
    let mut elem = Elem {
        name,
        ..Elem::default()
    };
    loop {
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        match bytes.get(j) {
            None => return Err("a tag is not closed".to_string()),
            Some(b'>') => return Ok((elem, false, j + 1)),
            Some(b'/') => {
                return if bytes.get(j + 1) == Some(&b'>') {
                    Ok((elem, true, j + 2))
                } else {
                    Err("a tag has a / in the wrong place".to_string())
                };
            }
            _ => {}
        }
        let key_start = j;
        while j < bytes.len()
            && !bytes[j].is_ascii_whitespace()
            && !matches!(bytes[j], b'=' | b'/' | b'>')
        {
            j += 1;
        }
        let key = rest[key_start..j].to_string();
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if bytes.get(j) != Some(&b'=') {
            return Err("an attribute has no value".to_string());
        }
        j += 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        let quote = match bytes.get(j) {
            Some(&q) if q == b'"' || q == b'\'' => q,
            _ => return Err("an attribute value is not quoted".to_string()),
        };
        j += 1;
        let close = rest[j..]
            .find(quote as char)
            .ok_or("an attribute value is not closed")?;
        if elem.attrs.len() >= MAX_ATTRS {
            return Err("a tag has too many attributes".to_string());
        }
        let raw = &rest[j..j + close];
        // a picture or a preview is far too long to be of use here; its length is not an error
        let value = if raw.len() > MAX_KEPT_ATTR {
            String::new()
        } else {
            crate::svg::decode(raw)
        };
        elem.attrs.push((key, value));
        j += close + 1;
    }
}

// ---------------------------------------------------------------- the layers

/// What a layer does in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CutKind {
    /// The line of the shape is followed (`Cut`).
    Cut,
    /// The inside of the shape is filled with lines (`Scan`).
    Scan,
    /// A picture is engraved (`Image`).
    Image,
    /// A type this reader does not know, as written.
    Other(String),
}

impl CutKind {
    fn from_word(word: &str) -> CutKind {
        match word {
            "Cut" => CutKind::Cut,
            "Scan" => CutKind::Scan,
            "Image" => CutKind::Image,
            other => CutKind::Other(other.to_string()),
        }
    }
}

/// One layer of the project (`CutSetting`). Every value the file gives is kept under its name; the typed ones are read from them.
#[derive(Debug, Clone, PartialEq)]
pub struct CutSetting {
    /// The type of the layer.
    pub kind: CutKind,
    /// The number the shapes name it by (`CutIndex`).
    pub index: u32,
    /// The name, empty when the file gave none.
    pub name: String,
    /// Every value as the file wrote it, in order.
    pub values: Vec<(String, String)>,
}

impl CutSetting {
    fn value(&self, key: &str) -> Option<f64> {
        self.values
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
    }

    /// The speed the file gives, in mm/s.
    pub fn speed_mm_s(&self) -> Option<f64> {
        self.value("speed")
    }

    /// The highest power of the layer, in percent.
    pub fn max_power(&self) -> Option<f64> {
        self.value("maxPower")
    }

    /// The lowest power of the layer, in percent (a cut with a power that follows the speed).
    pub fn min_power(&self) -> Option<f64> {
        self.value("minPower")
    }

    /// The distance between the lines of a fill, in mm.
    pub fn interval_mm(&self) -> Option<f64> {
        self.value("interval")
    }

    /// The number of passes (1 when the file does not say).
    pub fn passes(&self) -> u32 {
        self.value("numPasses")
            .filter(|v| (1.0..=1000.0).contains(v))
            .map_or(1, |v| v as u32)
    }

    /// The order the layers are done in (the lowest first), as the file gives it.
    pub fn priority(&self) -> Option<i32> {
        self.value("priority")
            .filter(|v| (-1000.0..=1000.0).contains(v))
            .map(|v| v as i32)
    }

    /// Whether the layer is sent to the laser (`doOutput`, on when the file does not say).
    pub fn output(&self) -> bool {
        self.value("doOutput").is_none_or(|v| v != 0.0)
    }

    /// The name of the layer in the table: its name, or `C` and its number with two digits.
    pub fn label(&self) -> String {
        if self.name.trim().is_empty() {
            format!("C{:02}", self.index)
        } else {
            self.name.trim().to_string()
        }
    }

    /// What the table of layers is filled with. A layer that is not sent to the laser is skipped. The speed of the file is in mm/s and becomes mm/min. Nothing is checked here: the limits
    /// of the machine are applied when the work is made, like for any value the person types.
    pub fn hint(&self) -> LayerParams {
        let d = LayerParams::default();
        let mode = if !self.output() {
            LayerMode::Skip
        } else if self.kind == CutKind::Cut {
            LayerMode::Cut
        } else {
            LayerMode::Engrave
        };
        LayerParams {
            mode,
            power_percent: self.max_power().unwrap_or(d.power_percent),
            speed_mm_min: self.speed_mm_s().map_or(d.speed_mm_min, |v| v * 60.0),
            passes: self.passes(),
            hatch_spacing_mm: self.interval_mm().unwrap_or(d.hatch_spacing_mm),
            order: self.priority().unwrap_or(d.order),
            ..d
        }
    }
}

// ---------------------------------------------------------------- the report of a reading

/// What was found in a project, for the person and for the tests.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    /// The elements of the file.
    pub elements: usize,
    /// The shapes by their `Type`, in the order they first appear.
    pub shape_types: Vec<(String, usize)>,
    /// The shapes whose outline became part of the drawing.
    pub read_shapes: usize,
    /// The vertices of the lists that were used.
    pub vertices: usize,
    /// The straight pieces and the Bezier pieces that were followed.
    pub lines: usize,
    /// The Bezier pieces.
    pub beziers: usize,
    /// The paths that took their lists from an earlier path.
    pub shared_lists: usize,
    /// The shapes that were left out, by the reason.
    pub skipped: Vec<(String, usize)>,
    /// The elements that sit inside a `Shape` and that this reader does not know, by name.
    pub unknown_parts: Vec<(String, usize)>,
}

fn bump(list: &mut Vec<(String, usize)>, key: &str) {
    match list.iter_mut().find(|(k, _)| k == key) {
        Some((_, n)) => *n += 1,
        None => list.push((key.to_string(), 1)),
    }
}

/// A project as read.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// The version of the program that wrote it (`AppVersion`).
    pub app_version: String,
    /// The device the project was made for, when it says so.
    pub device_name: String,
    /// The version of the form (`FormatVersion`).
    pub format_version: String,
    /// `MirrorX` and `MirrorY` of the root, as written.
    pub mirror: (bool, bool),
    /// The layers.
    pub cut_settings: Vec<CutSetting>,
    /// What was found.
    pub stats: Stats,
    /// The drawing: every outline that could be followed, in millimetres, with the layer of each.
    pub drawing: Drawing,
}

impl Project {
    /// The layers of the project as hints for the table of layers, by the label of the layer in lower case (the key the drawing uses).
    pub fn hints(&self) -> Vec<(String, LayerParams)> {
        self.cut_settings
            .iter()
            .map(|c| (c.label().to_lowercase(), c.hint()))
            .collect()
    }
}

// ---------------------------------------------------------------- the lists of a path

#[derive(Debug, Clone, Copy)]
struct Vert {
    p: Pt,
    c0: Option<Pt>,
    c1: Option<Pt>,
}

fn read_number(s: &str, at: &mut usize) -> Option<f64> {
    let b = s.as_bytes();
    let start = *at;
    let mut j = start;
    while j < b.len() && (b[j].is_ascii_digit() || matches!(b[j], b'-' | b'+' | b'.')) {
        j += 1;
    }
    // an exponent: `e` or `E` with an optional sign and digits (a following handle name starts with `c`, never with `e`)
    if j < b.len() && matches!(b[j], b'e' | b'E') {
        let mut k = j + 1;
        if k < b.len() && matches!(b[k], b'-' | b'+') {
            k += 1;
        }
        if k < b.len() && b[k].is_ascii_digit() {
            while k < b.len() && b[k].is_ascii_digit() {
                k += 1;
            }
            j = k;
        }
    }
    if j == start {
        return None;
    }
    *at = j;
    s[start..j].parse::<f64>().ok().filter(|v| v.is_finite())
}

fn skip_blanks(s: &str, at: &mut usize) {
    while s.as_bytes().get(*at).is_some_and(u8::is_ascii_whitespace) {
        *at += 1;
    }
}

/// Reads `V x y` followed by the handles.
fn parse_verts(text: &str) -> Result<Vec<Vert>, String> {
    let mut out: Vec<Vert> = Vec::new();
    let mut at = 0usize;
    skip_blanks(text, &mut at);
    while at < text.len() {
        if text.as_bytes()[at] != b'V' {
            return Err(format!(
                "a list of vertices has '{}' where V was expected",
                &text[at..text.len().min(at + 8)]
            ));
        }
        at += 1;
        let x = read_number(text, &mut at).ok_or("a vertex has no x")?;
        skip_blanks(text, &mut at);
        let y = read_number(text, &mut at).ok_or("a vertex has no y")?;
        let mut h: [Option<f64>; 4] = [None; 4];
        loop {
            let b = text.as_bytes();
            if b.get(at) != Some(&b'c') {
                break;
            }
            let (Some(&k), Some(&axis)) = (b.get(at + 1), b.get(at + 2)) else {
                return Err("a handle of a vertex is cut short".to_string());
            };
            if !matches!(k, b'0' | b'1') || !matches!(axis, b'x' | b'y') {
                return Err("a vertex has a handle this reader does not know".to_string());
            }
            at += 3;
            let v = read_number(text, &mut at).ok_or("a handle has no number")?;
            let slot = usize::from(k - b'0') * 2 + usize::from(axis == b'y');
            h[slot] = Some(v);
        }
        skip_blanks(text, &mut at);
        // a handle is there when both its numbers are
        let pair = |a: Option<f64>, b: Option<f64>| a.zip(b);
        out.push(Vert {
            p: (x, y),
            c0: pair(h[0], h[1]),
            c1: pair(h[2], h[3]),
        });
        if out.len() > MAX_LIST_ITEMS {
            return Err("a list of vertices is too long".to_string());
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prim {
    Line(usize, usize),
    Bezier(usize, usize),
}

/// What the list of primitives says: pieces, or the shorthand for a closed polygon through every vertex.
#[derive(Debug, Clone)]
enum Prims {
    Pieces(Vec<Prim>),
    LineClosed,
}

fn parse_prims(text: &str) -> Result<Prims, String> {
    let t = text.trim();
    if t == "LineClosed" {
        return Ok(Prims::LineClosed);
    }
    let mut out: Vec<Prim> = Vec::new();
    let mut at = 0usize;
    while at < t.len() {
        let kind = t.as_bytes()[at];
        if !matches!(kind, b'L' | b'B') {
            return Err(format!(
                "a list of pieces has '{}' where L or B was expected",
                &t[at..t.len().min(at + 8)]
            ));
        }
        at += 1;
        let a = read_index(t, &mut at).ok_or("a piece has no first vertex")?;
        skip_blanks(t, &mut at);
        let b = read_index(t, &mut at).ok_or("a piece has no second vertex")?;
        skip_blanks(t, &mut at);
        out.push(if kind == b'L' {
            Prim::Line(a, b)
        } else {
            Prim::Bezier(a, b)
        });
        if out.len() > MAX_LIST_ITEMS {
            return Err("a list of pieces is too long".to_string());
        }
    }
    Ok(Prims::Pieces(out))
}

fn read_index(s: &str, at: &mut usize) -> Option<usize> {
    let b = s.as_bytes();
    let start = *at;
    while b.get(*at).is_some_and(u8::is_ascii_digit) {
        *at += 1;
    }
    if *at == start || *at - start > 9 {
        return None;
    }
    s[start..*at].parse().ok()
}

fn parse_matrix(text: &str) -> Option<Mat> {
    let v: Vec<f64> = text
        .split_whitespace()
        .map(|t| t.parse::<f64>().ok().filter(|x| x.is_finite()))
        .collect::<Option<Vec<f64>>>()?;
    <[f64; 6]>::try_from(v).ok()
}

// ---------------------------------------------------------------- the shapes

struct Reader<'a> {
    col: Collector,
    cuts: &'a [CutSetting],
    stats: Stats,
    verts: HashMap<String, Rc<Vec<Vert>>>,
    prims: HashMap<String, Rc<Prims>>,
    tol: f64,
    max_curve: usize,
    max_vertices: usize,
}

impl Reader<'_> {
    fn layer_of(&self, cut_index: Option<u32>) -> String {
        match cut_index {
            Some(i) => self
                .cuts
                .iter()
                .find(|c| c.index == i)
                .map_or_else(|| format!("C{i:02}"), CutSetting::label),
            None => "C00".to_string(),
        }
    }

    /// The matrix of an element: none when it has no `XForm`; a shape whose transform is not six numbers has no place, so it is left out (`None`) and never drawn where it is not.
    fn matrix_of(&mut self, e: &Elem, id: &str) -> Option<Mat> {
        let Some(x) = e.kid("XForm") else {
            return Some(IDENTITY);
        };
        let m = parse_matrix(&x.text);
        if m.is_none() {
            self.col.warn(format!(
                "a {id} has a transform that is not six numbers and was skipped"
            ));
            bump(
                &mut self.stats.skipped,
                "shape with a transform that is not six numbers",
            );
        }
        m
    }

    fn walk(&mut self, list: &[Elem], parent: &Mat) -> Result<(), String> {
        // the children of the project that are not shapes (layers, preview, notes) are not drawn
        for e in list.iter().filter(|e| e.name == "Shape") {
            self.shape(e, parent)?;
        }
        Ok(())
    }

    fn shape(&mut self, e: &Elem, parent: &Mat) -> Result<(), String> {
        let kind = e.attr("Type").unwrap_or("").to_string();
        bump(&mut self.stats.shape_types, &kind);
        let cut = e.number("CutIndex").filter(|v| *v >= 0.0).map(|v| v as u32);
        let layer = self.layer_of(cut);
        for k in &e.kids {
            if !matches!(
                k.name.as_str(),
                "XForm" | "VertList" | "PrimList" | "Children" | "BackupPath"
            ) {
                bump(&mut self.stats.unknown_parts, &k.name);
            }
        }
        match kind.as_str() {
            "Group" => {
                let Some(own) = self.matrix_of(e, "group") else {
                    return Ok(());
                };
                let m = mul(parent, &own);
                if let Some(children) = e.kid("Children") {
                    self.walk(&children.kids, &m)?;
                }
            }
            "Path" => {
                let Some(own) = self.matrix_of(e, "path") else {
                    return Ok(());
                };
                self.path(e, &mul(parent, &own), &layer)?;
            }
            "Rect" => {
                let Some(own) = self.matrix_of(e, "rectangle") else {
                    return Ok(());
                };
                self.rect(e, &mul(parent, &own), &layer)?;
            }
            "Text" => match e.kid("BackupPath") {
                // the outline of the letters is in the backup path, which carries the transform of the text in its own XForm
                Some(back) => {
                    let Some(own) = self.matrix_of(back, "text") else {
                        return Ok(());
                    };
                    self.path(back, &mul(parent, &own), &layer)?;
                }
                None => bump(&mut self.stats.skipped, "text without an outline"),
            },
            "Bitmap" => bump(&mut self.stats.skipped, "picture"),
            other => {
                bump(&mut self.stats.skipped, &format!("shape of type {other}"));
            }
        }
        Ok(())
    }

    /// The lists of a path: its own, or the ones an earlier path wrote under the same number.
    fn lists(&mut self, e: &Elem) -> Option<(Rc<Vec<Vert>>, Rc<Prims>)> {
        let vid = e.attr("VertID").map(str::to_string);
        let pid = e.attr("PrimID").map(str::to_string);
        let verts = match e.kid("VertList") {
            Some(v) => match parse_verts(&v.text) {
                Ok(list) => {
                    let list = Rc::new(list);
                    if let Some(id) = &vid {
                        self.verts.insert(id.clone(), Rc::clone(&list));
                    }
                    list
                }
                Err(m) => {
                    self.col.warn(format!("a path was skipped: {m}"));
                    return None;
                }
            },
            None => {
                let found = vid.as_ref().and_then(|id| self.verts.get(id));
                match found {
                    Some(list) => {
                        self.stats.shared_lists += 1;
                        Rc::clone(list)
                    }
                    None => {
                        self.col.warn(
                            "a path names a list of vertices that no earlier path wrote, and was skipped",
                        );
                        return None;
                    }
                }
            }
        };
        let prims = match e.kid("PrimList") {
            Some(p) => match parse_prims(&p.text) {
                Ok(list) => {
                    let list = Rc::new(list);
                    if let Some(id) = &pid {
                        self.prims.insert(id.clone(), Rc::clone(&list));
                    }
                    list
                }
                Err(m) => {
                    self.col.warn(format!("a path was skipped: {m}"));
                    return None;
                }
            },
            None => {
                let found = pid.as_ref().and_then(|id| self.prims.get(id));
                match found {
                    Some(list) => {
                        self.stats.shared_lists += 1;
                        Rc::clone(list)
                    }
                    None => {
                        self.col.warn(
                            "a path names a list of pieces that no earlier path wrote, and was skipped",
                        );
                        return None;
                    }
                }
            }
        };
        Some((verts, prims))
    }

    fn path(&mut self, e: &Elem, m: &Mat, layer: &str) -> Result<(), String> {
        let Some((verts, prims)) = self.lists(e) else {
            bump(&mut self.stats.skipped, "path without readable lists");
            return Ok(());
        };
        self.stats.vertices += verts.len();
        if self.stats.vertices > self.max_vertices {
            return Err(format!(
                "the paths use more than {} vertices (max_points)",
                self.max_vertices
            ));
        }
        let world: Vec<Vert> = verts
            .iter()
            .map(|v| Vert {
                p: apply(m, v.p),
                c0: v.c0.map(|c| apply(m, c)),
                c1: v.c1.map(|c| apply(m, c)),
            })
            .collect();
        let pieces: Vec<Prim> = match &*prims {
            Prims::LineClosed => {
                let n = world.len();
                if n < 2 {
                    bump(&mut self.stats.skipped, "path with fewer than two vertices");
                    return Ok(());
                }
                (0..n).map(|i| Prim::Line(i, (i + 1) % n)).collect()
            }
            Prims::Pieces(list) => list.clone(),
        };
        if let Some(bad) = pieces.iter().find_map(|p| {
            let (a, b) = match p {
                Prim::Line(a, b) | Prim::Bezier(a, b) => (*a, *b),
            };
            (a >= world.len() || b >= world.len()).then_some((a, b))
        }) {
            self.col.warn(format!(
                "a path was skipped: a piece joins vertices {} and {} but there are {}",
                bad.0,
                bad.1,
                world.len()
            ));
            bump(
                &mut self.stats.skipped,
                "path with a piece outside its vertices",
            );
            return Ok(());
        }
        // follow the pieces: a new outline starts where a piece does not begin at the end of the one before
        let mut pts: Vec<Pt> = Vec::new();
        let mut start: Option<usize> = None;
        let mut end: Option<usize> = None;
        let mut drew = false;
        for piece in pieces {
            let (a, b, bez) = match piece {
                Prim::Line(a, b) => (a, b, false),
                Prim::Bezier(a, b) => (a, b, true),
            };
            if end != Some(a) {
                if self.flush(&mut pts, start, end, layer)? {
                    drew = true;
                }
                pts.clear();
                start = Some(a);
                pts.push(world[a].p);
            }
            if bez {
                self.stats.beziers += 1;
                let p0 = world[a].p;
                let p1 = world[a].c0.unwrap_or(p0);
                let p3 = world[b].p;
                let p2 = world[b].c1.unwrap_or(p3);
                flatten::cubic(p0, p1, p2, p3, self.tol, self.max_curve, &mut pts);
            } else {
                self.stats.lines += 1;
                pts.push(world[b].p);
            }
            end = Some(b);
        }
        if self.flush(&mut pts, start, end, layer)? {
            drew = true;
        }
        if drew {
            self.stats.read_shapes += 1;
        }
        Ok(())
    }

    /// Hands an outline to the drawing; it is closed when it ends at the vertex it began at.
    fn flush(
        &mut self,
        pts: &mut Vec<Pt>,
        start: Option<usize>,
        end: Option<usize>,
        layer: &str,
    ) -> Result<bool, String> {
        if pts.len() < 2 {
            return Ok(false);
        }
        let closed = start.is_some() && start == end;
        let before = self.col_len();
        self.col.push(layer, None, std::mem::take(pts), closed)?;
        Ok(self.col_len() > before)
    }

    fn col_len(&self) -> usize {
        self.col.count()
    }

    fn rect(&mut self, e: &Elem, m: &Mat, layer: &str) -> Result<(), String> {
        let (Some(w), Some(h)) = (e.number("W"), e.number("H")) else {
            self.col
                .warn("a rectangle without a width and a height was skipped");
            bump(&mut self.stats.skipped, "rectangle without a size");
            return Ok(());
        };
        if w <= 0.0 || h <= 0.0 {
            bump(&mut self.stats.skipped, "rectangle without a size");
            return Ok(());
        }
        let (hw, hh) = (w / 2.0, h / 2.0);
        let r = e.number("Cr").unwrap_or(0.0).clamp(0.0, hw.min(hh));
        let local: Vec<Pt> = if r <= 1e-9 {
            vec![(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
        } else {
            // a corner radius: each corner is a quarter of a circle of radius r
            let mut v: Vec<Pt> = Vec::new();
            let corners = [
                ((hw - r, -hh + r), -90.0_f64),
                ((hw - r, hh - r), 0.0),
                ((-hw + r, hh - r), 90.0),
                ((-hw + r, -hh + r), 180.0),
            ];
            let pieces =
                flatten::arc_pieces(r, std::f64::consts::FRAC_PI_2, self.tol, self.max_curve);
            for (c, from) in corners {
                for k in 0..=pieces {
                    let a = (from + 90.0 * k as f64 / pieces as f64).to_radians();
                    v.push((c.0 + r * a.cos(), c.1 + r * a.sin()));
                }
            }
            v
        };
        let pts: Vec<Pt> = local.into_iter().map(|p| apply(m, p)).collect();
        self.stats.read_shapes += 1;
        self.col.push(layer, None, pts, true)
    }
}

/// Reads the layers of the project: the `CutSetting` and `CutSetting_Img` elements of the root.
fn read_cut_settings(root: &Elem, col: &mut Collector) -> Vec<CutSetting> {
    let mut out: Vec<CutSetting> = Vec::new();
    for e in root
        .kids
        .iter()
        .filter(|k| k.name == "CutSetting" || k.name == "CutSetting_Img")
    {
        if out.len() >= MAX_CUT_SETTINGS {
            col.warn(format!(
                "more than {MAX_CUT_SETTINGS} layers: the rest were left out"
            ));
            break;
        }
        let mut values: Vec<(String, String)> = Vec::new();
        for k in &e.kids {
            if let Some(v) = k.attr("Value") {
                values.push((k.name.clone(), v.to_string()));
            }
        }
        let get = |key: &str| {
            values
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
        };
        let Some(index) = get("index").and_then(|v| v.trim().parse::<u32>().ok()) else {
            col.warn("a layer without a number was left out");
            continue;
        };
        out.push(CutSetting {
            kind: CutKind::from_word(e.attr("type").unwrap_or("")),
            index,
            name: get("name").unwrap_or_default(),
            values,
        });
    }
    out
}

fn flag(v: Option<&str>) -> bool {
    v.is_some_and(|s| s.trim().eq_ignore_ascii_case("true") || s.trim() == "1")
}

/// Reads a project file. The shapes are followed to their outlines and flattened to within the tolerance of `opts`; the limits of `opts` hold for the size of the file, the depth of the
/// elements, their number and the shapes and points of the drawing.
pub fn read_project(bytes: &[u8], opts: &ReadOptions) -> Result<Project, String> {
    opts.validate()?;
    let limits = opts.limits;
    if bytes.len() > limits.max_file_bytes {
        return Err(format!(
            "the file is larger than {} bytes (max_file_bytes)",
            limits.max_file_bytes
        ));
    }
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let root = parse_elements(text, limits.max_depth, limits.max_nodes)?;
    if root.name != "LightBurnProject" {
        return Err("this is not a project file: there is no LightBurnProject element".to_string());
    }
    let mut col = Collector::new(Source::Svg, limits);
    let cuts = read_cut_settings(&root, &mut col);
    let mut reader = Reader {
        col,
        cuts: &cuts,
        stats: Stats::default(),
        verts: HashMap::new(),
        prims: HashMap::new(),
        tol: opts.tolerance_mm,
        max_curve: limits.max_curve_segments,
        max_vertices: limits.max_points,
    };
    reader.walk(&root.kids, &IDENTITY)?;
    let mut stats = std::mem::take(&mut reader.stats);
    stats.elements = count_elements(&root);
    for (what, n) in &stats.skipped {
        reader.col.warn(format!("{n} {what} not read"));
    }
    if !cuts.is_empty() {
        reader.col.warn(
            "the power, the speed and the passes of the layers are the ones the project gives: check them for this machine",
        );
    }
    let drawing = reader.col.finish();
    Ok(Project {
        app_version: root.attr("AppVersion").unwrap_or("").to_string(),
        device_name: root.attr("DeviceName").unwrap_or("").to_string(),
        format_version: root.attr("FormatVersion").unwrap_or("").to_string(),
        mirror: (flag(root.attr("MirrorX")), flag(root.attr("MirrorY"))),
        cut_settings: cuts,
        stats,
        drawing,
    })
}

fn count_elements(root: &Elem) -> usize {
    let mut n = 0usize;
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        n += 1;
        stack.extend(e.kids.iter());
    }
    n
}

/// Reads the drawing of a project file (see [`read_project`]).
pub fn read_lbrn2(bytes: &[u8], opts: &ReadOptions) -> Result<Drawing, String> {
    read_project(bytes, opts).map(|p| p.drawing)
}

// ---------------------------------------------------------------- what a file holds and what the reader uses of it

/// The elements this reader uses.
pub const READ_ELEMENTS: [&str; 18] = [
    "LightBurnProject",
    "CutSetting",
    "CutSetting_Img",
    "index",
    "name",
    "maxPower",
    "minPower",
    "speed",
    "numPasses",
    "interval",
    "doOutput",
    "priority",
    "Shape",
    "XForm",
    "VertList",
    "PrimList",
    "Children",
    "BackupPath",
];

/// The attributes this reader uses, as `element.attribute`.
pub const READ_ATTRIBUTES: [&str; 23] = [
    "LightBurnProject.AppVersion",
    "LightBurnProject.DeviceName",
    "LightBurnProject.FormatVersion",
    "LightBurnProject.MirrorX",
    "LightBurnProject.MirrorY",
    "CutSetting.type",
    "CutSetting_Img.type",
    "index.Value",
    "name.Value",
    "maxPower.Value",
    "minPower.Value",
    "speed.Value",
    "numPasses.Value",
    "interval.Value",
    "doOutput.Value",
    "priority.Value",
    "Shape.Type",
    "Shape.CutIndex",
    "Shape.VertID",
    "Shape.PrimID",
    "Shape.W",
    "Shape.H",
    "Shape.Cr",
];

/// The elements and the attributes of a file with how many times each appears, to see what a reader leaves out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inventory {
    /// Every element name and its count, in the order they first appear.
    pub elements: Vec<(String, usize)>,
    /// Every `element.attribute` and its count, in the order they first appear.
    pub attributes: Vec<(String, usize)>,
}

impl Inventory {
    /// The elements the file has that this reader does not use.
    pub fn unread_elements(&self) -> Vec<&(String, usize)> {
        self.elements
            .iter()
            .filter(|(k, _)| !READ_ELEMENTS.contains(&k.as_str()))
            .collect()
    }

    /// The attributes the file has that this reader does not use.
    pub fn unread_attributes(&self) -> Vec<&(String, usize)> {
        self.attributes
            .iter()
            .filter(|(k, _)| !READ_ATTRIBUTES.contains(&k.as_str()))
            .collect()
    }
}

/// Counts the elements and the attributes of a project file (the file is held to the same limits as a reading).
pub fn inventory(bytes: &[u8], opts: &ReadOptions) -> Result<Inventory, String> {
    opts.validate()?;
    let limits = opts.limits;
    if bytes.len() > limits.max_file_bytes {
        return Err(format!(
            "the file is larger than {} bytes (max_file_bytes)",
            limits.max_file_bytes
        ));
    }
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let root = parse_elements(text, limits.max_depth, limits.max_nodes)?;
    let mut inv = Inventory::default();
    let mut stack = vec![&root];
    while let Some(e) = stack.pop() {
        bump(&mut inv.elements, &e.name);
        for (k, _) in &e.attrs {
            bump(&mut inv.attributes, &format!("{}.{k}", e.name));
        }
        stack.extend(e.kids.iter().rev());
    }
    Ok(inv)
}
