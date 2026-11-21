//! The hatches and the solids of a DXF drawing, written here as text (nothing is a copy of a drawing made by somebody else): the outline of a hatch with a polyline path or with a path of
//! edges, the order of the corners of a solid, a hatch in a block, damaged hatches and a fuzz with a fixed seed.

use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::vector::{Drawing, ReadOptions};

fn doc(entities: &str) -> String {
    format!("0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n")
}

fn read(entities: &str) -> Drawing {
    read_dxf(doc(entities).as_bytes(), &ReadOptions::default()).unwrap()
}

/// A hatch whose only path is a polyline path of a square of 10 mm.
const POLY_HATCH: &str = "0\nHATCH\n8\nFILL\n10\n0\n20\n0\n30\n0\n210\n0\n220\n0\n230\n1\n2\nSOLID\n70\n1\n71\n0\n91\n1\n92\n7\n72\n0\n73\n1\n93\n4\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n97\n0\n75\n0\n76\n1\n98\n0\n";

/// A hatch whose path is made of four edges (three lines and a half circle), as a drawing program writes a rounded slot.
const EDGE_HATCH: &str = "0\nHATCH\n8\n0\n91\n1\n92\n1\n93\n4\n72\n1\n10\n0\n20\n0\n11\n20\n21\n0\n72\n2\n10\n20\n20\n5\n40\n5\n50\n270\n51\n90\n73\n1\n72\n1\n10\n20\n20\n10\n11\n0\n21\n10\n72\n1\n10\n0\n20\n10\n11\n0\n21\n0\n97\n0\n75\n0\n";

#[test]
fn the_outline_of_a_hatch_with_a_polyline_path_is_drawn_and_its_fill_is_not() {
    let d = read(POLY_HATCH);
    assert_eq!(d.shapes.len(), 1);
    let s = &d.shapes[0];
    assert!(s.closed);
    assert_eq!(s.layer, "FILL");
    assert_eq!(s.pts.len(), 4);
    assert_eq!(s.pts[2], (10.0, 10.0));
    assert!(d.warnings.iter().any(|w| w.contains("fill pattern is not")));
}

#[test]
fn a_hatch_with_edges_chains_its_lines_and_arcs_into_one_closed_outline() {
    let d = read(EDGE_HATCH);
    assert_eq!(d.shapes.len(), 1, "{:?}", d.warnings);
    let s = &d.shapes[0];
    assert!(s.closed);
    // the half circle goes from the bottom of the right end (20, 0) to its top (20, 10) round the right side: it reaches x = 25 at most and stays at the right of x = 20
    let (x0, x1) = s
        .pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    assert!(
        (x0 - 0.0).abs() < 1e-9 && (x1 - 25.0).abs() < 0.05,
        "{x0} {x1}"
    );
    assert!(s
        .pts
        .iter()
        .filter(|p| p.0 > 20.5)
        .all(|p| p.1 > -0.001 && p.1 < 10.001));
    assert!(s.pts.iter().filter(|p| p.0 > 20.5).count() >= 4);
    // the last point does not repeat the first
    assert!(s.pts.first() != s.pts.last());
}

#[test]
fn an_arc_edge_that_turns_clockwise_goes_the_other_way_round() {
    let ccw = "0\nHATCH\n8\n0\n91\n1\n92\n1\n93\n3\n72\n2\n10\n0\n20\n0\n40\n5\n50\n0\n51\n180\n73\n1\n72\n1\n10\n-5\n20\n0\n11\n5\n21\n0\n72\n1\n10\n5\n20\n0\n11\n5\n21\n0\n97\n0\n";
    let cw = ccw.replace("51\n180\n73\n1", "51\n180\n73\n0");
    let a = read(ccw);
    let b = read(&cw);
    let top = |d: &Drawing| d.shapes[0].pts.iter().any(|p| p.1 > 4.9);
    let bottom = |d: &Drawing| d.shapes[0].pts.iter().any(|p| p.1 < -4.9);
    assert!(top(&a) && !bottom(&a));
    assert!(bottom(&b) && !top(&b));
}

#[test]
fn a_hatch_with_several_paths_gives_one_shape_for_each() {
    let two = "0\nHATCH\n8\n0\n91\n2\n92\n7\n72\n0\n73\n1\n93\n3\n10\n0\n20\n0\n10\n10\n20\n0\n10\n0\n20\n10\n92\n23\n72\n0\n73\n1\n93\n3\n10\n2\n20\n2\n10\n4\n20\n2\n10\n2\n20\n4\n97\n0\n";
    let d = read(two);
    assert_eq!(d.shapes.len(), 2);
    assert!(d.warnings.iter().any(|w| w.contains("2 path(s)")));
}

#[test]
fn a_hatch_that_has_nothing_to_draw_or_only_splines_says_so() {
    let d = read("0\nHATCH\n8\n0\n91\n0\n");
    assert!(d.shapes.is_empty());
    assert!(d.warnings.iter().any(|w| w.starts_with("HATCH")));
    let spline = "0\nHATCH\n8\n0\n91\n1\n92\n1\n93\n1\n72\n4\n94\n3\n97\n0\n";
    let d = read(spline);
    assert!(d.shapes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("splines")));
}

#[test]
fn a_solid_takes_its_corners_in_the_order_of_the_file_and_a_triangle_has_three() {
    // the file gives the corners 1, 2, 4, 3: the outline goes round the square
    let square = "0\nSOLID\n8\n0\n10\n0\n20\n0\n11\n10\n21\n0\n12\n0\n22\n10\n13\n10\n23\n10\n";
    let d = read(square);
    assert_eq!(d.shapes.len(), 1);
    assert_eq!(
        d.shapes[0].pts,
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
    );
    assert!(d.shapes[0].closed);
    // the fourth corner equal to the third is a triangle, and a missing one is too
    let tri = "0\nSOLID\n8\n0\n10\n0\n20\n0\n11\n10\n21\n0\n12\n0\n22\n10\n13\n0\n23\n10\n";
    assert_eq!(read(tri).shapes[0].pts.len(), 3);
    let tri = "0\nTRACE\n8\n0\n10\n0\n20\n0\n11\n10\n21\n0\n12\n0\n22\n10\n";
    assert_eq!(read(tri).shapes[0].pts.len(), 3);
    // two corners are not a solid
    let d = read("0\nSOLID\n8\n0\n10\n0\n20\n0\n11\n10\n21\n0\n");
    assert!(d.shapes.is_empty());
    assert!(d.warnings.iter().any(|w| w.starts_with("a SOLID")));
}

#[test]
fn a_hatch_in_a_block_is_drawn_where_the_insert_puts_it() {
    let blocks = format!(
        "0\nSECTION\n2\nBLOCKS\n0\nBLOCK\n2\nB\n10\n0\n20\n0\n{POLY_HATCH}0\nENDBLK\n0\nENDSEC\n"
    );
    let text = format!(
        "{blocks}{}",
        doc("0\nINSERT\n8\nPLACED\n2\nB\n10\n100\n20\n50\n41\n2\n42\n2\n")
    );
    let d = read_dxf(text.as_bytes(), &ReadOptions::default()).unwrap();
    assert_eq!(d.shapes.len(), 1);
    let s = &d.shapes[0];
    assert_eq!(s.layer, "FILL");
    assert_eq!(s.pts[0], (100.0, 50.0));
    assert_eq!(s.pts[2], (120.0, 70.0));
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
fn damaged_hatches_never_panic_with_a_fixed_seed() {
    let mut rng = Rng(0x5eed_1a5e_0000_0039);
    for source in [POLY_HATCH, EDGE_HATCH] {
        let lines: Vec<&str> = source.lines().collect();
        for _ in 0..400 {
            let mut l: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
            for _ in 0..1 + rng.below(4) {
                if l.is_empty() {
                    break;
                }
                let i = rng.below(l.len());
                match rng.below(4) {
                    0 => l[i] = ["", "x", "-1", "1e999", "nan", "99999999999"][rng.below(6)].into(),
                    1 => {
                        l.remove(i);
                    }
                    2 => l.insert(i, (rng.next() % 100).to_string()),
                    _ => l.truncate(i.max(1)),
                }
            }
            let text = doc(&format!("{}\n", l.join("\n")));
            // either a drawing or a refusal; the only thing that must not happen is a panic, and no shape may hold a point that is not a number
            if let Ok(d) = read_dxf(text.as_bytes(), &ReadOptions::default()) {
                assert!(d
                    .shapes
                    .iter()
                    .all(|s| s.pts.iter().all(|p| p.0.is_finite() && p.1.is_finite())));
            }
        }
    }
}
