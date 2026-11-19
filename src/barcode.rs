// EH3D-PHOENIX-LASER - a Code 128 barcode burnt with lines.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A Code 128 barcode of the program's own, so that a serial number or a short code can be burnt with no picture. The characters from space to `~` are written in set B, and a text made only of
//! an even number of figures is written in set C (two figures to a symbol), which is half as wide. Each bar is filled with lines along its height. Data that is not a printable character, more
//! characters than the settings allow, a barcode wider than the size the settings allow, a power or a speed over the limit or work outside of the bed is refused, never cut down.

use crate::gcode_out::number as n;
use crate::qr::Machine;
use crate::vconfig::BarcodeSettings;

/// The widths of the bars and spaces of each symbol, from 0 to 105 (the 103 characters and the three start symbols) and the stop symbol (106), as the digits of the width of each element in
/// modules; they begin with a bar and every symbol of six elements is 11 modules wide.
const PATTERNS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];

/// The symbols that start a barcode in set B and in set C, and the one that stops it.
const START_B: u32 = 104;
const START_C: u32 = 105;
const STOP: u32 = 106;

/// A barcode once it is encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Barcode {
    /// The symbols: the start, the characters, the check symbol and the stop.
    pub codes: Vec<u32>,
    /// Whether the characters are in set C (pairs of figures) and not in set B.
    pub set_c: bool,
    /// The modules from left to right, `true` for a bar. The quiet zones are not in it.
    pub modules: Vec<bool>,
}

impl Barcode {
    /// The set of the characters, `B` or `C`.
    pub fn set(&self) -> char {
        if self.set_c {
            'C'
        } else {
            'B'
        }
    }
}

/// The symbols of some data and the modules of the bars. Data of no byte or with a byte that is not a printable character (from space to `~`) is refused.
pub fn encode(data: &[u8]) -> Result<Barcode, String> {
    if data.is_empty() {
        return Err("the data of the barcode is empty".to_string());
    }
    if let Some((i, b)) = data
        .iter()
        .enumerate()
        .find(|(_, b)| !(32..=126).contains(*b))
    {
        return Err(format!(
            "byte {} of the data is {b}: code 128 holds the characters from space to ~",
            i + 1
        ));
    }
    let set_c = data.len().is_multiple_of(2) && data.iter().all(u8::is_ascii_digit);
    let mut codes = vec![if set_c { START_C } else { START_B }];
    if set_c {
        for pair in data.chunks(2) {
            codes.push(u32::from((pair[0] - b'0') * 10 + (pair[1] - b'0')));
        }
    } else {
        codes.extend(data.iter().map(|&b| u32::from(b - 32)));
    }
    let sum: u32 = codes[0]
        + codes[1..]
            .iter()
            .enumerate()
            .map(|(i, c)| (i as u32 + 1) * c)
            .sum::<u32>();
    codes.push(sum % 103);
    codes.push(STOP);
    let mut modules = Vec::new();
    for &c in &codes {
        let mut bar = true;
        for w in PATTERNS[c as usize].bytes() {
            modules.extend(std::iter::repeat_n(bar, usize::from(w - b'0')));
            bar = !bar;
        }
    }
    Ok(Barcode {
        codes,
        set_c,
        modules,
    })
}

/// The data that some modules hold: the symbols are found by their widths, the start sets the set, the check symbol and the stop are checked. A reader of what is written, to see that a barcode
/// says what it was asked to say.
pub fn decode(modules: &[bool]) -> Result<Vec<u8>, String> {
    let mut runs: Vec<(bool, usize)> = Vec::new();
    for &m in modules {
        match runs.last_mut() {
            Some((kind, len)) if *kind == m => *len += 1,
            _ => runs.push((m, 1)),
        }
    }
    if modules.first() != Some(&true) || modules.last() != Some(&true) {
        return Err("a barcode starts and ends with a bar".to_string());
    }
    let widths: Vec<u8> = runs.iter().map(|r| r.1 as u8).collect();
    let mut codes: Vec<u32> = Vec::new();
    let mut at = 0;
    while at < widths.len() {
        let take = if widths.len() - at == 7 { 7 } else { 6 };
        if widths.len() - at < take {
            return Err("the bars end in the middle of a symbol".to_string());
        }
        let text: String = widths[at..at + take]
            .iter()
            .map(|w| char::from(b'0' + w))
            .collect();
        let code = PATTERNS
            .iter()
            .position(|p| *p == text)
            .ok_or_else(|| format!("the widths {text} are not a symbol"))?;
        codes.push(code as u32);
        at += take;
    }
    if codes.len() < 4 || codes.last() != Some(&STOP) {
        return Err("the barcode does not end with the stop symbol".to_string());
    }
    let set_c = match codes[0] {
        START_B => false,
        START_C => true,
        other => {
            return Err(format!(
                "the start symbol is {other}: only sets B and C are read"
            ))
        }
    };
    let body = &codes[1..codes.len() - 2];
    let check = codes[codes.len() - 2];
    let sum: u32 = codes[0]
        + body
            .iter()
            .enumerate()
            .map(|(i, c)| (i as u32 + 1) * c)
            .sum::<u32>();
    if sum % 103 != check {
        return Err("the check symbol does not agree".to_string());
    }
    let mut out = Vec::new();
    for &c in body {
        if set_c {
            if c > 99 {
                return Err(format!("the symbol {c} is not a pair of figures"));
            }
            out.push(b'0' + (c / 10) as u8);
            out.push(b'0' + (c % 10) as u8);
        } else {
            if c > 94 {
                return Err(format!("the symbol {c} is not a character of set B"));
            }
            out.push(c as u8 + 32);
        }
    }
    Ok(out)
}

/// The width of the barcode in modules: the symbols and the stop, with no quiet zone.
pub fn width_modules(code: &Barcode) -> usize {
    code.modules.len()
}

/// The G-code of a barcode whose lower left corner is at `origin`: each bar is filled with lines along its height `line_mm` apart, burnt at the power and the speed of the settings as many
/// times as `passes` says, with the laser off between the lines. Data over `max_chars` characters, a barcode wider than `max_width_mm`, a fill of more than `max_lines` moves, a burning of
/// more than `max_minutes`, a power or a speed over the limit, or a barcode outside of the bed is refused and nothing is written. The data is never written into a comment of the program.
pub fn program(
    data: &str,
    origin: (f64, f64),
    b: &BarcodeSettings,
    m: &Machine,
) -> Result<(String, Barcode), String> {
    b.validate()?;
    crate::safety::validate_max(m.max_power_percent)?;
    crate::safety::check_power(
        "the power of the barcode",
        b.power_percent,
        m.max_power_percent,
    )?;
    crate::safety::check_speed(
        "the speed of the barcode",
        b.speed_mm_min,
        m.max_speed_mm_min,
    )?;
    if m.s_max == 0 || m.s_max > 100_000 {
        return Err("s_max must be from 1 to 100000".to_string());
    }
    if !(origin.0.is_finite()
        && origin.1.is_finite()
        && origin.0.abs() <= 100_000.0
        && origin.1.abs() <= 100_000.0)
    {
        return Err("the origin of the barcode is not a number from -100000 to 100000".to_string());
    }
    let bytes = data.as_bytes();
    if bytes.len() > b.max_chars as usize {
        return Err(format!(
            "the data is {} characters: it can be at most {} (max_chars in [barcode])",
            bytes.len(),
            b.max_chars
        ));
    }
    let code = encode(bytes)?;
    let width = width_modules(&code) as f64 * b.module_mm;
    if width > b.max_width_mm + 1e-9 {
        return Err(format!(
            "the barcode is {width:.1} mm wide ({} modules of {} mm): it can be at most {} mm (max_width_mm in [barcode])",
            width_modules(&code),
            b.module_mm,
            b.max_width_mm
        ));
    }
    if let Some((bx, by)) = m.bed_mm {
        if origin.0 < 0.0
            || origin.1 < 0.0
            || origin.0 + width > bx + 1e-9
            || origin.1 + b.height_mm > by + 1e-9
        {
            return Err(format!(
                "the barcode is {width:.1} x {} mm and, from {}, {}, it goes outside of the bed of {bx} x {by} mm",
                b.height_mm, origin.0, origin.1
            ));
        }
    }
    // the bars from the left (a mirror image has the last module where the first one was)
    let modules: Vec<bool> = if b.mirror {
        code.modules.iter().rev().copied().collect()
    } else {
        code.modules.clone()
    };
    let mut bars: Vec<(usize, usize)> = Vec::new();
    let mut x = 0;
    while x < modules.len() {
        if modules[x] {
            let start = x;
            while x + 1 < modules.len() && modules[x + 1] {
                x += 1;
            }
            bars.push((start, x));
        }
        x += 1;
    }
    // the lines of each bar, along its height: their places across the barcode
    let mut lines: Vec<f64> = Vec::new();
    for &(a, z) in &bars {
        let wide = (z - a + 1) as f64 * b.module_mm;
        let count = (wide / b.line_mm - 1e-9).ceil().max(1.0) as usize;
        for k in 0..count {
            lines.push(origin.0 + a as f64 * b.module_mm + (k as f64 + 0.5) * wide / count as f64);
        }
    }
    let moves = lines.len() * b.passes as usize;
    if moves > b.max_lines as usize {
        return Err(format!(
            "the fill needs {moves} burning moves: it can have at most {} (max_lines in [barcode])",
            b.max_lines
        ));
    }
    let burn_mm = lines.len() as f64 * b.height_mm * f64::from(b.passes);
    let minutes = burn_mm / b.speed_mm_min;
    if minutes > b.max_minutes + 1e-9 {
        return Err(format!(
            "burning the barcode would take about {minutes:.1} minutes ({burn_mm:.0} mm at {} mm/min): it can take at most {} (max_minutes in [barcode])",
            b.speed_mm_min, b.max_minutes
        ));
    }
    let s = (f64::from(m.s_max) * b.power_percent / 100.0).round() as u32;
    if s == 0 {
        return Err(format!(
            "a power of {} % of S{} is less than one step of S",
            b.power_percent, m.s_max
        ));
    }
    let feed = n(b.speed_mm_min)?;
    let mut out = String::from("G21\nG90\nM4 S0\n");
    let mirrored = if b.mirror { ", mirrored" } else { "" };
    out.push_str(&format!(
        "(code 128 barcode in set {}, {} symbols, {} passes{mirrored})\n",
        code.set(),
        code.codes.len(),
        b.passes
    ));
    let (y0, y1) = (origin.1, origin.1 + b.height_mm);
    let mut lit = false;
    for _ in 0..b.passes {
        let mut up = true;
        for &x in &lines {
            let (from, to) = if up { (y0, y1) } else { (y1, y0) };
            // no rapid move with the power of the last line still set
            if lit {
                out.push_str("S0\n");
            }
            out.push_str(&format!("G0 X{} Y{}\n", n(x)?, n(from)?));
            out.push_str(&format!("G1 X{} Y{} S{s} F{feed}\n", n(x)?, n(to)?));
            lit = true;
            up = !up;
        }
    }
    out.push_str("S0\nM5\nM2\n");
    Ok((out, code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{analyse, read, Options};
    use crate::verify::{verify, Limits};

    fn machine() -> Machine {
        Machine {
            s_max: 1000,
            max_power_percent: 100.0,
            max_speed_mm_min: 30_000.0,
            bed_mm: None,
        }
    }

    #[test]
    fn every_symbol_is_eleven_modules_wide_and_the_table_has_no_symbol_twice() {
        for (i, p) in PATTERNS.iter().take(106).enumerate() {
            assert_eq!(p.len(), 6, "{i}");
            assert_eq!(
                p.bytes().map(|w| u32::from(w - b'0')).sum::<u32>(),
                11,
                "{i}"
            );
            // the widths of the bars of a symbol add up to an even number
            let bars: u32 = p.bytes().step_by(2).map(|w| u32::from(w - b'0')).sum();
            assert_eq!(bars % 2, 0, "{i} {p}");
            assert!(p.bytes().all(|w| (b'1'..=b'4').contains(&w)), "{i}");
        }
        assert_eq!(PATTERNS[106], "2331112");
        assert_eq!(
            PATTERNS[106]
                .bytes()
                .map(|w| u32::from(w - b'0'))
                .sum::<u32>(),
            13
        );
        let mut seen: Vec<&str> = PATTERNS.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 107);
    }

    #[test]
    fn a_text_is_written_in_set_b_with_the_check_symbol_of_the_standard() {
        // the symbols of a text in set B are the codes of the characters less 32
        let c = encode(b"PJJ123C").unwrap();
        assert_eq!(c.set(), 'B');
        assert_eq!(
            c.codes,
            vec![104, 48, 42, 42, 17, 18, 19, 35, 55, 106],
            "{:?}",
            c.codes
        );
        // 104 + 1*48 + 2*42 + 3*42 + 4*17 + 5*18 + 6*19 + 7*35 = 104 + 48 + 84 + 126 + 68 + 90 + 114 + 245 = 879; 879 % 103 = 55
        assert_eq!(c.modules.len(), 11 * 9 + 13);
        assert_eq!(decode(&c.modules).unwrap(), b"PJJ123C");
    }

    #[test]
    fn figures_in_pairs_go_in_set_c_and_are_half_as_wide() {
        let c = encode(b"123456").unwrap();
        assert_eq!(c.set(), 'C');
        assert_eq!(c.codes[..4], [105, 12, 34, 56]);
        assert_eq!(c.modules.len(), 11 * 5 + 13);
        assert_eq!(decode(&c.modules).unwrap(), b"123456");
        // an odd count of figures, or a figure with a letter, is written in set B
        assert_eq!(encode(b"12345").unwrap().set(), 'B');
        assert_eq!(encode(b"12AB").unwrap().set(), 'B');
        assert!(encode(b"12345").unwrap().modules.len() > c.modules.len() - 11);
    }

    #[test]
    fn everything_that_can_be_written_reads_back() {
        let all: Vec<u8> = (32u8..=126).collect();
        for data in [
            b" ".to_vec(),
            b"~".to_vec(),
            b"0".to_vec(),
            b"00".to_vec(),
            b"99".to_vec(),
            b"EH3D-42".to_vec(),
            all.clone(),
            all.iter().rev().copied().collect(),
        ] {
            let c = encode(&data).unwrap();
            assert_eq!(decode(&c.modules).unwrap(), data, "{data:?}");
        }
    }

    #[test]
    fn data_that_is_empty_or_is_not_a_printable_character_is_refused() {
        assert!(encode(b"").unwrap_err().contains("empty"));
        for bad in [&b"A\x1f"[..], b"\x7f", b"caf\xc3\xa9", b"a\nb", b"\0"] {
            let e = encode(bad).unwrap_err();
            assert!(e.contains("from space to ~"), "{e}");
        }
    }

    #[test]
    fn a_damaged_barcode_is_not_read() {
        let c = encode(b"EH3D").unwrap();
        let mut flipped = c.modules.clone();
        flipped[14] = !flipped[14];
        assert!(decode(&flipped).is_err());
        // a wrong check symbol: the symbol before the stop is replaced by another one
        let mut wrong = c.clone();
        let at = wrong.codes.len() - 2;
        wrong.codes[at] = (wrong.codes[at] + 1) % 103;
        let mut modules = Vec::new();
        for &code in &wrong.codes {
            let mut bar = true;
            for w in PATTERNS[code as usize].bytes() {
                modules.extend(std::iter::repeat_n(bar, usize::from(w - b'0')));
                bar = !bar;
            }
        }
        assert!(decode(&modules).unwrap_err().contains("check symbol"));
        assert!(decode(&[]).is_err());
        assert!(decode(&c.modules[..30]).is_err());
        assert!(decode(&[false, true]).is_err());
    }

    fn settings() -> BarcodeSettings {
        BarcodeSettings::default()
    }

    #[test]
    fn the_program_burns_the_bars_and_only_the_bars_and_passes_its_own_check() {
        let (g, code) = program("EH3D-42", (5.0, 7.0), &settings(), &machine()).unwrap();
        let p = read(&g);
        assert!(p.ended && p.notes.is_empty(), "{:?}", p.notes);
        let b = settings();
        // every burning move goes along the height of the bars, at the power of the settings
        for mv in p.moves.iter().filter(|m| m.power > 0.0) {
            assert_eq!(mv.from.0, mv.to.0);
            assert!((mv.length() - b.height_mm).abs() < 1e-6, "{mv:?}");
            assert_eq!(mv.power, 200.0);
        }
        // the lines are inside the dark modules, and every dark module has one
        let mut covered = vec![false; code.modules.len()];
        for mv in p.moves.iter().filter(|m| m.power > 0.0) {
            let module = ((mv.from.0 - 5.0) / b.module_mm).floor() as usize;
            assert!(code.modules[module], "a line in the light module {module}");
            covered[module] = true;
        }
        for (i, dark) in code.modules.iter().enumerate() {
            assert_eq!(*dark, covered[i], "module {i}");
        }
        let r = analyse(&p, &Options::default()).unwrap();
        let wide = code.modules.len() as f64 * b.module_mm;
        let (lo, hi) = r.burn_bounds.unwrap();
        assert!(lo.0 >= 5.0 && hi.0 <= 5.0 + wide && lo.1 == 7.0 && hi.1 == 7.0 + b.height_mm);
        let v = verify(g.as_bytes(), &Limits::default()).unwrap();
        assert!(v.passes(), "{:?}", v.problems);
        assert!(
            !g.contains("EH3D"),
            "the data is not written into the program"
        );
    }

    #[test]
    fn a_mirror_image_has_the_modules_the_other_way_round_and_the_same_length() {
        let plain = program("A1", (0.0, 0.0), &settings(), &machine())
            .unwrap()
            .0;
        let b = BarcodeSettings {
            mirror: true,
            ..settings()
        };
        let (mirror, code) = program("A1", (0.0, 0.0), &b, &machine()).unwrap();
        assert!(mirror.contains("mirrored"));
        let lines = |g: &str| {
            read(g)
                .moves
                .iter()
                .filter(|m| m.power > 0.0)
                .map(|m| m.from.0)
                .collect::<Vec<f64>>()
        };
        let (a, z) = (lines(&plain), lines(&mirror));
        assert_eq!(a.len(), z.len());
        let wide = code.modules.len() as f64 * b.module_mm;
        let mut flipped: Vec<f64> = a.iter().map(|x| wide - x).collect();
        flipped.sort_by(f64::total_cmp);
        let mut z = z;
        z.sort_by(f64::total_cmp);
        for (u, w) in flipped.iter().zip(&z) {
            assert!((u - w).abs() < 1e-6, "{u} {w}");
        }
    }

    #[test]
    fn what_goes_over_a_limit_is_refused_and_named() {
        let m = machine();
        let big = BarcodeSettings {
            max_chars: 3,
            ..settings()
        };
        assert!(program("ABCD", (0.0, 0.0), &big, &m)
            .unwrap_err()
            .contains("max_chars"));
        assert!(program("ABC", (0.0, 0.0), &big, &m).is_ok());
        let wide = BarcodeSettings {
            max_width_mm: 20.0,
            ..settings()
        };
        assert!(program("EH3D-42", (0.0, 0.0), &wide, &m)
            .unwrap_err()
            .contains("max_width_mm"));
        let dense = BarcodeSettings {
            max_lines: 10,
            ..settings()
        };
        assert!(program("EH3D", (0.0, 0.0), &dense, &m)
            .unwrap_err()
            .contains("max_lines"));
        let slow = BarcodeSettings {
            speed_mm_min: 1.0,
            ..settings()
        };
        assert!(program("EH3D", (0.0, 0.0), &slow, &m)
            .unwrap_err()
            .contains("max_minutes"));
        let low = Machine {
            max_power_percent: 10.0,
            ..m
        };
        assert!(program("EH3D", (0.0, 0.0), &settings(), &low)
            .unwrap_err()
            .contains("max_power_percent"));
        let slow_machine = Machine {
            max_speed_mm_min: 500.0,
            ..m
        };
        assert!(program("EH3D", (0.0, 0.0), &settings(), &slow_machine).is_err());
        let tiny = BarcodeSettings {
            power_percent: 0.01,
            ..settings()
        };
        assert!(program("EH3D", (0.0, 0.0), &tiny, &m)
            .unwrap_err()
            .contains("one step"));
        assert!(program("EH3D", (f64::NAN, 0.0), &settings(), &m).is_err());
        assert!(program("EH3D", (1e12, 0.0), &settings(), &m)
            .unwrap_err()
            .contains("-100000"));
        assert!(program("", (0.0, 0.0), &settings(), &m).is_err());
    }

    #[test]
    fn a_barcode_outside_of_the_bed_is_refused() {
        let on = |bed: (f64, f64), at: (f64, f64)| {
            program(
                "EH3D",
                at,
                &settings(),
                &Machine {
                    bed_mm: Some(bed),
                    ..machine()
                },
            )
        };
        assert!(on((100.0, 100.0), (0.0, 0.0)).is_ok());
        assert!(on((100.0, 100.0), (90.0, 0.0)).is_err());
        assert!(on((100.0, 100.0), (0.0, 95.0)).is_err());
        assert!(on((100.0, 100.0), (-1.0, 0.0)).is_err());
        assert!(on((20.0, 100.0), (0.0, 0.0)).is_err());
    }

    #[test]
    fn the_passes_repeat_the_lines_and_the_time_counts_them_all() {
        let one = program("EH3D", (0.0, 0.0), &settings(), &machine())
            .unwrap()
            .0;
        let b = BarcodeSettings {
            passes: 3,
            ..settings()
        };
        let three = program("EH3D", (0.0, 0.0), &b, &machine()).unwrap().0;
        let count = |g: &str| read(g).moves.iter().filter(|m| m.power > 0.0).count();
        assert_eq!(count(&three), 3 * count(&one));
        // the lines go up and down, so the head does not run back with the power set
        assert!(verify(three.as_bytes(), &Limits::default())
            .unwrap()
            .passes());
    }

    #[test]
    fn a_wider_line_spacing_burns_fewer_lines_and_a_value_out_of_range_is_refused() {
        let coarse = BarcodeSettings {
            line_mm: 0.4,
            ..settings()
        };
        let a = program("EH3D", (0.0, 0.0), &settings(), &machine())
            .unwrap()
            .0;
        let z = program("EH3D", (0.0, 0.0), &coarse, &machine()).unwrap().0;
        assert!(a.len() > z.len());
        for bad in [
            BarcodeSettings {
                module_mm: 0.05,
                ..settings()
            },
            BarcodeSettings {
                height_mm: 0.5,
                ..settings()
            },
            BarcodeSettings {
                line_mm: 1.0,
                ..settings()
            },
            BarcodeSettings {
                power_percent: 0.0,
                ..settings()
            },
            BarcodeSettings {
                passes: 0,
                ..settings()
            },
            BarcodeSettings {
                max_chars: 81,
                ..settings()
            },
            BarcodeSettings {
                max_minutes: 1441.0,
                ..settings()
            },
            BarcodeSettings {
                max_width_mm: f64::NAN,
                ..settings()
            },
        ] {
            assert!(
                program("EH3D", (0.0, 0.0), &bad, &machine()).is_err(),
                "{bad:?}"
            );
        }
    }
}
