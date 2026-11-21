// EH3D-PHOENIX-LASER - presets of a material and a thickness, given to a layer by copy or by link.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A preset is a power, a speed and a number of passes for a material (and a thickness, when it is cut). It is given to a layer, or to one of its sub-layers, in one of two ways. To **assign**
//! it is to copy its values into the layer: the layer is then its own and a later change of the preset does not reach it. To **link** it is to keep the layer tied to the preset: when the
//! values of the preset change, every layer linked to it changes with it, the change is told (which layer, from what to what), and the layer shows that it is linked.
//!
//! The values are never invented here. They come from a person, or from the common database of materials through [`crate::commonmat`], exactly as the database gives them; the database says
//! whether anybody tried a point on a machine and its seed never does, so a preset that was not tried carries a warning in every language wherever it is shown. A second figure that another
//! source gives for the same case (an alternative) is kept apart and is not used unless the person asks for it. A value over a limit of the machine is an error that names the layer and the
//! preset, never lowered in silence, and a change that any linked layer cannot take is refused as a whole: the preset and every layer stay as they were.

use crate::commonmat::{Found, What};
use crate::lang::{fill, Lang, Msg};
use crate::vconfig::{LayerMode, LayerParams, Settings};

/// The most presets a book holds.
pub const MAX_PRESETS: usize = 500;

/// The most bindings (layers and sub-layers given a preset) a book holds.
pub const MAX_BINDINGS: usize = 1_000;

/// The longest name of a preset.
pub const MAX_NAME_CHARS: usize = 80;

/// What a layer is given: power, speed and passes for a material.
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    /// The name, unique in a book (compared without capitals).
    pub name: String,
    /// The material as the common database names it (`wood/birch-plywood`), or free text.
    pub material: String,
    /// The thickness in mm, when the preset is for a cut.
    pub thickness_mm: Option<f64>,
    pub what: What,
    /// A percent of full power.
    pub power_percent: f64,
    pub speed_mm_min: f64,
    pub passes: u32,
    pub air_assist: Option<bool>,
    /// Whether anybody tried the values on a machine.
    pub verified: bool,
    /// A second figure of another source for the same case: never given to a layer unless the person asks for it.
    pub alternative: bool,
}

impl Preset {
    /// A preset made from a starting point of the common database, with its values as the database gives them.
    pub fn from_found(
        name: &str,
        material: &str,
        thickness_mm: Option<f64>,
        what: What,
        f: &Found,
    ) -> Preset {
        Preset {
            name: name.trim().to_string(),
            material: material.to_string(),
            thickness_mm,
            what,
            power_percent: f.power_percent,
            speed_mm_min: f.speed_mm_min,
            passes: f.passes,
            air_assist: f.air_assist,
            verified: f.verified,
            alternative: false,
        }
    }

    /// The same preset marked as the second figure of another source.
    pub fn as_alternative(mut self) -> Preset {
        self.alternative = true;
        self
    }

    /// The settings of a layer with the values of the preset over them: the mode follows what the preset does, and nothing else of the layer is touched.
    pub fn apply_to(&self, base: &LayerParams) -> LayerParams {
        LayerParams {
            mode: match self.what {
                What::Cut => LayerMode::Cut,
                What::Engrave => LayerMode::Engrave,
            },
            power_percent: self.power_percent,
            speed_mm_min: self.speed_mm_min,
            passes: self.passes,
            ..*base
        }
    }

    /// Checks the preset on its own: a name, a power from 0.001 to 100, a speed from 1 to 30000, and passes from 1 to 1000.
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty()
            || name.chars().count() > MAX_NAME_CHARS
            || name.chars().any(char::is_control)
        {
            return Err(format!(
                "the name of a preset is from 1 to {MAX_NAME_CHARS} characters with no control character"
            ));
        }
        let range = |what: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!(
                    "preset {name}: {what} is {v}: it must be from {lo} to {hi}"
                ))
            }
        };
        range("power", self.power_percent, 0.001, 100.0)?;
        range("speed", self.speed_mm_min, 1.0, 30_000.0)?;
        if self.passes == 0 || self.passes > 1_000 {
            return Err(format!(
                "preset {name}: passes is {}: it must be from 1 to 1000",
                self.passes
            ));
        }
        if let Some(t) = self.thickness_mm {
            range("thickness", t, 0.0, 1_000.0)?;
        }
        Ok(())
    }

    /// The warning that the values were not tried on a machine, in a language; `None` when somebody tried them.
    pub fn warning(&self, lang: Lang) -> Option<String> {
        (!self.verified).then(|| fill(Msg::PresetNotTried.tr(lang), &[&self.name]))
    }
}

/// The layer, or the sub-layer of a layer, that a preset is given to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The layer, in lower case.
    pub layer: String,
    /// The label of the sub-layer; empty for the layer itself.
    pub label: String,
}

impl Target {
    pub fn layer(layer: &str) -> Target {
        Target {
            layer: layer.trim().to_lowercase(),
            label: String::new(),
        }
    }

    pub fn sublayer(layer: &str, label: &str) -> Target {
        Target {
            layer: layer.trim().to_lowercase(),
            label: label.trim().to_lowercase(),
        }
    }

    pub fn name(&self) -> String {
        if self.label.is_empty() {
            self.layer.clone()
        } else {
            format!("{}/{}", self.layer, self.label)
        }
    }
}

/// A preset given to a target, by copy (`linked` is false) or by link.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub target: Target,
    /// The name of the preset, in lower case.
    pub preset: String,
    pub linked: bool,
}

/// One layer that changed because its preset changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Update {
    pub target: Target,
    /// Power, speed and passes before.
    pub from: (f64, f64, u32),
    /// Power, speed and passes after.
    pub to: (f64, f64, u32),
}

/// The presets and who holds them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Book {
    pub presets: Vec<Preset>,
    pub bindings: Vec<Binding>,
}

fn params_mut<'a>(s: &'a mut Settings, t: &Target) -> Option<&'a mut LayerParams> {
    if t.label.is_empty() {
        s.layers
            .iter_mut()
            .find(|(n, _)| *n == t.layer)
            .map(|(_, p)| p)
    } else {
        s.sublayers
            .iter_mut()
            .find(|x| x.layer == t.layer && x.label == t.label)
            .map(|x| &mut x.params)
    }
}

impl Book {
    pub fn new() -> Book {
        Book::default()
    }

    /// The preset with this name (capitals do not matter).
    pub fn find(&self, name: &str) -> Option<&Preset> {
        let k = name.trim().to_lowercase();
        self.presets.iter().find(|p| p.name.to_lowercase() == k)
    }

    /// The presets for a material and a thickness (`None` matches any), in the order they were added. An alternative is left out unless `with_alternatives` is true.
    pub fn for_material(
        &self,
        material: &str,
        thickness_mm: Option<f64>,
        with_alternatives: bool,
    ) -> Vec<&Preset> {
        let m = material.trim().to_lowercase();
        self.presets
            .iter()
            .filter(|p| p.material.to_lowercase() == m)
            .filter(|p| match (thickness_mm, p.thickness_mm) {
                (None, _) => true,
                (Some(a), Some(b)) => (a - b).abs() < 1e-6,
                (Some(_), None) => false,
            })
            .filter(|p| with_alternatives || !p.alternative)
            .collect()
    }

    /// Adds a preset: it must be valid and its name must be new.
    pub fn add(&mut self, p: Preset) -> Result<(), String> {
        p.validate()?;
        if self.presets.len() >= MAX_PRESETS {
            return Err(format!("a book holds {MAX_PRESETS} presets at the most"));
        }
        if self.find(&p.name).is_some() {
            return Err(format!("there is a preset named '{}' already", p.name));
        }
        self.presets.push(p);
        Ok(())
    }

    /// The binding of a target, if it has one.
    pub fn binding(&self, t: &Target) -> Option<&Binding> {
        self.bindings.iter().find(|b| b.target == *t)
    }

    /// Gives a preset to a target by copy: the values go into the layer and the layer stays its own. Same checks as [`Book::link`].
    pub fn assign(
        &mut self,
        s: &mut Settings,
        target: &Target,
        preset: &str,
        with_alternative: bool,
        lang: Lang,
    ) -> Result<Vec<String>, String> {
        self.give(s, target, preset, with_alternative, false, lang)
    }

    /// Gives a preset to a target by link: the values go into the layer and the layer follows the preset from then on. The preset must exist, an alternative needs `with_alternative`, the
    /// target must be a layer or sub-layer of the settings, and the values must be inside the limits of the machine; otherwise nothing changes. The warnings it returns are in the language given.
    pub fn link(
        &mut self,
        s: &mut Settings,
        target: &Target,
        preset: &str,
        with_alternative: bool,
        lang: Lang,
    ) -> Result<Vec<String>, String> {
        self.give(s, target, preset, with_alternative, true, lang)
    }

    fn give(
        &mut self,
        s: &mut Settings,
        target: &Target,
        preset: &str,
        with_alternative: bool,
        linked: bool,
        lang: Lang,
    ) -> Result<Vec<String>, String> {
        let p = self
            .find(preset)
            .ok_or_else(|| format!("there is no preset named '{}'", preset.trim()))?
            .clone();
        if p.alternative && !with_alternative {
            return Err(format!(
                "preset {} is the second figure of another source: it is not given to a layer unless it is asked for",
                p.name
            ));
        }
        if self.binding(target).is_none() && self.bindings.len() >= MAX_BINDINGS {
            return Err(format!("a book holds {MAX_BINDINGS} bindings at the most"));
        }
        let mut trial = s.clone();
        let slot = params_mut(&mut trial, target).ok_or_else(|| {
            format!(
                "the settings have no layer or sub-layer '{}'",
                target.name()
            )
        })?;
        *slot = p.apply_to(slot);
        trial
            .validate()
            .map_err(|e| format!("preset {} for {}: {e}", p.name, target.name()))?;
        *s = trial;
        let binding = Binding {
            target: target.clone(),
            preset: p.name.to_lowercase(),
            linked,
        };
        match self.bindings.iter_mut().find(|b| b.target == *target) {
            Some(b) => *b = binding,
            None => self.bindings.push(binding),
        }
        Ok(p.warning(lang).into_iter().collect())
    }

    /// The book as text, to keep next to the settings: a `[preset NAME]` section for each preset (its material, thickness, what it does, power, speed, passes, air assist, whether it was
    /// tried and whether it is the second figure of another source) and a `[bind LAYER]` or `[bind LAYER/LABEL]` section for each binding (the preset and `linked = on` or `off`). The
    /// text is always the same for the same book, so that a file under version control only changes when the book does. A name with a bracket is refused.
    pub fn to_text(&self) -> Result<String, String> {
        let mut out = String::from("# presets and the layers they are given to\n");
        for p in &self.presets {
            p.validate()?;
            if p.name.contains(['[', ']']) || p.material.contains(['\n', '\r']) {
                return Err(format!(
                    "preset {}: a name or a material with a bracket or a line break cannot be kept",
                    p.name
                ));
            }
            let flag = |v: bool| if v { "on" } else { "off" };
            out.push_str(&format!(
                "\n[preset {}]\nmaterial = {}\n",
                p.name, p.material
            ));
            if let Some(t) = p.thickness_mm {
                out.push_str(&format!("thickness = {t}\n"));
            }
            out.push_str(&format!(
                "what = {}\npower = {}\nspeed = {}\npasses = {}\n",
                match p.what {
                    What::Cut => "cut",
                    What::Engrave => "engrave",
                },
                p.power_percent,
                p.speed_mm_min,
                p.passes
            ));
            if let Some(a) = p.air_assist {
                out.push_str(&format!("air = {}\n", flag(a)));
            }
            out.push_str(&format!(
                "verified = {}\nalternative = {}\n",
                flag(p.verified),
                flag(p.alternative)
            ));
        }
        for b in &self.bindings {
            out.push_str(&format!(
                "\n[bind {}]\npreset = {}\nlinked = {}\n",
                b.target.name(),
                b.preset,
                if b.linked { "on" } else { "off" }
            ));
        }
        Ok(out)
    }

    /// Reads a book written by [`Book::to_text`]. The text is held to a size (1 MiB), every line is a comment, a section head or `key = value` with a key that the section knows, a key
    /// is not given twice, every preset is checked like [`Book::add`], and a binding must name a preset of the book and be there once for its target. A mistake is an error that names its
    /// line, and nothing of a damaged text is kept.
    pub fn from_text(text: &str) -> Result<Book, String> {
        if text.len() > 1 << 20 {
            return Err("the book of presets is larger than 1 MiB".to_string());
        }
        enum Sec {
            None,
            Preset(Vec<(String, String)>, String, usize),
            Bind(Vec<(String, String)>, String, usize),
        }
        let mut book = Book::new();
        let mut sec = Sec::None;
        fn close(book: &mut Book, sec: Sec) -> Result<(), String> {
            let get = |kv: &[(String, String)], k: &str| {
                kv.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone())
            };
            let on_off = |v: Option<String>, line: usize, k: &str| match v.as_deref() {
                None => Ok(None),
                Some("on") => Ok(Some(true)),
                Some("off") => Ok(Some(false)),
                Some(o) => Err(format!("line {line}: {k} is on or off, not '{o}'")),
            };
            let number = |v: Option<String>, line: usize, k: &str| -> Result<Option<f64>, String> {
                v.map(|t| {
                    t.parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| format!("line {line}: {k} '{t}' is not a number"))
                })
                .transpose()
            };
            match sec {
                Sec::None => Ok(()),
                Sec::Preset(kv, name, line) => {
                    let need = |k: &str| {
                        get(&kv, k).ok_or_else(|| format!("line {line}: preset {name} has no {k}"))
                    };
                    let what = match need("what")?.as_str() {
                        "cut" => What::Cut,
                        "engrave" => What::Engrave,
                        o => return Err(format!("line {line}: what is cut or engrave, not '{o}'")),
                    };
                    let value = |k: &str| -> Result<f64, String> {
                        number(Some(need(k)?), line, k).map(|v| v.unwrap_or(0.0))
                    };
                    let passes = need("passes")?
                        .parse::<u32>()
                        .map_err(|_| format!("line {line}: passes is a whole number"))?;
                    let p = Preset {
                        name: name.clone(),
                        material: need("material")?,
                        thickness_mm: number(get(&kv, "thickness"), line, "thickness")?,
                        what,
                        power_percent: value("power")?,
                        speed_mm_min: value("speed")?,
                        passes,
                        air_assist: on_off(get(&kv, "air"), line, "air")?,
                        verified: on_off(get(&kv, "verified"), line, "verified")?.unwrap_or(false),
                        alternative: on_off(get(&kv, "alternative"), line, "alternative")?
                            .unwrap_or(false),
                    };
                    book.add(p).map_err(|e| format!("line {line}: {e}"))
                }
                Sec::Bind(kv, name, line) => {
                    let (layer, label) = name.split_once('/').unwrap_or((name.as_str(), ""));
                    if layer.trim().is_empty() {
                        return Err(format!("line {line}: a binding names a layer"));
                    }
                    let target = Target::sublayer(layer, label);
                    let preset = get(&kv, "preset")
                        .ok_or_else(|| format!("line {line}: binding {name} has no preset"))?
                        .to_lowercase();
                    if book.find(&preset).is_none() {
                        return Err(format!(
                            "line {line}: binding {name} names the preset '{preset}', which the book does not have"
                        ));
                    }
                    if book.binding(&target).is_some() {
                        return Err(format!("line {line}: {name} is bound twice"));
                    }
                    if book.bindings.len() >= MAX_BINDINGS {
                        return Err(format!("a book holds {MAX_BINDINGS} bindings at the most"));
                    }
                    let linked = on_off(get(&kv, "linked"), line, "linked")?.unwrap_or(false);
                    book.bindings.push(Binding {
                        target,
                        preset,
                        linked,
                    });
                    Ok(())
                }
            }
        }
        for (n, raw) in text.lines().enumerate() {
            let line = n + 1;
            let t = raw.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if let Some(head) = t.strip_prefix('[') {
                let head = head
                    .strip_suffix(']')
                    .ok_or_else(|| format!("line {line}: a section head ends with ]"))?;
                let old = std::mem::replace(&mut sec, Sec::None);
                close(&mut book, old)?;
                sec = match head.split_once(' ') {
                    Some(("preset", name)) if !name.trim().is_empty() => {
                        Sec::Preset(Vec::new(), name.trim().to_string(), line)
                    }
                    Some(("bind", name)) if !name.trim().is_empty() => {
                        Sec::Bind(Vec::new(), name.trim().to_string(), line)
                    }
                    _ => {
                        return Err(format!(
                            "line {line}: '[{head}]' is not [preset NAME] or [bind LAYER]"
                        ))
                    }
                };
                continue;
            }
            let (k, v) = t
                .split_once('=')
                .ok_or_else(|| format!("line {line}: '{t}' is not key = value"))?;
            let (k, v) = (k.trim().to_string(), v.trim().to_string());
            let (kv, known): (&mut Vec<(String, String)>, &[&str]) = match &mut sec {
                Sec::None => return Err(format!("line {line}: '{k}' is outside of a section")),
                Sec::Preset(kv, _, _) => (
                    kv,
                    &[
                        "material",
                        "thickness",
                        "what",
                        "power",
                        "speed",
                        "passes",
                        "air",
                        "verified",
                        "alternative",
                    ],
                ),
                Sec::Bind(kv, _, _) => (kv, &["preset", "linked"]),
            };
            if !known.contains(&k.as_str()) {
                return Err(format!("line {line}: '{k}' is not a key of this section"));
            }
            if kv.iter().any(|(a, _)| *a == k) {
                return Err(format!("line {line}: '{k}' is given twice"));
            }
            kv.push((k, v));
        }
        close(&mut book, sec)?;
        Ok(book)
    }

    /// Cuts the link of a target: it keeps the values it has and follows the preset no more. False when the target was not linked.
    pub fn unlink(&mut self, target: &Target) -> bool {
        match self
            .bindings
            .iter_mut()
            .find(|b| b.target == *target && b.linked)
        {
            Some(b) => {
                b.linked = false;
                true
            }
            None => false,
        }
    }

    /// Changes the values of a preset and, with them, every layer linked to it. A value that is left out stays as it was. The preset is marked as not tried on a machine, because values that a
    /// person changed were not the ones that were tried. If the preset or any linked layer cannot take the new values (a limit of the machine, a range) nothing changes at all.
    pub fn change(
        &mut self,
        s: &mut Settings,
        name: &str,
        power_percent: Option<f64>,
        speed_mm_min: Option<f64>,
        passes: Option<u32>,
    ) -> Result<Vec<Update>, String> {
        let at = self
            .presets
            .iter()
            .position(|p| p.name.to_lowercase() == name.trim().to_lowercase())
            .ok_or_else(|| format!("there is no preset named '{}'", name.trim()))?;
        let mut next = self.presets[at].clone();
        if let Some(v) = power_percent {
            next.power_percent = v;
        }
        if let Some(v) = speed_mm_min {
            next.speed_mm_min = v;
        }
        if let Some(v) = passes {
            next.passes = v;
        }
        next.verified = false;
        next.validate()?;
        let mut trial = s.clone();
        let mut updates = Vec::new();
        for b in self
            .bindings
            .iter()
            .filter(|b| b.linked && b.preset == next.name.to_lowercase())
        {
            let Some(slot) = params_mut(&mut trial, &b.target) else {
                continue;
            };
            let from = (slot.power_percent, slot.speed_mm_min, slot.passes);
            *slot = next.apply_to(slot);
            updates.push(Update {
                target: b.target.clone(),
                from,
                to: (slot.power_percent, slot.speed_mm_min, slot.passes),
            });
        }
        trial.validate().map_err(|e| {
            format!(
                "preset {} cannot change: {e} (nothing was changed)",
                next.name
            )
        })?;
        *s = trial;
        self.presets[at] = next;
        Ok(updates)
    }

    /// What the book shows, one line for each binding in the language given, and the warning of every preset that was not tried and is in use. A layer that is linked says so.
    pub fn describe(&self, lang: Lang) -> Vec<String> {
        let mut lines = Vec::new();
        for b in &self.bindings {
            let Some(p) = self.find(&b.preset) else {
                continue;
            };
            let how = if b.linked {
                Msg::PresetLinked
            } else {
                Msg::PresetAssigned
            };
            lines.push(format!(
                "{} <- {} [{}]: {} %, {} mm/min, {}x",
                b.target.name(),
                p.name,
                how.tr(lang),
                p.power_percent,
                p.speed_mm_min,
                p.passes
            ));
        }
        let mut warned: Vec<&str> = Vec::new();
        for b in &self.bindings {
            if let Some(p) = self.find(&b.preset) {
                if !warned.contains(&p.name.as_str()) {
                    warned.push(&p.name);
                    lines.extend(p.warning(lang));
                }
            }
        }
        lines
    }

    /// The line that tells which layers a change reached, in a language.
    pub fn updated_text(name: &str, updates: &[Update], lang: Lang) -> String {
        fill(
            Msg::PresetUpdated.tr(lang),
            &[name, &updates.len().to_string()],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str) -> Preset {
        Preset {
            name: name.to_string(),
            material: "wood/test".to_string(),
            thickness_mm: Some(3.0),
            what: What::Cut,
            power_percent: 60.0,
            speed_mm_min: 300.0,
            passes: 2,
            air_assist: None,
            verified: false,
            alternative: false,
        }
    }

    #[test]
    fn a_preset_is_checked_and_its_name_is_unique() {
        let mut book = Book::new();
        assert!(book.add(preset("Birch 3")).is_ok());
        assert!(book.add(preset("birch 3")).is_err());
        let mut bad = preset("x");
        bad.power_percent = 0.0;
        assert!(book.add(bad).is_err());
        let mut bad = preset("y");
        bad.passes = 0;
        assert!(book.add(bad).is_err());
        assert!(book.add(preset("")).is_err());
        assert!(book.add(preset(&"n".repeat(MAX_NAME_CHARS + 1))).is_err());
    }

    #[test]
    fn the_warning_is_in_the_eight_languages_and_only_for_values_not_tried() {
        let mut p = preset("birch");
        for l in Lang::ALL {
            let w = p.warning(l).unwrap();
            assert!(w.contains("birch"), "{w}");
        }
        p.verified = true;
        assert!(p.warning(Lang::En).is_none());
    }
}
