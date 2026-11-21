// EH3D-PHOENIX-LASER - several passes over one layer, each with its own settings.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A layer may be worked more than once with different settings: a fill and then an outline, a light mark and then a deep cut. Each extra pass is a sub-layer (`[sublayer LAYER/LABEL]` in
//! the settings file): it works the same shapes as its layer, starts from the settings of the layer and changes what it says. The layer itself is the first operation, the sub-layers follow it
//! in the order of the file. The order of the whole work is the one of the layers: engraving comes before cutting, then the lowest `order`, then the name, and a sub-layer never goes before
//! its own layer when the rest is equal. Every sub-layer is checked against the limits of the machine like a layer is, and a value over a limit is an error that names the layer and the label.

use crate::vconfig::{LayerMode, LayerParams, Settings};
use crate::vector::Drawing;

/// The most sub-layers one layer may have.
pub const MAX_SUBLAYERS_PER_LAYER: usize = 8;

/// The most sub-layers a settings file may describe.
pub const MAX_SUBLAYERS: usize = 512;

/// The longest label of a sub-layer.
pub const MAX_LABEL_CHARS: usize = 40;

/// One extra pass over a layer.
#[derive(Debug, Clone, PartialEq)]
pub struct SubLayer {
    /// The layer it works, in lower case.
    pub layer: String,
    /// The label that tells it from the other passes of the layer, in lower case.
    pub label: String,
    pub params: LayerParams,
}

/// One operation of the work: a layer or one of its sub-layers.
#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    /// The layer, as the drawing names it.
    pub layer: String,
    /// The label of the sub-layer; empty for the layer itself.
    pub label: String,
    pub params: LayerParams,
}

impl Operation {
    /// The name the program writes in the comment that opens the operation: `red` for a layer, `red/outline` for a sub-layer.
    pub fn name(&self) -> String {
        if self.label.is_empty() {
            self.layer.clone()
        } else {
            format!("{}/{}", self.layer, self.label)
        }
    }
}

/// Splits the name of a section `LAYER/LABEL` at its last slash. A name without a label, or with an empty layer, is an error.
pub fn split_name(name: &str) -> Result<(String, String), String> {
    let (layer, label) = name
        .trim()
        .rsplit_once('/')
        .ok_or_else(|| format!("'{}' is not LAYER/LABEL", name.trim()))?;
    let (layer, label) = (layer.trim().to_lowercase(), label.trim().to_lowercase());
    if layer.is_empty() {
        return Err(format!("'{}' names no layer before the slash", name.trim()));
    }
    check_label(&label)?;
    Ok((layer, label))
}

/// A label is 1 to [`MAX_LABEL_CHARS`] letters, figures, hyphens or underscores.
pub fn check_label(label: &str) -> Result<(), String> {
    if label.is_empty() || label.chars().count() > MAX_LABEL_CHARS {
        return Err(format!(
            "the label of a sub-layer is {} characters: it must be from 1 to {MAX_LABEL_CHARS}",
            label.chars().count()
        ));
    }
    if !label
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "the label '{label}' of a sub-layer holds a character that is not a letter, a figure, - or _"
        ));
    }
    Ok(())
}

/// The sub-layers of a layer, in the order of the file.
pub fn of<'a>(s: &'a Settings, layer: &str) -> impl Iterator<Item = &'a SubLayer> {
    let key = layer.trim().to_lowercase();
    s.sublayers.iter().filter(move |sub| sub.layer == key)
}

/// The operations of a layer: the layer first, then each sub-layer that is not skipped.
pub fn operations_of(s: &Settings, layer: &str) -> Vec<Operation> {
    let mut out = vec![Operation {
        layer: layer.to_string(),
        label: String::new(),
        params: s.params_for(layer),
    }];
    out.extend(
        of(s, layer)
            .filter(|sub| sub.params.mode != LayerMode::Skip)
            .map(|sub| Operation {
                layer: layer.to_string(),
                label: sub.label.clone(),
                params: sub.params,
            }),
    );
    out
}

/// What the order of the work is sorted by: engraving before cutting, the lowest `order`, the name of the layer and the place of the operation in its layer.
pub fn order_key<'a>(
    params: &LayerParams,
    layer: &'a str,
    seq: usize,
) -> (usize, i32, &'a str, usize) {
    (
        usize::from(params.mode == LayerMode::Cut),
        params.order,
        layer,
        seq,
    )
}

/// The operations of a drawing in the order they are worked, as the program of `plan` writes them: the layers that are skipped are not in it, and neither are their sub-layers.
pub fn execution_order(drawing: &Drawing, s: &Settings) -> Vec<Operation> {
    let mut all: Vec<(usize, Operation)> = Vec::new();
    for info in crate::cut::layers(drawing, s) {
        if s.params_for(&info.key).mode == LayerMode::Skip {
            continue;
        }
        for (seq, op) in operations_of(s, &info.key).into_iter().enumerate() {
            all.push((seq, op));
        }
    }
    all.sort_by(|(sa, a), (sb, b)| {
        order_key(&a.params, &a.layer, *sa).cmp(&order_key(&b.params, &b.layer, *sb))
    });
    all.into_iter().map(|(_, op)| op).collect()
}

/// Checks the sub-layers of the settings: how many there are, that each works a layer the file describes, and that each is valid and inside the limits of the machine.
pub fn validate(s: &Settings) -> Result<(), String> {
    if s.sublayers.len() > MAX_SUBLAYERS {
        return Err(format!(
            "the settings describe more than {MAX_SUBLAYERS} sub-layers"
        ));
    }
    let m = &s.machine;
    for (i, sub) in s.sublayers.iter().enumerate() {
        let name = format!("{}/{}", sub.layer, sub.label);
        check_label(&sub.label).map_err(|e| format!("sublayer {name}: {e}"))?;
        if !s.layers.iter().any(|(n, _)| *n == sub.layer) {
            return Err(format!(
                "sublayer {name}: the layer '{}' has no [layer {}] section",
                sub.layer, sub.layer
            ));
        }
        if s.sublayers[..i]
            .iter()
            .any(|o| o.layer == sub.layer && o.label == sub.label)
        {
            return Err(format!("sublayer {name} is described twice"));
        }
        if of(s, &sub.layer).count() > MAX_SUBLAYERS_PER_LAYER {
            return Err(format!(
                "layer {}: more than {MAX_SUBLAYERS_PER_LAYER} sub-layers",
                sub.layer
            ));
        }
        sub.params
            .validate(m.max_passes)
            .map_err(|e| format!("sublayer {name}: {e}"))?;
        if sub.params.mode != LayerMode::Skip {
            crate::safety::check_power("power", sub.params.power_percent, m.max_power_percent)
                .map_err(|e| format!("sublayer {name}: {e}"))?;
            crate::safety::check_speed("speed", sub.params.speed_mm_min, m.max_speed_mm_min)
                .map_err(|e| format!("sublayer {name}: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_name_is_split_at_its_last_slash_and_the_label_is_checked() {
        assert_eq!(
            split_name(" Red/Outline ").unwrap(),
            ("red".to_string(), "outline".to_string())
        );
        // a layer whose name holds a slash keeps it
        assert_eq!(
            split_name("a/b/fill").unwrap(),
            ("a/b".to_string(), "fill".to_string())
        );
        for bad in ["red", "/fill", "red/", "red/a b", "red/é"] {
            assert!(split_name(bad).is_err(), "{bad}");
        }
        assert!(check_label(&"x".repeat(MAX_LABEL_CHARS)).is_ok());
        assert!(check_label(&"x".repeat(MAX_LABEL_CHARS + 1)).is_err());
    }

    #[test]
    fn the_operations_of_a_layer_start_with_the_layer_and_leave_out_a_skipped_pass() {
        let s = crate::vconfig::parse(
            "[layer red]\nmode = cut\n[sublayer red/a]\nmode = engrave\n[sublayer red/b]\nmode = skip\n[sublayer red/c]\npower = 20\n",
        )
        .unwrap();
        let ops = operations_of(&s, "red");
        let names: Vec<String> = ops.iter().map(Operation::name).collect();
        assert_eq!(names, ["red", "red/a", "red/c"]);
        // a pass starts from the settings of its layer
        assert_eq!(ops[2].params.mode, LayerMode::Cut);
        assert_eq!(ops[2].params.power_percent, 20.0);
    }
}
