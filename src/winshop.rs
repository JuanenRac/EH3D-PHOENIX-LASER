// EH3D-PHOENIX-LASER - what the window does at the machine and with the results of the tests.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! The part of the state of the window for the shop floor: the saved answer of the controller to `$$` (once it is read, a serious finding refuses the export and the send), the meaning of an
//! alarm or of an error, the extra passes of the layers, the result of a test written into the library and another library brought into it. Nothing here talks to a machine: the answer of the
//! controller is a file the person saved and the alarm is text the person types.

use crate::calibrate::kerf_from_gap;
use crate::grblcodes;
use crate::grblset::{self, Finding};
use crate::libedit::{self, Measured, OnClash};
use crate::safety;
use crate::sublayers::{self, SubLayer, MAX_SUBLAYERS_PER_LAYER};
use crate::vconfig::{LayerMode, LayerParams};
use crate::window::{name_of, read_file_up_to, Status, Window};
use std::path::{Path, PathBuf};

/// The boxes of an extra pass, in this order.
pub const PASS_BOXES: usize = 3;

/// An extra pass over a layer as the window shows it: its power, its speed and its passes are typed; the rest is what the pass was made with.
#[derive(Debug, Clone, PartialEq)]
pub struct SubRow {
    pub layer: String,
    pub label: String,
    pub mode: LayerMode,
    /// Power (percent), speed (mm/min) and passes.
    pub fields: [String; PASS_BOXES],
    base: LayerParams,
}

impl SubRow {
    fn new(layer: &str, label: &str, p: &LayerParams) -> SubRow {
        SubRow {
            layer: layer.to_string(),
            label: label.to_string(),
            mode: p.mode,
            fields: [
                p.power_percent.to_string(),
                p.speed_mm_min.to_string(),
                p.passes.to_string(),
            ],
            base: *p,
        }
    }
}

/// The boxes of the result of a test, as they are typed; an empty box leaves that value of the material as it is.
#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    /// The material; empty for the one that is chosen in the library.
    pub name: String,
    pub power: String,
    pub speed: String,
    pub passes: String,
    /// The gap measured on the strips of the test of the width of the beam (mm).
    pub gap: String,
    pub strips: String,
}

impl Default for TestResult {
    fn default() -> Self {
        TestResult {
            name: String::new(),
            power: String::new(),
            speed: String::new(),
            passes: String::new(),
            gap: String::new(),
            strips: "10".to_string(),
        }
    }
}

/// The state of this part of the window.
#[derive(Debug, Default)]
pub struct Shop {
    /// The saved answer of the controller that was read: the name of its file and its settings.
    pub controller: Option<(String, grblset::Dump)>,
    /// A full power of the controller that is not the one of the box is only a warning (off unless the settings or the person say it).
    pub s_max_warns: bool,
    /// The alarm or the error that is typed to be explained.
    pub alarm: String,
    /// The extra passes of the layers of the drawing.
    pub passes: Vec<SubRow>,
    /// The row of the table of layers a new pass is made for, and its name.
    pub new_layer: usize,
    pub new_label: String,
    /// A line that is drawn twice in a layer is cut once.
    pub merge_doubled: bool,
    pub result: TestResult,
    /// The result that would write over other values and has to be asked for again.
    pub(crate) record_armed: Option<(String, Measured)>,
    /// Another library whose merge waits for the person: its file and the names both have with other values.
    pub merge_pending: Option<(PathBuf, Vec<String>)>,
}

fn typed(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(Some)
        .ok_or_else(|| format!("{what} '{t}' is not a number"))
}

fn whole(v: f64, what: &str) -> Result<u32, String> {
    if v.fract() == 0.0 && (1.0..=1_000_000.0).contains(&v) {
        Ok(v as u32)
    } else {
        Err(format!("{what} {v} is not a whole number from 1 up"))
    }
}

impl Window {
    /// Says what went wrong; false, for the callers that answer whether it was done.
    fn refused(&mut self, why: String) -> bool {
        self.status = Status::Failed(why);
        false
    }

    // ------------------------------------------------------------ the controller

    /// Reads a saved answer of the controller to `$$`. A file that cannot be read leaves the one that was read before.
    pub fn load_controller(&mut self, path: &Path) -> bool {
        let read = read_file_up_to(path, grblset::MAX_DUMP_BYTES as u64).and_then(|bytes| {
            grblset::parse(&String::from_utf8_lossy(&bytes))
                .map_err(|e| format!("{}: {e}", path.display()))
        });
        match read {
            Ok(dump) => {
                let count = dump.len();
                self.shop.controller = Some((name_of(path), dump));
                self.mark_stale();
                self.status = Status::ControllerLoaded(name_of(path), count);
                true
            }
            Err(e) => self.refused(e),
        }
    }

    /// Lets the answer of the controller go: its state is unknown again.
    pub fn forget_controller(&mut self) {
        if self.shop.controller.take().is_some() {
            self.mark_stale();
            self.status = Status::Ready;
        }
    }

    /// What the answer that was read says against the `S` of full power of the box and the bed of the settings; `None` when no answer was read. The speed of a program is compared when it is
    /// exported, sent or reported.
    pub fn controller_findings(&self) -> Option<Vec<Finding>> {
        let (_, dump) = self.shop.controller.as_ref()?;
        let s_max = self.s_max_of_box();
        let found = grblset::check(
            dump,
            &grblset::Expect {
                s_max,
                bed_mm: self.settings.machine.bed_mm,
                max_feed_mm_min: None,
            },
        );
        Some(grblset::judge(found, self.shop.s_max_warns))
    }

    /// The switch that turns a different full power into a warning.
    pub fn set_s_max_warns(&mut self, on: bool) {
        if self.shop.s_max_warns != on {
            self.shop.s_max_warns = on;
            self.mark_stale();
        }
    }

    /// The program may not be sent when the answer of the controller that was read has a serious finding against it.
    pub(crate) fn controller_refuses(&self, program: &str, s_max: u32) -> Result<(), Status> {
        let Some((_, dump)) = &self.shop.controller else {
            return Ok(());
        };
        let (Some(limits), Some(mut verdict)) =
            (self.verify_limits(s_max), self.verdict_of(program, s_max))
        else {
            return Err(Status::Failed(
                "the program could not be compared with the controller: check the boxes of the limits"
                    .to_string(),
            ));
        };
        let serious: Vec<Finding> = verdict
            .add_controller(dump, &limits, self.shop.s_max_warns)
            .into_iter()
            .filter(|f| f.kind.serious())
            .collect();
        if serious.is_empty() {
            Ok(())
        } else {
            Err(Status::ControllerRefused(serious))
        }
    }

    /// The meaning of the alarm or the error that is typed, in the language of the window; `None` while the box is empty.
    pub fn alarm_answer(&self) -> Option<Result<&'static str, String>> {
        let text = self.shop.alarm.trim();
        (!text.is_empty()).then(|| grblcodes::explain(text, self.lang))
    }

    // ------------------------------------------------------------ the extra passes

    /// The extra passes of the settings file, for the layers of the drawing that is open.
    pub(crate) fn rebuild_passes(&mut self) {
        self.shop.passes = self
            .settings
            .sublayers
            .iter()
            .filter(|sub| self.rows.iter().any(|r| r.key == sub.layer))
            .map(|sub| SubRow::new(&sub.layer, &sub.label, &sub.params))
            .collect();
        self.shop.new_layer = 0;
    }

    fn try_add_pass(&mut self) -> Result<(), String> {
        let row = self
            .rows
            .get(self.shop.new_layer)
            .cloned()
            .ok_or_else(|| "open a drawing first".to_string())?;
        let label = self.shop.new_label.trim().to_lowercase();
        sublayers::check_label(&label)?;
        let of_layer = self.shop.passes.iter().filter(|p| p.layer == row.key);
        if of_layer.clone().any(|p| p.label == label) {
            return Err(format!("sublayer {}/{label} is there already", row.key));
        }
        if of_layer.count() >= MAX_SUBLAYERS_PER_LAYER {
            return Err(format!(
                "layer {}: more than {MAX_SUBLAYERS_PER_LAYER} sub-layers",
                row.key
            ));
        }
        let params = self.layer_params(&row).map_err(|s| s.text(self.lang))?;
        self.shop
            .passes
            .push(SubRow::new(&row.key, &label, &params));
        self.shop.new_label.clear();
        Ok(())
    }

    /// Adds a pass to the layer that is chosen, with the settings its row has now. It lives in the window: the settings file is not written.
    pub fn add_pass(&mut self) -> bool {
        match self.try_add_pass() {
            Ok(()) => {
                self.mark_stale();
                true
            }
            Err(e) => self.refused(e),
        }
    }

    pub fn remove_pass(&mut self, at: usize) {
        if at < self.shop.passes.len() {
            self.shop.passes.remove(at);
            self.mark_stale();
        }
    }

    pub fn set_pass_field(&mut self, at: usize, field: usize, text: String) {
        if let Some(slot) = self
            .shop
            .passes
            .get_mut(at)
            .and_then(|p| p.fields.get_mut(field))
        {
            if *slot != text {
                *slot = text;
                self.mark_stale();
            }
        }
    }

    pub fn set_pass_mode(&mut self, at: usize, mode: LayerMode) {
        if let Some(p) = self.shop.passes.get_mut(at) {
            if p.mode != mode {
                p.mode = mode;
                self.mark_stale();
            }
        }
    }

    pub fn set_merge_doubled(&mut self, on: bool) {
        if self.shop.merge_doubled != on {
            self.shop.merge_doubled = on;
            self.mark_stale();
        }
    }

    /// The extra passes as the settings of the work take them; a box that is not a number names its pass.
    pub(crate) fn sublayers_of_window(&self) -> Result<Vec<SubLayer>, Status> {
        self.shop
            .passes
            .iter()
            .map(|row| {
                let name = format!("sublayer {}/{}", row.layer, row.label);
                let value = |i: usize| {
                    typed(&row.fields[i], &name)
                        .and_then(|v| v.ok_or_else(|| format!("{name}: a box is empty")))
                };
                let mut params = row.base;
                params.mode = row.mode;
                params.power_percent = value(0).map_err(Status::Failed)?;
                params.speed_mm_min = value(1).map_err(Status::Failed)?;
                params.passes = value(2)
                    .and_then(|v| whole(v, &name))
                    .map_err(Status::Failed)?;
                Ok(SubLayer {
                    layer: row.layer.clone(),
                    label: row.label.clone(),
                    params,
                })
            })
            .collect()
    }

    /// The `max_power` and `max_speed` of the material that is applied hold over what the window works out, as they do on the command line.
    pub(crate) fn material_ceiling(&self, power: f64, speed: f64) -> Result<(), Status> {
        let Some(m) = self
            .material
            .as_deref()
            .and_then(|name| self.materials.as_ref()?.find(name))
        else {
            return Ok(());
        };
        m.check_limits(Some(power), Some(speed))
            .map_err(Status::Failed)
    }

    // ------------------------------------------------------------ the library

    fn measured(&self) -> Result<(String, Measured), String> {
        let r = &self.shop.result;
        let name = match r.name.trim() {
            "" => self
                .material
                .clone()
                .ok_or_else(|| "type the name of the material or choose one".to_string())?,
            name => name.to_string(),
        };
        let (max_power, max_speed) = self.limits().map_err(|s| s.text(self.lang))?;
        let mut m = Measured {
            power: typed(&r.power, "the power")?,
            speed: typed(&r.speed, "the speed")?,
            ..Measured::default()
        };
        if let Some(p) = m.power {
            safety::check_power("the power", p, max_power)?;
        }
        if let Some(v) = m.speed {
            safety::check_speed("the speed", v, max_speed)?;
        }
        if let Some(v) = typed(&r.passes, "the passes")? {
            m.passes = Some(whole(v, "the passes")?);
        }
        if let Some(gap) = typed(&r.gap, "the gap")? {
            let strips = typed(&r.strips, "the strips")?
                .ok_or_else(|| "the number of strips is empty".to_string())?;
            let width = kerf_from_gap(gap, whole(strips, "the strips")?)?;
            m.kerf = Some((width * 1000.0).round() / 1000.0);
        }
        Ok((name, m))
    }

    fn try_record(&mut self) -> Result<Status, String> {
        let armed = self.shop.record_armed.take();
        let (name, m) = self.measured()?;
        let (path, text) = self.materials_text()?;
        let replace = armed.as_ref() == Some(&(name.clone(), m));
        let made = libedit::record(&text, &name, &m, replace)?;
        if !made.clashes.is_empty() {
            let list: Vec<String> = made
                .clashes
                .iter()
                .map(|c| format!("{} {} -> {}", c.key, c.old, c.new))
                .collect();
            self.shop.record_armed = Some((name.clone(), m));
            return Ok(Status::RecordAsk(name, list.join(", ")));
        }
        if made.changed.is_empty() {
            return Ok(Status::RecordUnchanged(name));
        }
        self.materials_written(&path, &made.text)?;
        self.mark_stale();
        Ok(Status::Recorded(name, name_of(&path)))
    }

    /// Writes the result of a test into the library that is open. Values the material has already are not written over until the button is pressed a second time for the same result.
    pub fn record_result(&mut self) -> bool {
        match self.try_record() {
            Ok(status) => {
                let written = matches!(status, Status::Recorded(..));
                self.status = status;
                written
            }
            Err(e) => self.refused(e),
        }
    }

    fn try_merge(&mut self, other: &Path, on_clash: OnClash) -> Result<bool, String> {
        let (path, text) = self.materials_text()?;
        let theirs = libedit::read_text(other)?;
        let made = libedit::merge(&text, &theirs, on_clash)?;
        if made.stopped {
            self.status = Status::MergeAsk(made.clashed.join(", "));
            self.shop.merge_pending = Some((other.to_path_buf(), made.clashed));
            return Ok(false);
        }
        if made.text != text {
            self.materials_written(&path, &made.text)?;
            self.mark_stale();
        }
        let replaced = if on_clash == OnClash::TakeTheirs {
            made.clashed.len()
        } else {
            0
        };
        self.shop.merge_pending = None;
        self.status = Status::Merged(made.added.len(), replaced, name_of(&path));
        Ok(true)
    }

    /// Brings another library into the one that is open. When both have a material with other values nothing is written and the window asks which stays.
    pub fn merge_library(&mut self, other: &Path) -> bool {
        self.shop.merge_pending = None;
        match self.try_merge(other, OnClash::Stop) {
            // a merge that waits for the person has chosen its file: the window of files may close
            Ok(_) => true,
            Err(e) => self.refused(e),
        }
    }

    /// The answer to the question of a merge that waits.
    pub fn merge_resolve(&mut self, take_theirs: bool) -> bool {
        let Some((other, _)) = self.shop.merge_pending.clone() else {
            return false;
        };
        let on_clash = if take_theirs {
            OnClash::TakeTheirs
        } else {
            OnClash::KeepMine
        };
        match self.try_merge(&other, on_clash) {
            Ok(done) => done,
            Err(e) => self.refused(e),
        }
    }

    pub fn merge_cancel(&mut self) {
        if self.shop.merge_pending.take().is_some() {
            self.status = Status::Ready;
        }
    }
}
