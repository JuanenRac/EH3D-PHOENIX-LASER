// EH3D-PHOENIX-LASER - a result of a test and another library put into a library of materials.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Two ways a library of materials grows: the square a person picks on a test grid (power, speed, passes) or the width of the beam measured on its test piece is written into a material, and the
//! materials of another library are brought in. Nothing is ever written over without being asked: a value that differs from the one the library has is a clash that stops the edit until the
//! person says which one stays. Whatever is written this way is marked `verified = off`, and a `max_power` or a `max_speed` is never raised. The functions here work on the text of a file, so
//! that its comments, its order and its other materials stay as they were; the caller writes the file.

use crate::materials::{self, Material};
use std::path::Path;

/// What was measured on a test piece; `None` leaves that value of the material as it is.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Measured {
    /// A percent of full power.
    pub power: Option<f64>,
    /// mm/min.
    pub speed: Option<f64>,
    pub passes: Option<u32>,
    /// The width of the beam (mm).
    pub kerf: Option<f64>,
}

impl Measured {
    fn pairs(&self) -> Vec<(&'static str, String)> {
        [
            ("power", self.power.map(|v| v.to_string())),
            ("speed", self.speed.map(|v| v.to_string())),
            ("passes", self.passes.map(|v| v.to_string())),
            ("kerf", self.kerf.map(|v| v.to_string())),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect()
    }
}

/// A value the library has that the new one would write over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clash {
    pub key: &'static str,
    pub old: String,
    pub new: String,
}

/// What recording a result did to the text of a library.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    /// The text of the library as it must be written; the text that was given when nothing changed.
    pub text: String,
    /// The material was not in the library.
    pub added: bool,
    /// The keys that were written.
    pub changed: Vec<&'static str>,
    /// The values that stopped the edit; nothing was changed when there is one.
    pub clashes: Vec<Clash>,
}

fn without_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// The text of a library that is going to be rewritten: it has to be UTF-8 and no larger than a library may be.
pub fn read_text(path: &Path) -> Result<String, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > materials::MAX_FILE_BYTES {
        return Err(format!(
            "{shown}: the library is larger than {} bytes",
            materials::MAX_FILE_BYTES
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{shown}: {e}"))?;
    String::from_utf8(bytes)
        .map_err(|_| format!("{shown}: the library is not UTF-8 text, so it is not rewritten"))
}

/// Writes what was measured into the material `name` of the library `text`, adding the material when it is not there. A value that differs from one the material has is a clash: without
/// `replace` nothing is changed. A material that is written is marked `verified = off`, and one over its own `max_power` or `max_speed` is an error.
pub fn record(text: &str, name: &str, m: &Measured, replace: bool) -> Result<Recorded, String> {
    let name = name.trim();
    materials::check_name(name)?;
    let pairs = m.pairs();
    if pairs.is_empty() {
        return Err(
            "there is nothing to write: give a power, a speed, passes or the width of the beam"
                .to_string(),
        );
    }
    if m.kerf.is_some_and(|k| k <= 0.0) {
        return Err("the width of the beam must be above 0".to_string());
    }
    let library = materials::parse(without_bom(text))?;
    let old = library.find(name);
    let mut made = old.cloned().unwrap_or_else(|| Material {
        name: name.to_string(),
        ..Material::default()
    });
    let (mut changed, mut clashes) = (Vec::new(), Vec::new());
    for (key, value) in pairs {
        let before = materials::value_of(&made, key);
        materials::set_value(&mut made, key, &value)?;
        let after = materials::value_of(&made, key);
        if before == after {
            continue;
        }
        changed.push(key);
        if let (Some(old), Some(new)) = (before, after) {
            clashes.push(Clash { key, old, new });
        }
    }
    let unchanged = Recorded {
        text: text.to_string(),
        added: false,
        changed: Vec::new(),
        clashes: Vec::new(),
    };
    if !clashes.is_empty() && !replace {
        return Ok(Recorded {
            clashes,
            ..unchanged
        });
    }
    if changed.is_empty() {
        return Ok(unchanged);
    }
    made.verified = Some(false);
    made.check_limits(made.power, made.speed)?;
    let replaced = old.map(|o| o.name.clone());
    Ok(Recorded {
        text: materials::upsert_text(text, replaced.as_deref(), &made)?,
        added: replaced.is_none(),
        changed,
        clashes: Vec::new(),
    })
}

/// What is done with a material that both libraries have with other values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnClash {
    /// Nothing at all is written, so that the person is asked.
    Stop,
    KeepMine,
    TakeTheirs,
}

/// What bringing a library into another did.
#[derive(Debug, Clone, PartialEq)]
pub struct Merged {
    /// The text of the library as it must be written; the text that was given when nothing changed.
    pub text: String,
    /// The names of the materials that were not in the library.
    pub added: Vec<String>,
    /// The names both libraries have with the same values.
    pub same: Vec<String>,
    /// The names both libraries have with other values.
    pub clashed: Vec<String>,
    /// The clashes stopped the merge: nothing was changed.
    pub stopped: bool,
}

/// Whether two materials give the same values and the same limits (their notes and whether they were verified aside).
fn same_values(a: &Material, b: &Material) -> bool {
    materials::KEYS
        .iter()
        .all(|k| materials::value_of(a, k) == materials::value_of(b, k))
        && a.max_power == b.max_power
        && a.max_speed == b.max_speed
}

/// The lower of two ceilings; the one there is when only one of them says it.
fn lower(a: Option<f64>, b: Option<f64>) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// Brings the materials of the library `theirs` into the library `mine`. What comes in is marked `verified = off`, since it was not tried here; a material that is taken in place of one of the
/// person keeps the lower `max_power` and `max_speed` of the two, and values over them are an error.
pub fn merge(mine: &str, theirs: &str, on_clash: OnClash) -> Result<Merged, String> {
    let have = materials::parse(without_bom(mine))?;
    let other =
        materials::parse(without_bom(theirs)).map_err(|e| format!("the other library: {e}"))?;
    let mut out = Merged {
        text: mine.to_string(),
        added: Vec::new(),
        same: Vec::new(),
        clashed: Vec::new(),
        stopped: false,
    };
    for m in &other.materials {
        match have.find(&m.name) {
            None => out.added.push(m.name.clone()),
            Some(old) if same_values(old, m) => out.same.push(m.name.clone()),
            Some(old) => out.clashed.push(old.name.clone()),
        }
    }
    if on_clash == OnClash::Stop && !out.clashed.is_empty() {
        out.stopped = true;
        return Ok(out);
    }
    for m in &other.materials {
        let old = have.find(&m.name);
        let replaced = match old {
            None => None,
            Some(old) if same_values(old, m) || on_clash != OnClash::TakeTheirs => continue,
            Some(old) => Some(old),
        };
        let mut made = m.clone();
        made.verified = Some(false);
        if let Some(old) = replaced {
            made.name = old.name.clone();
            made.max_power = lower(old.max_power, m.max_power);
            made.max_speed = lower(old.max_speed, m.max_speed);
            made.check_limits(made.power, made.speed)?;
        }
        out.text = materials::upsert_text(&out.text, replaced.map(|o| o.name.as_str()), &made)?;
    }
    Ok(out)
}
