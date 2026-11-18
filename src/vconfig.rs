//! The settings of the work with vector drawings, kept in a plain text file so that nothing that was tuned lives in the code: the limits of the readers, how a file is read, how the
//! cuts are ordered, the machine (full power, overscan, travel speed, the bed) and the settings of each layer or colour (cut or engrave, power, speed, passes, the step of the focus,
//! the width of the beam, the tabs and the hatch).
//!
//! The file is a list of `key = value` lines in sections (`[limits]`, `[read]`, `[order]`, `[offset]`, `[hatch]`, `[machine]`, `[defaults]` and one `[layer NAME]` for each layer or
//! colour); a line that starts with `#` or `;` is a note. A key that is not known is an error that names its line: a typing mistake never changes nothing silently.

use crate::vector::{GroupBy, Limits, Pt, ReadOptions};
use std::path::Path;

/// The largest settings file that is read.
pub const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

/// The most layers a settings file may describe.
pub const MAX_LAYERS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerMode {
    /// Follow the contours with the laser.
    Cut,
    /// Fill the closed shapes with lines (hatch) and follow the open ones.
    Engrave,
    Skip,
}

impl LayerMode {
    pub fn from_word(word: &str) -> Option<LayerMode> {
        match word.trim().to_ascii_lowercase().as_str() {
            "cut" => Some(LayerMode::Cut),
            "engrave" => Some(LayerMode::Engrave),
            "skip" | "off" => Some(LayerMode::Skip),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            LayerMode::Cut => "cut",
            LayerMode::Engrave => "engrave",
            LayerMode::Skip => "skip",
        }
    }
}

/// On which side of the line the beam travels, so that the width of the beam (the kerf) does not change the size of the piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerfSide {
    /// The beam follows the line.
    None,
    /// The piece keeps its size: the beam goes outside the outline of a piece and inside the outline of a hole.
    Auto,
    /// Always inside every closed contour.
    Inside,
    /// Always outside every closed contour.
    Outside,
}

impl KerfSide {
    pub fn from_word(word: &str) -> Option<KerfSide> {
        match word.trim().to_ascii_lowercase().as_str() {
            "none" | "off" => Some(KerfSide::None),
            "auto" => Some(KerfSide::Auto),
            "inside" | "in" => Some(KerfSide::Inside),
            "outside" | "out" => Some(KerfSide::Outside),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            KerfSide::None => "none",
            KerfSide::Auto => "auto",
            KerfSide::Inside => "inside",
            KerfSide::Outside => "outside",
        }
    }
}

/// What is done with the shapes of one layer or colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerParams {
    pub mode: LayerMode,
    /// A percent of full power, more than 0 and at most 100.
    pub power_percent: f64,
    pub speed_mm_min: f64,
    pub passes: u32,
    /// How far the focus moves on at each pass (mm; the Z axis goes by this much, a negative value goes down). 0 leaves it where it is.
    pub focus_step_mm: f64,
    /// The width of the beam (mm); the line is moved by half of it to the side `kerf_side` says.
    pub kerf_mm: f64,
    pub kerf_side: KerfSide,
    /// Bridges that are left uncut on the outline of a piece so that it does not fall (0 for none).
    pub tabs: u32,
    pub tab_width_mm: f64,
    /// The angle of the lines of the hatch, in degrees.
    pub hatch_angle_deg: f64,
    pub hatch_spacing_mm: f64,
    /// The layers are done in this order (the lowest first); engraving always comes before cutting.
    pub order: i32,
}

impl Default for LayerParams {
    fn default() -> Self {
        LayerParams {
            mode: LayerMode::Cut,
            power_percent: 50.0,
            speed_mm_min: 600.0,
            passes: 1,
            focus_step_mm: 0.0,
            kerf_mm: 0.0,
            kerf_side: KerfSide::None,
            tabs: 0,
            tab_width_mm: 1.0,
            hatch_angle_deg: 0.0,
            hatch_spacing_mm: 0.1,
            order: 0,
        }
    }
}

impl LayerParams {
    /// Checks the values the way the engraving checks its own: a number outside its range is an error that names it.
    pub fn validate(&self, max_passes: u32) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        if self.mode == LayerMode::Skip {
            return Ok(());
        }
        range("power", self.power_percent, 0.001, 100.0)?;
        range("speed", self.speed_mm_min, 1.0, 30_000.0)?;
        range("focus_step", self.focus_step_mm, -20.0, 20.0)?;
        range("kerf", self.kerf_mm, 0.0, 5.0)?;
        range("tab_width", self.tab_width_mm, 0.0, 100.0)?;
        range("hatch_angle", self.hatch_angle_deg, -360.0, 360.0)?;
        range("hatch_spacing", self.hatch_spacing_mm, 0.01, 5.0)?;
        if self.passes == 0 || self.passes > max_passes {
            return Err(format!(
                "passes is {}: it must be from 1 to {max_passes}",
                self.passes
            ));
        }
        if self.tabs > 100 {
            return Err(format!("tabs is {}: it must be from 0 to 100", self.tabs));
        }
        if self.focus_step_mm.abs() * f64::from(self.passes - 1) > 100.0 {
            return Err("the focus would move more than 100 mm over the passes".to_string());
        }
        Ok(())
    }
}

/// The laser and its bed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// The spindle value that is full power for the controller (`$30` of GRBL).
    pub s_max: u32,
    /// How far the head runs past the end of a hatch line, so that it is at speed when the laser is on.
    pub overscan_mm: f64,
    /// The speed of the moves with the laser off, only to estimate the time.
    pub rapid_mm_min: f64,
    /// The optical power of the laser at full power, in watts; 0 when it is not known (the energy of a layer is then not worked out).
    pub laser_watts: f64,
    /// Where the lower left corner of the work goes.
    pub origin: Pt,
    /// Where the focus (Z) is for the first pass.
    pub focus_z_mm: f64,
    /// The size of the bed, when it is known; the work may not go outside of it.
    pub bed_mm: Option<Pt>,
    /// The most passes a layer may ask for.
    pub max_passes: u32,
    /// The limit of power, as a percent of full power: a layer that asks for more is an error, never lowered without a word.
    pub max_power_percent: f64,
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            s_max: 1000,
            overscan_mm: 2.0,
            rapid_mm_min: 6000.0,
            laser_watts: 0.0,
            origin: (0.0, 0.0),
            focus_z_mm: 0.0,
            bed_mm: None,
            max_passes: 50,
            max_power_percent: crate::safety::DEFAULT_MAX_POWER_PERCENT,
        }
    }
}

/// The frame of the work (`phoenix-laser frame`): the rectangle that the head traces to show where the work will be. The laser stays off unless `pilot_percent` is more than 0 and the
/// command is also told `--confirm-laser-on`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSettings {
    pub feed_mm_min: f64,
    /// How many times the rectangle is traced.
    pub laps: u32,
    /// How far outside of the work the rectangle goes.
    pub margin_mm: f64,
    /// A small power to make the frame visible; 0 keeps the laser off.
    pub pilot_percent: f64,
}

impl Default for FrameSettings {
    fn default() -> Self {
        FrameSettings {
            feed_mm_min: 3000.0,
            laps: 1,
            margin_mm: 0.0,
            pilot_percent: 0.0,
        }
    }
}

impl FrameSettings {
    pub fn validate(&self, max_power_percent: f64) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("frame feed_mm_min", self.feed_mm_min, 1.0, 30_000.0)?;
        range("frame margin_mm", self.margin_mm, 0.0, 100.0)?;
        range(
            "frame pilot_percent",
            self.pilot_percent,
            0.0,
            crate::safety::MAX_PILOT_PERCENT,
        )?;
        if self.laps == 0 || self.laps > 20 {
            return Err(format!(
                "frame laps is {}: it must be from 1 to 20",
                self.laps
            ));
        }
        if self.pilot_percent > 0.0 {
            crate::safety::check_power(
                "frame pilot_percent",
                self.pilot_percent,
                max_power_percent,
            )?;
        }
        Ok(())
    }
}

/// The focus test (`phoenix-laser focus`): a ladder of short lines burnt at different heights (Z) to find where the beam is sharpest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusSettings {
    /// The height of the first line.
    pub z_start_mm: f64,
    /// How much the height changes from a line to the next. It is 0 until the person says it, and then the test is refused: which way is down depends on the machine.
    pub z_step_mm: f64,
    pub steps: u32,
    pub length_mm: f64,
    /// The distance between two lines.
    pub spacing_mm: f64,
    pub feed_mm_min: f64,
    pub power_percent: f64,
}

impl Default for FocusSettings {
    fn default() -> Self {
        FocusSettings {
            z_start_mm: 0.0,
            z_step_mm: 0.0,
            steps: 9,
            length_mm: 10.0,
            spacing_mm: 2.0,
            feed_mm_min: 1000.0,
            power_percent: 30.0,
        }
    }
}

impl FocusSettings {
    /// The ranges. The limit of power is checked when the test is made, not here: a file with a low limit and no `[focus]` is still a good file.
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("focus z_start_mm", self.z_start_mm, -100.0, 100.0)?;
        range("focus z_step_mm", self.z_step_mm, -20.0, 20.0)?;
        range("focus length_mm", self.length_mm, 1.0, 100.0)?;
        range("focus spacing_mm", self.spacing_mm, 0.5, 20.0)?;
        range("focus feed_mm_min", self.feed_mm_min, 1.0, 30_000.0)?;
        range("focus power_percent", self.power_percent, 0.001, 100.0)?;
        if self.steps < 2 || self.steps > 30 {
            return Err(format!(
                "focus steps is {}: it must be from 2 to 30",
                self.steps
            ));
        }
        if self.z_step_mm.abs() * f64::from(self.steps - 1) > 50.0 {
            return Err("the focus would move more than 50 mm over the test".to_string());
        }
        Ok(())
    }
}

/// Where the lower left corner of the drawing goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// The lower left corner of the shapes is the origin (like a picture).
    Bbox,
    /// The page of the file is kept: its lower left corner is the origin when the file says its size.
    Page,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub read: ReadOptions,
    pub group_by: GroupBy,
    pub align: Align,
    /// The ends of two open paths closer than this (mm) are joined.
    pub join_tolerance_mm: f64,
    pub join_max_paths: usize,
    pub order_max_items: usize,
    pub containment_max_items: usize,
    pub two_opt: bool,
    /// The most moves the improvement of the order tries.
    pub two_opt_budget: u64,
    /// The 2-opt improvement is not tried for more contours than this.
    pub two_opt_max_items: usize,
    /// A contour with more points than this is not offset.
    pub offset_max_points: usize,
    pub max_hatch_lines: usize,
    pub machine: Machine,
    pub frame: FrameSettings,
    pub focus: FocusSettings,
    pub defaults: LayerParams,
    /// The settings of the layers that the file names, by name (lower case).
    pub layers: Vec<(String, LayerParams)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            read: ReadOptions::default(),
            group_by: GroupBy::Auto,
            align: Align::Bbox,
            join_tolerance_mm: 0.01,
            join_max_paths: 5_000,
            order_max_items: 20_000,
            containment_max_items: 1_500,
            two_opt: true,
            two_opt_budget: 20_000_000,
            two_opt_max_items: 2_000,
            offset_max_points: 5_000,
            max_hatch_lines: 100_000,
            machine: Machine::default(),
            frame: FrameSettings::default(),
            focus: FocusSettings::default(),
            defaults: LayerParams::default(),
            layers: Vec::new(),
        }
    }
}

impl Settings {
    /// The settings of a layer: the ones the file names for it, or the defaults.
    pub fn params_for(&self, key: &str) -> LayerParams {
        let k = key.trim().to_lowercase();
        self.layers
            .iter()
            .find(|(name, _)| *name == k)
            .map_or(self.defaults, |(_, p)| *p)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.read.validate()?;
        let m = &self.machine;
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        if m.s_max == 0 || m.s_max > 100_000 {
            return Err(format!("s_max is {}: it must be from 1 to 100000", m.s_max));
        }
        if m.max_passes == 0 || m.max_passes > 1_000 {
            return Err(format!(
                "max_passes is {}: it must be from 1 to 1000",
                m.max_passes
            ));
        }
        crate::safety::validate_max(m.max_power_percent)?;
        range("overscan", m.overscan_mm, 0.0, 50.0)?;
        range("rapid", m.rapid_mm_min, 1.0, 100_000.0)?;
        range("laser_watts", m.laser_watts, 0.0, 1_000.0)?;
        range("origin x", m.origin.0, -100_000.0, 100_000.0)?;
        range("origin y", m.origin.1, -100_000.0, 100_000.0)?;
        range("focus_z", m.focus_z_mm, -1_000.0, 1_000.0)?;
        if let Some((bx, by)) = m.bed_mm {
            range("bed x", bx, 1.0, 100_000.0)?;
            range("bed y", by, 1.0, 100_000.0)?;
        }
        range("join_tolerance", self.join_tolerance_mm, 0.0, 1.0)?;
        for (name, v, hi) in [
            ("join_max_paths", self.join_max_paths, 1_000_000),
            ("order_max_items", self.order_max_items, 1_000_000),
            ("two_opt_max_items", self.two_opt_max_items, 100_000),
            ("containment_max_items", self.containment_max_items, 100_000),
            ("offset_max_points", self.offset_max_points, 1_000_000),
            ("max_hatch_lines", self.max_hatch_lines, 10_000_000),
        ] {
            if v == 0 || v > hi {
                return Err(format!("{name} is {v}: it must be from 1 to {hi}"));
            }
        }
        if self.two_opt_budget > 10_000_000_000 {
            return Err("two_opt_budget is too large".to_string());
        }
        self.frame.validate(m.max_power_percent)?;
        self.focus.validate()?;
        self.defaults.validate(m.max_passes)?;
        if self.defaults.mode != LayerMode::Skip {
            crate::safety::check_power("power", self.defaults.power_percent, m.max_power_percent)
                .map_err(|e| format!("[defaults]: {e}"))?;
        }
        if self.layers.len() > MAX_LAYERS {
            return Err(format!(
                "the settings describe more than {MAX_LAYERS} layers"
            ));
        }
        for (name, p) in &self.layers {
            p.validate(m.max_passes)
                .map_err(|e| format!("layer {name}: {e}"))?;
            if p.mode != LayerMode::Skip {
                crate::safety::check_power("power", p.power_percent, m.max_power_percent)
                    .map_err(|e| format!("layer {name}: {e}"))?;
            }
        }
        Ok(())
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

fn whole(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("'{}' is not a whole number", value.trim()))
}

fn switch(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        other => Err(format!("'{other}' is not on or off")),
    }
}

fn to_usize(v: u64) -> Result<usize, String> {
    usize::try_from(v).map_err(|_| format!("{v} is too large"))
}

fn set_layer(p: &mut LayerParams, key: &str, value: &str) -> Result<(), String> {
    match key {
        "mode" => {
            p.mode = LayerMode::from_word(value)
                .ok_or_else(|| format!("'{}' is not cut, engrave or skip", value.trim()))?
        }
        "power" => p.power_percent = number(value)?,
        "speed" => p.speed_mm_min = number(value)?,
        "passes" => {
            p.passes =
                u32::try_from(whole(value)?).map_err(|_| "passes is too large".to_string())?
        }
        "focus_step" => p.focus_step_mm = number(value)?,
        "kerf" => p.kerf_mm = number(value)?,
        "kerf_side" => {
            p.kerf_side = KerfSide::from_word(value)
                .ok_or_else(|| format!("'{}' is not none, auto, inside or outside", value.trim()))?
        }
        "tabs" => {
            p.tabs = u32::try_from(whole(value)?).map_err(|_| "tabs is too large".to_string())?
        }
        "tab_width" => p.tab_width_mm = number(value)?,
        "hatch_angle" => p.hatch_angle_deg = number(value)?,
        "hatch_spacing" => p.hatch_spacing_mm = number(value)?,
        "order" => p.order = number(value)?.clamp(-1_000_000.0, 1_000_000.0) as i32,
        _ => return Err(format!("'{key}' is not a setting of a layer")),
    }
    Ok(())
}

fn set_global(s: &mut Settings, section: &str, key: &str, value: &str) -> Result<(), String> {
    let lim: &mut Limits = &mut s.read.limits;
    match (section, key) {
        ("limits", "max_file_bytes") => lim.max_file_bytes = to_usize(whole(value)?)?,
        ("limits", "max_depth") => lim.max_depth = to_usize(whole(value)?)?,
        ("limits", "max_nodes") => lim.max_nodes = to_usize(whole(value)?)?,
        ("limits", "max_shapes") => lim.max_shapes = to_usize(whole(value)?)?,
        ("limits", "max_points") => lim.max_points = to_usize(whole(value)?)?,
        ("limits", "max_curve_segments") => lim.max_curve_segments = to_usize(whole(value)?)?,
        ("limits", "max_coordinate_mm") => lim.max_coordinate_mm = number(value)?,
        ("read", "dpi") => s.read.dpi = number(value)?,
        ("read", "tolerance_mm") => s.read.tolerance_mm = number(value)?,
        ("read", "dxf_unit_mm") => s.read.dxf_unit_mm = number(value)?,
        ("read", "group_by") => {
            s.group_by = GroupBy::from_word(value)
                .ok_or_else(|| format!("'{}' is not auto, layer or color", value.trim()))?
        }
        ("read", "align") => {
            s.align = match value.trim().to_ascii_lowercase().as_str() {
                "bbox" => Align::Bbox,
                "page" => Align::Page,
                other => return Err(format!("'{other}' is not bbox or page")),
            }
        }
        ("order", "join_tolerance_mm") => s.join_tolerance_mm = number(value)?,
        ("order", "join_max_paths") => s.join_max_paths = to_usize(whole(value)?)?,
        ("order", "order_max_items") => s.order_max_items = to_usize(whole(value)?)?,
        ("order", "containment_max_items") => s.containment_max_items = to_usize(whole(value)?)?,
        ("order", "two_opt") => s.two_opt = switch(value)?,
        ("order", "two_opt_max_items") => s.two_opt_max_items = to_usize(whole(value)?)?,
        ("order", "two_opt_budget") => s.two_opt_budget = whole(value)?,
        ("offset", "max_points") => s.offset_max_points = to_usize(whole(value)?)?,
        ("hatch", "max_lines") => s.max_hatch_lines = to_usize(whole(value)?)?,
        ("machine", "s_max") => {
            s.machine.s_max =
                u32::try_from(whole(value)?).map_err(|_| "s_max is too large".to_string())?
        }
        ("machine", "overscan_mm") => s.machine.overscan_mm = number(value)?,
        ("machine", "rapid_mm_min") => s.machine.rapid_mm_min = number(value)?,
        ("machine", "laser_watts") => s.machine.laser_watts = number(value)?,
        ("machine", "origin_x") => s.machine.origin.0 = number(value)?,
        ("machine", "origin_y") => s.machine.origin.1 = number(value)?,
        ("machine", "focus_z_mm") => s.machine.focus_z_mm = number(value)?,
        ("machine", "bed_x_mm") => {
            let v = number(value)?;
            let y = s.machine.bed_mm.map_or(0.0, |b| b.1);
            s.machine.bed_mm = (v > 0.0 || y > 0.0).then_some((v, y));
        }
        ("machine", "bed_y_mm") => {
            let v = number(value)?;
            let x = s.machine.bed_mm.map_or(0.0, |b| b.0);
            s.machine.bed_mm = (v > 0.0 || x > 0.0).then_some((x, v));
        }
        ("focus", "z_start_mm") => s.focus.z_start_mm = number(value)?,
        ("focus", "z_step_mm") => s.focus.z_step_mm = number(value)?,
        ("focus", "steps") => {
            s.focus.steps =
                u32::try_from(whole(value)?).map_err(|_| "steps is too large".to_string())?
        }
        ("focus", "length_mm") => s.focus.length_mm = number(value)?,
        ("focus", "spacing_mm") => s.focus.spacing_mm = number(value)?,
        ("focus", "feed_mm_min") => s.focus.feed_mm_min = number(value)?,
        ("focus", "power_percent") => s.focus.power_percent = number(value)?,
        ("frame", "feed_mm_min") => s.frame.feed_mm_min = number(value)?,
        ("frame", "laps") => {
            s.frame.laps =
                u32::try_from(whole(value)?).map_err(|_| "laps is too large".to_string())?
        }
        ("frame", "margin_mm") => s.frame.margin_mm = number(value)?,
        ("frame", "pilot_percent") => s.frame.pilot_percent = number(value)?,
        ("machine", "max_power_percent") => s.machine.max_power_percent = number(value)?,
        ("machine", "max_passes") => {
            s.machine.max_passes =
                u32::try_from(whole(value)?).map_err(|_| "max_passes is too large".to_string())?
        }
        _ => return Err(format!("'{key}' is not a setting of [{section}]")),
    }
    Ok(())
}

/// Reads the text of a settings file. The values are checked, so a file that gives a power of 500 % is refused here.
pub fn parse(text: &str) -> Result<Settings, String> {
    struct Entry<'a> {
        section: String,
        line: usize,
        key: &'a str,
        value: &'a str,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut named: Vec<String> = Vec::new();
    let mut section = String::new();
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
            let (word, rest) = head.split_once(char::is_whitespace).unwrap_or((head, ""));
            let word = word.to_ascii_lowercase();
            section = match word.as_str() {
                "limits" | "read" | "order" | "offset" | "hatch" | "machine" | "frame" | "focus" | "defaults" if rest.trim().is_empty() => word,
                "layer" if !rest.trim().is_empty() => {
                    named.push(rest.trim().to_lowercase());
                    format!("layer {}", rest.trim().to_lowercase())
                }
                _ => return Err(format!("line {line}: '[{head}]' is not a section (limits, read, order, offset, hatch, machine, frame, focus, defaults or layer NAME)")),
            };
            continue;
        }
        let (key, value) = t
            .split_once('=')
            .ok_or_else(|| format!("line {line}: a setting is written key = value"))?;
        if section.is_empty() {
            return Err(format!(
                "line {line}: a setting is outside of every section"
            ));
        }
        entries.push(Entry {
            section: section.clone(),
            line,
            key: key.trim(),
            value: value.trim(),
        });
    }
    let mut s = Settings::default();
    for e in entries.iter().filter(|e| e.section == "defaults") {
        set_layer(&mut s.defaults, e.key, e.value).map_err(|m| format!("line {}: {m}", e.line))?;
    }
    for e in &entries {
        if e.section == "defaults" {
            continue;
        }
        if let Some(name) = e.section.strip_prefix("layer ") {
            let at = match s.layers.iter().position(|(n, _)| n == name) {
                Some(i) => i,
                None => {
                    if s.layers.len() >= MAX_LAYERS {
                        return Err(format!("line {}: more than {MAX_LAYERS} layers", e.line));
                    }
                    s.layers.push((name.to_string(), s.defaults));
                    s.layers.len() - 1
                }
            };
            set_layer(&mut s.layers[at].1, e.key, e.value)
                .map_err(|m| format!("line {}: {m}", e.line))?;
        } else {
            set_global(&mut s, &e.section, e.key, e.value)
                .map_err(|m| format!("line {}: {m}", e.line))?;
        }
    }
    // a layer section with no key still names the layer, with the defaults
    for name in named {
        if !s.layers.iter().any(|(n, _)| *n == name) {
            if s.layers.len() >= MAX_LAYERS {
                return Err(format!("more than {MAX_LAYERS} layers"));
            }
            s.layers.push((name, s.defaults));
        }
    }
    s.validate()?;
    Ok(s)
}

/// Reads a settings file.
pub fn load(path: &Path) -> Result<Settings, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > MAX_CONFIG_BYTES {
        return Err(format!(
            "{shown}: the settings file is larger than {MAX_CONFIG_BYTES} bytes"
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{shown}: {e}"))?;
    let text = String::from_utf8_lossy(&bytes);
    parse(text.strip_prefix('\u{feff}').unwrap_or(&text)).map_err(|e| format!("{shown}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        Settings::default().validate().unwrap();
        assert_eq!(
            Settings::default().params_for("anything"),
            LayerParams::default()
        );
    }

    #[test]
    fn a_file_sets_the_globals_the_defaults_and_the_layers() {
        let s = parse(
            "# a note\n\
             [read]\ndpi = 72\ntolerance_mm = 0.02\ngroup_by = layer\nalign = page\n\
             [limits]\nmax_depth = 10\n\
             [order]\ntwo_opt = off\njoin_tolerance_mm = 0.05\n\
             [machine]\ns_max = 255\noverscan_mm = 3\norigin_x = 5\nbed_x_mm = 300\nbed_y_mm = 200\nlaser_watts = 10\n\
             [layer #FF0000]\nmode = cut\npower = 80\nspeed = 300\npasses = 2\nfocus_step = -0.5\nkerf = 0.2\nkerf_side = auto\ntabs = 4\ntab_width = 1.5\n\
             [defaults]\nspeed = 900\npower = 30\n\
             [layer Marks]\nmode = engrave\nhatch_angle = 45\nhatch_spacing = 0.2\n",
        )
        .unwrap();
        assert_eq!(s.read.dpi, 72.0);
        assert_eq!(s.read.tolerance_mm, 0.02);
        assert_eq!(s.read.limits.max_depth, 10);
        assert_eq!(s.group_by, GroupBy::Layer);
        assert_eq!(s.align, Align::Page);
        assert!(!s.two_opt);
        assert_eq!(s.join_tolerance_mm, 0.05);
        assert_eq!(s.machine.s_max, 255);
        assert_eq!(s.machine.laser_watts, 10.0);
        assert_eq!(s.machine.overscan_mm, 3.0);
        assert_eq!(s.machine.origin, (5.0, 0.0));
        assert_eq!(s.machine.bed_mm, Some((300.0, 200.0)));
        assert_eq!(s.defaults.speed_mm_min, 900.0);
        // the layer before [defaults] still inherits what the defaults say and overrides its own keys
        let red = s.params_for("#ff0000");
        assert_eq!(red.power_percent, 80.0);
        assert_eq!(red.speed_mm_min, 300.0);
        assert_eq!(red.passes, 2);
        assert_eq!(red.focus_step_mm, -0.5);
        assert_eq!(
            (red.kerf_mm, red.kerf_side, red.tabs, red.tab_width_mm),
            (0.2, KerfSide::Auto, 4, 1.5)
        );
        let marks = s.params_for("MARKS");
        assert_eq!(marks.mode, LayerMode::Engrave);
        assert_eq!(marks.power_percent, 30.0, "from the defaults");
        assert_eq!(marks.speed_mm_min, 900.0);
        assert_eq!((marks.hatch_angle_deg, marks.hatch_spacing_mm), (45.0, 0.2));
        // a layer the file does not name has the defaults
        assert_eq!(s.params_for("other"), s.defaults);
    }

    #[test]
    fn a_layer_with_no_key_is_named_with_the_defaults() {
        let s = parse("[layer Holes]\n").unwrap();
        assert_eq!(s.layers.len(), 1);
        assert_eq!(s.layers[0].0, "holes");
    }

    #[test]
    fn a_mistake_is_an_error_that_names_the_line() {
        for (text, needle) in [
            ("[machine]\ns_maxx = 1000\n", "line 2"),
            ("[machine]\ns_max = abc\n", "line 2"),
            ("[machine]\ns_max = 0\n", "s_max"),
            ("[defaults]\npower = 500\n", "power"),
            ("[defaults]\npower = 0\n", "power"),
            ("[defaults]\nspeed = 0\n", "speed"),
            ("[defaults]\npasses = 0\n", "passes"),
            ("[defaults]\npasses = 51\n", "passes"),
            ("[defaults]\nmode = burn\n", "line 2"),
            ("[defaults]\nkerf_side = left\n", "line 2"),
            ("[defaults]\nkerf = -1\n", "kerf"),
            ("[defaults]\nhatch_spacing = 0\n", "hatch_spacing"),
            ("[defaults]\nfocus_step = 5\npasses = 30\n", "focus"),
            ("[nothing]\n", "line 1"),
            ("[machine\n", "line 1"),
            ("[layer]\n", "line 1"),
            ("dpi = 96\n", "outside"),
            ("[read]\nthis is not a setting\n", "line 2"),
            ("[read]\ndpi = nan\n", "not a number"),
            ("[limits]\nmax_points = -4\n", "whole"),
            ("[limits]\nmax_depth = 0\n", "limit"),
            ("[read]\nalign = left\n", "line 2"),
            ("[order]\ntwo_opt = maybe\n", "line 2"),
        ] {
            let e = parse(text).map(|_| ()).expect_err(text);
            assert!(e.contains(needle), "{text:?} gave {e:?}");
        }
    }

    #[test]
    fn a_bed_needs_both_sizes_and_zero_means_none() {
        let s = parse("[machine]\nbed_x_mm = 100\nbed_y_mm = 0\n").unwrap_err();
        assert!(s.contains("bed"));
        let s = parse("[machine]\nbed_x_mm = 0\nbed_y_mm = 0\n").unwrap();
        assert_eq!(s.machine.bed_mm, None);
    }

    #[test]
    fn too_many_layers_are_refused() {
        let mut text = String::new();
        for i in 0..=MAX_LAYERS {
            text.push_str(&format!("[layer l{i}]\npower = 10\n"));
        }
        assert!(parse(&text).unwrap_err().contains("layers"));
    }

    #[test]
    fn a_skipped_layer_is_not_checked_for_its_numbers() {
        let s = parse("[layer a]\nmode = skip\npower = 999\n").unwrap();
        assert_eq!(s.params_for("a").mode, LayerMode::Skip);
    }

    #[test]
    fn the_words_of_the_choices_round_trip() {
        for m in [LayerMode::Cut, LayerMode::Engrave, LayerMode::Skip] {
            assert_eq!(LayerMode::from_word(m.word()), Some(m));
        }
        for k in [
            KerfSide::None,
            KerfSide::Auto,
            KerfSide::Inside,
            KerfSide::Outside,
        ] {
            assert_eq!(KerfSide::from_word(k.word()), Some(k));
        }
        assert_eq!(LayerMode::from_word(" CUT "), Some(LayerMode::Cut));
        assert_eq!(KerfSide::from_word("x"), None);
    }

    #[test]
    fn a_file_is_read_from_disk_with_a_limit_on_its_size() {
        let dir = std::env::temp_dir().join(format!("eh3d-laser-vconfig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("a.cfg");
        std::fs::write(&good, "\u{feff}[machine]\ns_max = 500\n").unwrap();
        assert_eq!(load(&good).unwrap().machine.s_max, 500);
        let bad = dir.join("b.cfg");
        std::fs::write(&bad, "[machine]\ns_max = x\n").unwrap();
        let e = load(&bad).unwrap_err();
        assert!(e.contains("b.cfg") && e.contains("line 2"), "{e}");
        let big = dir.join("c.cfg");
        std::fs::write(&big, "#".repeat(MAX_CONFIG_BYTES as usize + 1)).unwrap();
        assert!(load(&big).unwrap_err().contains("larger"));
        assert!(load(&dir.join("missing.cfg")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_in_a_damaged_file_panics() {
        let base = "[read]\ndpi = 96\n[layer #f00]\npower = 50\nmode = cut\n";
        for end in 0..=base.len() {
            let _ = parse(&base[..end]);
        }
        for junk in [
            "[",
            "]",
            "[]",
            "=",
            "[layer ]\n",
            "[layer x]\n=\n",
            "\u{0}\u{0}",
            "[defaults]\npower=1e999\n",
        ] {
            let _ = parse(junk);
        }
    }

    #[test]
    fn the_limit_of_power_is_full_power_unless_the_file_says_otherwise() {
        assert_eq!(Settings::default().machine.max_power_percent, 100.0);
        let s = parse("[machine]\nmax_power_percent = 30\n[defaults]\npower = 20\n").unwrap();
        assert_eq!(s.machine.max_power_percent, 30.0);
    }

    #[test]
    fn a_layer_over_the_limit_of_power_is_refused_and_names_the_layer() {
        let e = parse(
            "[machine]\nmax_power_percent = 30\n[defaults]\npower = 20\n[layer red]\npower = 31\n",
        )
        .unwrap_err();
        assert!(
            e.contains("layer red") && e.contains("max_power_percent"),
            "{e}"
        );
        // at the limit it is accepted, and a skipped layer is not checked
        parse("[machine]\nmax_power_percent = 30\n[defaults]\npower = 30\n").unwrap();
        parse(
            "[machine]\nmax_power_percent = 30\n[defaults]\npower = 20\n[layer a]\nmode = skip\npower = 90\n",
        )
        .unwrap();
    }

    #[test]
    fn the_defaults_over_the_limit_of_power_are_refused_too() {
        let e = parse("[machine]\nmax_power_percent = 30\n").unwrap_err();
        assert!(e.contains("[defaults]"), "{e}");
    }

    #[test]
    fn the_limit_of_power_has_a_range() {
        for bad in ["0", "0.05", "100.5", "-1", "x"] {
            let text = format!("[machine]\nmax_power_percent = {bad}\n[defaults]\npower = 0.1\n");
            assert!(parse(&text).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_frame_keeps_the_laser_off_by_default() {
        let f = Settings::default().frame;
        assert_eq!((f.feed_mm_min, f.laps, f.margin_mm), (3000.0, 1, 0.0));
        assert_eq!(f.pilot_percent, 0.0);
        let s =
            parse("[frame]\nfeed_mm_min = 1200\nlaps = 2\nmargin_mm = 1.5\npilot_percent = 2\n")
                .unwrap();
        assert_eq!(
            s.frame,
            FrameSettings {
                feed_mm_min: 1200.0,
                laps: 2,
                margin_mm: 1.5,
                pilot_percent: 2.0
            }
        );
    }

    #[test]
    fn a_frame_setting_out_of_range_is_refused() {
        for text in [
            "[frame]\nfeed_mm_min = 0\n",
            "[frame]\nfeed_mm_min = 30001\n",
            "[frame]\nlaps = 0\n",
            "[frame]\nlaps = 21\n",
            "[frame]\nmargin_mm = -1\n",
            "[frame]\nmargin_mm = 101\n",
            "[frame]\npilot_percent = 5.1\n",
            "[frame]\npilot_percent = -1\n",
            "[frame]\nlaps = 1.5\n",
            "[frame]\nspeed = 5\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
        // the pilot power obeys the limit of power of the machine
        let e = parse(
            "[machine]\nmax_power_percent = 2\n[defaults]\npower = 1\n[frame]\npilot_percent = 3\n",
        )
        .unwrap_err();
        assert!(e.contains("pilot_percent"), "{e}");
    }

    #[test]
    fn the_focus_test_needs_a_step_that_the_person_gives() {
        let f = Settings::default().focus;
        assert_eq!(f.z_step_mm, 0.0, "which way is down depends on the machine");
        assert_eq!((f.steps, f.length_mm, f.spacing_mm), (9, 10.0, 2.0));
        let s = parse(
            "[focus]\nz_start_mm = 1\nz_step_mm = -0.5\nsteps = 5\nlength_mm = 8\nspacing_mm = 3\nfeed_mm_min = 800\npower_percent = 20\n",
        )
        .unwrap();
        assert_eq!(
            s.focus,
            FocusSettings {
                z_start_mm: 1.0,
                z_step_mm: -0.5,
                steps: 5,
                length_mm: 8.0,
                spacing_mm: 3.0,
                feed_mm_min: 800.0,
                power_percent: 20.0
            }
        );
    }

    #[test]
    fn a_focus_setting_out_of_range_is_refused() {
        for text in [
            "[focus]\nz_start_mm = 101\n",
            "[focus]\nz_step_mm = 21\n",
            "[focus]\nz_step_mm = 6\nsteps = 30\n",
            "[focus]\nsteps = 1\n",
            "[focus]\nsteps = 31\n",
            "[focus]\nlength_mm = 0.5\n",
            "[focus]\nspacing_mm = 21\n",
            "[focus]\nfeed_mm_min = 0\n",
            "[focus]\npower_percent = 0\n",
            "[focus]\npower_percent = 101\n",
            "[focus]\nz = 1\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }
}
