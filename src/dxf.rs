//! The reader of DXF drawings (the text form): `LINE`, `LWPOLYLINE` and the old `POLYLINE` with its `VERTEX` list (both with arcs given as bulges), `ARC`, `CIRCLE`, `ELLIPSE` (whole or an arc
//! of it) and `SPLINE` (evaluated from its control points and knots, or taken through its fit points), and `INSERT`, which draws a block of the `BLOCKS` section at its place, scale and
//! rotation (also as an array of columns and rows, and a block inside a block). The unit of the file (`$INSUNITS`) turns everything into millimetres; the layer and the colour of each
//! entity are kept, and an entity of a block that is on layer 0 or has no colour takes the layer and the colour of the `INSERT`. Everything else is skipped with a note. The size of the
//! file, the number of entities (what a block adds counts), how deep blocks nest, the points and the pieces of a curve are limited by [`ReadOptions`].

use crate::flatten::{bulge_arc, circle_arc, wrap_degrees};
use crate::vector::{Collector, Drawing, Pt, ReadOptions, Source};
use std::collections::HashMap;
use std::f64::consts::TAU;
use std::rc::Rc;

/// A file that defines more blocks than this is refused.
pub const MAX_BLOCKS: usize = 20_000;

/// A 2D transform `[a, b, c, d, e, f]`: `x' = a x + c y + e` and `y' = b x + d y + f`.
type Xf = [f64; 6];

const IDENTITY: Xf = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `outer` after `inner`.
fn compose(o: &Xf, i: &Xf) -> Xf {
    [
        o[0] * i[0] + o[2] * i[1],
        o[1] * i[0] + o[3] * i[1],
        o[0] * i[2] + o[2] * i[3],
        o[1] * i[2] + o[3] * i[3],
        o[0] * i[4] + o[2] * i[5] + o[4],
        o[1] * i[4] + o[3] * i[5] + o[5],
    ]
}

fn apply(m: &Xf, p: Pt) -> Pt {
    (
        m[0] * p.0 + m[2] * p.1 + m[4],
        m[1] * p.0 + m[3] * p.1 + m[5],
    )
}

/// A block of the `BLOCKS` section: its base point (in millimetres) and the groups of each of its entities, kept as text until an `INSERT` draws them.
struct Block {
    base: Pt,
    entities: Vec<Vec<(i32, String)>>,
}

/// One group of the file: the code and its value.
type Group<'a> = (i32, &'a str);

/// The millimetres of a drawing unit (`$INSUNITS`); a code that this reader does not know, or 0 (unitless), gives the default of the settings.
fn insunits_mm(code: i64, default: f64) -> f64 {
    match code {
        1 => 25.4,
        2 => 304.8,
        3 => 1_609_344.0,
        4 => 1.0,
        5 => 10.0,
        6 => 1000.0,
        9 => 0.0254,
        10 => 914.4,
        13 => 0.001,
        14 => 100.0,
        _ => default,
    }
}

/// The colour of an AutoCAD index: the nine basic ones as RGB, the others by their index.
fn aci_color(index: i64) -> Option<String> {
    let hex = match index {
        1 => "#ff0000",
        2 => "#ffff00",
        3 => "#00ff00",
        4 => "#00ffff",
        5 => "#0000ff",
        6 => "#ff00ff",
        7 => "#ffffff",
        8 => "#808080",
        9 => "#c0c0c0",
        i if (10..=255).contains(&i) => return Some(format!("aci{i}")),
        _ => return None,
    };
    Some(hex.to_string())
}

fn float(g: &[Group], code: i32) -> Option<f64> {
    g.iter()
        .find(|(c, _)| *c == code)
        .and_then(|(_, v)| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn text(g: &[Group], code: i32) -> Option<String> {
    g.iter()
        .find(|(c, _)| *c == code)
        .map(|(_, v)| v.trim().to_string())
}

fn all_floats(g: &[Group], code: i32) -> Vec<f64> {
    g.iter()
        .filter(|(c, _)| *c == code)
        .filter_map(|(_, v)| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .collect()
}

/// A polyline of the old kind while its vertices are being read.
struct OldPolyline {
    layer: String,
    color: Option<String>,
    closed: bool,
    /// A mesh: its vertices are read and left out.
    skip: bool,
    verts: Vec<(f64, f64, f64)>,
}

struct Reader<'a> {
    opts: &'a ReadOptions,
    col: Collector,
    /// The millimetres of a drawing unit.
    unit: f64,
    in_header: bool,
    in_entities: bool,
    in_blocks: bool,
    old: Option<OldPolyline>,
    nodes: usize,
    /// The blocks defined so far, by name in capitals.
    blocks: HashMap<String, Rc<Block>>,
    /// The block whose definition is being read.
    current_block: Option<(String, Block)>,
    /// What turns the geometry of a block into the place where it is drawn (the identity outside of a block).
    xf: Xf,
    /// The layer and the colour of the `INSERT` being drawn, taken by the entities of its block that are on layer 0 or have no colour.
    inherit: Option<(String, Option<String>)>,
    /// How many blocks deep the entity being read is.
    depth: usize,
}

impl Reader<'_> {
    fn tol(&self) -> f64 {
        self.opts.tolerance_mm
    }

    fn max_seg(&self) -> usize {
        self.opts.limits.max_curve_segments
    }

    fn layer_and_color(&self, g: &[Group]) -> (String, Option<String>) {
        let layer = text(g, 8)
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| "0".to_string());
        let true_color = float(g, 420)
            .filter(|v| (0.0..=16_777_215.0).contains(v))
            .map(|v| format!("#{:06x}", v as u32));
        let color = true_color.or_else(|| float(g, 62).and_then(|v| aci_color(v as i64)));
        (layer, color)
    }

    /// Whether the entity is drawn in the mirror (its extrusion points down), which changes the side of the x axis.
    fn mirrored(g: &[Group]) -> bool {
        float(g, 230).is_some_and(|z| z < 0.0)
    }

    fn handle(&mut self, g: &[Group]) -> Result<(), String> {
        let Some(&(0, kind)) = g.first() else {
            return Ok(());
        };
        let kind = kind.trim().to_ascii_uppercase();
        match kind.as_str() {
            "SECTION" => {
                let name = text(g, 2).unwrap_or_default().to_ascii_uppercase();
                self.in_header = name == "HEADER";
                self.in_entities = name == "ENTITIES";
                self.in_blocks = name == "BLOCKS";
                if self.in_header {
                    for w in g.windows(2) {
                        if w[0].0 == 9
                            && w[0].1.trim().eq_ignore_ascii_case("$INSUNITS")
                            && w[1].0 == 70
                        {
                            if let Ok(code) = w[1].1.trim().parse::<i64>() {
                                self.unit = insunits_mm(code, self.opts.dxf_unit_mm);
                            }
                        }
                    }
                }
                return Ok(());
            }
            "ENDSEC" => {
                self.finish_old()?;
                self.in_header = false;
                self.in_entities = false;
                self.in_blocks = false;
                self.current_block = None;
                return Ok(());
            }
            _ => {}
        }
        if self.in_blocks {
            let u = self.unit;
            match kind.as_str() {
                "BLOCK" => {
                    let name = text(g, 2).unwrap_or_default().to_ascii_uppercase();
                    let base = (
                        float(g, 10).unwrap_or(0.0) * u,
                        float(g, 20).unwrap_or(0.0) * u,
                    );
                    self.current_block = Some((
                        name,
                        Block {
                            base,
                            entities: Vec::new(),
                        },
                    ));
                }
                "ENDBLK" => {
                    if let Some((name, block)) = self.current_block.take() {
                        if self.blocks.len() >= MAX_BLOCKS {
                            return Err(format!("the file defines more than {MAX_BLOCKS} blocks"));
                        }
                        self.blocks.insert(name, Rc::new(block));
                    }
                }
                _ => {
                    if let Some((_, block)) = self.current_block.as_mut() {
                        block
                            .entities
                            .push(g.iter().map(|(c, v)| (*c, (*v).to_string())).collect());
                    }
                }
            }
            return Ok(());
        }
        if !self.in_entities {
            return Ok(());
        }
        self.entity(g)
    }

    /// One entity of the drawing, or of a block that an `INSERT` draws.
    fn entity(&mut self, g: &[Group]) -> Result<(), String> {
        let Some(&(0, kind)) = g.first() else {
            return Ok(());
        };
        let kind = kind.trim().to_ascii_uppercase();
        self.nodes += 1;
        if self.nodes > self.opts.limits.max_nodes {
            return Err(format!(
                "the file has more than {} entities (max_nodes)",
                self.opts.limits.max_nodes
            ));
        }
        match kind.as_str() {
            "VERTEX" => return self.old_vertex(g),
            "SEQEND" => return self.finish_old(),
            _ => self.finish_old()?,
        }
        let (layer, color) = self.layer_and_color(g);
        let (layer, color) = self.inherited(layer, color);
        let u = self.unit;
        let (tol, max) = (self.tol(), self.max_seg());
        let mirror = Self::mirrored(g);
        let flip = |x: f64| if mirror { -x } else { x };
        match kind.as_str() {
            "LINE" => {
                let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                    (float(g, 10), float(g, 20), float(g, 11), float(g, 21))
                else {
                    self.col.warn("a LINE without its points was skipped");
                    return Ok(());
                };
                self.emit(
                    &layer,
                    color.as_deref(),
                    vec![(x1 * u, y1 * u), (x2 * u, y2 * u)],
                    false,
                )?;
            }
            "LWPOLYLINE" => {
                let mut verts: Vec<(f64, f64, f64)> = Vec::new();
                for &(code, value) in g {
                    let Ok(v) = value.trim().parse::<f64>() else {
                        continue;
                    };
                    match code {
                        10 => verts.push((flip(v) * u, 0.0, 0.0)),
                        20 => {
                            if let Some(last) = verts.last_mut() {
                                last.1 = v * u;
                            }
                        }
                        42 => {
                            if let Some(last) = verts.last_mut() {
                                last.2 = if mirror { -v } else { v };
                            }
                        }
                        _ => {}
                    }
                }
                let closed = float(g, 70).is_some_and(|f| (f as i64) & 1 == 1);
                self.push_poly(&layer, color.as_deref(), &verts, closed)?;
            }
            "POLYLINE" => {
                let flags = float(g, 70).unwrap_or(0.0) as i64;
                if flags & (16 | 64) != 0 {
                    self.col.warn("a POLYLINE that is a mesh was skipped");
                    self.old = Some(OldPolyline {
                        layer,
                        color,
                        closed: false,
                        skip: true,
                        verts: Vec::new(),
                    });
                    return Ok(());
                }
                self.old = Some(OldPolyline {
                    layer,
                    color,
                    closed: flags & 1 == 1,
                    skip: false,
                    verts: Vec::new(),
                });
            }
            "ARC" | "CIRCLE" => {
                let (Some(cx), Some(cy), Some(r)) = (float(g, 10), float(g, 20), float(g, 40))
                else {
                    self.col.warn(format!(
                        "a {kind} without its centre and radius was skipped"
                    ));
                    return Ok(());
                };
                let (centre, r) = ((flip(cx) * u, cy * u), r * u);
                if r <= 0.0 {
                    self.col
                        .warn(format!("a {kind} with no radius was skipped"));
                    return Ok(());
                }
                if kind == "CIRCLE" {
                    let mut pts = vec![(centre.0 + r, centre.1)];
                    pts.extend(circle_arc(centre, r, 0.0, TAU, tol, max));
                    self.emit(&layer, color.as_deref(), pts, true)?;
                } else {
                    let (mut a0, mut a1) =
                        (float(g, 50).unwrap_or(0.0), float(g, 51).unwrap_or(360.0));
                    if mirror {
                        (a0, a1) = (180.0 - a1, 180.0 - a0);
                    }
                    let mut sweep = wrap_degrees(a1 - a0);
                    if sweep == 0.0 {
                        sweep = 360.0;
                    }
                    let start = wrap_degrees(a0).to_radians();
                    let mut pts = vec![(centre.0 + r * start.cos(), centre.1 + r * start.sin())];
                    pts.extend(circle_arc(centre, r, start, sweep.to_radians(), tol, max));
                    self.emit(&layer, color.as_deref(), pts, false)?;
                }
            }
            "SPLINE" => {
                let pts = self.spline(g);
                if pts.len() < 2 {
                    self.col.warn("a SPLINE that cannot be read was skipped");
                    return Ok(());
                }
                let closed = float(g, 70).is_some_and(|f| (f as i64) & 1 == 1);
                self.emit(&layer, color.as_deref(), pts, closed)?;
            }
            "ELLIPSE" => self.ellipse(g, &layer, color.as_deref())?,
            "INSERT" => self.insert(g, &layer, color.as_deref())?,
            other => self.col.warn(format!(
                "{other} entities are not supported and were skipped"
            )),
        }
        Ok(())
    }

    /// Puts a shape in the drawing, moved by the transform of the block that is being drawn.
    fn emit(
        &mut self,
        layer: &str,
        color: Option<&str>,
        pts: Vec<Pt>,
        closed: bool,
    ) -> Result<(), String> {
        let pts = if self.xf == IDENTITY {
            pts
        } else {
            pts.into_iter().map(|p| apply(&self.xf, p)).collect()
        };
        self.col.push(layer, color, pts, closed)
    }

    /// The layer and the colour of an entity of a block: layer 0 and no colour mean the ones of the `INSERT`.
    fn inherited(&self, layer: String, color: Option<String>) -> (String, Option<String>) {
        match &self.inherit {
            Some((l, c)) => (
                if layer == "0" { l.clone() } else { layer },
                color.or_else(|| c.clone()),
            ),
            None => (layer, color),
        }
    }

    /// An `ELLIPSE`: the centre and the end of the major axis (relative to the centre) are in the world, the ratio is minor over major and the parameters are the angles (radians) of the
    /// circle it is drawn from; a whole ellipse closes on itself.
    fn ellipse(&mut self, g: &[Group], layer: &str, color: Option<&str>) -> Result<(), String> {
        let u = self.unit;
        let (Some(cx), Some(cy), Some(mx), Some(my)) =
            (float(g, 10), float(g, 20), float(g, 11), float(g, 21))
        else {
            self.col
                .warn("an ELLIPSE without its centre and major axis was skipped");
            return Ok(());
        };
        let ratio = float(g, 40).filter(|r| *r > 0.0 && *r <= 1.0);
        let major = (mx * u).hypot(my * u);
        let Some(ratio) = ratio.filter(|_| major > 0.0) else {
            self.col
                .warn("an ELLIPSE with no usable ratio or major axis was skipped");
            return Ok(());
        };
        let (c, m) = ((cx * u, cy * u), (mx * u, my * u));
        // the minor axis is the major one turned a quarter turn about the normal of the ellipse, which points down when the extrusion does
        let minor = if Self::mirrored(g) {
            (m.1 * ratio, -m.0 * ratio)
        } else {
            (-m.1 * ratio, m.0 * ratio)
        };
        let t0 = float(g, 41).unwrap_or(0.0);
        let t1 = float(g, 42).unwrap_or(TAU);
        let mut sweep = (t1 - t0) % TAU;
        if sweep <= 1e-12 {
            sweep += TAU;
        }
        let full = (sweep - TAU).abs() < 1e-9;
        let (tol, max) = (self.tol(), self.max_seg());
        let step = if major > tol {
            2.0 * (1.0 - tol / major).acos()
        } else {
            std::f64::consts::PI
        };
        let pieces = ((sweep / step).ceil() as usize).clamp(8, max.max(8));
        let mut pts: Vec<Pt> = (0..=pieces)
            .map(|k| {
                let t = t0 + sweep * k as f64 / pieces as f64;
                (
                    c.0 + t.cos() * m.0 + t.sin() * minor.0,
                    c.1 + t.cos() * m.1 + t.sin() * minor.1,
                )
            })
            .collect();
        if full {
            pts.pop();
        }
        self.emit(layer, color, pts, full)
    }

    /// An `INSERT`: the entities of its block drawn at its place, scale and rotation, as many times as its columns and rows say.
    fn insert(&mut self, g: &[Group], layer: &str, color: Option<&str>) -> Result<(), String> {
        let u = self.unit;
        let name = text(g, 2).unwrap_or_default();
        let Some(block) = self.blocks.get(&name.to_ascii_uppercase()).cloned() else {
            self.col.warn(format!(
                "INSERT of the block {name}, which the file does not define, was skipped"
            ));
            return Ok(());
        };
        if self.depth >= self.opts.limits.max_depth {
            return Err(format!(
                "blocks are nested deeper than {} (max_depth)",
                self.opts.limits.max_depth
            ));
        }
        let (sx, sy) = (float(g, 41).unwrap_or(1.0), float(g, 42).unwrap_or(1.0));
        if sx == 0.0 || sy == 0.0 {
            self.col.warn(format!(
                "INSERT of the block {name} with a scale of 0 was skipped"
            ));
            return Ok(());
        }
        let count = |code: i32| float(g, code).map_or(1usize, |v| v.clamp(1.0, 1e9) as usize);
        let (cols, rows) = (count(70), count(71));
        if cols.saturating_mul(rows) > self.opts.limits.max_nodes {
            return Err(format!(
                "an INSERT of the block {name} repeats it more than {} times (max_nodes)",
                self.opts.limits.max_nodes
            ));
        }
        let (dx, dy) = (
            float(g, 44).unwrap_or(0.0) * u,
            float(g, 45).unwrap_or(0.0) * u,
        );
        let rotation = float(g, 50).unwrap_or(0.0).to_radians();
        let (sin, cos) = rotation.sin_cos();
        let ins = (
            float(g, 10).unwrap_or(0.0) * u,
            float(g, 20).unwrap_or(0.0) * u,
        );
        let flip: Xf = if Self::mirrored(g) {
            [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        } else {
            IDENTITY
        };
        let owned_layer = layer.to_string();
        let owned_color = color.map(str::to_string);
        let (outer, outer_inherit) = (self.xf, self.inherit.take());
        self.inherit = Some((owned_layer, owned_color));
        self.depth += 1;
        let mut result = Ok(());
        'copies: for row in 0..rows {
            for col in 0..cols {
                // the spacing of an array is measured along the turned axes of the block
                let (ox, oy) = (col as f64 * dx, row as f64 * dy);
                let (px, py) = (ins.0 + ox * cos - oy * sin, ins.1 + ox * sin + oy * cos);
                let (a, b, c, d) = (cos * sx, sin * sx, -sin * sy, cos * sy);
                let local: Xf = [
                    a,
                    b,
                    c,
                    d,
                    px - (a * block.base.0 + c * block.base.1),
                    py - (b * block.base.0 + d * block.base.1),
                ];
                self.xf = compose(&outer, &compose(&flip, &local));
                for owned in &block.entities {
                    let groups: Vec<Group> = owned.iter().map(|(c, v)| (*c, v.as_str())).collect();
                    if let Err(e) = self.entity(&groups) {
                        result = Err(e);
                        break 'copies;
                    }
                }
                if let Err(e) = self.finish_old() {
                    result = Err(e);
                    break 'copies;
                }
            }
        }
        self.depth -= 1;
        self.xf = outer;
        self.inherit = outer_inherit;
        result
    }

    fn push_poly(
        &mut self,
        layer: &str,
        color: Option<&str>,
        verts: &[(f64, f64, f64)],
        closed: bool,
    ) -> Result<(), String> {
        if verts.len() < 2 {
            return Ok(());
        }
        let (tol, max) = (self.tol(), self.max_seg());
        let mut pts: Vec<Pt> = vec![(verts[0].0, verts[0].1)];
        for w in verts.windows(2) {
            pts.extend(bulge_arc(
                (w[0].0, w[0].1),
                (w[1].0, w[1].1),
                w[0].2,
                tol,
                max,
            ));
        }
        if closed {
            let (last, first) = (verts[verts.len() - 1], verts[0]);
            pts.extend(bulge_arc(
                (last.0, last.1),
                (first.0, first.1),
                last.2,
                tol,
                max,
            ));
        }
        self.emit(layer, color, pts, closed)
    }

    fn old_vertex(&mut self, g: &[Group]) -> Result<(), String> {
        let u = self.unit;
        let Some(old) = self.old.as_mut() else {
            return Ok(());
        };
        if old.skip {
            return Ok(());
        }
        let flags = float(g, 70).unwrap_or(0.0) as i64;
        // a control point of a fitted curve is not on the curve
        if flags & 16 != 0 {
            return Ok(());
        }
        if let (Some(x), Some(y)) = (float(g, 10), float(g, 20)) {
            old.verts.push((x * u, y * u, float(g, 42).unwrap_or(0.0)));
        }
        Ok(())
    }

    fn finish_old(&mut self) -> Result<(), String> {
        if let Some(old) = self.old.take() {
            if !old.skip {
                self.push_poly(&old.layer, old.color.as_deref(), &old.verts, old.closed)?;
            }
        }
        Ok(())
    }

    /// The points of a spline: through its fit points when it has them, otherwise the curve of its control points and knots (rational when it has weights).
    fn spline(&self, g: &[Group]) -> Vec<Pt> {
        let u = self.unit;
        let pair = |cx: i32, cy: i32| -> Vec<Pt> {
            // the groups of one code come in order, and the x of a point is followed by its y
            let xs = all_floats(g, cx);
            let ys = all_floats(g, cy);
            xs.iter()
                .zip(ys.iter())
                .map(|(x, y)| (x * u, y * u))
                .collect()
        };
        let fit = pair(11, 21);
        if fit.len() >= 2 {
            return fit;
        }
        let ctrl = pair(10, 20);
        let degree = float(g, 71).map_or(3, |d| d.max(1.0) as usize);
        let knots = all_floats(g, 40);
        let n = ctrl.len();
        let weights = {
            let w = all_floats(g, 41);
            if w.len() == n && w.iter().all(|v| *v > 0.0) {
                w
            } else {
                vec![1.0; n]
            }
        };
        let sorted = knots.windows(2).all(|w| w[0] <= w[1]);
        if n <= degree || knots.len() != n + degree + 1 || !sorted || knots[n] <= knots[degree] {
            return ctrl;
        }
        let (t0, t1) = (knots[degree], knots[n]);
        let pieces = (n * 16).clamp(8, self.max_seg().max(8));
        let mut out = Vec::with_capacity(pieces + 1);
        for k in 0..=pieces {
            let t = if k == pieces {
                t1
            } else {
                t0 + (t1 - t0) * k as f64 / pieces as f64
            };
            out.push(de_boor(&ctrl, &weights, &knots, degree, t));
        }
        out
    }
}

/// The point of a spline at the parameter `t` (the algorithm of de Boor, in homogeneous coordinates so that the weights are taken into account).
fn de_boor(ctrl: &[Pt], weights: &[f64], knots: &[f64], p: usize, t: f64) -> Pt {
    let n = ctrl.len();
    // the span: the last k with knots[k] <= t, from p to n - 1
    let mut k = p;
    while k + 1 < n && knots[k + 1] <= t {
        k += 1;
    }
    let mut d: Vec<[f64; 3]> = (0..=p)
        .map(|j| {
            let i = j + k - p;
            [ctrl[i].0 * weights[i], ctrl[i].1 * weights[i], weights[i]]
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let left = knots[j + k - p];
            let right = knots[j + 1 + k - r];
            let den = right - left;
            let alpha = if den.abs() > 1e-300 {
                ((t - left) / den).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let previous = d[j - 1];
            for (value, before) in d[j].iter_mut().zip(previous) {
                *value = (1.0 - alpha) * before + alpha * *value;
            }
        }
    }
    let [x, y, w] = d[p];
    if w.abs() > 1e-300 {
        (x / w, y / w)
    } else {
        (x, y)
    }
}

/// Reads a DXF file (text form).
pub fn read_dxf(bytes: &[u8], opts: &ReadOptions) -> Result<Drawing, String> {
    opts.validate()?;
    if bytes.len() > opts.limits.max_file_bytes {
        return Err(format!(
            "the file is larger than {} bytes (max_file_bytes)",
            opts.limits.max_file_bytes
        ));
    }
    if bytes.starts_with(b"AutoCAD Binary DXF") {
        return Err("a binary DXF file is not supported: save it as a text DXF".to_string());
    }
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut rd = Reader {
        opts,
        col: Collector::new(Source::Dxf, opts.limits),
        unit: opts.dxf_unit_mm,
        in_header: false,
        in_entities: false,
        in_blocks: false,
        old: None,
        nodes: 0,
        blocks: HashMap::new(),
        current_block: None,
        xf: IDENTITY,
        inherit: None,
        depth: 0,
    };
    let mut lines = text.lines();
    let mut entity: Vec<Group> = Vec::new();
    let mut number = 0usize;
    let mut seen_group = false;
    let mut ended = false;
    while let Some(code_line) = lines.next() {
        number += 1;
        if code_line.trim().is_empty() && lines.clone().next().is_none() {
            break;
        }
        let code = code_line.trim().parse::<i32>().map_err(|_| {
            format!(
                "line {number}: a group code was expected, not '{}'",
                code_line.trim().chars().take(20).collect::<String>()
            )
        })?;
        let Some(value) = lines.next() else {
            return Err(format!(
                "line {number}: the file ends in the middle of a group"
            ));
        };
        number += 1;
        seen_group = true;
        if code == 0 {
            if !entity.is_empty() {
                rd.handle(&entity)?;
                entity.clear();
            }
            if value.trim().eq_ignore_ascii_case("EOF") {
                ended = true;
                break;
            }
        }
        entity.push((code, value));
    }
    if !ended && !entity.is_empty() {
        rd.handle(&entity)?;
    }
    rd.finish_old()?;
    if !seen_group {
        return Err("this is not a DXF file: it has no groups".to_string());
    }
    Ok(rd.col.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::dist;

    /// A DXF file with the groups of the entities between the markers of the section.
    fn entities(body: &str) -> String {
        format!("0\nSECTION\n2\nENTITIES\n{body}0\nENDSEC\n0\nEOF\n")
    }

    fn read(text: &str) -> Drawing {
        read_dxf(text.as_bytes(), &ReadOptions::default()).unwrap()
    }

    fn close(a: Pt, b: Pt) -> bool {
        dist(a, b) < 1e-9
    }

    #[test]
    fn a_line_keeps_its_points_and_its_layer() {
        let d = read(&entities(
            "0\nLINE\n8\nCut\n10\n0.0\n20\n0.0\n30\n0.0\n11\n10.0\n21\n5.0\n31\n0.0\n",
        ));
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (10.0, 5.0)]);
        assert_eq!(d.shapes[0].layer, "Cut");
        assert!(!d.shapes[0].closed);
    }

    #[test]
    fn a_closed_light_polyline_is_a_closed_shape() {
        let d = read(&entities("0\nLWPOLYLINE\n8\n0\n90\n4\n70\n1\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n"));
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(
            d.shapes[0].pts,
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        );
        assert!(d.shapes[0].closed);
    }

    #[test]
    fn a_bulge_makes_an_arc() {
        // from (0,0) to (10,0) with a bulge of 1: the half circle below the chord, of radius 5 about (5,0)
        let d = read(&entities(
            "0\nLWPOLYLINE\n8\n0\n90\n2\n70\n0\n10\n0\n20\n0\n42\n1\n10\n10\n20\n0\n",
        ));
        let pts = &d.shapes[0].pts;
        assert!(close(pts[0], (0.0, 0.0)) && close(*pts.last().unwrap(), (10.0, 0.0)));
        assert!(pts.len() > 10);
        for p in pts {
            assert!((dist(*p, (5.0, 0.0)) - 5.0).abs() < 1e-9);
            assert!(p.1 <= 1e-9);
        }
    }

    #[test]
    fn the_old_polyline_with_its_vertices_ends_at_seqend() {
        let d = read(&entities("0\nPOLYLINE\n8\nShape\n66\n1\n70\n1\n0\nVERTEX\n8\nShape\n10\n0\n20\n0\n0\nVERTEX\n8\nShape\n10\n6\n20\n0\n0\nVERTEX\n8\nShape\n10\n6\n20\n8\n0\nSEQEND\n8\nShape\n0\nLINE\n8\nOther\n10\n0\n20\n0\n11\n1\n21\n1\n"));
        assert_eq!(d.shapes.len(), 2);
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (6.0, 0.0), (6.0, 8.0)]);
        assert!(d.shapes[0].closed);
        assert_eq!(d.shapes[0].layer, "Shape");
        assert_eq!(d.shapes[1].layer, "Other");
    }

    #[test]
    fn an_arc_runs_counterclockwise_from_its_start_angle_to_its_end_angle() {
        let d = read(&entities(
            "0\nARC\n8\n0\n10\n0\n20\n0\n40\n10\n50\n0\n51\n90\n",
        ));
        let pts = &d.shapes[0].pts;
        assert!(close(pts[0], (10.0, 0.0)));
        assert!(close(*pts.last().unwrap(), (0.0, 10.0)));
        for p in pts {
            assert!((dist(*p, (0.0, 0.0)) - 10.0).abs() < 1e-9);
            assert!(p.0 >= -1e-9 && p.1 >= -1e-9);
        }
        // a quarter at r = 10 and 0.01 mm: 2 acos(0.999) = 0.0895 rad and (pi/2) / 0.0895 = 17.5, so 18 pieces and 19 points
        assert_eq!(pts.len(), 19);
    }

    #[test]
    fn an_arc_through_zero_degrees_wraps_round() {
        // from 350 to 10 is 20 degrees: the points are on both sides of the x axis
        let d = read(&entities(
            "0\nARC\n8\n0\n10\n0\n20\n0\n40\n10\n50\n350\n51\n10\n",
        ));
        let pts = &d.shapes[0].pts;
        assert!(pts.iter().any(|p| p.1 < -1.0) && pts.iter().any(|p| p.1 > 1.0));
        assert!(pts.iter().all(|p| p.0 > 9.0));
    }

    #[test]
    fn a_circle_is_a_closed_shape_of_the_right_size() {
        let d = read(&entities("0\nCIRCLE\n8\n0\n10\n1\n20\n2\n40\n5\n"));
        let s = &d.shapes[0];
        assert!(s.closed);
        // 2 acos(1 - 0.01/5) = 0.1265 rad: 2 pi / 0.1265 = 49.7, so 50 points
        assert_eq!(s.pts.len(), 50);
        assert!(close(s.pts[0], (6.0, 2.0)));
        for p in &s.pts {
            assert!((dist(*p, (1.0, 2.0)) - 5.0).abs() < 1e-9);
        }
    }

    #[test]
    fn the_unit_of_the_file_turns_everything_into_millimetres() {
        let line = "0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n2\n";
        let with = |unit: i32| {
            read(&format!(
                "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n{unit}\n0\nENDSEC\n{}",
                entities(line)
            ))
        };
        assert_eq!(with(4).shapes[0].pts[1], (1.0, 2.0));
        assert_eq!(with(1).shapes[0].pts[1], (25.4, 50.8));
        assert_eq!(with(5).shapes[0].pts[1], (10.0, 20.0));
        assert_eq!(with(6).shapes[0].pts[1], (1000.0, 2000.0));
        // unitless takes the unit of the settings
        let o = ReadOptions {
            dxf_unit_mm: 2.0,
            ..ReadOptions::default()
        };
        let d = read_dxf(entities(line).as_bytes(), &o).unwrap();
        assert_eq!(d.shapes[0].pts[1], (2.0, 4.0));
        // a circle in inches: the radius is scaled too
        let c = read(&format!(
            "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n1\n0\nENDSEC\n{}",
            entities("0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n1\n")
        ));
        assert!(close(c.shapes[0].pts[0], (25.4, 0.0)));
    }

    #[test]
    fn the_colour_is_kept_as_an_index_or_a_true_colour() {
        let d = read(&entities("0\nLINE\n8\nA\n62\n1\n10\n0\n20\n0\n11\n1\n21\n0\n0\nLINE\n8\nA\n62\n256\n10\n0\n20\n0\n11\n2\n21\n0\n0\nLINE\n8\nA\n420\n255\n10\n0\n20\n0\n11\n3\n21\n0\n0\nLINE\n8\nA\n62\n30\n10\n0\n20\n0\n11\n4\n21\n0\n"));
        let colors: Vec<_> = d.shapes.iter().map(|s| s.color.clone()).collect();
        assert_eq!(
            colors,
            vec![
                Some("#ff0000".to_string()),
                None,
                Some("#0000ff".to_string()),
                Some("aci30".to_string())
            ]
        );
    }

    #[test]
    fn a_straight_spline_passes_through_its_control_points() {
        // degree 1, three control points: the knots are 0 0 1 2 2
        let d = read(&entities("0\nSPLINE\n8\n0\n70\n8\n71\n1\n72\n5\n73\n3\n40\n0\n40\n0\n40\n1\n40\n2\n40\n2\n10\n0\n20\n0\n10\n4\n20\n3\n10\n8\n20\n0\n"));
        let pts = &d.shapes[0].pts;
        assert!(close(pts[0], (0.0, 0.0)) && close(*pts.last().unwrap(), (8.0, 0.0)));
        // the parameter 1 is the middle sample of 48 and the corner of the control polygon
        assert!(pts.iter().any(|p| close(*p, (4.0, 3.0))), "{pts:?}");
    }

    #[test]
    fn a_quadratic_spline_is_a_parabola() {
        // control points (0,0) (5,10) (10,0), knots 0 0 0 1 1 1: at t = 0.5 the point is (5, 5)
        let d = read(&entities("0\nSPLINE\n8\n0\n70\n8\n71\n2\n72\n6\n73\n3\n40\n0\n40\n0\n40\n0\n40\n1\n40\n1\n40\n1\n10\n0\n20\n0\n10\n5\n20\n10\n10\n10\n20\n0\n"));
        let pts = &d.shapes[0].pts;
        assert!(close(pts[0], (0.0, 0.0)) && close(*pts.last().unwrap(), (10.0, 0.0)));
        assert!(pts.iter().any(|p| close(*p, (5.0, 5.0))), "{pts:?}");
    }

    #[test]
    fn a_rational_spline_with_weights_is_a_quarter_circle() {
        // the quarter circle of radius 10 as a NURBS: controls (10,0) (10,10) (0,10), weights 1, sqrt(2)/2, 1
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let d = read(&entities(&format!(
            "0\nSPLINE\n8\n0\n70\n8\n71\n2\n72\n6\n73\n3\n40\n0\n40\n0\n40\n0\n40\n1\n40\n1\n40\n1\n41\n1\n41\n{w}\n41\n1\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n"
        )));
        for p in &d.shapes[0].pts {
            assert!((dist(*p, (0.0, 0.0)) - 10.0).abs() < 1e-9, "{p:?}");
        }
    }

    #[test]
    fn a_spline_with_fit_points_goes_through_them() {
        let d = read(&entities(
            "0\nSPLINE\n8\n0\n70\n0\n71\n3\n74\n3\n11\n0\n21\n0\n11\n5\n21\n5\n11\n9\n21\n0\n",
        ));
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (5.0, 5.0), (9.0, 0.0)]);
    }

    #[test]
    fn a_mirrored_arc_is_drawn_on_the_other_side() {
        // an arc about (10,0) from 0 to 90 degrees with the extrusion pointing down is the arc about (-10,0) from 90 to 180
        let d = read(&entities(
            "0\nARC\n8\n0\n10\n10\n20\n0\n40\n5\n50\n0\n51\n90\n210\n0\n220\n0\n230\n-1\n",
        ));
        let pts = &d.shapes[0].pts;
        assert!(close(pts[0], (-10.0, 5.0)));
        assert!(close(*pts.last().unwrap(), (-15.0, 0.0)));
    }

    /// A file with the blocks given and the entities that draw them.
    fn with_blocks(blocks: &str, body: &str) -> String {
        format!(
            "0\nSECTION\n2\nBLOCKS\n{blocks}0\nENDSEC\n{}",
            entities(body)
        )
    }

    /// The block `name` with the groups of its entities (base point at 0, 0).
    fn block(name: &str, body: &str) -> String {
        format!("0\nBLOCK\n8\n0\n2\n{name}\n70\n0\n10\n0\n20\n0\n{body}0\nENDBLK\n8\n0\n")
    }

    const UNIT_LINE: &str = "0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n";

    #[test]
    fn a_whole_ellipse_is_a_closed_shape_with_the_extent_of_its_axes() {
        // centre 5, 3; the major axis 4 along x; the minor one is half of it
        let d = read(&entities(
            "0\nELLIPSE\n8\nE\n10\n5\n20\n3\n11\n4\n21\n0\n40\n0.5\n41\n0\n42\n6.283185307179586\n",
        ));
        assert_eq!(d.shapes.len(), 1);
        let s = &d.shapes[0];
        assert!(s.closed && s.pts.len() >= 8);
        let (lo, hi) = crate::vector::bounds_of(s.pts.iter().copied()).unwrap();
        assert!(
            (lo.0 - 1.0).abs() < 0.02 && (hi.0 - 9.0).abs() < 0.02,
            "{lo:?} {hi:?}"
        );
        assert!(
            (lo.1 - 1.0).abs() < 0.02 && (hi.1 - 5.0).abs() < 0.02,
            "{lo:?} {hi:?}"
        );
        // a point of it satisfies the equation of the ellipse
        for p in &s.pts {
            let v = ((p.0 - 5.0) / 4.0).powi(2) + ((p.1 - 3.0) / 2.0).powi(2);
            assert!((v - 1.0).abs() < 1e-9, "{p:?} {v}");
        }
    }

    #[test]
    fn an_ellipse_arc_starts_and_ends_at_its_parameters_and_a_turned_major_axis_turns_it() {
        let q = read(&entities(
            "0\nELLIPSE\n8\nE\n10\n0\n20\n0\n11\n4\n21\n0\n40\n0.5\n41\n0\n42\n1.5707963267948966\n",
        ));
        let s = &q.shapes[0];
        assert!(!s.closed);
        assert!(close(s.pts[0], (4.0, 0.0)));
        assert!(close(*s.pts.last().unwrap(), (0.0, 2.0)));
        // the major axis along y: the same arc is turned a quarter turn
        let t = read(&entities(
            "0\nELLIPSE\n8\nE\n10\n0\n20\n0\n11\n0\n21\n4\n40\n0.5\n41\n0\n42\n1.5707963267948966\n",
        ));
        let s = &t.shapes[0];
        assert!(close(s.pts[0], (0.0, 4.0)));
        assert!(close(*s.pts.last().unwrap(), (-2.0, 0.0)));
        // a ratio of 1 is a circle
        let c = read(&entities(
            "0\nELLIPSE\n8\nE\n10\n0\n20\n0\n11\n3\n21\n0\n40\n1\n41\n0\n42\n6.283185307179586\n",
        ));
        assert!(c.shapes[0]
            .pts
            .iter()
            .all(|p| (p.0.hypot(p.1) - 3.0).abs() < 1e-9));
        // an extrusion that points down runs the arc the other way round
        let down = read(&entities(
            "0\nELLIPSE\n8\nE\n10\n0\n20\n0\n11\n4\n21\n0\n40\n0.5\n41\n0\n42\n1.5707963267948966\n230\n-1\n",
        ));
        assert!(close(*down.shapes[0].pts.last().unwrap(), (0.0, -2.0)));
    }

    #[test]
    fn an_ellipse_with_no_centre_ratio_or_axis_is_skipped_and_the_pieces_follow_the_tolerance() {
        for body in [
            "0\nELLIPSE\n8\n0\n10\n0\n20\n0\n40\n0.5\n",
            "0\nELLIPSE\n8\n0\n10\n0\n20\n0\n11\n4\n21\n0\n",
            "0\nELLIPSE\n8\n0\n10\n0\n20\n0\n11\n4\n21\n0\n40\n0\n",
            "0\nELLIPSE\n8\n0\n10\n0\n20\n0\n11\n4\n21\n0\n40\n2\n",
            "0\nELLIPSE\n8\n0\n10\n0\n20\n0\n11\n0\n21\n0\n40\n0.5\n",
        ] {
            let d = read(&entities(body));
            assert!(d.shapes.is_empty(), "{body:?}");
            assert!(
                d.warnings.iter().any(|w| w.starts_with("an ELLIPSE")),
                "{body:?}"
            );
        }
        let text = entities("0\nELLIPSE\n8\n0\n10\n0\n20\n0\n11\n50\n21\n0\n40\n0.5\n");
        let coarse = read_dxf(
            text.as_bytes(),
            &ReadOptions {
                tolerance_mm: 0.5,
                ..ReadOptions::default()
            },
        )
        .unwrap();
        let fine = read_dxf(
            text.as_bytes(),
            &ReadOptions {
                tolerance_mm: 0.01,
                ..ReadOptions::default()
            },
        )
        .unwrap();
        assert!(fine.shapes[0].pts.len() > coarse.shapes[0].pts.len());
    }

    #[test]
    fn an_insert_draws_the_block_at_its_place_scale_and_rotation() {
        let blocks = block("SQ", UNIT_LINE);
        // moved
        let d = read(&with_blocks(
            &blocks,
            "0\nINSERT\n8\nCut\n2\nSQ\n10\n10\n20\n20\n",
        ));
        assert_eq!(d.shapes.len(), 1);
        assert!(close(d.shapes[0].pts[0], (10.0, 20.0)) && close(d.shapes[0].pts[1], (11.0, 20.0)));
        // scaled by 2 in x and 3 in y, then turned a quarter turn: the line of length 1 along x becomes a line of length 2 along y
        let r = read(&with_blocks(
            &blocks,
            "0\nINSERT\n8\nCut\n2\nSQ\n10\n10\n20\n20\n41\n2\n42\n3\n50\n90\n",
        ));
        let p = &r.shapes[0].pts;
        assert!(
            close(p[0], (10.0, 20.0)) && close(p[1], (10.0, 22.0)),
            "{p:?}"
        );
        // the name is read in any case
        assert_eq!(
            read(&with_blocks(&blocks, "0\nINSERT\n8\n0\n2\nsq\n"))
                .shapes
                .len(),
            1
        );
    }

    #[test]
    fn the_base_point_of_a_block_is_the_place_that_lands_on_the_insertion_point() {
        let blocks = "0\nBLOCK\n8\n0\n2\nB\n70\n0\n10\n1\n20\n1\n0\nLINE\n8\n0\n10\n1\n20\n1\n11\n2\n21\n1\n0\nENDBLK\n8\n0\n";
        let d = read(&with_blocks(
            blocks,
            "0\nINSERT\n8\n0\n2\nB\n10\n5\n20\n5\n",
        ));
        let p = &d.shapes[0].pts;
        assert!(close(p[0], (5.0, 5.0)) && close(p[1], (6.0, 5.0)), "{p:?}");
    }

    #[test]
    fn an_entity_of_a_block_on_layer_zero_takes_the_layer_and_the_colour_of_the_insert() {
        let blocks = block(
            "B",
            "0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n0\nLINE\n8\nMark\n62\n5\n10\n0\n20\n1\n11\n1\n21\n1\n",
        );
        let d = read(&with_blocks(&blocks, "0\nINSERT\n8\nCut\n62\n1\n2\nB\n"));
        assert_eq!(d.shapes.len(), 2);
        assert_eq!(
            (d.shapes[0].layer.as_str(), d.shapes[0].color.as_deref()),
            ("Cut", Some("#ff0000"))
        );
        // an entity that names its own layer and colour keeps them
        assert_eq!(
            (d.shapes[1].layer.as_str(), d.shapes[1].color.as_deref()),
            ("Mark", Some("#0000ff"))
        );
    }

    #[test]
    fn a_block_inside_a_block_and_an_array_compose_their_places() {
        let inner = block("IN", UNIT_LINE);
        // OUT draws IN at 0, 5 scaled by 2
        let outer = block(
            "OUT",
            "0\nINSERT\n8\n0\n2\nIN\n10\n0\n20\n5\n41\n2\n42\n2\n",
        );
        let d = read(&with_blocks(
            &format!("{inner}{outer}"),
            "0\nINSERT\n8\n0\n2\nOUT\n10\n100\n20\n0\n",
        ));
        let p = &d.shapes[0].pts;
        assert!(
            close(p[0], (100.0, 5.0)) && close(p[1], (102.0, 5.0)),
            "{p:?}"
        );
        // an array of 3 columns and 2 rows 10 and 20 apart is six copies
        let arr = read(&with_blocks(
            &inner,
            "0\nINSERT\n8\n0\n2\nIN\n10\n0\n20\n0\n70\n3\n71\n2\n44\n10\n45\n20\n",
        ));
        assert_eq!(arr.shapes.len(), 6);
        let mut starts: Vec<Pt> = arr.shapes.iter().map(|s| s.pts[0]).collect();
        starts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            starts,
            vec![
                (0.0, 0.0),
                (0.0, 20.0),
                (10.0, 0.0),
                (10.0, 20.0),
                (20.0, 0.0),
                (20.0, 20.0)
            ]
        );
        // turned a quarter turn, the columns go up
        let turned = read(&with_blocks(
            &inner,
            "0\nINSERT\n8\n0\n2\nIN\n50\n90\n70\n2\n44\n10\n",
        ));
        assert!(
            close(turned.shapes[1].pts[0], (0.0, 10.0)),
            "{:?}",
            turned.shapes[1].pts
        );
    }

    #[test]
    fn the_insertion_point_is_in_the_unit_of_the_file() {
        let header = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n1\n0\nENDSEC\n";
        let file = format!(
            "{header}{}",
            with_blocks(
                &block("B", UNIT_LINE),
                "0\nINSERT\n8\n0\n2\nB\n10\n1\n20\n0\n"
            )
        );
        let d = read(&file);
        let p = &d.shapes[0].pts;
        assert!(
            close(p[0], (25.4, 0.0)) && close(p[1], (50.8, 0.0)),
            "{p:?}"
        );
    }

    #[test]
    fn an_insert_that_cannot_be_drawn_is_skipped_or_refused_and_a_block_cannot_draw_itself() {
        let blocks = block("B", UNIT_LINE);
        // a block the file does not define, and a scale of 0
        let d = read(&with_blocks(
            &blocks,
            "0\nINSERT\n8\n0\n2\nNOPE\n0\nINSERT\n8\n0\n2\nB\n41\n0\n",
        ));
        assert!(d.shapes.is_empty());
        assert!(d
            .warnings
            .iter()
            .any(|w| w.starts_with("INSERT of the block NOPE")));
        assert!(d.warnings.iter().any(|w| w.contains("scale of 0")));
        // a block that draws itself reaches the depth limit instead of looping
        let looping = block("L", "0\nINSERT\n8\n0\n2\nL\n");
        let e = read_dxf(
            with_blocks(&looping, "0\nINSERT\n8\n0\n2\nL\n").as_bytes(),
            &ReadOptions::default(),
        )
        .unwrap_err();
        assert!(e.contains("nested deeper"), "{e}");
        // an array that would repeat a block more than the limit of entities is refused before it is drawn
        let mut opts = ReadOptions::default();
        opts.limits.max_nodes = 50;
        let e = read_dxf(
            with_blocks(&blocks, "0\nINSERT\n8\n0\n2\nB\n70\n100\n71\n100\n").as_bytes(),
            &opts,
        )
        .unwrap_err();
        assert!(e.contains("repeats it"), "{e}");
        // every copy counts as an entity, so a block that holds many cannot get round the limit
        let many = block("M", &UNIT_LINE.repeat(10));
        let e = read_dxf(
            with_blocks(&many, "0\nINSERT\n8\n0\n2\nM\n70\n10\n").as_bytes(),
            &opts,
        )
        .unwrap_err();
        assert!(e.contains("more than"), "{e}");
    }

    #[test]
    fn what_is_not_supported_is_skipped_with_a_note() {
        let d = read(&entities("0\nTEXT\n8\n0\n10\n0\n20\n0\n1\nhello\n0\nHATCH\n8\n0\n0\nINSERT\n8\n0\n2\nB\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n1\n"));
        assert_eq!(d.shapes.len(), 1);
        assert!(d.warnings.iter().any(|w| w.starts_with("TEXT")));
        assert!(d.warnings.iter().any(|w| w.starts_with("HATCH")));
        assert!(d.warnings.iter().any(|w| w.starts_with("INSERT")));
    }

    #[test]
    fn entities_outside_the_entities_section_are_not_drawn() {
        let blocks = "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nB\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n1\n0\nENDBLK\n0\nENDSEC\n";
        let d = read(&format!(
            "{blocks}{}",
            entities("0\nLINE\n8\n0\n10\n0\n20\n0\n11\n2\n21\n2\n")
        ));
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].pts[1], (2.0, 2.0));
    }

    #[test]
    fn a_line_without_its_points_and_a_circle_without_a_radius_are_skipped() {
        let d = read(&entities("0\nLINE\n8\n0\n10\n0\n20\n0\n0\nCIRCLE\n8\n0\n10\n0\n20\n0\n0\nCIRCLE\n8\n0\n10\n0\n20\n0\n40\n-3\n"));
        assert!(d.shapes.is_empty());
        assert_eq!(d.warnings.len(), 3);
    }

    #[test]
    fn the_limits_stop_the_reading() {
        let two = entities("0\nLINE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n1\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n2\n21\n2\n");
        let mut o = ReadOptions::default();
        o.limits.max_nodes = 1;
        assert!(read_dxf(two.as_bytes(), &o)
            .unwrap_err()
            .contains("max_nodes"));
        let mut o = ReadOptions::default();
        o.limits.max_shapes = 1;
        assert!(read_dxf(two.as_bytes(), &o)
            .unwrap_err()
            .contains("max_shapes"));
        let mut o = ReadOptions::default();
        o.limits.max_file_bytes = 10;
        assert!(read_dxf(two.as_bytes(), &o)
            .unwrap_err()
            .contains("max_file_bytes"));
    }

    #[test]
    fn a_binary_file_and_a_file_that_is_not_dxf_are_errors() {
        assert!(
            read_dxf(b"AutoCAD Binary DXF\r\n\x1a\0", &ReadOptions::default())
                .unwrap_err()
                .contains("binary")
        );
        for bad in ["", "hello\nworld\n", "0\nSECTION\n2", "x\n0\n", "\n\n"] {
            assert!(
                read_dxf(bad.as_bytes(), &ReadOptions::default()).is_err(),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_file_cut_at_any_place_or_made_of_garbage_never_panics() {
        let full = entities(
            "0\nLINE\n8\nA\n10\n0\n20\n0\n11\n1\n21\n1\n0\nLWPOLYLINE\n8\nA\n90\n3\n70\n1\n10\n0\n20\n0\n42\n0.5\n10\n4\n20\n0\n10\n4\n20\n4\n0\nARC\n8\nA\n10\n0\n20\n0\n40\n3\n50\n10\n51\n200\n0\nCIRCLE\n8\nA\n10\n0\n20\n0\n40\n3\n0\nSPLINE\n8\nA\n71\n2\n73\n3\n40\n0\n40\n0\n40\n0\n40\n1\n40\n1\n40\n1\n10\n0\n20\n0\n10\n5\n20\n10\n10\n10\n20\n0\n0\nPOLYLINE\n8\nA\n70\n1\n0\nVERTEX\n10\n0\n20\n0\n0\nVERTEX\n10\n1\n20\n1\n0\nSEQEND\n",
        );
        for end in 0..=full.len() {
            let _ = read_dxf(&full.as_bytes()[..end], &ReadOptions::default());
        }
        let mut seed = 99u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };
        for _ in 0..300 {
            let mut bytes = full.as_bytes().to_vec();
            for _ in 0..3 {
                let at = next() % bytes.len();
                bytes[at] = b"0123456789.-\nXA"[next() % 15];
            }
            let _ = read_dxf(&bytes, &ReadOptions::default());
        }
        for _ in 0..100 {
            let junk: Vec<u8> = (0..200).map(|_| (next() % 256) as u8).collect();
            let _ = read_dxf(&junk, &ReadOptions::default());
        }
    }

    #[test]
    fn numbers_that_are_huge_or_not_numbers_do_not_break_a_spline() {
        // knots that go down, a degree larger than the points, a weight of zero: the control polygon is used and nothing panics
        let d = read(&entities(
            "0\nSPLINE\n8\n0\n71\n9\n73\n2\n40\n3\n40\n1\n10\n0\n20\n0\n10\n1\n20\n1\n",
        ));
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (1.0, 1.0)]);
        let d = read(&entities("0\nSPLINE\n8\n0\n71\n1\n73\n3\n40\n0\n40\n0\n40\n1\n40\n2\n40\n2\n41\n0\n41\n0\n41\n0\n10\n0\n20\n0\n10\n1\n20\n1\n10\n2\n20\n0\n"));
        assert_eq!(d.shapes.len(), 1);
    }
}
