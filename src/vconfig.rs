//! The settings of the work with vector drawings, kept in a plain text file so that nothing that was tuned lives in the code: the limits of the readers, how a file is read, how the
//! cuts are ordered, the machine (full power, overscan, travel speed, the bed) and the settings of each layer or colour (cut or engrave, power, speed, passes, the step of the focus,
//! the width of the beam, the tabs and the hatch).
//!
//! The file is a list of `key = value` lines in sections (`[limits]`, `[read]`, `[order]`, `[offset]`, `[hatch]`, `[machine]`, `[defaults]` and one `[layer NAME]` for each layer or
//! colour); a line that starts with `#` or `;` is a note. A key that is not known is an error that names its line: a typing mistake never changes nothing silently.

use crate::tone::Tone;
use crate::vector::{GroupBy, Limits, Pt, ReadOptions};
use std::path::Path;

/// The largest settings file that is read.
pub const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

/// The most layers a settings file may describe.
pub const MAX_LAYERS: usize = 256;

/// The most machine profiles a settings file may describe.
pub const MAX_PROFILES: usize = 64;

/// The longest name of a profile.
pub const MAX_PROFILE_NAME_CHARS: usize = 80;

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
    /// The shapes are filled a second time with lines at right angles to the first ones (a cross hatch), in every pass.
    pub hatch_cross: bool,
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
            hatch_cross: false,
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
    /// The limit of speed (mm/min): a layer or a command that asks for more is an error, never lowered without a word.
    pub max_speed_mm_min: f64,
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
            max_speed_mm_min: crate::safety::DEFAULT_MAX_SPEED_MM_MIN,
        }
    }
}

/// A machine and laser that the person owns (`[profile NAME]`): its travel, its limits and its full-power value. A person with two machines, or a laser module that is changed for
/// another, keeps one profile for each in the same file and picks one with `profile = NAME` in `[machine]` or `--profile NAME`. What a profile leaves out stays as `[machine]` says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Profile {
    /// The name, in lower case.
    pub name: String,
    /// The travel of the machine in X (mm): the width of the bed.
    pub travel_x_mm: Option<f64>,
    /// The travel of the machine in Y (mm): the depth of the bed.
    pub travel_y_mm: Option<f64>,
    /// The limit of power, as a percent of full power.
    pub max_power_percent: Option<f64>,
    /// The limit of speed (mm/min).
    pub max_speed_mm_min: Option<f64>,
    /// The spindle value that is full power for the controller.
    pub s_max: Option<u32>,
    /// The optical power of the laser at full power, in watts.
    pub laser_watts: Option<f64>,
}

impl Profile {
    /// The machine with what the profile says over it: the profile is the machine, so it wins over `[machine]`, and the options of the command line win over both.
    pub fn apply_to(&self, m: &mut Machine) {
        if let (Some(x), Some(y)) = (self.travel_x_mm, self.travel_y_mm) {
            m.bed_mm = Some((x, y));
        }
        m.max_power_percent = self.max_power_percent.unwrap_or(m.max_power_percent);
        m.max_speed_mm_min = self.max_speed_mm_min.unwrap_or(m.max_speed_mm_min);
        m.s_max = self.s_max.unwrap_or(m.s_max);
        m.laser_watts = self.laser_watts.unwrap_or(m.laser_watts);
    }

    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        if self.name.is_empty() || self.name.chars().count() > MAX_PROFILE_NAME_CHARS {
            return Err(format!(
                "the name of a profile must be from 1 to {MAX_PROFILE_NAME_CHARS} characters"
            ));
        }
        match (self.travel_x_mm, self.travel_y_mm) {
            (None, None) => {}
            (Some(x), Some(y)) => {
                range("travel_x_mm", x, 1.0, 100_000.0)?;
                range("travel_y_mm", y, 1.0, 100_000.0)?;
            }
            _ => return Err("the travel needs both travel_x_mm and travel_y_mm".to_string()),
        }
        if let Some(p) = self.max_power_percent {
            crate::safety::validate_max(p)?;
        }
        if let Some(v) = self.max_speed_mm_min {
            crate::safety::validate_max_speed(v)?;
        }
        if let Some(s) = self.s_max {
            if s == 0 || s > 100_000 {
                return Err(format!("s_max is {s}: it must be from 1 to 100000"));
            }
        }
        if let Some(w) = self.laser_watts {
            range("laser_watts", w, 0.0, 1_000.0)?;
        }
        Ok(())
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
    /// The most that `pilot_percent` may be (a percent of full power); the limit of power of the machine still holds over it.
    pub max_pilot_percent: f64,
}

impl Default for FrameSettings {
    fn default() -> Self {
        FrameSettings {
            feed_mm_min: 3000.0,
            laps: 1,
            margin_mm: 0.0,
            pilot_percent: 0.0,
            max_pilot_percent: crate::safety::DEFAULT_MAX_PILOT_PERCENT,
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
        crate::safety::validate_max_pilot(self.max_pilot_percent)?;
        range(
            "frame pilot_percent",
            self.pilot_percent,
            0.0,
            self.max_pilot_percent,
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

/// The test grid that is made from a material (`phoenix-laser calibrate --material`) and the figures burnt next to its squares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrateSettings {
    /// How many powers and how many feeds the ladder around the material has.
    pub steps: u32,
    /// How far the ladder goes below and above what the material says, as a percent of it.
    pub spread_percent: f64,
    /// The power of each column and the feed of each row are burnt next to the squares of a grid made from a material (`--no-labels` takes them away; `--labels` also asks for them without a material).
    pub labels: bool,
    /// The height of the figures (mm).
    pub label_mm: f64,
    /// The lowest power of the ladder (a percent of full power): the ladder is held at it and never goes under.
    pub min_power_percent: f64,
    /// The lowest feed of the ladder (mm/min).
    pub min_feed_mm_min: f64,
}

impl Default for CalibrateSettings {
    fn default() -> Self {
        CalibrateSettings {
            steps: 5,
            spread_percent: 40.0,
            labels: true,
            label_mm: 3.0,
            min_power_percent: 1.0,
            min_feed_mm_min: 10.0,
        }
    }
}

impl CalibrateSettings {
    pub fn validate(&self) -> Result<(), String> {
        let max = crate::calibrate::MAX_CELLS;
        if self.steps < 1 || self.steps as usize > max {
            return Err(format!(
                "calibrate steps is {}: it must be from 1 to {max}",
                self.steps
            ));
        }
        if !(self.spread_percent.is_finite() && (0.0..=90.0).contains(&self.spread_percent)) {
            return Err(format!(
                "calibrate spread_percent is {}: it must be from 0 to 90",
                self.spread_percent
            ));
        }
        let top = crate::calibrate::MAX_LABEL_MM;
        if !(self.label_mm.is_finite() && (1.0..=top).contains(&self.label_mm)) {
            return Err(format!(
                "calibrate label_mm is {}: it must be from 1 to {top}",
                self.label_mm
            ));
        }
        if !(self.min_power_percent.is_finite()
            && self.min_power_percent > 0.0
            && self.min_power_percent <= 100.0)
        {
            return Err(format!(
                "calibrate min_power_percent is {}: it must be more than 0 up to 100",
                self.min_power_percent
            ));
        }
        if !(self.min_feed_mm_min.is_finite()
            && self.min_feed_mm_min > 0.0
            && self.min_feed_mm_min <= 30000.0)
        {
            return Err(format!(
                "calibrate min_feed_mm_min is {}: it must be more than 0 up to 30000",
                self.min_feed_mm_min
            ));
        }
        Ok(())
    }
}

/// How much of a QR symbol can be damaged and still be read: the more, the larger the symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrEcc {
    /// About 7 % of the symbol.
    L,
    /// About 15 %.
    M,
    /// About 25 %.
    Q,
    /// About 30 %.
    H,
}

impl QrEcc {
    /// The level a word names (`l`, `m`, `q` or `h`, in any case).
    pub fn from_word(word: &str) -> Option<QrEcc> {
        match word.trim().to_ascii_lowercase().as_str() {
            "l" => Some(QrEcc::L),
            "m" => Some(QrEcc::M),
            "q" => Some(QrEcc::Q),
            "h" => Some(QrEcc::H),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            QrEcc::L => "L",
            QrEcc::M => "M",
            QrEcc::Q => "Q",
            QrEcc::H => "H",
        }
    }
}

/// A QR symbol burnt by filling its dark modules (`phoenix-laser qr`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QrSettings {
    /// The side of a module (mm).
    pub module_mm: f64,
    /// The distance between the lines that fill a module (mm); not more than a module.
    pub line_mm: f64,
    /// A percent of full power; the limit of power of the machine still holds over it.
    pub power_percent: f64,
    pub speed_mm_min: f64,
    /// How many times the whole symbol is burnt.
    pub passes: u32,
    /// The most bytes that the data may have.
    pub max_bytes: u32,
    /// The largest version of the symbol (1 to 4); data that does not fit is refused, never cut.
    pub max_version: u32,
    pub ecc: QrEcc,
    /// The largest side of the symbol (mm); a bigger one is refused, never shrunk.
    pub max_size_mm: f64,
    /// The most burning moves of the program, all passes together.
    pub max_lines: u32,
    /// Burn the light modules instead of the dark ones, so that the symbol is left bare in a burnt plate (off by default).
    pub invert: bool,
    /// The run-up at each end of a burnt line (mm), burnt with the laser off so that the head is at speed when the laser comes on (0 is none, the default).
    pub overscan_mm: f64,
}

impl Default for QrSettings {
    fn default() -> Self {
        QrSettings {
            module_mm: 1.0,
            line_mm: 0.1,
            power_percent: 20.0,
            speed_mm_min: 1000.0,
            passes: 1,
            max_bytes: 40,
            max_version: 4,
            ecc: QrEcc::M,
            max_size_mm: 100.0,
            max_lines: 100_000,
            invert: false,
            overscan_mm: 0.0,
        }
    }
}

impl QrSettings {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("qr {name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("module_mm", self.module_mm, 0.2, 10.0)?;
        range("line_mm", self.line_mm, 0.02, self.module_mm)?;
        if !(self.power_percent.is_finite()
            && self.power_percent > 0.0
            && self.power_percent <= 100.0)
        {
            return Err(format!(
                "qr power_percent is {}: it must be more than 0 up to 100",
                self.power_percent
            ));
        }
        range("speed_mm_min", self.speed_mm_min, 1.0, 30_000.0)?;
        range("max_size_mm", self.max_size_mm, 1.0, 1000.0)?;
        range("overscan_mm", self.overscan_mm, 0.0, 20.0)?;
        for (name, v, hi) in [
            ("passes", self.passes, 20),
            ("max_bytes", self.max_bytes, 78),
            ("max_version", self.max_version, 4),
            ("max_lines", self.max_lines, 1_000_000),
        ] {
            if v == 0 || v > hi {
                return Err(format!("qr {name} is {v}: it must be from 1 to {hi}"));
            }
        }
        Ok(())
    }
}

/// A line of text burnt with the font of strokes of the program (`phoenix-laser text`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSettings {
    /// The height of a capital letter (mm).
    pub height_mm: f64,
    /// The tallest letter that is accepted (mm): a bigger one is refused, never cut down.
    pub max_height_mm: f64,
    /// The gap between two letters, as a percent of the height.
    pub spacing_percent: f64,
    /// The gap between two lines, as a percent of the height.
    pub line_spacing_percent: f64,
    /// A percent of full power; the limit of power of the machine still holds over it.
    pub power_percent: f64,
    pub speed_mm_min: f64,
    /// How many times the whole text is burnt.
    pub passes: u32,
    /// The most characters that a text may have.
    pub max_chars: u32,
}

impl Default for TextSettings {
    fn default() -> Self {
        TextSettings {
            height_mm: 10.0,
            max_height_mm: 100.0,
            spacing_percent: 25.0,
            line_spacing_percent: 50.0,
            power_percent: 20.0,
            speed_mm_min: 1000.0,
            passes: 1,
            max_chars: 200,
        }
    }
}

impl TextSettings {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("text {name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("max_height_mm", self.max_height_mm, 1.0, 1000.0)?;
        range("height_mm", self.height_mm, 1.0, self.max_height_mm)?;
        range("spacing_percent", self.spacing_percent, 0.0, 200.0)?;
        range(
            "line_spacing_percent",
            self.line_spacing_percent,
            0.0,
            300.0,
        )?;
        if !(self.power_percent.is_finite()
            && self.power_percent > 0.0
            && self.power_percent <= 100.0)
        {
            return Err(format!(
                "text power_percent is {}: it must be more than 0 up to 100",
                self.power_percent
            ));
        }
        range("speed_mm_min", self.speed_mm_min, 1.0, 30_000.0)?;
        if self.passes == 0 || self.passes > 20 {
            return Err(format!(
                "text passes is {}: it must be from 1 to 20",
                self.passes
            ));
        }
        if self.max_chars == 0 || self.max_chars > 5_000 {
            return Err(format!(
                "text max_chars is {}: it must be from 1 to 5000",
                self.max_chars
            ));
        }
        Ok(())
    }
}

/// The check of a program (`phoenix-laser verify`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VerifySettings {
    /// How far (mm) the head may go past the bed with the laser off, as the overscan of another program does: 0 allows none. Burning outside the bed always fails.
    pub overscan_mm: f64,
}

impl VerifySettings {
    pub fn validate(&self) -> Result<(), String> {
        let max = crate::verify::MAX_OVERSCAN_MM;
        if self.overscan_mm.is_finite() && (0.0..=max).contains(&self.overscan_mm) {
            Ok(())
        } else {
            Err(format!(
                "verify overscan_mm is {}: it must be from 0 to {max}",
                self.overscan_mm
            ))
        }
    }
}

/// How a picture is followed for its contours (`phoenix-laser trace`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceSettings {
    /// From 0 to 255: a grey level under it is ink (over it with `invert`).
    pub threshold: u8,
    /// The light parts are ink and the dark parts are paper.
    pub invert: bool,
    /// A point is dropped when the line stays within this distance (mm) without it; 0 keeps them all.
    pub simplify_mm: f64,
    /// A contour that encloses less than this (mm2) is a speck and is left out.
    pub min_area_mm2: f64,
    /// A picture with more pixels than this is reduced before it is followed.
    pub max_pixels: usize,
    /// A picture with more contours than this is an error.
    pub max_contours: usize,
    /// A picture with more pieces of border than this is an error.
    pub max_segments: usize,
}

impl Default for TraceSettings {
    fn default() -> Self {
        TraceSettings {
            threshold: 128,
            invert: false,
            simplify_mm: 0.02,
            min_area_mm2: 0.05,
            max_pixels: 4_000_000,
            max_contours: 20_000,
            max_segments: 2_000_000,
        }
    }
}

impl TraceSettings {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("trace {name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("simplify_mm", self.simplify_mm, 0.0, 5.0)?;
        range("min_area_mm2", self.min_area_mm2, 0.0, 1_000_000.0)?;
        for (name, v, hi) in [
            ("max_pixels", self.max_pixels, 25_000_000),
            ("max_contours", self.max_contours, 1_000_000),
            ("max_segments", self.max_segments, 50_000_000),
        ] {
            if v == 0 || v > hi {
                return Err(format!("trace {name} is {v}: it must be from 1 to {hi}"));
            }
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
    /// The machines and lasers that the file describes (`[profile NAME]`).
    pub profiles: Vec<Profile>,
    /// The profile that is applied to `machine` (its name, in lower case), if one was chosen.
    pub profile: Option<String>,
    /// The machine as `[machine]` wrote it, before any profile: a profile chosen later is applied over this, never over another profile.
    pub base_machine: Machine,
    pub frame: FrameSettings,
    pub focus: FocusSettings,
    pub trace: TraceSettings,
    pub verify: VerifySettings,
    pub calibrate: CalibrateSettings,
    pub text: TextSettings,
    /// The QR symbol (`phoenix-laser qr`).
    pub qr: QrSettings,
    /// The tone curve of a picture before it becomes dots (`phoenix-laser engrave` and the window).
    pub tone: Tone,
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
            profiles: Vec::new(),
            profile: None,
            base_machine: Machine::default(),
            frame: FrameSettings::default(),
            focus: FocusSettings::default(),
            trace: TraceSettings::default(),
            verify: VerifySettings::default(),
            calibrate: CalibrateSettings::default(),
            text: TextSettings::default(),
            qr: QrSettings::default(),
            tone: Tone::default(),
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

    /// Applies the profile `name` to the machine: its values win over `[machine]`. A name that the file does not have is an error that lists the ones it has.
    pub fn apply_profile(&mut self, name: &str) -> Result<(), String> {
        let key = name.trim().to_lowercase();
        let Some(p) = self.profiles.iter().find(|p| p.name == key).cloned() else {
            let have: Vec<&str> = self.profiles.iter().map(|p| p.name.as_str()).collect();
            return Err(format!(
                "there is no profile '{}' (the file has: {})",
                name.trim(),
                if have.is_empty() {
                    "none".to_string()
                } else {
                    have.join(", ")
                }
            ));
        };
        self.machine = self.base_machine;
        p.apply_to(&mut self.machine);
        self.profile = Some(key);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        self.read.validate()?;
        if self.profiles.len() > MAX_PROFILES {
            return Err(format!(
                "the settings describe more than {MAX_PROFILES} profiles"
            ));
        }
        for p in &self.profiles {
            p.validate()
                .map_err(|e| format!("profile {}: {e}", p.name))?;
        }
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
        crate::safety::validate_max_speed(m.max_speed_mm_min)?;
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
        self.trace.validate()?;
        self.verify.validate()?;
        self.calibrate.validate()?;
        self.text.validate()?;
        self.qr.validate()?;
        self.tone.validate()?;
        self.defaults.validate(m.max_passes)?;
        if self.defaults.mode != LayerMode::Skip {
            crate::safety::check_power("power", self.defaults.power_percent, m.max_power_percent)
                .map_err(|e| format!("[defaults]: {e}"))?;
            crate::safety::check_speed("speed", self.defaults.speed_mm_min, m.max_speed_mm_min)
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
                crate::safety::check_speed("speed", p.speed_mm_min, m.max_speed_mm_min)
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
        "hatch_cross" => p.hatch_cross = switch(value)?,
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
        ("tone", "gamma") => s.tone.gamma = number(value)?,
        ("tone", "contrast") => s.tone.contrast = number(value)?,
        ("tone", "brightness") => s.tone.brightness = number(value)?,
        ("tone", "invert") => s.tone.invert = switch(value)?,
        ("tone", "auto_level") => s.tone.auto_level = switch(value)?,
        ("verify", "overscan_mm") => s.verify.overscan_mm = number(value)?,
        ("calibrate", "steps") => {
            s.calibrate.steps =
                u32::try_from(whole(value)?).map_err(|_| "steps is too large".to_string())?
        }
        ("calibrate", "spread_percent") => s.calibrate.spread_percent = number(value)?,
        ("calibrate", "labels") => s.calibrate.labels = switch(value)?,
        ("calibrate", "label_mm") => s.calibrate.label_mm = number(value)?,
        ("calibrate", "min_power_percent") => s.calibrate.min_power_percent = number(value)?,
        ("calibrate", "min_feed_mm_min") => s.calibrate.min_feed_mm_min = number(value)?,
        ("text", "height_mm") => s.text.height_mm = number(value)?,
        ("text", "max_height_mm") => s.text.max_height_mm = number(value)?,
        ("text", "spacing_percent") => s.text.spacing_percent = number(value)?,
        ("text", "line_spacing_percent") => s.text.line_spacing_percent = number(value)?,
        ("text", "power_percent") => s.text.power_percent = number(value)?,
        ("text", "speed_mm_min") => s.text.speed_mm_min = number(value)?,
        ("text", "passes") => {
            s.text.passes =
                u32::try_from(whole(value)?).map_err(|_| "passes is too large".to_string())?
        }
        ("text", "max_chars") => {
            s.text.max_chars =
                u32::try_from(whole(value)?).map_err(|_| "max_chars is too large".to_string())?
        }
        ("qr", "module_mm") => s.qr.module_mm = number(value)?,
        ("qr", "line_mm") => s.qr.line_mm = number(value)?,
        ("qr", "power_percent") => s.qr.power_percent = number(value)?,
        ("qr", "speed_mm_min") => s.qr.speed_mm_min = number(value)?,
        ("qr", "passes") => {
            s.qr.passes =
                u32::try_from(whole(value)?).map_err(|_| "passes is too large".to_string())?
        }
        ("qr", "max_bytes") => {
            s.qr.max_bytes =
                u32::try_from(whole(value)?).map_err(|_| "max_bytes is too large".to_string())?
        }
        ("qr", "max_version") => {
            s.qr.max_version =
                u32::try_from(whole(value)?).map_err(|_| "max_version is too large".to_string())?
        }
        ("qr", "ecc") => {
            s.qr.ecc = QrEcc::from_word(value)
                .ok_or_else(|| format!("'{}' is not L, M, Q or H", value.trim()))?
        }
        ("qr", "max_size_mm") => s.qr.max_size_mm = number(value)?,
        ("qr", "max_lines") => {
            s.qr.max_lines =
                u32::try_from(whole(value)?).map_err(|_| "max_lines is too large".to_string())?
        }
        ("qr", "invert") => s.qr.invert = switch(value)?,
        ("qr", "overscan_mm") => s.qr.overscan_mm = number(value)?,
        ("trace", "threshold") => {
            s.trace.threshold = u8::try_from(whole(value)?)
                .map_err(|_| "trace threshold must be from 0 to 255".to_string())?
        }
        ("trace", "invert") => s.trace.invert = switch(value)?,
        ("trace", "simplify_mm") => s.trace.simplify_mm = number(value)?,
        ("trace", "min_area_mm2") => s.trace.min_area_mm2 = number(value)?,
        ("trace", "max_pixels") => s.trace.max_pixels = to_usize(whole(value)?)?,
        ("trace", "max_contours") => s.trace.max_contours = to_usize(whole(value)?)?,
        ("trace", "max_segments") => s.trace.max_segments = to_usize(whole(value)?)?,
        ("frame", "feed_mm_min") => s.frame.feed_mm_min = number(value)?,
        ("frame", "laps") => {
            s.frame.laps =
                u32::try_from(whole(value)?).map_err(|_| "laps is too large".to_string())?
        }
        ("frame", "margin_mm") => s.frame.margin_mm = number(value)?,
        ("frame", "pilot_percent") => s.frame.pilot_percent = number(value)?,
        ("frame", "max_pilot_percent") => s.frame.max_pilot_percent = number(value)?,
        ("machine", "max_power_percent") => s.machine.max_power_percent = number(value)?,
        ("machine", "max_speed_mm_min") => s.machine.max_speed_mm_min = number(value)?,
        ("machine", "profile") => s.profile = Some(value.trim().to_lowercase()),
        ("machine", "max_passes") => {
            s.machine.max_passes =
                u32::try_from(whole(value)?).map_err(|_| "max_passes is too large".to_string())?
        }
        _ => return Err(format!("'{key}' is not a setting of [{section}]")),
    }
    Ok(())
}

fn set_profile(p: &mut Profile, key: &str, value: &str) -> Result<(), String> {
    match key {
        "travel_x_mm" => p.travel_x_mm = Some(number(value)?),
        "travel_y_mm" => p.travel_y_mm = Some(number(value)?),
        "max_power_percent" => p.max_power_percent = Some(number(value)?),
        "max_speed_mm_min" => p.max_speed_mm_min = Some(number(value)?),
        "s_max" => {
            p.s_max =
                Some(u32::try_from(whole(value)?).map_err(|_| "s_max is too large".to_string())?)
        }
        "laser_watts" => p.laser_watts = Some(number(value)?),
        _ => return Err(format!("'{key}' is not a setting of a profile")),
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
    let mut named_profiles: Vec<String> = Vec::new();
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
                "limits" | "read" | "order" | "offset" | "hatch" | "machine" | "frame" | "focus" | "trace" | "tone" | "verify" | "calibrate" | "text" | "qr" | "defaults" if rest.trim().is_empty() => word,
                "layer" if !rest.trim().is_empty() => {
                    named.push(rest.trim().to_lowercase());
                    format!("layer {}", rest.trim().to_lowercase())
                }
                "profile" if !rest.trim().is_empty() => {
                    named_profiles.push(rest.trim().to_lowercase());
                    format!("profile {}", rest.trim().to_lowercase())
                }
                _ => return Err(format!("line {line}: '[{head}]' is not a section (limits, read, order, offset, hatch, machine, frame, focus, trace, tone, verify, calibrate, text, qr, defaults, layer NAME or profile NAME)")),
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
        if let Some(name) = e.section.strip_prefix("profile ") {
            let at = match s.profiles.iter().position(|p| p.name == name) {
                Some(i) => i,
                None => {
                    if s.profiles.len() >= MAX_PROFILES {
                        return Err(format!(
                            "line {}: more than {MAX_PROFILES} profiles",
                            e.line
                        ));
                    }
                    s.profiles.push(Profile {
                        name: name.to_string(),
                        ..Profile::default()
                    });
                    s.profiles.len() - 1
                }
            };
            set_profile(&mut s.profiles[at], e.key, e.value)
                .map_err(|m| format!("line {}: {m}", e.line))?;
        } else if let Some(name) = e.section.strip_prefix("layer ") {
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
    // a profile section with no key still names the profile
    for name in named_profiles {
        if !s.profiles.iter().any(|p| p.name == name) {
            if s.profiles.len() >= MAX_PROFILES {
                return Err(format!("more than {MAX_PROFILES} profiles"));
            }
            s.profiles.push(Profile {
                name,
                ..Profile::default()
            });
        }
    }
    // the profile that was chosen is the machine
    s.base_machine = s.machine;
    if let Some(name) = s.profile.clone() {
        s.apply_profile(&name)?;
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
                pilot_percent: 2.0,
                max_pilot_percent: 5.0,
            }
        );
    }

    const TWO_MACHINES: &str = "[machine]\ns_max = 255\nbed_x_mm = 100\nbed_y_mm = 100\nprofile = Diode\n\
        [profile diode]\ntravel_x_mm = 400\ntravel_y_mm = 300\nmax_power_percent = 80\nmax_speed_mm_min = 6000\ns_max = 1000\nlaser_watts = 10\n\
        [profile CO2]\ns_max = 100\nmax_power_percent = 90\n";

    #[test]
    fn a_profile_is_the_machine_and_wins_over_the_machine_section() {
        let s = parse(TWO_MACHINES).unwrap();
        assert_eq!(s.profile.as_deref(), Some("diode"));
        assert_eq!(s.profiles.len(), 2);
        let m = s.machine;
        assert_eq!(m.s_max, 1000);
        assert_eq!(m.bed_mm, Some((400.0, 300.0)));
        assert_eq!((m.max_power_percent, m.max_speed_mm_min), (80.0, 6000.0));
        assert_eq!(m.laser_watts, 10.0);
        // what the file wrote in [machine] is kept apart
        assert_eq!(s.base_machine.s_max, 255);
        assert_eq!(s.base_machine.bed_mm, Some((100.0, 100.0)));
        assert_eq!(s.base_machine.max_power_percent, 100.0);
    }

    #[test]
    fn a_profile_chosen_later_is_applied_over_the_machine_and_not_over_the_first_profile() {
        let mut s = parse(TWO_MACHINES).unwrap();
        s.apply_profile(" co2 ").unwrap();
        assert_eq!(s.profile.as_deref(), Some("co2"));
        let m = s.machine;
        assert_eq!((m.s_max, m.max_power_percent), (100, 90.0));
        // the CO2 profile has no travel and no speed: the machine section decides, not the diode
        assert_eq!(m.bed_mm, Some((100.0, 100.0)));
        assert_eq!(m.max_speed_mm_min, 30_000.0);
        assert_eq!(m.laser_watts, 0.0);
        s.validate().unwrap();
    }

    #[test]
    fn a_profile_that_does_not_exist_is_an_error_that_lists_the_ones_there_are() {
        let mut s = parse(TWO_MACHINES).unwrap();
        let e = s.apply_profile("plasma").unwrap_err();
        assert!(e.contains("plasma") && e.contains("diode, co2"), "{e}");
        let e = parse("[machine]\nprofile = nothing\n").unwrap_err();
        assert!(e.contains("none"), "{e}");
        // the failed choice left the machine as it was
        assert_eq!(s.machine.s_max, 1000);
    }

    #[test]
    fn a_profile_with_a_wrong_value_is_refused_and_the_error_names_it() {
        for (text, word) in [
            ("[profile a]\ntravel_x_mm = 100\n", "travel"),
            (
                "[profile a]\ntravel_x_mm = 0\ntravel_y_mm = 100\n",
                "travel_x_mm",
            ),
            ("[profile a]\nmax_power_percent = 0\n", "max_power_percent"),
            (
                "[profile a]\nmax_power_percent = 101\n",
                "max_power_percent",
            ),
            ("[profile a]\nmax_speed_mm_min = 0\n", "max_speed_mm_min"),
            (
                "[profile a]\nmax_speed_mm_min = 30001\n",
                "max_speed_mm_min",
            ),
            ("[profile a]\ns_max = 0\n", "s_max"),
            ("[profile a]\ns_max = 100001\n", "s_max"),
            ("[profile a]\nlaser_watts = -1\n", "laser_watts"),
            ("[profile a]\nspeed = 5\n", "not a setting of a profile"),
        ] {
            let e = parse(text).unwrap_err();
            assert!(e.contains(word), "{text:?}: {e}");
        }
        // a profile section needs a name, and the sections of the file keep their own words
        assert!(parse("[profile]\ns_max = 5\n").is_err());
        // a key with no name of profile outside of a profile section is not a profile key
        assert!(parse("[machine]\ntravel_x_mm = 5\n").is_err());
    }

    #[test]
    fn a_profile_section_with_no_key_still_names_the_profile() {
        let s = parse("[profile empty]\n").unwrap();
        assert_eq!(s.profiles.len(), 1);
        assert_eq!(s.profiles[0].name, "empty");
        let mut s = s;
        s.apply_profile("empty").unwrap();
        assert_eq!(s.machine, Machine::default());
    }

    #[test]
    fn the_limits_of_the_profile_hold_over_the_layers_and_the_defaults() {
        // the power of a layer over the limit of the profile
        let e = parse("[machine]\nprofile = low\n[profile low]\nmax_power_percent = 30\n[defaults]\npower = 20\n[layer red]\npower = 40\n").unwrap_err();
        assert!(
            e.contains("layer red") && e.contains("max_power_percent"),
            "{e}"
        );
        // the speed of a layer over the limit of speed
        let e = parse("[machine]\nprofile = slow\n[profile slow]\nmax_speed_mm_min = 500\n[defaults]\nspeed = 100\n[layer red]\nspeed = 600\n").unwrap_err();
        assert!(
            e.contains("layer red") && e.contains("max_speed_mm_min"),
            "{e}"
        );
        // the same for [defaults], which has a speed of 600 unless it says another
        let e = parse("[machine]\nmax_speed_mm_min = 100\n").unwrap_err();
        assert!(
            e.contains("[defaults]") && e.contains("max_speed_mm_min"),
            "{e}"
        );
        parse("[machine]\nmax_speed_mm_min = 100\n[defaults]\nspeed = 100\n").unwrap();
        // a speed at the limit is allowed
        parse("[machine]\nmax_speed_mm_min = 900\n[defaults]\nspeed = 100\n[layer red]\nspeed = 900\n").unwrap();
    }

    #[test]
    fn the_trace_settings_are_read_and_the_defaults_are_the_ones_of_the_example() {
        assert_eq!(Settings::default().trace, TraceSettings::default());
        let s = parse(
            "[trace]\nthreshold = 90\ninvert = on\nsimplify_mm = 0.1\nmin_area_mm2 = 1\nmax_pixels = 1000\nmax_contours = 10\nmax_segments = 5000\n",
        )
        .unwrap();
        assert_eq!(
            s.trace,
            TraceSettings {
                threshold: 90,
                invert: true,
                simplify_mm: 0.1,
                min_area_mm2: 1.0,
                max_pixels: 1000,
                max_contours: 10,
                max_segments: 5000,
            }
        );
    }

    #[test]
    fn the_tone_settings_are_read_and_checked() {
        assert!(Settings::default().tone.is_identity());
        let s = parse(
            "[tone]\ngamma = 1.4\ncontrast = 10\nbrightness = -5\ninvert = on\nauto_level = yes\n",
        )
        .unwrap();
        assert_eq!(
            s.tone,
            Tone {
                brightness: -5.0,
                contrast: 10.0,
                gamma: 1.4,
                invert: true,
                auto_level: true,
            }
        );
        for text in [
            "[tone]\ngamma = 0\n",
            "[tone]\ngamma = 10.5\n",
            "[tone]\ncontrast = 101\n",
            "[tone]\nbrightness = -101\n",
            "[tone]\ninvert = maybe\n",
            "[tone]\nspeed = 5\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_trace_setting_out_of_range_is_refused() {
        for text in [
            "[trace]\nthreshold = 256\n",
            "[trace]\nthreshold = -1\n",
            "[trace]\nthreshold = 1.5\n",
            "[trace]\ninvert = maybe\n",
            "[trace]\nsimplify_mm = -0.1\n",
            "[trace]\nsimplify_mm = 5.1\n",
            "[trace]\nmin_area_mm2 = -1\n",
            "[trace]\nmax_pixels = 0\n",
            "[trace]\nmax_pixels = 25000001\n",
            "[trace]\nmax_contours = 0\n",
            "[trace]\nmax_segments = 50000001\n",
            "[trace]\nspeed = 5\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
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
            "[frame]\nmax_pilot_percent = 101\n",
            "[frame]\nmax_pilot_percent = -1\n",
            "[frame]\npilot_percent = 3\nmax_pilot_percent = 2\n",
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

    #[test]
    fn the_overscan_that_verify_allows_is_read_and_the_default_is_none() {
        assert_eq!(Settings::default().verify, VerifySettings::default());
        assert_eq!(VerifySettings::default().overscan_mm, 0.0);
        let s = parse("[verify]\noverscan_mm = 3.5\n").unwrap();
        assert_eq!(s.verify.overscan_mm, 3.5);
        for text in [
            "[verify]\noverscan_mm = -1\n",
            "[verify]\noverscan_mm = 100.5\n",
            "[verify]\noverscan_mm = x\n",
            "[verify]\nspeed = 5\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
        assert!(parse("[verify]\noverscan_mm = 100\n").is_ok());
    }

    #[test]
    fn the_qr_settings_are_read_checked_and_the_defaults_are_the_ones_of_the_example() {
        assert_eq!(Settings::default().qr, QrSettings::default());
        let s = parse(
            "[qr]\nmodule_mm = 0.8\nline_mm = 0.2\npower_percent = 35\nspeed_mm_min = 1500\npasses = 2\nmax_bytes = 30\nmax_version = 3\necc = q\nmax_size_mm = 40\nmax_lines = 5000\n",
        )
        .unwrap();
        assert_eq!(
            s.qr,
            QrSettings {
                module_mm: 0.8,
                line_mm: 0.2,
                power_percent: 35.0,
                speed_mm_min: 1500.0,
                passes: 2,
                max_bytes: 30,
                max_version: 3,
                ecc: QrEcc::Q,
                max_size_mm: 40.0,
                max_lines: 5000,
                invert: false,
                overscan_mm: 0.0,
            }
        );
        assert_eq!(
            parse("[qr]\noverscan_mm = 2.5\n").unwrap().qr.overscan_mm,
            2.5
        );
        assert!(parse("[qr]\noverscan_mm = 20\n").is_ok());
        assert!(parse("[qr]\noverscan_mm = -1\n").is_err());
        assert!(parse("[qr]\noverscan_mm = 20.5\n").is_err());
        assert!(parse("[qr]\noverscan_mm = x\n").is_err());
        assert!(parse("[qr]\ninvert = on\n").unwrap().qr.invert);
        assert!(!parse("[qr]\ninvert = off\n").unwrap().qr.invert);
        assert!(parse("[qr]\ninvert = maybe\n").is_err());
        for text in [
            "[qr]\nmodule_mm = 0.1\n",
            "[qr]\nmodule_mm = 11\n",
            "[qr]\nline_mm = 0.01\n",
            "[qr]\nline_mm = 1.5\n",
            "[qr]\nmodule_mm = 0.5\nline_mm = 0.6\n",
            "[qr]\npower_percent = 0\n",
            "[qr]\npower_percent = 100.5\n",
            "[qr]\nspeed_mm_min = 0\n",
            "[qr]\npasses = 0\n",
            "[qr]\npasses = 21\n",
            "[qr]\nmax_bytes = 0\n",
            "[qr]\nmax_bytes = 79\n",
            "[qr]\nmax_version = 0\n",
            "[qr]\nmax_version = 5\n",
            "[qr]\necc = x\n",
            "[qr]\nmax_size_mm = 0.5\n",
            "[qr]\nmax_size_mm = 1001\n",
            "[qr]\nmax_lines = 0\n",
            "[qr]\nmax_lines = 1000001\n",
            "[qr]\ncolour = red\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
        for (word, ecc) in [
            ("L", QrEcc::L),
            (" m ", QrEcc::M),
            ("q", QrEcc::Q),
            ("H", QrEcc::H),
        ] {
            assert_eq!(QrEcc::from_word(word), Some(ecc));
            assert_eq!(QrEcc::from_word(ecc.word()), Some(ecc));
        }
        assert_eq!(QrEcc::from_word(""), None);
        let example = load(std::path::Path::new(&format!(
            "{}/examples/plate.cfg",
            env!("CARGO_MANIFEST_DIR")
        )))
        .unwrap();
        assert_eq!(example.qr, QrSettings::default());
    }

    #[test]
    fn the_text_settings_are_read_checked_and_the_defaults_are_the_ones_of_the_example() {
        assert_eq!(Settings::default().text, TextSettings::default());
        let s = parse(
            "[text]\nheight_mm = 12.5\nmax_height_mm = 40\nspacing_percent = 10\nline_spacing_percent = 80\npower_percent = 35\nspeed_mm_min = 1500\npasses = 2\nmax_chars = 50\n",
        )
        .unwrap();
        assert_eq!(
            s.text,
            TextSettings {
                height_mm: 12.5,
                max_height_mm: 40.0,
                spacing_percent: 10.0,
                line_spacing_percent: 80.0,
                power_percent: 35.0,
                speed_mm_min: 1500.0,
                passes: 2,
                max_chars: 50,
            }
        );
        for text in [
            "[text]\nheight_mm = 0.5\n",
            "[text]\nheight_mm = 101\n",
            "[text]\nmax_height_mm = 1001\n",
            "[text]\nspacing_percent = -1\n",
            "[text]\nline_spacing_percent = 301\n",
            "[text]\npower_percent = 0\n",
            "[text]\npower_percent = 100.5\n",
            "[text]\nspeed_mm_min = 0\n",
            "[text]\npasses = 0\n",
            "[text]\npasses = 21\n",
            "[text]\nmax_chars = 0\n",
            "[text]\nmax_chars = 5001\n",
            "[text]\ncolour = red\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
        // the example file says the same as the defaults, for every section that has keys of this kind
        let example = load(std::path::Path::new(&format!(
            "{}/examples/plate.cfg",
            env!("CARGO_MANIFEST_DIR")
        )))
        .unwrap();
        assert_eq!(example.text, TextSettings::default());
        assert_eq!(example.calibrate, CalibrateSettings::default());
        assert_eq!(example.verify, VerifySettings::default());
    }

    #[test]
    fn the_calibrate_settings_are_read_and_the_defaults_are_the_ones_of_the_example() {
        assert_eq!(Settings::default().calibrate, CalibrateSettings::default());
        assert!(CalibrateSettings::default().labels);
        let s = parse(
            "[calibrate]\nsteps = 7\nspread_percent = 25.5\nlabels = off\nlabel_mm = 4\nmin_power_percent = 2.5\nmin_feed_mm_min = 50\n",
        )
        .unwrap();
        assert_eq!(
            s.calibrate,
            CalibrateSettings {
                steps: 7,
                spread_percent: 25.5,
                labels: false,
                label_mm: 4.0,
                min_power_percent: 2.5,
                min_feed_mm_min: 50.0,
            }
        );
        for text in [
            "[calibrate]\nsteps = 0\n",
            "[calibrate]\nsteps = 13\n",
            "[calibrate]\nsteps = 2.5\n",
            "[calibrate]\nspread_percent = -1\n",
            "[calibrate]\nspread_percent = 90.1\n",
            "[calibrate]\nlabels = maybe\n",
            "[calibrate]\nlabel_mm = 0.9\n",
            "[calibrate]\nlabel_mm = 20.1\n",
            "[calibrate]\nmin_power_percent = 0\n",
            "[calibrate]\nmin_power_percent = 101\n",
            "[calibrate]\nmin_feed_mm_min = 0\n",
            "[calibrate]\nmin_feed_mm_min = 30001\n",
            "[calibrate]\nspeed = 5\n",
        ] {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }
}
