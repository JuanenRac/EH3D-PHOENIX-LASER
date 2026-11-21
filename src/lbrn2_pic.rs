// EH3D-PHOENIX-LASER - the pictures inside an LBRN2 project.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! The pictures of an LBRN2 project (`Shape` of type `Bitmap`): the `Data` attribute holds the picture as base64 text, and in every real project that has one it is a PNG. The text is
//! decoded within the limits of `[limits]` (its length, the pixels the picture claims before anything is allocated, and how many pictures there are), the PNG is read by the reader of
//! pictures of the suite (a PGM too), and the picture is turned and mirrored as the transform of the shape places it, so that its first row is the top of the picture as it will be seen
//! on the machine. A picture that is rotated by something other than a quarter turn, that is of another format (a JPEG, say) or that is over a limit is refused with a stable code that
//! says why, and nothing of it is engraved.

use crate::vector::Limits;
use eh3d_image::Gray;

/// A picture longer (as text) than the limit.
pub const CODE_SIZE: &str = "PHOENIX-LASER-LBRN2-BITMAP-SIZE";
/// A picture with more pixels than the limit.
pub const CODE_PIXELS: &str = "PHOENIX-LASER-LBRN2-BITMAP-PIXELS";
/// A picture that is not in a format this reader knows.
pub const CODE_FORMAT: &str = "PHOENIX-LASER-LBRN2-BITMAP-FORMAT";
/// A picture whose text or image is damaged.
pub const CODE_DATA: &str = "PHOENIX-LASER-LBRN2-BITMAP-DATA";
/// A picture rotated by something other than a quarter turn.
pub const CODE_ROTATED: &str = "PHOENIX-LASER-LBRN2-BITMAP-ROTATED";
/// More pictures than the limit.
pub const CODE_COUNT: &str = "PHOENIX-LASER-LBRN2-BITMAP-COUNT";

/// A picture of a project, ready to be engraved.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    /// The layer of the picture (the same name the shapes of that layer use).
    pub layer: String,
    /// The width the project gives the picture, in millimetres.
    pub width_mm: f64,
    /// The height the project gives the picture, in millimetres.
    pub height_mm: f64,
    /// The grey levels, turned and mirrored so that the first row is the top as it will be seen on the machine.
    pub gray: Gray,
}

fn sextet(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

/// Decodes base64 text (the standard alphabet and the one for URLs; spaces and new lines are skipped; the padding is optional). The result is never longer than `max_bytes`, and the
/// text is checked for that before anything is allocated.
pub fn decode_base64(text: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    let digits = text
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .count();
    if digits / 4 * 3 > max_bytes.saturating_add(3) {
        return Err(format!(
            "{CODE_SIZE}: the picture is longer than {max_bytes} bytes once decoded"
        ));
    }
    let mut out: Vec<u8> = Vec::with_capacity(digits / 4 * 3 + 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    let mut padding = false;
    for b in text.bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'=' {
            padding = true;
            continue;
        }
        if padding {
            return Err(format!("{CODE_DATA}: the text goes on after its padding"));
        }
        let v = sextet(b).ok_or_else(|| format!("{CODE_DATA}: the text is not base64"))?;
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    if digits % 4 == 1 {
        return Err(format!("{CODE_DATA}: the text is cut short"));
    }
    Ok(out)
}

fn be32(b: &[u8]) -> u64 {
    u64::from(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads the picture of the `Data` text: a PNG (or a PGM), within the limits.
pub fn decode_picture(data: &str, lim: &Limits) -> Result<Gray, String> {
    let max_bytes = lim.max_bitmap_bytes / 4 * 3 + 3;
    let bytes = decode_base64(data, max_bytes)?;
    let claimed = |w: u64, h: u64| -> Result<(), String> {
        let px = usize::try_from(w.saturating_mul(h)).unwrap_or(usize::MAX);
        if w == 0 || h == 0 || px > lim.max_bitmap_pixels {
            Err(format!(
                "{CODE_PIXELS}: the picture has {w} x {h} pixels and the limit is {} (max_bitmap_pixels)",
                lim.max_bitmap_pixels
            ))
        } else {
            Ok(())
        }
    };
    let gray = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        // the size is in the header: it is checked before the picture is unpacked
        if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
            return Err(format!("{CODE_DATA}: the PNG has no header"));
        }
        claimed(be32(&bytes[16..20]), be32(&bytes[20..24]))?;
        eh3d_image::from_png(&bytes).map_err(|e| format!("{CODE_DATA}: {e}"))?
    } else if bytes.starts_with(b"P2") || bytes.starts_with(b"P5") {
        let g = eh3d_image::from_pgm(&bytes).map_err(|e| format!("{CODE_DATA}: {e}"))?;
        claimed(g.w as u64, g.h as u64)?;
        g
    } else {
        return Err(format!(
            "{CODE_FORMAT}: the picture is not a PNG or a PGM (a JPEG or any other format is not read)"
        ));
    };
    Ok(gray)
}

/// Turns and mirrors a picture as the matrix `m` (`x' = a x + c y + e`, `y' = b x + d y + f`, from the picture centred on its origin, its first row at the lowest y) places it, with
/// the y axis downwards in the result. Only a flip and a quarter turn are done; the size in millimetres is the one after the matrix.
pub fn orient(
    gray: &Gray,
    m: &[f64; 6],
    width: f64,
    height: f64,
) -> Result<(Gray, f64, f64), String> {
    let [a, b, c, d, _, _] = *m;
    let scale = a.abs().max(b.abs()).max(c.abs()).max(d.abs());
    if !(scale.is_finite() && scale > 0.0) {
        return Err(format!("{CODE_ROTATED}: the picture has no size"));
    }
    let eps = scale * 1e-6;
    let (w, h) = (gray.w, gray.h);
    let at = |x: usize, y: usize| gray.at(x, y);
    if b.abs() <= eps && c.abs() <= eps {
        let (flip_x, flip_y) = (a < 0.0, d < 0.0);
        let mut data = Vec::with_capacity(w * h);
        for r in 0..h {
            let j = if flip_y { h - 1 - r } else { r };
            for k in 0..w {
                data.push(at(if flip_x { w - 1 - k } else { k }, j));
            }
        }
        Ok((Gray::new(w, h, data)?, width * a.abs(), height * d.abs()))
    } else if a.abs() <= eps && d.abs() <= eps {
        // a quarter turn: the columns of the result are the rows of the picture and the other way round
        let mut data = Vec::with_capacity(w * h);
        for r in 0..w {
            let i = if b > 0.0 { r } else { w - 1 - r };
            for k in 0..h {
                data.push(at(i, if c > 0.0 { k } else { h - 1 - k }));
            }
        }
        Ok((Gray::new(h, w, data)?, height * c.abs(), width * b.abs()))
    } else {
        Err(format!(
            "{CODE_ROTATED}: the picture is turned by something other than a quarter turn"
        ))
    }
}
