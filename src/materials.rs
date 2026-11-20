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
    /// Whether the shapes that are engraved are filled a second time at right angles to the first lines.
    pub hatch_cross: Option<bool>,
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
        p.hatch_cross = self.hatch_cross.unwrap_or(p.hatch_cross);
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
        add("cross hatch", self.hatch_cross.map(|v| v.to_string()));
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
        "hatch_cross" => m.hatch_cross = Some(switch(value)?),
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

/// The keys of a material, in the order a section writes them (`note` comes first and is not one of them).
pub const KEYS: [&str; 17] = [
    "power",
    "speed",
    "passes",
    "focus_step",
    "kerf",
    "hatch_spacing",
    "hatch_angle",
    "hatch_cross",
    "dot",
    "overscan",
    "dither",
    "threshold",
    "gamma",
    "contrast",
    "brightness",
    "invert",
    "auto_level",
];

fn word(on: bool) -> String {
    if on { "on" } else { "off" }.to_string()
}

/// The value of a key of a material as the text a file writes, or `None` when the material leaves it as it is.
pub fn value_of(m: &Material, key: &str) -> Option<String> {
    match key {
        "power" => m.power.map(|v| v.to_string()),
        "speed" => m.speed.map(|v| v.to_string()),
        "passes" => m.passes.map(|v| v.to_string()),
        "focus_step" => m.focus_step.map(|v| v.to_string()),
        "kerf" => m.kerf.map(|v| v.to_string()),
        "hatch_spacing" => m.hatch_spacing.map(|v| v.to_string()),
        "hatch_angle" => m.hatch_angle.map(|v| v.to_string()),
        "hatch_cross" => m.hatch_cross.map(word),
        "dot" => m.dot.map(|v| v.to_string()),
        "overscan" => m.overscan.map(|v| v.to_string()),
        "dither" => m.dither.clone(),
        "threshold" => m.threshold.map(|v| v.to_string()),
        "gamma" => m.gamma.map(|v| v.to_string()),
        "contrast" => m.contrast.map(|v| v.to_string()),
        "brightness" => m.brightness.map(|v| v.to_string()),
        "invert" => m.invert.map(word),
        "auto_level" => m.auto_level.map(word),
        _ => None,
    }
}

/// Sets one key of a material from the text a person typed, with the range and the words the file reads: a value out of its range is an error, never cut down. `note` is a key too.
pub fn set_value(m: &mut Material, key: &str, value: &str) -> Result<(), String> {
    set(m, key.trim(), value)
}

/// Whether a name can be written as `[material NAME]`: not empty, no space around it, at most [`MAX_NAME_CHARS`] characters and no control character.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("the name of a material is empty".to_string());
    }
    if name != name.trim() {
        return Err("the name of a material has a space at its start or its end".to_string());
    }
    if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(format!(
            "a name is at most {MAX_NAME_CHARS} characters, with no control character"
        ));
    }
    Ok(())
}

/// The section of a material as the lines of a file (`[material NAME]`, the note and the keys it sets, in the order of [`KEYS`]), each ending in a newline.
pub fn render(m: &Material) -> String {
    let mut out = format!("[material {}]\n", m.name);
    if !m.note.is_empty() {
        out.push_str(&format!("note = {}\n", m.note));
    }
    for key in KEYS {
        if let Some(v) = value_of(m, key) {
            out.push_str(&format!("{key} = {v}\n"));
        }
    }
    out
}

fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// The name in a line that opens a section, when it is one.
fn header_name(line: &str) -> Option<String> {
    let head = line.trim().strip_prefix('[')?.strip_suffix(']')?.trim();
    let (_, name) = head.split_once(char::is_whitespace)?;
    Some(name.trim().to_string())
}

fn is_gap(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#') || t.starts_with(';')
}

/// The lines of the section of a material: from its header to the line before the next header, without the blank and comment lines that end it (they stay where they are).
fn span_of(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .position(|l| header_name(l).is_some_and(|n| same_name(&n, name)))?;
    let mut end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| l.trim().starts_with('['))
        .map_or(lines.len(), |(i, _)| i);
    while end > start + 1 && is_gap(lines[end - 1]) {
        end -= 1;
    }
    Some((start, end))
}

fn split_bom(text: &str) -> (&str, &str) {
    match text.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", text),
    }
}

fn finish_edit(bom: &str, body: String, expected: &Library) -> Result<String, String> {
    let result = format!("{bom}{body}");
    if result.len() as u64 > MAX_FILE_BYTES {
        return Err(format!(
            "the library would be larger than {MAX_FILE_BYTES} bytes"
        ));
    }
    // what is written is read again: it must be the library that was meant, with every other material as it was
    let read = parse(&body)?;
    if &read != expected {
        return Err("the file could not be edited without changing something else".to_string());
    }
    Ok(result)
}

/// The text of a library file with one material put in it, and nothing else changed: with `replace` the section of that material is rewritten in its place (the name may change), without it
/// the material is added at the end. The comments, the blank lines, the order, the line ends and the other materials stay as they were. A file that cannot be read as a library is not edited, a name
/// that another material has already is refused, and so is a result larger than [`MAX_FILE_BYTES`] or with more than [`MAX_MATERIALS`] materials.
pub fn upsert_text(text: &str, replace: Option<&str>, m: &Material) -> Result<String, String> {
    check_name(&m.name)?;
    let (bom, body) = split_bom(text);
    let old = parse(body)?;
    if let Some(r) = replace {
        if old.find(r).is_none() {
            return Err(format!("there is no material '{}' in the file", r.trim()));
        }
    }
    if let Some(other) = old.find(&m.name) {
        if !replace.is_some_and(|r| same_name(r, &other.name)) {
            return Err(format!("the file has a material '{}' already", other.name));
        }
    }
    let mine = parse(&render(m))?
        .materials
        .into_iter()
        .next()
        .ok_or_else(|| "the material could not be written".to_string())?;
    let nl = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let section: Vec<String> = render(m)
        .lines()
        .map(|l| format!("{l}{}", if nl == "\r\n" { "\r" } else { "" }))
        .collect();
    let mut expected = old.clone();
    let out = match replace {
        Some(r) => {
            let lines: Vec<&str> = body.split('\n').collect();
            let (start, end) = span_of(&lines, r)
                .ok_or_else(|| format!("there is no material '{}' in the file", r.trim()))?;
            let mut all: Vec<&str> = lines[..start].to_vec();
            all.extend(section.iter().map(String::as_str));
            all.extend_from_slice(&lines[end..]);
            if let Some(slot) = expected
                .materials
                .iter_mut()
                .find(|x| same_name(&x.name, r))
            {
                *slot = mine;
            }
            all.join("\n")
        }
        None => {
            expected.materials.push(mine);
            let kept = body.trim_end();
            let mut out = String::new();
            if !kept.is_empty() {
                out.push_str(kept);
                out.push_str(nl);
                out.push_str(nl);
            }
            for l in &section {
                out.push_str(l);
                out.push('\n');
            }
            out
        }
    };
    finish_edit(bom, out, &expected)
}

/// The text of a library file without one material, and nothing else changed (the comments and the blank lines around the other materials stay). A name that the file does not have is an error.
pub fn remove_text(text: &str, name: &str) -> Result<String, String> {
    let (bom, body) = split_bom(text);
    let old = parse(body)?;
    if old.find(name).is_none() {
        return Err(format!(
            "there is no material '{}' in the file",
            name.trim()
        ));
    }
    let lines: Vec<&str> = body.split('\n').collect();
    let (start, mut end) = span_of(&lines, name)
        .ok_or_else(|| format!("there is no material '{}' in the file", name.trim()))?;
    // one blank line between two blocks is enough: the one after the removed section goes with it when there is one before it
    let before_blank = start == 0 || is_gap(lines[start - 1]);
    if before_blank && end + 1 < lines.len() && lines[end].trim().is_empty() {
        end += 1;
    }
    let mut all: Vec<&str> = lines[..start].to_vec();
    all.extend_from_slice(&lines[end..]);
    let mut expected = old.clone();
    expected.materials.retain(|m| !same_name(&m.name, name));
    finish_edit(bom, all.join("\n"), &expected)
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
    fn a_material_can_ask_for_the_cross_hatch_and_says_so_in_its_list() {
        let lib = parse(
            "[material a]
hatch_cross = ON
[material b]
hatch_cross = off
[material c]
power = 5
",
        )
        .unwrap();
        let mut p = LayerParams {
            hatch_cross: false,
            ..LayerParams::default()
        };
        lib.materials[0].apply_to_layer(&mut p);
        assert!(p.hatch_cross);
        // a material that says off turns it off, and one that says nothing leaves it as it is
        lib.materials[1].apply_to_layer(&mut p);
        assert!(!p.hatch_cross);
        p.hatch_cross = true;
        lib.materials[2].apply_to_layer(&mut p);
        assert!(p.hatch_cross);
        assert_eq!(lib.materials[0].describe(), "cross hatch true");
        assert!(parse(
            "[material a]
hatch_cross = twice
"
        )
        .unwrap_err()
        .contains("line 2"));
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

    fn full() -> Material {
        Material {
            name: "Birch 3 mm cut".to_string(),
            note: "two passes, air on".to_string(),
            power: Some(90.0),
            speed: Some(300.0),
            passes: Some(2),
            focus_step: Some(-0.5),
            kerf: Some(0.15),
            hatch_spacing: Some(0.1),
            hatch_angle: Some(45.0),
            hatch_cross: Some(true),
            dot: Some(0.08),
            overscan: Some(2.5),
            dither: Some("atkinson".to_string()),
            threshold: Some(100),
            gamma: Some(1.4),
            contrast: Some(-10.0),
            brightness: Some(5.0),
            invert: Some(false),
            auto_level: Some(true),
        }
    }

    const FILE: &str = "# my library\n# of two materials\n\n[material A]\npower = 10\n# about b\n\n[material B]\nnote = x\nspeed = 500\n\n; the end\n";

    #[test]
    fn a_material_is_written_as_a_section_that_reads_back_the_same() {
        let m = full();
        let text = render(&m);
        assert!(
            text.starts_with("[material Birch 3 mm cut]\nnote = two passes, air on\npower = 90\n")
        );
        assert!(text.contains("focus_step = -0.5\n") && text.contains("hatch_cross = on\n"));
        assert!(text.contains("invert = off\n") && text.ends_with("auto_level = on\n"));
        assert_eq!(parse(&text).unwrap().materials, vec![m]);
        // a material that says nothing is only its header
        let empty = Material {
            name: "x".to_string(),
            ..Material::default()
        };
        assert_eq!(render(&empty), "[material x]\n");
        assert_eq!(parse(&render(&empty)).unwrap().materials, vec![empty]);
        // every key has a value and a name
        for key in KEYS {
            assert!(value_of(&full(), key).is_some(), "{key}");
        }
        assert_eq!(value_of(&full(), "width"), None);
    }

    #[test]
    fn a_value_is_set_from_text_with_the_ranges_of_the_file_and_never_cut_down() {
        let mut m = Material::default();
        for (key, text) in [
            ("power", "50"),
            ("speed", "1200"),
            ("passes", "3"),
            ("dither", "Bayer"),
            ("invert", "on"),
            ("note", "hello"),
        ] {
            set_value(&mut m, key, text).unwrap();
        }
        assert_eq!(m.power, Some(50.0));
        assert_eq!(m.dither.as_deref(), Some("bayer"));
        for (key, text) in [
            ("power", "0"),
            ("power", "100.1"),
            ("speed", "0.5"),
            ("passes", "1001"),
            ("passes", "2.5"),
            ("threshold", "256"),
            ("dither", "dots"),
            ("invert", "maybe"),
            ("gamma", "nan"),
            ("width", "5"),
        ] {
            let before = m.clone();
            assert!(set_value(&mut m, key, text).is_err(), "{key} {text}");
            assert_eq!(m, before, "a refused value changes nothing");
        }
    }

    #[test]
    fn a_name_is_checked_before_it_is_written() {
        for good in ["a", "Birch 3 mm cut", "x]y", "caf\u{e9}"] {
            assert!(check_name(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            " ",
            " a",
            "a ",
            "a\nb",
            "a\u{7}",
            &"x".repeat(MAX_NAME_CHARS + 1),
        ] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
        assert!(check_name(&"x".repeat(MAX_NAME_CHARS)).is_ok());
    }

    #[test]
    fn a_material_is_replaced_in_its_place_and_everything_else_stays_byte_for_byte() {
        let new = Material {
            name: "A".to_string(),
            power: Some(20.0),
            speed: Some(900.0),
            ..Material::default()
        };
        let out = upsert_text(FILE, Some("a"), &new).unwrap();
        assert_eq!(
            out,
            "# my library\n# of two materials\n\n[material A]\npower = 20\nspeed = 900\n# about b\n\n[material B]\nnote = x\nspeed = 500\n\n; the end\n"
        );
        // renamed: the new name takes the place, the others do not move
        let renamed = Material {
            name: "Alder".to_string(),
            ..new.clone()
        };
        let out = upsert_text(FILE, Some("A"), &renamed).unwrap();
        assert!(out.contains("[material Alder]\npower = 20\n"));
        assert_eq!(parse(&out).unwrap().names(), vec!["Alder", "B"]);
        // the last material is replaced too, with what follows it kept
        let b = Material {
            name: "B".to_string(),
            power: Some(1.0),
            ..Material::default()
        };
        let out = upsert_text(FILE, Some("B"), &b).unwrap();
        assert!(
            out.ends_with("[material B]\npower = 1\n\n; the end\n"),
            "{out:?}"
        );
        assert!(out.starts_with(
            "# my library\n# of two materials\n\n[material A]\npower = 10\n# about b\n"
        ));
    }

    #[test]
    fn a_material_is_added_at_the_end_and_the_line_ends_and_the_mark_of_the_file_are_kept() {
        let m = Material {
            name: "C".to_string(),
            power: Some(5.0),
            ..Material::default()
        };
        let out = upsert_text(FILE, None, &m).unwrap();
        assert!(out.starts_with(FILE.trim_end()));
        assert!(
            out.ends_with("; the end\n\n[material C]\npower = 5\n"),
            "{out:?}"
        );
        // an empty file and a file with no newline at its end
        assert_eq!(
            upsert_text("", None, &m).unwrap(),
            "[material C]\npower = 5\n"
        );
        let out = upsert_text("[material A]\npower = 1", None, &m).unwrap();
        assert_eq!(out, "[material A]\npower = 1\n\n[material C]\npower = 5\n");
        // a file with the lines ended by CR LF stays that way, and so does the mark at its start
        let crlf = "\u{feff}[material A]\r\npower = 10\r\n\r\n[material B]\r\nspeed = 500\r\n";
        let out = upsert_text(
            crlf,
            Some("A"),
            &Material {
                name: "A".to_string(),
                power: Some(11.0),
                ..Material::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            "\u{feff}[material A]\r\npower = 11\r\n\r\n[material B]\r\nspeed = 500\r\n"
        );
        let out = upsert_text(crlf, None, &m).unwrap();
        assert!(
            out.starts_with('\u{feff}')
                && out.contains("speed = 500\r\n\r\n[material C]\r\npower = 5\r\n"),
            "{out:?}"
        );
        assert_eq!(
            parse(out.strip_prefix('\u{feff}').unwrap())
                .unwrap()
                .materials
                .len(),
            3
        );
    }

    #[test]
    fn a_name_that_is_used_a_material_that_is_missing_or_a_file_that_is_broken_is_refused() {
        let a = Material {
            name: "a".to_string(),
            ..Material::default()
        };
        // a new material cannot take the name of another, with capitals or not
        assert!(upsert_text(FILE, None, &a).unwrap_err().contains("already"));
        // a replaced material cannot be renamed over another one
        let over = Material {
            name: "b".to_string(),
            ..Material::default()
        };
        assert!(upsert_text(FILE, Some("A"), &over)
            .unwrap_err()
            .contains("already"));
        // one that is not in the file cannot be replaced or removed
        let c = Material {
            name: "C".to_string(),
            ..Material::default()
        };
        assert!(upsert_text(FILE, Some("zzz"), &c)
            .unwrap_err()
            .contains("no material"));
        assert!(remove_text(FILE, "zzz")
            .unwrap_err()
            .contains("no material"));
        // a bad name is not written
        let bad = Material {
            name: " x".to_string(),
            ..Material::default()
        };
        assert!(upsert_text(FILE, None, &bad).is_err());
        // a file that is not a library is not edited
        for broken in [
            "[material a]\npower = x\n",
            "power = 5\n",
            "[material a]\n[material a]\n",
        ] {
            assert!(upsert_text(broken, None, &c).is_err(), "{broken:?}");
            assert!(remove_text(broken, "a").is_err(), "{broken:?}");
        }
        // too many materials, and a library that would be too large
        let mut full_lib = String::new();
        for i in 0..MAX_MATERIALS {
            full_lib.push_str(&format!("[material m{i}]\npower = 10\n"));
        }
        assert!(upsert_text(&full_lib, None, &c)
            .unwrap_err()
            .contains("more than"));
        let note = Material {
            name: "n".to_string(),
            note: "n".repeat(MAX_NOTE_CHARS),
            ..Material::default()
        };
        let mut big = String::new();
        let mut i = 0;
        while (big.len() as u64) < MAX_FILE_BYTES - 200 && i < MAX_MATERIALS - 1 {
            big.push_str(&format!(
                "[material m{i}]\nnote = {}\n",
                "x".repeat(MAX_NOTE_CHARS)
            ));
            i += 1;
        }
        // the sections above are short of the limit, so pad with comments to bring the file to the edge
        big.push_str(&format!(
            "# {}\n",
            "p".repeat((MAX_FILE_BYTES as usize).saturating_sub(big.len() + 100))
        ));
        assert!(upsert_text(&big, None, &note)
            .unwrap_err()
            .contains("larger"));
    }

    #[test]
    fn a_material_is_removed_with_its_section_and_nothing_else() {
        let out = remove_text(FILE, "a").unwrap();
        assert_eq!(out, "# my library\n# of two materials\n\n# about b\n\n[material B]\nnote = x\nspeed = 500\n\n; the end\n");
        let out = remove_text(FILE, "B").unwrap();
        assert_eq!(out, "# my library\n# of two materials\n\n[material A]\npower = 10\n# about b\n\n; the end\n");
        // the only material goes and its comments stay
        let one = "# top\n[material A]\npower = 1\n";
        assert_eq!(remove_text(one, "A").unwrap(), "# top\n");
        assert!(parse(&remove_text(one, "A").unwrap())
            .unwrap()
            .materials
            .is_empty());
        // a mark at the start and CR LF stay
        let crlf = "\u{feff}[material A]\r\npower = 10\r\n\r\n[material B]\r\nspeed = 500\r\n";
        assert_eq!(
            remove_text(crlf, "A").unwrap(),
            "\u{feff}[material B]\r\nspeed = 500\r\n"
        );
    }

    #[test]
    fn many_edits_never_change_the_materials_that_were_not_touched() {
        // a fixed seed picks what to do: add, replace, rename or remove
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = move |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        let mut text = String::from("# library\n\n");
        let mut count = 0u64;
        for _ in 0..300 {
            let before = parse(&text).unwrap();
            let pick = before
                .materials
                .get(next(before.materials.len().max(1) as u64) as usize)
                .map(|m| m.name.clone());
            let candidate = Material {
                name: format!("m{}", next(40)),
                power: Some(1.0 + next(99) as f64),
                note: if next(2) == 0 {
                    String::new()
                } else {
                    format!("note {}", next(1000))
                },
                passes: (next(2) == 0).then_some(1 + next(9) as u32),
                invert: (next(2) == 0).then_some(next(2) == 0),
                ..Material::default()
            };
            let result = match (next(3), &pick) {
                (0, _) | (_, None) => upsert_text(&text, None, &candidate),
                (1, Some(p)) => upsert_text(&text, Some(p), &candidate),
                (_, Some(p)) => remove_text(&text, p),
            };
            if let Ok(new) = result {
                let after = parse(&new).unwrap();
                // every material of the file before, other than the one that was named, is in the file after
                let named = pick.as_deref().unwrap_or("");
                for m in &before.materials {
                    if !same_name(&m.name, named) && !same_name(&m.name, &candidate.name) {
                        assert_eq!(after.find(&m.name), Some(m), "{} in {new:?}", m.name);
                    }
                }
                text = new;
                count += 1;
            }
        }
        assert!(count > 50, "{count}");
        assert!(text.starts_with("# library\n"));
    }
}
