//! From grey levels to dots: a dot is where the laser burns.

use crate::image::Gray;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Every pixel darker than the threshold burns.
    Threshold(u8),
    FloydSteinberg,
    /// Keeps only three quarters of the error: lighter greys come out cleaner, at the price of pure black and white areas.
    Atkinson,
    /// Ordered dithering with an 8 x 8 Bayer matrix: every dot is decided by its own place and its own grey, so the pattern is regular, nothing spreads from a dot to the next one and two
    /// pictures of the same grey burn the same dots. Good for flat greys and for a laser that does not like isolated dots.
    Bayer,
}

impl Method {
    /// The method of a word of the command line or of a file of settings: `fs`, `atkinson`, `threshold` (with the given level) or `bayer`.
    pub fn from_word(word: &str, threshold: u8) -> Option<Method> {
        match word.trim().to_ascii_lowercase().as_str() {
            "fs" | "floyd-steinberg" => Some(Method::FloydSteinberg),
            "atkinson" => Some(Method::Atkinson),
            "threshold" => Some(Method::Threshold(threshold)),
            "bayer" | "ordered" => Some(Method::Bayer),
            _ => None,
        }
    }
}

/// The 8 x 8 Bayer matrix: each of 0 to 63 once, the next level of the matrix of half the side being `[[4m, 4m + 2], [4m + 3, 4m + 1]]`.
pub fn bayer_matrix() -> [[u32; 8]; 8] {
    let mut m = [[0u32; 8]; 8];
    let mut n = 1;
    while n < 8 {
        let mut next = [[0u32; 8]; 8];
        for (y, row) in next.iter_mut().enumerate().take(2 * n) {
            for (x, slot) in row.iter_mut().enumerate().take(2 * n) {
                let base = 4 * m[y % n][x % n];
                let quarter = match (y / n, x / n) {
                    (0, 0) => 0,
                    (0, _) => 2,
                    (_, 0) => 3,
                    _ => 1,
                };
                *slot = base + quarter;
            }
        }
        m = next;
        n *= 2;
    }
    m
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dots {
    pub w: usize,
    pub h: usize,
    /// True where the laser burns.
    pub burn: Vec<bool>,
}

impl Dots {
    pub fn at(&self, x: usize, y: usize) -> bool {
        self.burn[y * self.w + x]
    }
    pub fn count(&self) -> usize {
        self.burn.iter().filter(|b| **b).count()
    }
}

pub fn dither(img: &Gray, method: Method) -> Dots {
    let (w, h) = (img.w, img.h);
    let mut burn = vec![false; w * h];
    match method {
        Method::Threshold(t) => {
            for (b, &v) in burn.iter_mut().zip(&img.data) {
                *b = v < t;
            }
        }
        Method::Bayer => {
            let matrix = bayer_matrix();
            for y in 0..h {
                for x in 0..w {
                    // a dot burns where the grey is darker than the level that the matrix has in that place
                    let level = (f64::from(matrix[y % 8][x % 8]) + 0.5) / 64.0;
                    burn[y * w + x] = f64::from(img.data[y * w + x]) / 255.0 < level;
                }
            }
        }
        Method::FloydSteinberg | Method::Atkinson => {
            let mut v: Vec<i32> = img.data.iter().map(|&b| i32::from(b)).collect();
            let spread: &[(isize, isize, i32, i32)] = match method {
                Method::FloydSteinberg => {
                    &[(1, 0, 7, 16), (-1, 1, 3, 16), (0, 1, 5, 16), (1, 1, 1, 16)]
                }
                _ => &[
                    (1, 0, 1, 8),
                    (2, 0, 1, 8),
                    (-1, 1, 1, 8),
                    (0, 1, 1, 8),
                    (1, 1, 1, 8),
                    (0, 2, 1, 8),
                ],
            };
            for y in 0..h {
                for x in 0..w {
                    let old = v[y * w + x].clamp(0, 255);
                    let new = if old < 128 { 0 } else { 255 };
                    burn[y * w + x] = new == 0;
                    let err = old - new;
                    for &(dx, dy, num, den) in spread {
                        let (nx, ny) = (x as isize + dx, y as isize + dy);
                        if nx >= 0 && (nx as usize) < w && (ny as usize) < h {
                            v[ny as usize * w + nx as usize] += err * num / den;
                        }
                    }
                }
            }
        }
    }
    Dots { w, h, burn }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: usize, h: usize, level: u8) -> Gray {
        Gray {
            w,
            h,
            data: vec![level; w * h],
        }
    }

    #[test]
    fn the_bayer_matrix_holds_each_level_once_and_starts_as_the_classic_one() {
        let m = bayer_matrix();
        let mut seen = [false; 64];
        for row in &m {
            for &v in row {
                assert!(!seen[v as usize], "{v} twice");
                seen[v as usize] = true;
            }
        }
        assert!(seen.iter().all(|s| *s));
        assert_eq!(m[0], [0, 32, 8, 40, 2, 34, 10, 42]);
        assert_eq!(m[1], [48, 16, 56, 24, 50, 18, 58, 26]);
    }

    #[test]
    fn bayer_burns_nothing_on_white_and_everything_on_black() {
        assert_eq!(dither(&flat(24, 17, 255), Method::Bayer).count(), 0);
        assert_eq!(dither(&flat(24, 17, 0), Method::Bayer).count(), 24 * 17);
    }

    #[test]
    fn bayer_keeps_the_darkness_of_a_grey() {
        for (level, expected) in [(64u8, 0.75), (128, 0.5), (192, 0.25)] {
            let d = dither(&flat(64, 64, level), Method::Bayer);
            let ratio = d.count() as f64 / (64.0 * 64.0);
            assert!(
                (ratio - expected).abs() < 0.02,
                "grey {level}: {ratio} against {expected}"
            );
        }
    }

    #[test]
    fn bayer_repeats_every_eight_dots_and_each_dot_depends_only_on_its_own_grey() {
        let d = dither(&flat(32, 32, 100), Method::Bayer);
        for y in 0..24 {
            for x in 0..24 {
                assert_eq!(d.at(x, y), d.at(x + 8, y + 8));
            }
        }
        // a picture with one different dot changes that dot only
        let mut g = flat(16, 16, 100);
        let before = dither(&g, Method::Bayer);
        g.data[5 * 16 + 5] = 255;
        let after = dither(&g, Method::Bayer);
        let changed = before
            .burn
            .iter()
            .zip(&after.burn)
            .filter(|(a, b)| a != b)
            .count();
        assert!(changed <= 1);
    }

    #[test]
    fn the_words_of_the_methods_are_known() {
        assert_eq!(Method::from_word("fs", 9), Some(Method::FloydSteinberg));
        assert_eq!(Method::from_word("Atkinson", 9), Some(Method::Atkinson));
        assert_eq!(
            Method::from_word("threshold", 9),
            Some(Method::Threshold(9))
        );
        assert_eq!(Method::from_word(" BAYER ", 9), Some(Method::Bayer));
        assert_eq!(Method::from_word("ordered", 9), Some(Method::Bayer));
        assert_eq!(Method::from_word("dots", 9), None);
        assert_eq!(Method::from_word("", 9), None);
    }
}
