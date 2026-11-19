//! A QR code of the program's own: versions 1 to 4 (21 to 33 modules a side) in byte mode, with the four levels of error correction (Reed-Solomon over GF(256)) and the mask with the lowest
//! penalty, so that a serial number, a web address or a short note can be burnt with no picture. It is burnt by filling the dark modules with lines. Data that does not fit in the version
//! the settings allow, a symbol bigger than the size the settings allow, a power or a speed over the limit or work outside of the bed is refused, never cut down.

use crate::gcode_out::number as n;
use crate::vconfig::{QrEcc, QrSettings};

/// The largest version that this encoder makes.
pub const MAX_VERSION: u32 = 4;

/// The error-correction codewords of each block and the number of blocks, for the versions 1 to 4 and the levels L, M, Q and H. Every block of a version and level has the same size.
const BLOCKS: [[(usize, usize); 4]; 4] = [
    [(7, 1), (10, 1), (13, 1), (17, 1)],
    [(10, 1), (16, 1), (22, 1), (28, 1)],
    [(15, 1), (26, 1), (18, 2), (22, 2)],
    [(20, 1), (18, 2), (26, 2), (16, 4)],
];

/// The codewords of the whole symbol, by version.
const TOTAL_CODEWORDS: [usize; 4] = [26, 44, 70, 100];

fn level_index(e: QrEcc) -> usize {
    match e {
        QrEcc::L => 0,
        QrEcc::M => 1,
        QrEcc::Q => 2,
        QrEcc::H => 3,
    }
}

/// The two bits of the format information that name a level.
fn level_bits(e: QrEcc) -> u32 {
    match e {
        QrEcc::L => 1,
        QrEcc::M => 0,
        QrEcc::Q => 3,
        QrEcc::H => 2,
    }
}

/// The data codewords of a version at a level.
pub fn data_codewords(version: u32, ecc: QrEcc) -> usize {
    let (ec, blocks) = BLOCKS[version as usize - 1][level_index(ecc)];
    TOTAL_CODEWORDS[version as usize - 1] - ec * blocks
}

/// The most bytes that a version holds at a level (the mode and the length take a byte and a half).
pub fn capacity_bytes(version: u32, ecc: QrEcc) -> usize {
    data_codewords(version, ecc) - 2
}

/// The side of a version, in modules.
pub fn side(version: u32) -> usize {
    17 + 4 * version as usize
}

/// The tables of the field of 256 elements that the codewords live in (x^8 + x^4 + x^3 + x^2 + 1).
struct Field {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Field {
    fn new() -> Field {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u32 = 1;
        for (i, e) in exp.iter_mut().enumerate().take(255) {
            *e = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= 0x11D;
            }
        }
        for i in 255..512 {
            exp[i] = exp[i - 255];
        }
        Field { exp, log }
    }

    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            self.exp[self.log[a as usize] as usize + self.log[b as usize] as usize]
        }
    }
}

/// The error-correction codewords of one block of data: the remainder of the data, as a polynomial, divided by the polynomial with the roots 1, a, a^2... a^(n-1).
fn reed_solomon(f: &Field, data: &[u8], ec: usize) -> Vec<u8> {
    // the divisor without its leading coefficient
    let mut divisor = vec![0u8; ec];
    divisor[ec - 1] = 1;
    let mut root = 1u8;
    for _ in 0..ec {
        for j in 0..ec {
            divisor[j] = f.mul(divisor[j], root);
            if j + 1 < ec {
                divisor[j] ^= divisor[j + 1];
            }
        }
        root = f.mul(root, 2);
    }
    let mut rem = vec![0u8; ec];
    for &b in data {
        let factor = b ^ rem[0];
        rem.remove(0);
        rem.push(0);
        for (x, &c) in rem.iter_mut().zip(&divisor) {
            *x ^= f.mul(c, factor);
        }
    }
    rem
}

/// The 15 bits of the format information: the level and the mask, with ten bits of BCH code, XORed with the pattern that keeps them from being all zero.
pub fn format_bits(ecc: QrEcc, mask: u32) -> u32 {
    let data = (level_bits(ecc) << 3) | mask;
    let mut rem = data;
    for _ in 0..10 {
        rem = (rem << 1) ^ ((rem >> 9) * 0x537);
    }
    ((data << 10) | rem) ^ 0x5412
}

/// A QR symbol: dark modules are `true`, row by row from the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub version: u32,
    pub ecc: QrEcc,
    pub mask: u32,
    pub size: usize,
    modules: Vec<bool>,
}

impl Symbol {
    /// Whether the module in column `x` and row `y` (from the top) is dark; outside of the symbol it is light.
    pub fn dark(&self, x: usize, y: usize) -> bool {
        x < self.size && y < self.size && self.modules[y * self.size + x]
    }
}

impl Symbol {
    /// The symbol as text, a row of text for each row of modules and two characters for each module (`##` dark, two spaces light), with no margin.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for y in 0..self.size {
            for x in 0..self.size {
                out.push_str(if self.dark(x, y) { "##" } else { "  " });
            }
            out.push('\n');
        }
        out
    }
}

/// The modules that are not data: the finder patterns with their separators, the timing lines, the alignment pattern, the format information and the dark module.
fn function_map(version: u32) -> Vec<bool> {
    let size = side(version);
    let mut m = vec![false; size * size];
    let mut mark = |x: i32, y: i32| {
        if x >= 0 && y >= 0 && (x as usize) < size && (y as usize) < size {
            m[y as usize * size + x as usize] = true;
        }
    };
    for i in 0..size as i32 {
        mark(6, i);
        mark(i, 6);
    }
    let s = size as i32;
    for (cx, cy) in [(3, 3), (s - 4, 3), (3, s - 4)] {
        for dy in -4..=4 {
            for dx in -4..=4 {
                mark(cx + dx, cy + dy);
            }
        }
    }
    if version >= 2 {
        for dy in -2..=2 {
            for dx in -2..=2 {
                mark(s - 7 + dx, s - 7 + dy);
            }
        }
    }
    // the format information, twice, and the dark module
    for i in 0..9 {
        mark(8, i);
        mark(i, 8);
    }
    for i in 0..8 {
        mark(s - 1 - i, 8);
        mark(8, s - 1 - i);
    }
    m
}

/// Whether a mask turns the module in column `x` and row `y` over.
fn masked(mask: u32, x: usize, y: usize) -> bool {
    let (x, y) = (x as u32, y as u32);
    match mask {
        0 => (x + y) % 2 == 0,
        1 => y % 2 == 0,
        2 => x % 3 == 0,
        3 => (x + y) % 3 == 0,
        4 => (x / 3 + y / 2) % 2 == 0,
        5 => x * y % 2 + x * y % 3 == 0,
        6 => (x * y % 2 + x * y % 3) % 2 == 0,
        _ => ((x + y) % 2 + x * y % 3) % 2 == 0,
    }
}

/// The patterns that are always the same, as a grid of the modules that are dark.
fn fixed_modules(version: u32) -> Vec<bool> {
    let size = side(version);
    let mut m = vec![false; size * size];
    let s = size as i32;
    let mut set = |x: i32, y: i32, dark: bool| {
        if x >= 0 && y >= 0 && x < s && y < s {
            m[y as usize * size + x as usize] = dark;
        }
    };
    for i in 0..s {
        set(6, i, i % 2 == 0);
        set(i, 6, i % 2 == 0);
    }
    for (cx, cy) in [(3, 3), (s - 4, 3), (3, s - 4)] {
        for dy in -4i32..=4 {
            for dx in -4i32..=4 {
                let d = dx.abs().max(dy.abs());
                set(cx + dx, cy + dy, d != 2 && d != 4);
            }
        }
    }
    if version >= 2 {
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                set(s - 7 + dx, s - 7 + dy, dx.abs().max(dy.abs()) != 1);
            }
        }
    }
    // the dark module beside the lower left finder
    set(8, s - 8, true);
    m
}

/// Writes the 15 bits of the format information in their two places.
fn draw_format(modules: &mut [bool], size: usize, bits: u32) {
    let s = size as i32;
    let bit = |i: u32| (bits >> i) & 1 != 0;
    let mut set = |x: i32, y: i32, v: bool| modules[y as usize * size + x as usize] = v;
    for i in 0..=5 {
        set(8, i, bit(i as u32));
    }
    set(8, 7, bit(6));
    set(8, 8, bit(7));
    set(7, 8, bit(8));
    for i in 9..15 {
        set(14 - i, 8, bit(i as u32));
    }
    for i in 0..8 {
        set(s - 1 - i, 8, bit(i as u32));
    }
    for i in 8..15 {
        set(8, s - 15 + i, bit(i as u32));
    }
}

/// How badly a symbol is drawn for a reader: long runs of one colour, blocks of one colour, patterns that look like a finder and a balance of dark and light that is far from half.
fn penalty(modules: &[bool], size: usize) -> u32 {
    let at = |x: usize, y: usize| modules[y * size + x];
    let mut total = 0u32;
    for line in 0..size {
        for horizontal in [true, false] {
            let get = |i: usize| if horizontal { at(i, line) } else { at(line, i) };
            let mut run = 1u32;
            for i in 1..size {
                if get(i) == get(i - 1) {
                    run += 1;
                } else {
                    if run >= 5 {
                        total += 3 + (run - 5);
                    }
                    run = 1;
                }
            }
            if run >= 5 {
                total += 3 + (run - 5);
            }
            // a finder-like pattern: 1011101 with four light modules on one side
            for i in 0..size.saturating_sub(10) {
                let w: Vec<bool> = (0..11).map(|k| get(i + k)).collect();
                let a = [
                    true, false, true, true, true, false, true, false, false, false, false,
                ];
                let b = [
                    false, false, false, false, true, false, true, true, true, false, true,
                ];
                if w == a || w == b {
                    total += 40;
                }
            }
        }
    }
    for y in 0..size - 1 {
        for x in 0..size - 1 {
            let c = at(x, y);
            if at(x + 1, y) == c && at(x, y + 1) == c && at(x + 1, y + 1) == c {
                total += 3;
            }
        }
    }
    let dark = modules.iter().filter(|d| **d).count();
    let all = size * size;
    let off = (dark * 100).abs_diff(all * 50);
    total + 10 * (off / (5 * all)) as u32
}

/// The data codewords of a text in byte mode, padded to the size of the version.
fn data_stream(data: &[u8], version: u32, ecc: QrEcc) -> Vec<u8> {
    let total = data_codewords(version, ecc);
    fn push(bits: &mut Vec<bool>, value: u32, count: u32) {
        for i in (0..count).rev() {
            bits.push((value >> i) & 1 != 0);
        }
    }
    let mut bits: Vec<bool> = Vec::new();
    push(&mut bits, 0b0100, 4);
    push(&mut bits, data.len() as u32, 8);
    for &b in data {
        push(&mut bits, u32::from(b), 8);
    }
    let room = total * 8 - bits.len();
    push(&mut bits, 0, room.min(4) as u32);
    while !bits.len().is_multiple_of(8) {
        bits.push(false);
    }
    let mut out: Vec<u8> = bits
        .chunks(8)
        .map(|c| c.iter().fold(0u8, |a, &b| (a << 1) | u8::from(b)))
        .collect();
    let mut pad = [0xECu8, 0x11].into_iter().cycle();
    while out.len() < total {
        out.extend(pad.next());
    }
    out
}

/// The codewords of the symbol: the data of each block, then the error-correction codewords of each block, interleaved.
fn codewords(stream: &[u8], version: u32, ecc: QrEcc) -> Vec<u8> {
    let f = Field::new();
    let (ec, blocks) = BLOCKS[version as usize - 1][level_index(ecc)];
    let each = stream.len() / blocks;
    let datas: Vec<&[u8]> = stream.chunks(each).collect();
    let ecs: Vec<Vec<u8>> = datas.iter().map(|d| reed_solomon(&f, d, ec)).collect();
    let mut out = Vec::new();
    for i in 0..each {
        for d in &datas {
            out.push(d[i]);
        }
    }
    for i in 0..ec {
        for e in &ecs {
            out.push(e[i]);
        }
    }
    out
}

/// Makes the symbol of some bytes: the smallest version up to `max_version` (1 to 4) that holds them at the level, drawn with the mask of the lowest penalty. Data that does not fit is an
/// error that says what it would need.
pub fn encode(data: &[u8], ecc: QrEcc, max_version: u32) -> Result<Symbol, String> {
    if data.is_empty() {
        return Err("the data has nothing to encode".to_string());
    }
    if !(1..=MAX_VERSION).contains(&max_version) {
        return Err(format!(
            "max_version is {max_version}: it must be from 1 to {MAX_VERSION}"
        ));
    }
    let Some(version) = (1..=max_version).find(|v| capacity_bytes(*v, ecc) >= data.len()) else {
        let best = capacity_bytes(max_version, ecc);
        let needs = (max_version + 1..=MAX_VERSION).find(|v| capacity_bytes(*v, ecc) >= data.len());
        return Err(match needs {
            Some(v) => format!(
                "the data is {} bytes: version {max_version} at level {} holds {best}, and it needs version {v} (max_version in [qr])",
                data.len(),
                ecc.word()
            ),
            None => format!(
                "the data is {} bytes: the largest symbol made here (version {MAX_VERSION}) holds at most {} at level {}",
                data.len(),
                capacity_bytes(MAX_VERSION, ecc),
                ecc.word()
            ),
        });
    };
    let size = side(version);
    let words = codewords(&data_stream(data, version, ecc), version, ecc);
    let function = function_map(version);
    let mut base = fixed_modules(version);
    // the codewords go in a zigzag of columns of two, from the lower right corner, upward and downward in turn, skipping the timing column
    let mut i = 0usize;
    let mut right = size as i32 - 1;
    while right >= 1 {
        if right == 6 {
            right = 5;
        }
        for vert in 0..size {
            for j in 0..2 {
                let x = (right - j) as usize;
                let upward = ((right + 1) & 2) == 0;
                let y = if upward { size - 1 - vert } else { vert };
                if !function[y * size + x] && i < words.len() * 8 {
                    base[y * size + x] = (words[i >> 3] >> (7 - (i & 7))) & 1 != 0;
                    i += 1;
                }
            }
        }
        right -= 2;
    }
    let mut best: Option<(u32, u32, Vec<bool>)> = None;
    for mask in 0..8u32 {
        let mut m = base.clone();
        for y in 0..size {
            for x in 0..size {
                if !function[y * size + x] && masked(mask, x, y) {
                    m[y * size + x] = !m[y * size + x];
                }
            }
        }
        draw_format(&mut m, size, format_bits(ecc, mask));
        let p = penalty(&m, size);
        if best.as_ref().is_none_or(|b| p < b.0) {
            best = Some((p, mask, m));
        }
    }
    let (_, mask, modules) = best.expect("eight masks were tried");
    Ok(Symbol {
        version,
        ecc,
        mask,
        size,
        modules,
    })
}

/// The side of the square that is burnt, in modules: the symbol and the border around it.
pub fn side_modules(sym: &Symbol, q: &QrSettings) -> usize {
    sym.size + 2 * q.border_modules as usize
}

/// What a program of a QR symbol needs from the machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// The spindle value that is full power.
    pub s_max: u32,
    pub max_power_percent: f64,
    pub max_speed_mm_min: f64,
    /// The size of the bed, when it is known: a symbol that would go outside of it is refused.
    pub bed_mm: Option<(f64, f64)>,
}

/// The G-code of a symbol whose lower left corner is at `origin`: each dark module (each light one when `invert` is on) is filled with lines `line_mm` apart, burnt at the power and the speed of the settings as many times as
/// `passes` says, with the laser off between the runs. Data over `max_bytes`, a symbol that needs a version over `max_version` or is larger than `max_size_mm`, a fill of more than
/// `max_lines` moves, a power or a speed over the limit, or a symbol (with the run-up of `overscan_mm` past each end of a line) outside of the bed is refused and nothing is written. The data is never written into a comment of the program.
pub fn program(
    data: &str,
    origin: (f64, f64),
    q: &QrSettings,
    m: &Machine,
) -> Result<(String, Symbol), String> {
    q.validate()?;
    crate::safety::validate_max(m.max_power_percent)?;
    crate::safety::check_power(
        "the power of the QR code",
        q.power_percent,
        m.max_power_percent,
    )?;
    crate::safety::check_speed(
        "the speed of the QR code",
        q.speed_mm_min,
        m.max_speed_mm_min,
    )?;
    if m.s_max == 0 || m.s_max > 100_000 {
        return Err("s_max must be from 1 to 100000".to_string());
    }
    if !(origin.0.is_finite() && origin.1.is_finite()) {
        return Err("the origin of the QR code is not a number".to_string());
    }
    let bytes = data.as_bytes();
    if bytes.len() > q.max_bytes as usize {
        return Err(format!(
            "the data is {} bytes: it can be at most {} (max_bytes in [qr])",
            bytes.len(),
            q.max_bytes
        ));
    }
    let sym = encode(bytes, q.ecc, q.max_version)?;
    if q.border_modules > 0 && !q.invert {
        return Err(format!(
            "border_modules is {}: a border is burnt only around an inverted symbol (invert in [qr])",
            q.border_modules
        ));
    }
    // the square that is burnt: the symbol and, when it is inverted, the border around it
    let total = side_modules(&sym, q);
    let width = total as f64 * q.module_mm;
    if width > q.max_size_mm + 1e-9 {
        return Err(format!(
            "the symbol is {width:.1} x {width:.1} mm ({} modules of {} mm): it can be at most {} mm (max_size_mm in [qr])",
            sym.size, q.module_mm, q.max_size_mm
        ));
    }
    if let Some((bx, by)) = m.bed_mm {
        if origin.0 < 0.0
            || origin.1 < 0.0
            || origin.0 + width > bx + 1e-9
            || origin.1 + width > by + 1e-9
        {
            return Err(format!(
                "the symbol is {width:.1} x {width:.1} mm and, from {}, {}, it goes outside of the bed of {bx} x {by} mm",
                origin.0, origin.1
            ));
        }
    }
    let lines = (q.module_mm / q.line_mm - 1e-9).ceil().max(1.0) as usize;
    // a module is burnt when it is dark, or when it is light and the symbol is inverted; the border is all burnt
    // (a mirror image has the first column where the last one was)
    let edge = q.border_modules as usize;
    let burns = |x: usize, y: usize| {
        if x < edge || y < edge || x >= edge + sym.size || y >= edge + sym.size {
            return true;
        }
        let (x, y) = (x - edge, y - edge);
        let x = if q.mirror { sym.size - 1 - x } else { x };
        sym.dark(x, y) != q.invert
    };
    // the runs of burnt modules of each row, from the bottom
    let mut rows: Vec<Vec<(usize, usize)>> = Vec::new();
    for rb in 0..total {
        let y = total - 1 - rb;
        let mut runs = Vec::new();
        let mut x = 0;
        while x < total {
            if burns(x, y) {
                let start = x;
                while x + 1 < total && burns(x + 1, y) {
                    x += 1;
                }
                runs.push((start, x));
            }
            x += 1;
        }
        rows.push(runs);
    }
    let moves: usize = rows.iter().map(Vec::len).sum::<usize>() * lines * q.passes as usize;
    if moves > q.max_lines as usize {
        return Err(format!(
            "the fill needs {moves} burning moves: it can have at most {} (max_lines in [qr])",
            q.max_lines
        ));
    }
    // the time of the burning itself: the length of every burnt line, in every pass, at the speed of the settings
    let burn_mm = rows
        .iter()
        .flatten()
        .map(|r| (r.1 - r.0 + 1) as f64 * q.module_mm)
        .sum::<f64>()
        * lines as f64
        * f64::from(q.passes);
    let minutes = burn_mm / q.speed_mm_min;
    if minutes > q.max_minutes + 1e-9 {
        return Err(format!(
            "burning the symbol would take about {minutes:.1} minutes ({burn_mm:.0} mm at {} mm/min): it can take at most {} (max_minutes in [qr])",
            q.speed_mm_min, q.max_minutes
        ));
    }
    // the run-up goes past each end of a burnt line with the laser off: it must stay inside the bed too
    let over = q.overscan_mm;
    if let (Some((bx, _)), true) = (m.bed_mm, over > 0.0) {
        let first = rows.iter().flatten().map(|r| r.0).min();
        let last = rows.iter().flatten().map(|r| r.1).max();
        if let (Some(a), Some(b)) = (first, last) {
            let lo = origin.0 + a as f64 * q.module_mm - over;
            let hi = origin.0 + (b + 1) as f64 * q.module_mm + over;
            if lo < -1e-9 || hi > bx + 1e-9 {
                return Err(format!(
                    "the run-up of {over} mm at each end of a line goes from X{lo:.1} to X{hi:.1}, outside of the bed of {bx} mm (overscan_mm in [qr])"
                ));
            }
        }
    }
    let s = (f64::from(m.s_max) * q.power_percent / 100.0).round() as u32;
    let feed = n(q.speed_mm_min)?;
    let mut out = String::from("G21\nG90\nM4 S0\n");
    let mut notes = String::new();
    if q.invert {
        notes.push_str(", inverted");
    }
    if q.mirror {
        notes.push_str(", mirrored");
    }
    if edge > 0 {
        notes.push_str(&format!(", border of {edge} modules"));
    }
    out.push_str(&format!(
        "(qr code of version {}, {} x {} modules, {} passes{notes})\n",
        sym.version, sym.size, sym.size, q.passes
    ));
    let mut lit = false;
    for _ in 0..q.passes {
        let mut backwards = false;
        for (rb, runs) in rows.iter().enumerate() {
            if runs.is_empty() {
                continue;
            }
            for k in 0..lines {
                let y = origin.1
                    + rb as f64 * q.module_mm
                    + (k as f64 + 0.5) * q.module_mm / lines as f64;
                let ordered: Vec<(usize, usize)> = if backwards {
                    runs.iter().rev().copied().collect()
                } else {
                    runs.clone()
                };
                for (a, b) in ordered {
                    let x0 = origin.0 + a as f64 * q.module_mm;
                    let x1 = origin.0 + (b + 1) as f64 * q.module_mm;
                    let (from, to) = if backwards { (x1, x0) } else { (x0, x1) };
                    // no rapid move with the power of the last run still set
                    if lit {
                        out.push_str("S0\n");
                    }
                    if over > 0.0 {
                        // the head runs up to the line with the laser off and runs out past its end, so it is at speed all along the burnt part
                        let dir = if backwards { -1.0 } else { 1.0 };
                        out.push_str(&format!("G0 X{} Y{}\n", n(from - dir * over)?, n(y)?));
                        out.push_str(&format!("G1 X{} Y{} S0 F{feed}\n", n(from)?, n(y)?));
                        out.push_str(&format!("G1 X{} Y{} S{s} F{feed}\n", n(to)?, n(y)?));
                        out.push_str(&format!(
                            "G1 X{} Y{} S0 F{feed}\n",
                            n(to + dir * over)?,
                            n(y)?
                        ));
                        lit = false;
                    } else {
                        out.push_str(&format!("G0 X{} Y{}\n", n(from)?, n(y)?));
                        out.push_str(&format!("G1 X{} Y{} S{s} F{feed}\n", n(to)?, n(y)?));
                        lit = true;
                    }
                }
                backwards = !backwards;
            }
        }
    }
    out.push_str("S0\nM5\nM2\n");
    Ok((out, sym))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::{verify, Limits};

    fn machine() -> Machine {
        Machine {
            s_max: 1000,
            max_power_percent: 100.0,
            max_speed_mm_min: 30_000.0,
            bed_mm: None,
        }
    }

    /// A reader of a symbol written for the tests: it finds the level and the mask in the format information, takes the mask off, reads the codewords in the zigzag, checks the
    /// error-correction codewords of every block and returns the bytes of the byte mode.
    fn read(sym: &Symbol) -> Result<Vec<u8>, String> {
        let size = sym.size;
        let bit = |x: usize, y: usize| u32::from(sym.dark(x, y));
        // the first copy of the format information
        let mut bits = 0u32;
        for i in 0..=5 {
            bits |= bit(8, i) << i;
        }
        bits |= bit(8, 7) << 6;
        bits |= bit(8, 8) << 7;
        bits |= bit(7, 8) << 8;
        for i in 9..15 {
            bits |= bit(14 - i, 8) << i;
        }
        // the second copy says the same
        let mut second = 0u32;
        for i in 0..8 {
            second |= bit(size - 1 - i, 8) << i;
        }
        for i in 8..15 {
            second |= bit(8, size - 15 + i) << i;
        }
        if bits != second {
            return Err("the two copies of the format information differ".to_string());
        }
        let (ecc, mask) = [QrEcc::L, QrEcc::M, QrEcc::Q, QrEcc::H]
            .into_iter()
            .flat_map(|e| (0..8).map(move |m| (e, m)))
            .find(|(e, m)| format_bits(*e, *m) == bits)
            .ok_or("the format information is not one of the 32")?;
        let function = function_map(sym.version);
        let mut words: Vec<u8> = Vec::new();
        let (mut cur, mut count) = (0u8, 0);
        let mut right = size as i32 - 1;
        while right >= 1 {
            if right == 6 {
                right = 5;
            }
            for vert in 0..size {
                for j in 0..2 {
                    let x = (right - j) as usize;
                    let upward = ((right + 1) & 2) == 0;
                    let y = if upward { size - 1 - vert } else { vert };
                    if function[y * size + x] {
                        continue;
                    }
                    let v = sym.dark(x, y) ^ masked(mask, x, y);
                    cur = (cur << 1) | u8::from(v);
                    count += 1;
                    if count == 8 {
                        words.push(cur);
                        cur = 0;
                        count = 0;
                    }
                }
            }
            right -= 2;
        }
        let total = TOTAL_CODEWORDS[sym.version as usize - 1];
        words.truncate(total);
        let (ec, blocks) = BLOCKS[sym.version as usize - 1][level_index(ecc)];
        let each = (total - ec * blocks) / blocks;
        let f = Field::new();
        let mut stream = Vec::new();
        for b in 0..blocks {
            let data: Vec<u8> = (0..each).map(|i| words[i * blocks + b]).collect();
            let rem: Vec<u8> = (0..ec)
                .map(|i| words[each * blocks + i * blocks + b])
                .collect();
            if reed_solomon(&f, &data, ec) != rem {
                return Err(format!("the codewords of block {b} do not agree"));
            }
            // every codeword of a block is a root-free multiple: the syndromes are zero
            let mut all = data.clone();
            all.extend(&rem);
            let mut root = 1u8;
            for _ in 0..ec {
                let mut acc = 0u8;
                for &c in &all {
                    acc = f.mul(acc, root) ^ c;
                }
                if acc != 0 {
                    return Err("a syndrome is not zero".to_string());
                }
                root = f.mul(root, 2);
            }
            stream.extend(data);
        }
        if stream[0] >> 4 != 0b0100 {
            return Err("the mode is not byte".to_string());
        }
        let len = (usize::from(stream[0] & 0xF) << 4) | usize::from(stream[1] >> 4);
        let bytes = (0..len)
            .map(|i| (stream[1 + i] << 4) | (stream[2 + i] >> 4))
            .collect();
        Ok(bytes)
    }

    #[test]
    fn the_format_information_is_the_one_of_the_standard() {
        // the known values for mask 0 at each level
        assert_eq!(format_bits(QrEcc::L, 0), 0b111011111000100);
        assert_eq!(format_bits(QrEcc::M, 0), 0b101010000010010);
        assert_eq!(format_bits(QrEcc::Q, 0), 0b011010101011111);
        assert_eq!(format_bits(QrEcc::H, 0), 0b001011010001001);
        // the 32 are different, 15 bits long, and differ in at least 7 bits from each other (the distance of the code)
        let all: Vec<u32> = [QrEcc::L, QrEcc::M, QrEcc::Q, QrEcc::H]
            .into_iter()
            .flat_map(|e| (0..8).map(move |m| format_bits(e, m)))
            .collect();
        for (i, a) in all.iter().enumerate() {
            assert!(*a < 1 << 15);
            for b in &all[i + 1..] {
                assert!((a ^ b).count_ones() >= 7);
            }
        }
    }

    #[test]
    fn the_error_correction_codewords_are_the_ones_of_the_standard() {
        let f = Field::new();
        // the field: a^8 is the polynomial of the standard, the logarithms invert the powers
        assert_eq!(f.exp[8], 0x1D);
        for x in 1..=255u32 {
            assert_eq!(f.exp[f.log[x as usize] as usize], x as u8);
        }
        assert_eq!(f.mul(0, 7), 0);
        assert_eq!(f.mul(2, 0x80), 0x1D);
        // the worked examples of the standard and of its commentaries (version 1 at level M)
        let numeric = [
            16u8, 32, 12, 86, 97, 128, 236, 17, 236, 17, 236, 17, 236, 17, 236, 17,
        ];
        assert_eq!(
            reed_solomon(&f, &numeric, 10),
            vec![165, 36, 212, 193, 237, 54, 199, 135, 44, 85]
        );
        let hello = [
            32u8, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236, 17, 236, 17,
        ];
        assert_eq!(
            reed_solomon(&f, &hello, 10),
            vec![196, 35, 39, 119, 235, 215, 231, 226, 93, 23]
        );
    }

    #[test]
    fn the_capacity_of_each_version_and_level_is_the_one_of_the_standard() {
        // bytes in byte mode: L, M, Q, H
        for (version, caps) in [
            (1, [17, 14, 11, 7]),
            (2, [32, 26, 20, 14]),
            (3, [53, 42, 32, 24]),
            (4, [78, 62, 46, 34]),
        ] {
            for (e, cap) in [QrEcc::L, QrEcc::M, QrEcc::Q, QrEcc::H]
                .into_iter()
                .zip(caps)
            {
                assert_eq!(capacity_bytes(version, e), cap, "{version} {e:?}");
                // the codewords add up to the whole symbol
                let (ec, blocks) = BLOCKS[version as usize - 1][level_index(e)];
                assert_eq!(
                    data_codewords(version, e) + ec * blocks,
                    TOTAL_CODEWORDS[version as usize - 1]
                );
            }
        }
        assert_eq!((side(1), side(2), side(3), side(4)), (21, 25, 29, 33));
    }

    #[test]
    fn the_fixed_patterns_are_where_the_standard_puts_them() {
        for v in 1..=4 {
            let s = encode(b"A", QrEcc::M, v).unwrap();
            assert_eq!(s.version, 1);
        }
        // a symbol of version 2 and up is made when the data needs it
        let s = encode(&[b'x'; 15], QrEcc::M, 4).unwrap();
        assert_eq!((s.version, s.size), (2, 25));
        let n = s.size;
        // the finder patterns: a dark ring of 7, a light ring, a dark centre of 3
        for (cx, cy) in [(3, 3), (n - 4, 3), (3, n - 4)] {
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let d = dx.abs().max(dy.abs());
                    let want = d != 2;
                    let (x, y) = ((cx as i32 + dx) as usize, (cy as i32 + dy) as usize);
                    assert_eq!(s.dark(x, y), want, "{x} {y}");
                }
            }
        }
        // the timing lines alternate between the finders
        for i in 8..n - 8 {
            assert_eq!(s.dark(i, 6), i % 2 == 0);
            assert_eq!(s.dark(6, i), i % 2 == 0);
        }
        // the dark module, and the alignment pattern of version 2 centred at 18, 18
        assert!(s.dark(8, n - 8));
        assert!(s.dark(18, 18) && !s.dark(17, 17) && s.dark(16, 16) && s.dark(20, 20));
        // outside of the symbol is light
        assert!(!s.dark(n, 0) && !s.dark(0, n));
    }

    #[test]
    fn every_version_level_and_length_reads_back_what_was_put_in() {
        let all = [QrEcc::L, QrEcc::M, QrEcc::Q, QrEcc::H];
        for e in all {
            for version in 1..=4u32 {
                let cap = capacity_bytes(version, e);
                // one byte, half and the full capacity; the shorter ones may fit in a smaller version
                for len in [1, cap / 2 + 1, cap] {
                    let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
                    let s = encode(&data, e, version).unwrap();
                    assert!(s.version <= version);
                    assert_eq!(read(&s).unwrap(), data, "{e:?} v{version} {len}");
                    // a damaged module is found by the check (an error codeword no longer agrees)
                    let mut bad = s.clone();
                    let i = (bad.size - 1) * bad.size + bad.size - 1;
                    bad.modules[i] = !bad.modules[i];
                    assert!(read(&bad).is_err());
                }
            }
        }
        // the bytes of a text of many alphabets are kept as they are
        let text = "Phoenix \u{3b1}\u{3b2} \u{4e2d}";
        let s = encode(text.as_bytes(), QrEcc::M, 4).unwrap();
        assert_eq!(String::from_utf8(read(&s).unwrap()).unwrap(), text);
    }

    #[test]
    fn the_same_data_gives_the_same_symbol_and_the_mask_is_the_one_of_the_lowest_penalty() {
        let a = encode(b"HTTPS://EH3D.EXAMPLE/1", QrEcc::M, 4).unwrap();
        let b = encode(b"HTTPS://EH3D.EXAMPLE/1", QrEcc::M, 4).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, encode(b"HTTPS://EH3D.EXAMPLE/2", QrEcc::M, 4).unwrap());
        // no other mask is better
        let words = codewords(
            &data_stream(b"HTTPS://EH3D.EXAMPLE/1", a.version, a.ecc),
            a.version,
            a.ecc,
        );
        assert_eq!(words.len(), TOTAL_CODEWORDS[a.version as usize - 1]);
        let p = penalty(&a.modules, a.size);
        for mask in 0..8u32 {
            let mut m = a.modules.clone();
            // take the chosen mask off and put this one on
            let function = function_map(a.version);
            for y in 0..a.size {
                for x in 0..a.size {
                    if !function[y * a.size + x] && masked(a.mask, x, y) != masked(mask, x, y) {
                        m[y * a.size + x] = !m[y * a.size + x];
                    }
                }
            }
            draw_format(&mut m, a.size, format_bits(a.ecc, mask));
            assert!(penalty(&m, a.size) >= p, "mask {mask}");
        }
    }

    #[test]
    fn data_that_does_not_fit_is_refused_and_says_what_it_would_need() {
        assert!(encode(&[], QrEcc::M, 4).unwrap_err().contains("nothing"));
        assert!(encode(b"A", QrEcc::M, 0).is_err());
        assert!(encode(b"A", QrEcc::M, 5).is_err());
        // 15 bytes do not fit in version 1 at level M (14), and fit in version 2
        let e = encode(&[b'a'; 15], QrEcc::M, 1).unwrap_err();
        assert!(
            e.contains("needs version 2") && e.contains("max_version"),
            "{e}"
        );
        assert!(encode(&[b'a'; 14], QrEcc::M, 1).is_ok());
        // more than version 4 holds
        let e = encode(&[b'a'; 63], QrEcc::M, 4).unwrap_err();
        assert!(e.contains("largest symbol") && e.contains("62"), "{e}");
        assert!(encode(&[b'a'; 62], QrEcc::M, 4).is_ok());
        assert!(encode(&[b'a'; 79], QrEcc::L, 4).is_err());
        assert!(encode(&[b'a'; 78], QrEcc::L, 4).is_ok());
    }

    fn settings() -> QrSettings {
        QrSettings::default()
    }

    #[test]
    fn the_program_fills_every_dark_module_at_the_power_and_speed_asked_and_passes_the_check() {
        let q = QrSettings {
            power_percent: 25.0,
            speed_mm_min: 1200.0,
            module_mm: 1.0,
            line_mm: 0.25,
            ..settings()
        };
        let (g, sym) = program("EH3D 42", (5.0, 5.0), &q, &machine()).unwrap();
        assert!(g.starts_with("G21\nG90\nM4 S0\n"));
        assert!(g.ends_with("S0\nM5\nM2\n"));
        assert!(g.contains("S250 F1200"), "{g}");
        // the burnt length is the area of the dark modules over the distance between the lines
        let dark = (0..sym.size)
            .flat_map(|y| (0..sym.size).map(move |x| (x, y)))
            .filter(|&(x, y)| sym.dark(x, y))
            .count();
        let lim = Limits {
            bed_mm: Some((100.0, 50.0)),
            ..Limits::default()
        };
        let v = verify(g.as_bytes(), &lim).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
        assert!(
            (v.burn_mm - dark as f64 * 1.0 / 0.25 * 1.0).abs() < 1e-6,
            "{} {dark}",
            v.burn_mm
        );
        // the box: only the far corner says where the symbol ends
        let (_, (x1, y1)) = v.bounds.unwrap();
        let w = sym.size as f64;
        // the last line of the top row is half a distance between lines under the edge
        assert!((x1 - (5.0 + w)).abs() < 1e-3 && (y1 - (5.0 + w - 0.125)).abs() < 1e-3);
        // two passes burn twice the length
        let two = QrSettings { passes: 2, ..q };
        let (g2, _) = program("EH3D 42", (5.0, 5.0), &two, &machine()).unwrap();
        let v2 = verify(g2.as_bytes(), &lim).unwrap();
        assert!((v2.burn_mm - 2.0 * v.burn_mm).abs() < 1e-3);
        // nothing moves fast with the laser on: every rapid move comes after S0
        let mut lit = false;
        for line in g.lines() {
            if line.starts_with("G1") && line.contains(" S") {
                lit = true;
            } else if line.starts_with("S0") {
                lit = false;
            } else if line.starts_with("G0") {
                assert!(!lit, "{line}");
            }
        }
    }

    #[test]
    fn a_power_a_speed_a_size_or_a_bed_over_the_limit_is_refused() {
        let q = settings();
        let weak = Machine {
            max_power_percent: 10.0,
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &q, &weak)
            .unwrap_err()
            .contains("the power of the QR code"));
        let slow = Machine {
            max_speed_mm_min: 500.0,
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &q, &slow)
            .unwrap_err()
            .contains("the speed of the QR code"));
        // too many bytes for the settings, and a symbol larger than the size they allow
        let few = QrSettings { max_bytes: 3, ..q };
        assert!(program("ABCD", (0.0, 0.0), &few, &machine())
            .unwrap_err()
            .contains("max_bytes"));
        assert!(program("ABC", (0.0, 0.0), &few, &machine()).is_ok());
        let small = QrSettings {
            max_size_mm: 20.0,
            ..q
        };
        assert!(program("A", (0.0, 0.0), &small, &machine())
            .unwrap_err()
            .contains("max_size_mm"));
        let exact = QrSettings {
            max_size_mm: 21.0,
            ..q
        };
        assert!(program("A", (0.0, 0.0), &exact, &machine()).is_ok());
        // the version the settings allow
        let v1 = QrSettings {
            max_version: 1,
            ..q
        };
        assert!(program(&"a".repeat(15), (0.0, 0.0), &v1, &machine())
            .unwrap_err()
            .contains("max_version"));
        // a fill of too many moves
        let dense = QrSettings { max_lines: 10, ..q };
        assert!(program("A", (0.0, 0.0), &dense, &machine())
            .unwrap_err()
            .contains("max_lines"));
        // the bed
        let bed = Machine {
            bed_mm: Some((25.0, 25.0)),
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &q, &bed).is_ok());
        assert!(program("A", (5.0, 0.0), &q, &bed)
            .unwrap_err()
            .contains("outside of the bed"));
        assert!(program("A", (-1.0, 0.0), &q, &bed).is_err());
        assert!(program("A", (f64::NAN, 0.0), &q, &bed).is_err());
        assert!(program("", (0.0, 0.0), &q, &machine()).is_err());
        let zero = Machine {
            s_max: 0,
            ..machine()
        };
        assert!(program("A", (0.0, 0.0), &q, &zero).is_err());
        // a setting out of its range
        let bad = QrSettings { line_mm: 2.0, ..q };
        assert!(program("A", (0.0, 0.0), &bad, &machine()).is_err());
    }

    #[test]
    fn an_inverted_symbol_burns_the_light_modules_and_the_two_make_the_whole_square() {
        let q = QrSettings {
            line_mm: 0.25,
            ..settings()
        };
        let inv = QrSettings { invert: true, ..q };
        let (plain, sym) = program("EH3D 42", (0.0, 0.0), &q, &machine()).unwrap();
        let (flipped, _) = program("EH3D 42", (0.0, 0.0), &inv, &machine()).unwrap();
        assert!(!plain.contains("inverted"));
        assert!(flipped.contains("(qr code of version 1, 21 x 21 modules, 1 passes, inverted)"));
        let lim = Limits {
            bed_mm: Some((100.0, 50.0)),
            ..Limits::default()
        };
        let a = verify(plain.as_bytes(), &lim).unwrap();
        let b = verify(flipped.as_bytes(), &lim).unwrap();
        assert!(
            a.passes() && b.passes(),
            "{:?} {:?}",
            a.problems,
            b.problems
        );
        // dark plus light is every module of the square, each filled with module / line lines
        let all = (sym.size * sym.size) as f64 * 1.0 / 0.25;
        assert!(
            (a.burn_mm + b.burn_mm - all).abs() < 1e-6,
            "{} {} {all}",
            a.burn_mm,
            b.burn_mm
        );
        let dark = (0..sym.size)
            .flat_map(|y| (0..sym.size).map(move |x| (x, y)))
            .filter(|&(x, y)| sym.dark(x, y))
            .count() as f64;
        assert!((b.burn_mm - (all - dark / 0.25)).abs() < 1e-6);
        // the limit of moves holds over the light modules too, and a rapid move never comes with the power set
        let dense = QrSettings {
            max_lines: 10,
            invert: true,
            ..q
        };
        assert!(program("A", (0.0, 0.0), &dense, &machine())
            .unwrap_err()
            .contains("max_lines"));
        let mut lit = false;
        for line in flipped.lines() {
            if line.starts_with("G1") && line.contains(" S") {
                lit = true;
            } else if line.starts_with("S0") {
                lit = false;
            } else if line.starts_with("G0") {
                assert!(!lit, "{line}");
            }
        }
    }

    #[test]
    fn the_overscan_runs_the_head_up_to_each_line_and_out_past_it_with_the_laser_off() {
        let q = QrSettings {
            line_mm: 0.5,
            overscan_mm: 2.0,
            ..settings()
        };
        let plain = QrSettings {
            overscan_mm: 0.0,
            ..q
        };
        let (g, _) = program("EH3D 42", (5.0, 5.0), &q, &machine()).unwrap();
        let (p, _) = program("EH3D 42", (5.0, 5.0), &plain, &machine()).unwrap();
        let x_of = |line: &str| -> f64 {
            line.split(' ')
                .find_map(|w| w.strip_prefix('X'))
                .unwrap()
                .parse()
                .unwrap()
        };
        let rows: Vec<&str> = g.lines().collect();
        let mut burning = 0;
        for (i, line) in rows.iter().enumerate() {
            if line.starts_with("G1") && !line.contains(" S0 ") {
                burning += 1;
                // before it, the run-up with the laser off, 2 mm long, on the line of its start
                let before = rows[i - 1];
                let after = rows[i + 1];
                assert!(
                    before.starts_with("G1") && before.contains(" S0 "),
                    "{before}"
                );
                assert!(after.starts_with("G1") && after.contains(" S0 "), "{after}");
                let (a, b) = (x_of(rows[i - 2]), x_of(before));
                assert!(((b - a).abs() - 2.0).abs() < 1e-9, "{a} {b}");
                let (end, out) = (x_of(line), x_of(after));
                assert!(((out - end).abs() - 2.0).abs() < 1e-9, "{end} {out}");
                // the run-out goes on the way the line went
                assert_eq!((out - end).signum(), (end - b).signum());
            }
        }
        // the same lines are burnt, and nothing that moves fast has the power set
        assert_eq!(burning, p.lines().filter(|l| l.starts_with("G1")).count());
        let lim = Limits::default();
        let (v, w) = (
            verify(g.as_bytes(), &lim).unwrap(),
            verify(p.as_bytes(), &lim).unwrap(),
        );
        assert!(v.passes(), "{:?}", v.problems);
        assert!((v.burn_mm - w.burn_mm).abs() < 1e-6);
        assert!(v.travel_mm > w.travel_mm);
        let mut lit = false;
        for line in g.lines() {
            if line.starts_with("G1") && !line.contains(" S0 ") && line.contains(" S") {
                lit = true;
            } else if line.starts_with("S0") || line.contains(" S0 ") {
                lit = false;
            } else if line.starts_with("G0") {
                assert!(!lit, "{line}");
            }
        }
        // two passes and an inverted symbol keep the same run-up
        let both = QrSettings {
            passes: 2,
            invert: true,
            ..q
        };
        let (g2, _) = program("EH3D 42", (5.0, 5.0), &both, &machine()).unwrap();
        assert!(verify(g2.as_bytes(), &lim).unwrap().passes());
    }

    #[test]
    fn the_run_up_must_stay_inside_the_bed_and_inside_its_range() {
        let q = QrSettings {
            overscan_mm: 2.0,
            ..settings()
        };
        let bed = |x: f64| Machine {
            bed_mm: Some((x, 50.0)),
            ..machine()
        };
        // the first module of a row touches the left edge: 2 mm before it is outside of the bed
        let e = program("A", (0.0, 0.0), &q, &bed(50.0)).unwrap_err();
        assert!(
            e.contains("overscan_mm") && e.contains("outside of the bed"),
            "{e}"
        );
        assert!(program("A", (2.0, 0.0), &q, &bed(50.0)).is_ok());
        // and past the right end
        let e = program("A", (2.0, 0.0), &q, &bed(24.0)).unwrap_err();
        assert!(e.contains("overscan_mm"), "{e}");
        assert!(program("A", (2.0, 0.0), &q, &bed(25.0)).is_ok());
        // without a run-up the same place is fine, and with no bed it is never refused
        let none = QrSettings {
            overscan_mm: 0.0,
            ..q
        };
        assert!(program("A", (0.0, 0.0), &none, &bed(21.0)).is_ok());
        assert!(program("A", (0.0, 0.0), &q, &machine()).is_ok());
        // the range of the setting
        for bad in [-0.1, 20.5, f64::NAN, f64::INFINITY] {
            let b = QrSettings {
                overscan_mm: bad,
                ..q
            };
            assert!(program("A", (5.0, 5.0), &b, &machine()).is_err(), "{bad}");
        }
        let top = QrSettings {
            overscan_mm: 20.0,
            ..q
        };
        assert!(program("A", (25.0, 5.0), &top, &machine()).is_ok());
    }

    #[test]
    fn a_mirrored_symbol_burns_the_same_runs_turned_about_its_vertical_axis() {
        // one line to a module, so each burnt line is one run of one row
        let q = QrSettings {
            line_mm: 1.0,
            ..settings()
        };
        let flipped = QrSettings { mirror: true, ..q };
        let (a, sym) = program("EH3D 42", (3.0, 4.0), &q, &machine()).unwrap();
        let (b, _) = program("EH3D 42", (3.0, 4.0), &flipped, &machine()).unwrap();
        assert!(b.contains(", mirrored)") && !a.contains("mirrored"));
        let runs = |g: &str| -> Vec<(i64, i64, i64)> {
            let mut out = Vec::new();
            let mut from = (0.0, 0.0);
            for line in g.lines() {
                let num = |axis: char| -> f64 {
                    line.split(' ')
                        .find_map(|w| w.strip_prefix(axis))
                        .unwrap()
                        .parse()
                        .unwrap()
                };
                if line.starts_with("G0") {
                    from = (num('X'), num('Y'));
                } else if line.starts_with("G1") {
                    let to = num('X');
                    let (lo, hi) = (from.0.min(to), from.0.max(to));
                    out.push((
                        (from.1 * 10.0) as i64,
                        (lo * 10.0) as i64,
                        (hi * 10.0) as i64,
                    ));
                }
            }
            out.sort_unstable();
            out
        };
        let w = (sym.size as f64 * 10.0) as i64;
        let left = 30;
        let mut expect: Vec<(i64, i64, i64)> = runs(&a)
            .into_iter()
            .map(|(y, lo, hi)| (y, left + w - (hi - left), left + w - (lo - left)))
            .collect();
        expect.sort_unstable();
        assert_eq!(runs(&b), expect);
        // the same length is burnt, the symbol inside the same square, and it can be inverted too
        let lim = Limits {
            bed_mm: Some((100.0, 50.0)),
            ..Limits::default()
        };
        let (va, vb) = (
            verify(a.as_bytes(), &lim).unwrap(),
            verify(b.as_bytes(), &lim).unwrap(),
        );
        assert!(vb.passes(), "{:?}", vb.problems);
        assert!((va.burn_mm - vb.burn_mm).abs() < 1e-6);
        let both = QrSettings {
            invert: true,
            ..flipped
        };
        let (c, _) = program("EH3D 42", (3.0, 4.0), &both, &machine()).unwrap();
        assert!(c.contains(", inverted, mirrored)"));
        assert!(verify(c.as_bytes(), &lim).unwrap().passes());
    }

    #[test]
    fn a_burning_that_would_take_longer_than_the_limit_is_refused_never_shortened() {
        let q = settings();
        let (g, _) = program("EH3D 42", (0.0, 0.0), &q, &machine()).unwrap();
        let lim = Limits::default();
        let v = verify(g.as_bytes(), &lim).unwrap();
        // the time of the burning is its length over the speed
        let minutes = v.burn_mm / q.speed_mm_min;
        let just = QrSettings {
            max_minutes: 1.0f64.max(minutes + 0.5),
            ..q
        };
        assert!(program("EH3D 42", (0.0, 0.0), &just, &machine()).is_ok());
        // at a speed so low that it takes hours, or with passes that add up, it is refused and says how long
        let slow = QrSettings {
            speed_mm_min: 2.0,
            ..q
        };
        let e = program("EH3D 42", (0.0, 0.0), &slow, &machine()).unwrap_err();
        assert!(e.contains("max_minutes") && e.contains("minutes"), "{e}");
        let short = QrSettings {
            max_minutes: 1.0,
            passes: 20,
            ..q
        };
        assert!(program("EH3D 42", (0.0, 0.0), &short, &machine())
            .unwrap_err()
            .contains("max_minutes"));
        // a higher limit lets the slow one through, and a limit out of range is an error of the settings
        let patient = QrSettings {
            max_minutes: 1440.0,
            ..slow
        };
        assert!(program("EH3D 42", (0.0, 0.0), &patient, &machine()).is_ok());
        for bad in [0.0, 0.9, 1441.0, f64::NAN] {
            let b = QrSettings {
                max_minutes: bad,
                ..q
            };
            assert!(program("A", (0.0, 0.0), &b, &machine()).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_border_is_burnt_around_an_inverted_symbol_and_counts_for_the_size_and_the_bed() {
        let q = QrSettings {
            line_mm: 0.25,
            invert: true,
            border_modules: 2,
            ..settings()
        };
        let (g, sym) = program("EH3D 42", (5.0, 5.0), &q, &machine()).unwrap();
        assert_eq!(side_modules(&sym, &q), sym.size + 4);
        assert!(g.contains(", inverted, border of 2 modules)"), "{g}");
        // the burnt area is the whole square but the dark modules of the symbol
        let dark = (0..sym.size)
            .flat_map(|y| (0..sym.size).map(move |x| (x, y)))
            .filter(|&(x, y)| sym.dark(x, y))
            .count();
        let side = sym.size + 4;
        let lim = Limits {
            bed_mm: Some((100.0, 50.0)),
            ..Limits::default()
        };
        let v = verify(g.as_bytes(), &lim).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
        assert!(
            (v.burn_mm - ((side * side - dark) as f64) * 4.0).abs() < 1e-6,
            "{} {side} {dark}",
            v.burn_mm
        );
        // the first lines burn the whole bottom row of the border in one run
        let first = g.lines().find(|l| l.starts_with("G1")).unwrap();
        assert!(first.starts_with(&format!("G1 X{} ", 5 + side)), "{first}");
        // without the border the same inverted symbol burns less, and a border is refused when it is not inverted
        let none = QrSettings {
            border_modules: 0,
            ..q
        };
        let (h, _) = program("EH3D 42", (5.0, 5.0), &none, &machine()).unwrap();
        assert!(verify(h.as_bytes(), &lim).unwrap().burn_mm < v.burn_mm);
        let plain = QrSettings { invert: false, ..q };
        let e = program("EH3D 42", (5.0, 5.0), &plain, &machine()).unwrap_err();
        assert!(e.contains("border_modules") && e.contains("invert"), "{e}");
        // the border counts for the largest side and for the bed, and its width has a limit
        let wide = (side as f64) * 1.0;
        let small = QrSettings {
            max_size_mm: wide - 0.5,
            ..q
        };
        assert!(program("EH3D 42", (5.0, 5.0), &small, &machine())
            .unwrap_err()
            .contains("max_size_mm"));
        let bed = |x: f64| Machine {
            bed_mm: Some((x, 50.0)),
            ..machine()
        };
        assert!(program("EH3D 42", (5.0, 5.0), &q, &bed(5.0 + wide)).is_ok());
        assert!(program("EH3D 42", (5.0, 5.0), &q, &bed(4.0 + wide))
            .unwrap_err()
            .contains("outside of the bed"));
        let over = QrSettings {
            border_modules: 9,
            ..q
        };
        assert!(program("EH3D 42", (5.0, 5.0), &over, &machine()).is_err());
        let most = QrSettings {
            border_modules: 8,
            ..q
        };
        assert!(program("EH3D 42", (20.0, 5.0), &most, &machine()).is_ok());
        // the moves of the border count for the limit of moves
        let dense = QrSettings { max_lines: 50, ..q };
        assert!(program("EH3D 42", (5.0, 5.0), &dense, &machine())
            .unwrap_err()
            .contains("max_lines"));
    }

    #[test]
    fn the_symbol_can_be_shown_as_text() {
        let s = encode(b"A", QrEcc::L, 1).unwrap();
        let t = s.to_text();
        assert_eq!(t.lines().count(), 21);
        assert!(t.lines().all(|l| l.chars().count() == 42));
        // the top left finder pattern: seven dark modules, then a light one
        assert!(t.starts_with("##############  "), "{t}");
        assert_eq!(
            t.matches("##").count(),
            (0..21)
                .flat_map(|y| (0..21).map(move |x| (x, y)))
                .filter(|&(x, y)| s.dark(x, y))
                .count()
        );
    }

    #[test]
    fn the_data_is_never_written_into_a_comment_of_the_program() {
        // a parenthesis or a line break in the data would end a comment and let the rest through
        let (g, _) = program("A) (B\nM3", (0.0, 0.0), &settings(), &machine()).unwrap();
        for line in g.lines().filter(|l| l.starts_with('(')) {
            assert!(
                line.starts_with("(qr code of ") && line.ends_with(')'),
                "{line}"
            );
            assert_eq!(line.matches(')').count(), 1);
        }
        assert!(!g.contains("M3"));
    }
}
