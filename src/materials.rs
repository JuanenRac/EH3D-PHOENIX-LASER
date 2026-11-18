// EH3D-PHOENIX-LASER - the library of materials.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! What suits a material is found once with the test grid and then used again and again: a library is a plain text file with one `[material NAME]` section for each material and way of
//! working (the cut of 3 mm birch plywood is another entry than its engraving). A material holds the power, the speed, the passes, the focus step, the beam width, the hatch, and for a
//! picture the size of a dot, the dithering and the tone curve. Everything in it is optional: the commands take from it only what it says. A material is applied before the options of the
//! command line, which win, and the limit of power still holds over it. The values are the ones the person found for their own machine; nothing here knows a material.

use crate::dither::Method;
use crate::raster::Params;
use crate::tone::Tone;
use crate::vconfig::LayerParams;

/// A library file larger than this is refused.
pub const MAX_FILE_BYTES: u64 = 1 << 20;
/// A library with more materials than this is refused.
pub const MAX_MATERIALS: usize = 500;
/// The longest name of a material.
pub const MAX_NAME_CHARS: usize = 80;
/// The longest note of a material.
pub const MAX_NOTE_CHARS: usize = 300;

/// One material and way of working. Every value that is `None` is left as it is wherever the material is applied.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Material {
    pub name: String,
    pub note: String,
    /// A percent of full power.
    pub power: Option<f64>,
    /// mm/min.
    pub speed: Option<f64>,
    pub passes: Option<u32>,
    pub focus_step: Option<f64>,
    pub kerf: Option<f64>,
    pub hatch_spacing: Option<f64>,
    pub hatch_angle: Option<f64>,
    /// The size of a dot of a picture, in mm.
    pub dot: Option<f64>,
    pub overscan: Option<f64>,
    /// The word of the dithering (`fs`, `atkinson`, `threshold` or `bayer`).
    pub dither: Option<String>,
    pub threshold: Option<u8>,
    pub gamma: Option<f64>,
    pub contrast: Option<f64>,
    pub brightness: Option<f64>,
    pub invert: Option<bool>,
    pub auto_level: Option<bool>,
}

/// The level of the threshold when a material asks for it and does not say which.
pub const DEFAULT_THRESHOLD: u8 = 128;

impl Material {
    /// The dithering the material asks for, if it asks for one.
    pub fn method(&self) -> Option<Method> {
        self.dither
            .as_deref()
            .and_then(|w| Method::from_word(w, self.threshold.unwrap_or(DEFAULT_THRESHOLD)))
    }

    /// The settings of a layer of a drawing with what the material says over them.
    pub fn apply_to_layer(&self, p: &mut LayerParams) {
        p.power_percent = self.power.unwrap_or(p.power_percent);
        p.speed_mm_min = self.speed.unwrap_or(p.speed_mm_min);
        p.passes = self.passes.unwrap_or(p.passes);
        p.focus_step_mm = self.focus_step.unwrap_or(p.focus_step_mm);
        p.kerf_mm = self.kerf.unwrap_or(p.kerf_mm);
        p.hatch_spacing_mm = self.hatch_spacing.unwrap_or(p.hatch_spacing_mm);
        p.hatch_angle_deg = self.hatch_angle.unwrap_or(p.hatch_angle_deg);
    }

    /// The parameters of the engraving of a picture with what the material says over them.
    pub fn apply_to_engraving(&self, p: &mut Params) {
        p.power_percent = self.power.unwrap_or(p.power_percent);
        p.feed_mm_min = self.speed.unwrap_or(p.feed_mm_min);
        p.pixel_mm = self.dot.unwrap_or(p.pixel_mm);
        p.overscan_mm = self.overscan.unwrap_or(p.overscan_mm);
    }

    /// The tone curve with what the material says over it.
    pub fn apply_to_tone(&self, t: &mut Tone) {
        t.gamma = self.gamma.unwrap_or(t.gamma);
        t.contrast = self.contrast.unwrap_or(t.contrast);
        t.brightness = self.brightness.unwrap_or(t.brightness);
        t.invert = self.invert.unwrap_or(t.invert);
        t.auto_level = self.auto_level.unwrap_or(t.auto_level);
    }

    /// One line with what the material says, for the list of a library.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut add = |label: &str, v: Option<String>| {
            if let Some(v) = v {
                parts.push(format!("{label} {v}"));
            }
        };
        add("power", self.power.map(|v| format!("{v} %")));
        add("speed", self.speed.map(|v| format!("{v} mm/min")));
        add("passes", self.passes.map(|v| v.to_string()));
        add("focus step", self.focus_step.map(|v| format!("{v} mm")));
        add("kerf", self.kerf.map(|v| format!("{v} mm")));
        add("hatch", self.hatch_spacing.map(|v| format!("{v} mm")));
        add("angle", self.hatch_angle.map(|v| format!("{v} deg")));
        add("dot", self.dot.map(|v| format!("{v} mm")));
        add("overscan", self.overscan.map(|v| format!("{v} mm")));
        add("dither", self.dither.clone());
        add("threshold", self.threshold.map(|v| v.to_string()));
        add("gamma", self.gamma.map(|v| v.to_string()));
        add("contrast", self.contrast.map(|v| v.to_string()));
        add("brightness", self.brightness.map(|v| v.to_string()));
        add("invert", self.invert.map(|v| v.to_string()));
        add("auto level", self.auto_level.map(|v| v.to_string()));
        let mut line = if parts.is_empty() {
            "nothing set".to_string()
        } else {
            parts.join(", ")
        };
        if !self.note.is_empty() {
            line.push_str(&format!(" ({})", self.note));
        }
        line
    }
}

/// The materials of a file, in the order it gives them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    pub materials: Vec<Material>,
}

impl Library {
    /// The material with that name, without telling capitals apart or the spaces around it.
    pub fn find(&self, name: &str) -> Option<&Material> {
        let wanted = name.trim().to_lowercase();
        self.materials
            .iter()
            .find(|m| m.name.to_lowercase() == wanted)
    }

    pub fn names(&self) -> Vec<&str> {
        self.materials.iter().map(|m| m.name.as_str()).collect()
    }
}

fn number(value: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("'{}' is not a number", value.trim()))
}

fn ranged(value: &str, name: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let v = number(value)?;
    if (lo..=hi).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
    }
}

fn whole(value: &str, name: &str, lo: u32, hi: u32) -> Result<u32, String> {
    let v: u32 = value
        .trim()
        .parse()
        .map_err(|_| format!("'{}' is not a whole number", value.trim()))?;
    if (lo..=hi).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
    }
}

fn switch(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        other => Err(format!("'{other}' is not on or off")),
    }
}

fn set(m: &mut Material, key: &str, value: &str) -> Result<(), String> {
    match key {
        "note" => {
            let v = value.trim();
            if v.chars().count() > MAX_NOTE_CHARS || v.chars().any(char::is_control) {
                return Err(format!(
                    "the note is longer than {MAX_NOTE_CHARS} characters or has a control character"
                ));
            }
            m.note = v.to_string();
        }
        "power" => m.power = Some(ranged(value, "power", 0.001, 100.0)?),
        "speed" => m.speed = Some(ranged(value, "speed", 1.0, 30_000.0)?),
        "passes" => m.passes = Some(whole(value, "passes", 1, 1_000)?),
        "focus_step" => m.focus_step = Some(ranged(value, "focus_step", -20.0, 20.0)?),
        "kerf" => m.kerf = Some(ranged(value, "kerf", 0.0, 5.0)?),
        "hatch_spacing" => m.hatch_spacing = Some(ranged(value, "hatch_spacing", 0.01, 5.0)?),
        "hatch_angle" => m.hatch_angle = Some(ranged(value, "hatch_angle", -360.0, 360.0)?),
        "dot" => m.dot = Some(ranged(value, "dot", 0.01, 5.0)?),
        "overscan" => m.overscan = Some(ranged(value, "overscan", 0.0, 50.0)?),
        "dither" => {
            let w = value.trim().to_ascii_lowercase();
            if Method::from_word(&w, DEFAULT_THRESHOLD).is_none() {
                return Err(format!("'{w}' is not fs, atkinson, threshold or bayer"));
            }
            m.dither = Some(w);
        }
        "threshold" => m.threshold = Some(whole(value, "threshold", 0, 255)? as u8),
        "gamma" => m.gamma = Some(ranged(value, "gamma", 0.1, 10.0)?),
        "contrast" => m.contrast = Some(ranged(value, "contrast", -100.0, 100.0)?),
        "brightness" => m.brightness = Some(ranged(value, "brightness", -100.0, 100.0)?),
        "invert" => m.invert = Some(switch(value)?),
        "auto_level" => m.auto_level = Some(switch(value)?),
        _ => return Err(format!("'{key}' is not a setting of a material")),
    }
    Ok(())
}

/// Reads the text of a library. A line that is not understood, a key that is not known, a value out of its range and a name used twice are errors that name the line.
pub fn parse(text: &str) -> Result<Library, String> {
    let mut lib = Library::default();
    for (n, raw) in text.lines().enumerate() {
        let line = n + 1;
        let t = raw.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if let Some(head) = t.strip_prefix('[') {
            let head = head
                .strip_suffix(']')
                .ok_or_else(|| format!("line {line}: a section is not closed with ]"))?
                .trim();
            let (word, name) = head.split_once(char::is_whitespace).unwrap_or((head, ""));
            let name = name.trim();
            if !word.eq_ignore_ascii_case("material") || name.is_empty() {
                return Err(format!(
                    "line {line}: '[{head}]' is not a section (write [material NAME])"
                ));
            }
            if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
                return Err(format!(
                    "line {line}: a name is at most {MAX_NAME_CHARS} characters, with no control character"
                ));
            }
            if lib.find(name).is_some() {
                return Err(format!("line {line}: the material '{name}' is given twice"));
            }
            if lib.materials.len() >= MAX_MATERIALS {
                return Err(format!("line {line}: more than {MAX_MATERIALS} materials"));
            }
            lib.materials.push(Material {
                name: name.to_string(),
                ..Material::default()
            });
            continue;
        }
        let (key, value) = t
            .split_once('=')
            .ok_or_else(|| format!("line {line}: a setting is written key = value"))?;
        let Some(current) = lib.materials.last_mut() else {
            return Err(format!(
                "line {line}: a setting is outside of every [material NAME]"
            ));
        };
        set(current, key.trim(), value).map_err(|e| format!("line {line}: {e}"))?;
    }
    Ok(lib)
}

/// Reads a library from a file, with a limit on its size.
pub fn load(path: &std::path::Path) -> Result<Library, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!(
            "{shown}: the library is larger than {MAX_FILE_BYTES} bytes"
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{shown}: {e}"))?;
    let text = String::from_utf8_lossy(&bytes);
    parse(text.strip_prefix('\u{feff}').unwrap_or(&text)).map_err(|e| format!("{shown}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# two ways of working\n[material Birch 3 mm cut]\nnote = two passes\npower = 90\nspeed = 300\npasses = 2\nkerf = 0.15\n\n[material Birch photo]\npower = 30\nspeed = 3000\ndot = 0.1\ndither = atkinson\ngamma = 1.4\nauto_level = on\n";

    #[test]
    fn a_library_is_read_in_order_with_what_each_material_says() {
        let lib = parse(SAMPLE).unwrap();
        assert_eq!(lib.names(), vec!["Birch 3 mm cut", "Birch photo"]);
        let cut = &lib.materials[0];
        assert_eq!(cut.power, Some(90.0));
        assert_eq!(cut.passes, Some(2));
        assert_eq!(cut.note, "two passes");
        assert_eq!(cut.dither, None);
        let photo = &lib.materials[1];
        assert_eq!(photo.method(), Some(Method::Atkinson));
        assert_eq!(photo.auto_level, Some(true));
        assert_eq!(photo.kerf, None);
    }

    #[test]
    fn a_material_is_found_without_telling_capitals_or_outer_spaces_apart() {
        let lib = parse(SAMPLE).unwrap();
        assert!(lib.find("  birch PHOTO ").is_some());
        assert!(lib.find("birch").is_none());
        assert!(lib.find("").is_none());
    }

    #[test]
    fn a_material_changes_only_what_it_says() {
        let lib = parse(SAMPLE).unwrap();
        let mut layer = LayerParams {
            hatch_spacing_mm: 0.3,
            ..LayerParams::default()
        };
        lib.materials[0].apply_to_layer(&mut layer);
        assert_eq!(layer.power_percent, 90.0);
        assert_eq!(layer.speed_mm_min, 300.0);
        assert_eq!(layer.passes, 2);
        assert_eq!(layer.kerf_mm, 0.15);
        assert_eq!(layer.hatch_spacing_mm, 0.3, "the material does not say it");

        let mut p = Params::default();
        lib.materials[1].apply_to_engraving(&mut p);
        assert_eq!(
            (p.power_percent, p.feed_mm_min, p.pixel_mm, p.overscan_mm),
            (30.0, 3000.0, 0.1, Params::default().overscan_mm)
        );
        let mut t = Tone::default();
        lib.materials[1].apply_to_tone(&mut t);
        assert_eq!((t.gamma, t.auto_level, t.contrast), (1.4, true, 0.0));
        assert!(t.validate().is_ok());
    }

    #[test]
    fn a_threshold_uses_the_level_of_the_material_or_the_default() {
        let lib = parse("[material a]\ndither = threshold\n[material b]\ndither = Threshold\nthreshold = 90\n[material c]\nthreshold = 90\n").unwrap();
        assert_eq!(
            lib.materials[0].method(),
            Some(Method::Threshold(DEFAULT_THRESHOLD))
        );
        assert_eq!(lib.materials[1].method(), Some(Method::Threshold(90)));
        assert_eq!(lib.materials[2].method(), None);
    }

    #[test]
    fn what_is_wrong_is_an_error_that_names_the_line() {
        for (text, needle) in [
            ("power = 5\n", "line 1"),
            ("[material]\n", "line 1"),
            ("[materials x]\n", "line 1"),
            ("[material a\n", "line 1"),
            ("[material a]\npower\n", "line 2"),
            ("[material a]\nwidth = 5\n", "line 2"),
            ("[material a]\npower = 0\n", "line 2"),
            ("[material a]\npower = 100.5\n", "line 2"),
            ("[material a]\nspeed = 0.5\n", "line 2"),
            ("[material a]\npasses = 0\n", "line 2"),
            ("[material a]\npasses = 1.5\n", "line 2"),
            ("[material a]\ndot = 6\n", "line 2"),
            ("[material a]\ngamma = 0\n", "line 2"),
            ("[material a]\nthreshold = 256\n", "line 2"),
            ("[material a]\ndither = dots\n", "line 2"),
            ("[material a]\ninvert = maybe\n", "line 2"),
            ("[material a]\nnote = a\u{7}b\n", "line 2"),
            ("[material a]\n[material A]\n", "line 2"),
            ("[material a]\npower = nan\n", "line 2"),
        ] {
            let e = parse(text).expect_err(text);
            assert!(e.contains(needle), "{text:?} gave {e:?}");
        }
    }

    #[test]
    fn the_edges_of_every_range_are_accepted() {
        parse("[material a]\npower = 0.001\nspeed = 1\npasses = 1\nfocus_step = -20\nkerf = 0\nhatch_spacing = 0.01\nhatch_angle = -360\ndot = 0.01\noverscan = 0\nthreshold = 0\ngamma = 0.1\ncontrast = -100\nbrightness = 100\n").unwrap();
        parse("[material a]\npower = 100\nspeed = 30000\npasses = 1000\nfocus_step = 20\nkerf = 5\nhatch_spacing = 5\nhatch_angle = 360\ndot = 5\noverscan = 50\nthreshold = 255\ngamma = 10\ncontrast = 100\nbrightness = -100\n").unwrap();
    }

    #[test]
    fn the_size_of_a_library_and_of_a_name_are_limited() {
        let mut text = String::new();
        for i in 0..=MAX_MATERIALS {
            text.push_str(&format!("[material m{i}]\npower = 10\n"));
        }
        assert!(parse(&text).unwrap_err().contains("more than"));
        let long = format!("[material {}]\n", "x".repeat(MAX_NAME_CHARS + 1));
        assert!(parse(&long).is_err());
        let note = format!("[material a]\nnote = {}\n", "n".repeat(MAX_NOTE_CHARS + 1));
        assert!(parse(&note).is_err());
    }

    #[test]
    fn a_note_may_hold_an_equal_sign_and_the_description_lists_what_is_set() {
        let lib = parse("[material a]\nnote = test with 1 = 2\npower = 10\nspeed = 500\n").unwrap();
        assert_eq!(lib.materials[0].note, "test with 1 = 2");
        let d = lib.materials[0].describe();
        assert!(d.starts_with("power 10 %, speed 500 mm/min"), "{d}");
        assert!(d.ends_with("(test with 1 = 2)"), "{d}");
        assert_eq!(Material::default().describe(), "nothing set");
    }

    #[test]
    fn a_library_is_read_from_disk_with_a_limit_on_its_size() {
        let dir = std::env::temp_dir().join(format!("eh3d-laser-materials-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("a.cfg");
        std::fs::write(&good, format!("\u{feff}{SAMPLE}")).unwrap();
        assert_eq!(load(&good).unwrap().materials.len(), 2);
        let bad = dir.join("b.cfg");
        std::fs::write(&bad, "[material a]\npower = x\n").unwrap();
        let e = load(&bad).unwrap_err();
        assert!(e.contains("b.cfg") && e.contains("line 2"), "{e}");
        let big = dir.join("c.cfg");
        std::fs::write(&big, vec![b'#'; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert!(load(&big).unwrap_err().contains("larger"));
        assert!(load(&dir.join("missing.cfg")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
