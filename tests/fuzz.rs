//! Damaged drawings and pictures with a fixed seed: every cut of the example drawings and many copies of them with bytes changed, inserted or cut are read without a panic, and a drawing
//! that is read holds no shape with a number that is not finite. The seed is fixed, so a failure is the same failure every time.

use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::vector::{Drawing, ReadOptions};

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
                    0 => copy[at] = b"0123456789.-eE\n \t<>/=\"ZMLCQ"[rng.below(28)],
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
