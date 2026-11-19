//! The state of the window without a screen: the picture that is open, the settings, the engraving (or the calibration grid) that is worked out, the passes of the laser to draw, the
//! file to write and the status line. The window (`gui`) only draws this and passes on what the person does, so everything that can be decided is decided here, in code that is tested.
//! The engraving is made with the same functions as the command line (`dither`, `engrave`, `grid`) and written with the same header.

use crate::calibrate::{focus_test, grid, Grid};
use crate::cut::{layers as layer_list, plan as plan_cut};
use crate::dither::{dither, Method};
use crate::dxf::read_dxf;
use crate::frame::{self, Frame};
use crate::gcode_out::{header, sha256_hex, Header};
use crate::image::{from_bytes, Gray};
use crate::lang::{fill, Lang, Msg};
use crate::materials::{self, Library, Material};
use crate::preview;
use crate::raster::{engrave, overscan_for, Params};
use crate::safety;
use crate::svg::read_svg;
use crate::tone::Tone;
use crate::trace;
use crate::vconfig::{
    self, FocusSettings, FrameSettings, KerfSide, LayerMode, LayerParams, Settings, TraceSettings,
};
use crate::vector::{Drawing, ReadOptions};
use crate::viewport::Viewport;
use std::path::{Path, PathBuf};

/// The largest file the window reads.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// The most passes (pieces of the path of the head) that are kept to be drawn.
pub const MAX_SEGMENTS: usize = 300_000;

/// What a file chosen in the window is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// A picture (PNG, PGM) or a drawing (SVG, DXF).
    OpenImage,
    /// A settings file for the vector work.
    OpenConfig,
    ExportGcode,
    /// A library of materials.
    OpenMaterials,
    /// A G-code program (this program's or another's) to see, count and check.
    OpenGcode,
}

impl Job {
    pub fn is_export(self) -> bool {
        self == Job::ExportGcode
    }

    /// The extensions the list of files shows.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Job::OpenImage => &["png", "pgm", "svg", "dxf"],
            Job::OpenConfig => &["cfg"],
            Job::ExportGcode => &["nc", "ngc", "gcode", "gc", "tap", "txt"],
            Job::OpenMaterials => &["cfg"],
            Job::OpenGcode => &["nc", "ngc", "gcode", "gc", "tap", "txt"],
        }
    }

    pub fn title(self) -> Msg {
        match self {
            Job::OpenImage => Msg::TitleOpenImage,
            Job::OpenConfig => Msg::TitleOpenSettings,
            Job::ExportGcode => Msg::TitleExport,
            Job::OpenMaterials => Msg::TitleOpenMaterials,
            Job::OpenGcode => Msg::TitleOpenGcode,
        }
    }
}

/// What the program makes: the engraving of a picture, the grid that finds the power and the feed that suit a material, or the cutting and engraving of a vector drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Engrave,
    Calibrate,
    Cut,
    /// The focus test: short lines side by side, each burnt with the head at another height.
    Focus,
    /// The frame of the work: the rectangle that holds it, traced with the laser off (or, only when the person confirms it, with a weak pilot power).
    Frame,
    /// A G-code program that was opened to see: its path, its time and what looks dangerous. It is never written again.
    Preview,
    /// The outline of the picture that is open (a logo, a silhouette, a stencil), cut or filled with lines like the shapes of a drawing.
    Trace,
}

/// How a grey picture becomes dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    FloydSteinberg,
    Atkinson,
    Bayer,
    Threshold,
}

impl Kind {
    pub const ALL: [Kind; 4] = [
        Kind::FloydSteinberg,
        Kind::Atkinson,
        Kind::Bayer,
        Kind::Threshold,
    ];

    /// The kind of dots a word of a material or of the command line names.
    pub fn from_word(word: &str) -> Option<Kind> {
        match word.trim().to_ascii_lowercase().as_str() {
            "fs" | "floyd-steinberg" => Some(Kind::FloydSteinberg),
            "atkinson" => Some(Kind::Atkinson),
            "bayer" | "ordered" => Some(Kind::Bayer),
            "threshold" => Some(Kind::Threshold),
            _ => None,
        }
    }

    pub fn label(self) -> Msg {
        match self {
            Kind::FloydSteinberg => Msg::DitherFs,
            Kind::Atkinson => Msg::DitherAtkinson,
            Kind::Bayer => Msg::DitherBayer,
            Kind::Threshold => Msg::DitherThreshold,
        }
    }
}

/// A setting that is typed in a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    WidthMm,
    DotMm,
    Threshold,
    Feed,
    Power,
    SMax,
    Overscan,
    Accel,
    OriginX,
    OriginY,
    Powers,
    Feeds,
    Dpi,
    Tolerance,
    Gamma,
    Contrast,
    Brightness,
    MaxPower,
    MaxSpeed,
    ZStart,
    ZStep,
    FocusLines,
    FocusLength,
    FocusSpacing,
    FocusFeed,
    FocusPower,
    FrameMargin,
    FramePilot,
    FrameLaps,
    FrameFeed,
    TraceThreshold,
    TraceSimplify,
    TraceMinArea,
}

/// The group of the side panel a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Image,
    Laser,
    Calibration,
    Vector,
    Focus,
    Frame,
    Trace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: Key,
    pub label: Msg,
    pub section: Section,
    pub text: String,
}

fn default_fields() -> Vec<Field> {
    let f = |key, label, section, text: &str| Field {
        key,
        label,
        section,
        text: text.to_string(),
    };
    vec![
        f(Key::WidthMm, Msg::WidthMm, Section::Image, "50"),
        f(Key::DotMm, Msg::DotMm, Section::Image, "0.1"),
        f(Key::Threshold, Msg::Threshold, Section::Image, "128"),
        f(Key::Gamma, Msg::Gamma, Section::Image, "1"),
        f(Key::Contrast, Msg::Contrast, Section::Image, "0"),
        f(Key::Brightness, Msg::Brightness, Section::Image, "0"),
        f(Key::Feed, Msg::Feed, Section::Laser, "3000"),
        f(Key::Power, Msg::Power, Section::Laser, "50"),
        f(Key::SMax, Msg::SMax, Section::Laser, "1000"),
        f(Key::Overscan, Msg::Overscan, Section::Laser, "2"),
        f(Key::Accel, Msg::Accel, Section::Laser, "1000"),
        f(Key::OriginX, Msg::OriginX, Section::Laser, "0"),
        f(Key::OriginY, Msg::OriginY, Section::Laser, "0"),
        f(Key::MaxPower, Msg::MaxPower, Section::Laser, "100"),
        f(Key::MaxSpeed, Msg::MaxSpeed, Section::Laser, "30000"),
        f(
            Key::Powers,
            Msg::Powers,
            Section::Calibration,
            "20,40,60,80",
        ),
        f(
            Key::Feeds,
            Msg::Feeds,
            Section::Calibration,
            "1000,2000,3000",
        ),
        f(Key::Dpi, Msg::Dpi, Section::Vector, "96"),
        f(Key::Tolerance, Msg::Tolerance, Section::Vector, "0.01"),
        f(Key::ZStart, Msg::ZStart, Section::Focus, "0"),
        f(Key::ZStep, Msg::ZStep, Section::Focus, "0"),
        f(Key::FocusLines, Msg::FocusLines, Section::Focus, "9"),
        f(Key::FocusLength, Msg::FocusLength, Section::Focus, "10"),
        f(Key::FocusSpacing, Msg::FocusSpacing, Section::Focus, "2"),
        f(Key::FocusFeed, Msg::Feed, Section::Focus, "1000"),
        f(Key::FocusPower, Msg::Power, Section::Focus, "30"),
        f(Key::FrameMargin, Msg::Margin, Section::Frame, "0"),
        f(Key::FramePilot, Msg::PilotPower, Section::Frame, "0"),
        f(Key::FrameLaps, Msg::Laps, Section::Frame, "1"),
        f(Key::FrameFeed, Msg::FrameFeed, Section::Frame, "3000"),
        f(Key::TraceThreshold, Msg::Threshold, Section::Trace, "128"),
        f(
            Key::TraceSimplify,
            Msg::TraceSimplify,
            Section::Trace,
            "0.02",
        ),
        f(Key::TraceMinArea, Msg::TraceMinArea, Section::Trace, "0.05"),
    ]
}

/// A setting of a layer that is typed in a box of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerField {
    Power,
    Speed,
    Passes,
    Focus,
    Kerf,
    Tabs,
    TabWidth,
    HatchAngle,
    HatchSpacing,
}

impl LayerField {
    pub const ALL: [LayerField; 9] = [
        LayerField::Power,
        LayerField::Speed,
        LayerField::Passes,
        LayerField::Focus,
        LayerField::Kerf,
        LayerField::Tabs,
        LayerField::TabWidth,
        LayerField::HatchAngle,
        LayerField::HatchSpacing,
    ];

    pub fn label(self) -> Msg {
        match self {
            LayerField::Power => Msg::LayerPower,
            LayerField::Speed => Msg::LayerSpeed,
            LayerField::Passes => Msg::Passes,
            LayerField::Focus => Msg::FocusStep,
            LayerField::Kerf => Msg::Kerf,
            LayerField::Tabs => Msg::Tabs,
            LayerField::TabWidth => Msg::TabWidth,
            LayerField::HatchAngle => Msg::HatchAngle,
            LayerField::HatchSpacing => Msg::HatchSpacing,
        }
    }
}

/// A row of the table of layers: a layer or a colour of the drawing with what is done with it.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRow {
    pub key: String,
    pub shapes: usize,
    pub mode: LayerMode,
    pub kerf_side: KerfSide,
    /// The text of each box, in the order of [`LayerField::ALL`].
    pub fields: [String; 9],
}

impl LayerRow {
    fn new(key: &str, shapes: usize, p: &LayerParams) -> LayerRow {
        let t = |v: f64| format!("{v}");
        LayerRow {
            key: key.to_string(),
            shapes,
            mode: p.mode,
            kerf_side: p.kerf_side,
            fields: [
                t(p.power_percent),
                t(p.speed_mm_min),
                p.passes.to_string(),
                t(p.focus_step_mm),
                t(p.kerf_mm),
                p.tabs.to_string(),
                t(p.tab_width_mm),
                t(p.hatch_angle_deg),
                t(p.hatch_spacing_mm),
            ],
        }
    }
}

/// A straight pass of the head, seen from above.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    /// The laser is on.
    pub burn: bool,
    /// The power, from 0 to 1 of the full power.
    pub level: f64,
}

/// The passes of a G-code program of the laser, read from its text (the same text that is written), and whether the list was cut short at [`MAX_SEGMENTS`]. A move with `G1` is a
/// pass; it burns when the spindle value `S` is more than 0. `smax` is the value that is full power.
pub fn passes(gcode: &str, smax: u32) -> (Vec<Seg>, bool) {
    passes_of(gcode, smax, false)
}

/// The same, and with `rapid` also the moves with `G0` (the travel between the contours of a drawing) as passes with the laser off.
pub fn passes_of(gcode: &str, smax: u32, rapid: bool) -> (Vec<Seg>, bool) {
    let mut segs = Vec::new();
    let (mut x, mut y, mut s) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut line_mode = false;
    for raw in gcode.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('(') {
            continue;
        }
        let (mut nx, mut ny) = (x, y);
        let mut moved = false;
        for token in line.split_whitespace() {
            let mut chars = token.chars();
            let Some(letter) = chars.next() else {
                continue;
            };
            let Ok(v) = chars.as_str().parse::<f64>() else {
                continue;
            };
            if !v.is_finite() {
                continue;
            }
            match letter.to_ascii_uppercase() {
                'G' if v == 0.0 => line_mode = false,
                'G' if v == 1.0 => line_mode = true,
                'X' => {
                    nx = v;
                    moved = true;
                }
                'Y' => {
                    ny = v;
                    moved = true;
                }
                'S' => s = v,
                _ => {}
            }
        }
        if moved && (line_mode || rapid) && (nx - x).hypot(ny - y) > 0.0 {
            if segs.len() >= MAX_SEGMENTS {
                return (segs, true);
            }
            segs.push(Seg {
                x0: x,
                y0: y,
                x1: nx,
                y1: ny,
                burn: line_mode && s > 0.0,
                level: (s / f64::from(smax.max(1))).clamp(0.0, 1.0),
            });
        }
        x = nx;
        y = ny;
    }
    (segs, false)
}

/// The words that say what a program does: the moves, the burnt and travelled length and the time it takes.
fn preview_line(lang: Lang, moves: usize, burn_mm: f64, travel_mm: f64, seconds: f64) -> String {
    fill(
        Msg::PreviewLine.tr(lang),
        &[
            &moves.to_string(),
            &format!("{burn_mm:.0}"),
            &format!("{travel_mm:.0}"),
            &preview::clock(seconds),
        ],
    )
}

/// The colour of a burning pass between the lowest power (0) and the full power (1): dark red, red, orange, yellow.
pub fn fire(t: f64) -> [u8; 3] {
    const STOPS: [[f64; 3]; 4] = [
        [0.45, 0.05, 0.05],
        [0.95, 0.15, 0.10],
        [1.00, 0.60, 0.10],
        [1.00, 0.95, 0.55],
    ];
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let pos = t * 3.0;
    let i = (pos.floor() as usize).min(2);
    let f = pos - i as f64;
    std::array::from_fn(|k| {
        let v = STOPS[i][k] + (STOPS[i + 1][k] - STOPS[i][k]) * f;
        (v * 255.0).round() as u8
    })
}

/// The bytes of a file, if it is not too big.
fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("{shown}: the file is larger than the window reads"));
    }
    std::fs::read(path).map_err(|e| format!("{shown}: {e}"))
}

/// Written beside and renamed, so that a file is never left half written.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
}

/// True when two paths name the same file (compared once resolved; by their text when one does not exist yet).
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The extension of a file in lower case ("" when it has none).
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// The bytes of a file, if it is not larger than `max` bytes.
fn read_file_up_to(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > max {
        return Err(format!(
            "{shown}: the file is larger than {max} bytes (max_file_bytes)"
        ));
    }
    std::fs::read(path).map_err(|e| format!("{shown}: {e}"))
}

/// Reads a drawing according to the extension of its file.
fn parse_drawing(bytes: &[u8], ext: &str, opts: &ReadOptions) -> Result<Drawing, String> {
    match ext {
        "svg" => read_svg(bytes, opts),
        "dxf" => read_dxf(bytes, opts),
        _ => Err("not a drawing this program reads (SVG, DXF)".to_string()),
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The G-code program that is open to be seen.
#[derive(Debug, Clone)]
pub struct ProgramFile {
    pub path: PathBuf,
    pub name: String,
    pub text: String,
}

/// The picture that is open.
#[derive(Debug, Clone)]
pub struct Picture {
    pub path: PathBuf,
    pub name: String,
    pub bytes: Vec<u8>,
    pub gray: Gray,
}

/// The drawing that is open: the file as it was read (its bytes, for the hash of the header and to read it again when the dpi or the tolerance changes) and what it holds.
#[derive(Debug, Clone)]
pub struct VectorFile {
    pub path: PathBuf,
    pub name: String,
    /// `svg` or `dxf`.
    pub ext: String,
    pub bytes: Vec<u8>,
    pub drawing: Drawing,
}

/// What was worked out, for the status line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Summary {
    Engraved {
        dots_w: usize,
        dots_h: usize,
        width_mm: f64,
        height_mm: f64,
        rows: usize,
        runs: usize,
        burn_mm: f64,
        seconds: f64,
    },
    Calibrated {
        squares: usize,
    },
    Focused {
        lines: u32,
        length_mm: f64,
    },
    /// A program that was opened to be seen.
    Previewed {
        moves: usize,
        burn_mm: f64,
        travel_mm: f64,
        seconds: f64,
    },
    /// The rectangle of the work; `pilot_percent` is 0 when the laser stays off.
    Framed {
        width_mm: f64,
        height_mm: f64,
        laps: u32,
        pilot_percent: f64,
    },
    Cut {
        layers: usize,
        paths: usize,
        cut_mm: f64,
        engrave_mm: f64,
        travel_mm: f64,
        seconds: f64,
    },
    /// The outline of a picture, cut or engraved.
    Traced {
        contours: usize,
        points: usize,
        cut_mm: f64,
        engrave_mm: f64,
        seconds: f64,
    },
}

/// The G-code that was worked out and what goes in its header.
#[derive(Debug, Clone)]
pub struct Done {
    pub gcode: String,
    /// The name of the picture and its SHA-256 (none for a calibration grid).
    pub source: Option<(String, String)>,
    pub tools: String,
    pub s_max: u32,
    /// The width of a line of burning, for the picture.
    pub line_mm: f64,
    pub summary: Summary,
    /// What was skipped or could not be done, for the list of notes.
    pub notes: Vec<String>,
}

/// What the status line says.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    ImageOpened {
        name: String,
        w: usize,
        h: usize,
    },
    VectorOpened {
        name: String,
        shapes: usize,
        layers: usize,
    },
    ConfigLoaded(String),
    Done(Summary),
    Written(String),
    NeedImage,
    NeedVector,
    /// The work would turn the laser on and the person has not confirmed it.
    NeedConfirm,
    /// A program that was opened to be seen is not written again.
    NothingToWrite,
    BadValue(Msg),
    SameFile(String),
    /// A library of materials was read: its name and how many materials it holds.
    MaterialsLoaded(String, usize),
    /// A material was applied to the boxes of the window.
    MaterialApplied(String),
    /// A machine profile was chosen: its name, its full-power value and its limits of power and speed.
    ProfileApplied {
        name: String,
        s_max: u32,
        max_power: f64,
        max_speed: f64,
    },
    Failed(String),
}

impl Status {
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Status::NeedImage
                | Status::NeedVector
                | Status::NeedConfirm
                | Status::NothingToWrite
                | Status::BadValue(_)
                | Status::SameFile(_)
                | Status::Failed(_)
        )
    }

    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::ImageOpened { name, w, h } => fill(
                Msg::ImageOpened.tr(lang),
                &[name, &w.to_string(), &h.to_string()],
            ),
            Status::Done(Summary::Engraved {
                dots_w,
                dots_h,
                width_mm,
                height_mm,
                rows,
                runs,
                burn_mm,
                seconds,
            }) => fill(
                Msg::Engraved.tr(lang),
                &[
                    &dots_w.to_string(),
                    &dots_h.to_string(),
                    &format!("{width_mm:.1}"),
                    &format!("{height_mm:.1}"),
                    &rows.to_string(),
                    &runs.to_string(),
                    &format!("{burn_mm:.0}"),
                    &format!("{seconds:.0}"),
                ],
            ),
            Status::VectorOpened {
                name,
                shapes,
                layers,
            } => fill(
                Msg::VectorOpened.tr(lang),
                &[name, &shapes.to_string(), &layers.to_string()],
            ),
            Status::ConfigLoaded(name) => fill(Msg::ConfigLoaded.tr(lang), &[name]),
            Status::Done(Summary::Calibrated { squares }) => {
                fill(Msg::Calibrated.tr(lang), &[&squares.to_string()])
            }
            Status::Done(Summary::Cut {
                layers,
                paths,
                cut_mm,
                engrave_mm,
                travel_mm,
                seconds,
            }) => fill(
                Msg::Cutting.tr(lang),
                &[
                    &layers.to_string(),
                    &paths.to_string(),
                    &format!("{cut_mm:.0}"),
                    &format!("{engrave_mm:.0}"),
                    &format!("{travel_mm:.0}"),
                    &format!("{seconds:.0}"),
                ],
            ),
            Status::Done(Summary::Traced {
                contours,
                points,
                cut_mm,
                engrave_mm,
                seconds,
            }) => fill(
                Msg::TraceMade.tr(lang),
                &[
                    &contours.to_string(),
                    &points.to_string(),
                    &format!("{cut_mm:.0}"),
                    &format!("{engrave_mm:.0}"),
                    &format!("{seconds:.0}"),
                ],
            ),
            Status::Done(Summary::Focused { lines, length_mm }) => fill(
                Msg::FocusMade.tr(lang),
                &[&lines.to_string(), &format!("{length_mm}")],
            ),
            Status::Done(Summary::Framed {
                width_mm,
                height_mm,
                laps,
                pilot_percent,
            }) => {
                let (w, h, l) = (
                    format!("{width_mm:.1}"),
                    format!("{height_mm:.1}"),
                    laps.to_string(),
                );
                if *pilot_percent > 0.0 {
                    fill(
                        Msg::FrameDonePilot.tr(lang),
                        &[&w, &h, &l, &format!("{pilot_percent}")],
                    )
                } else {
                    fill(Msg::FrameDone.tr(lang), &[&w, &h, &l])
                }
            }
            Status::Done(Summary::Previewed {
                moves,
                burn_mm,
                travel_mm,
                seconds,
            }) => preview_line(lang, *moves, *burn_mm, *travel_mm, *seconds),
            Status::NeedConfirm => Msg::NeedConfirm.tr(lang).to_string(),
            Status::NothingToWrite => Msg::NothingToWrite.tr(lang).to_string(),
            Status::Written(name) => fill(Msg::Written.tr(lang), &[name]),
            Status::NeedImage => Msg::NeedImage.tr(lang).to_string(),
            Status::NeedVector => Msg::NeedVector.tr(lang).to_string(),
            Status::BadValue(label) => fill(Msg::BadValue.tr(lang), &[label.tr(lang)]),
            Status::SameFile(name) => fill(Msg::SameFile.tr(lang), &[name]),
            Status::MaterialsLoaded(name, n) => {
                fill(Msg::MaterialsLoaded.tr(lang), &[name, &n.to_string()])
            }
            Status::MaterialApplied(name) => fill(Msg::MaterialApplied.tr(lang), &[name]),
            Status::ProfileApplied {
                name,
                s_max,
                max_power,
                max_speed,
            } => fill(
                Msg::ProfileApplied.tr(lang),
                &[
                    name,
                    &s_max.to_string(),
                    &format!("{max_power}"),
                    &format!("{max_speed}"),
                ],
            ),
            Status::Failed(e) => fill(Msg::Failed.tr(lang), &[e]),
        }
    }
}

/// The state of the whole window.
#[derive(Debug)]
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    pub mode: Mode,
    pub kind: Kind,
    /// The tone curve inverts the picture (black burns least).
    pub tone_invert: bool,
    /// The tone curve stretches the darkest level to black and the lightest to white.
    pub tone_auto_level: bool,
    /// The library of materials that is loaded, and the file it came from.
    pub materials: Option<Library>,
    pub materials_path: Option<PathBuf>,
    /// The name of the material that was applied last.
    pub material: Option<String>,
    /// The G-code program that is open to be seen, when there is one.
    pub program: Option<ProgramFile>,
    /// What the preview says about the G-code that was worked out or opened: the moves, the lengths, the time and what looks dangerous.
    pub report: Option<preview::Report>,
    /// The work that the frame is made of: the picture or the drawing, whichever was opened or chosen last.
    pub work_mode: Mode,
    /// The person has confirmed that the frame may turn the laser on. Any change to the window takes the confirmation back, and so does writing the frame.
    pub pilot_confirmed: bool,
    /// The overscan is worked out from the feed and the acceleration instead of being typed.
    pub overscan_auto: bool,
    /// The light parts of the picture are the ink when its outline is followed.
    pub trace_invert: bool,
    /// The outline is filled with lines (engraved) instead of being cut.
    pub trace_engrave: bool,
    pub picture: Option<Picture>,
    /// The drawing that is open and the table of its layers.
    pub vector: Option<VectorFile>,
    pub rows: Vec<LayerRow>,
    /// The settings read from a file (the limits, the machine, the defaults of a layer...), under what is typed in the window.
    pub settings: Settings,
    pub config_path: Option<PathBuf>,
    /// What was skipped when the drawing was read and planned.
    pub warnings: Vec<String>,
    pub done: Option<Done>,
    pub segs: Vec<Seg>,
    /// The picture of the passes was cut short at [`MAX_SEGMENTS`].
    pub truncated: bool,
    /// A setting changed after the G-code was worked out.
    pub stale: bool,
    pub status: Status,
    pub viewport: Viewport,
    pub show_travel: bool,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        Window {
            lang,
            fields: default_fields(),
            mode: Mode::Engrave,
            kind: Kind::FloydSteinberg,
            tone_invert: false,
            tone_auto_level: false,
            materials: None,
            materials_path: None,
            material: None,
            program: None,
            report: None,
            work_mode: Mode::Engrave,
            pilot_confirmed: false,
            overscan_auto: false,
            trace_invert: false,
            trace_engrave: false,
            picture: None,
            vector: None,
            rows: Vec::new(),
            settings: Settings::default(),
            config_path: None,
            warnings: Vec::new(),
            done: None,
            segs: Vec::new(),
            truncated: false,
            stale: false,
            status: Status::Ready,
            viewport: Viewport::default(),
            show_travel: true,
        }
    }

    pub fn has_work(&self) -> bool {
        self.picture.is_some()
            || self.vector.is_some()
            || self.program.is_some()
            || self.done.is_some()
    }

    /// Whether there is something to write: not a program that was opened to be seen.
    pub fn can_export(&self) -> bool {
        self.mode != Mode::Preview && self.can_compute()
    }

    /// Whether the G-code can be worked out now.
    pub fn can_compute(&self) -> bool {
        match self.mode {
            Mode::Calibrate | Mode::Focus => true,
            Mode::Frame => self.framed_mode().is_some(),
            Mode::Preview => self.program.is_some(),
            Mode::Engrave => self.picture.is_some(),
            Mode::Cut => self.vector.is_some(),
            Mode::Trace => self.picture.is_some(),
        }
    }

    fn mark_stale(&mut self) {
        // whatever changes, the laser is not lit unless the person says it again
        self.pilot_confirmed = false;
        if self.done.is_some() {
            self.stale = true;
        }
    }

    /// Writes a setting that the person typed.
    pub fn set_field(&mut self, i: usize, text: String) {
        if let Some(f) = self.fields.get_mut(i) {
            if f.text != text {
                f.text = text;
                self.mark_stale();
            }
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode != mode {
            self.mode = mode;
            if matches!(mode, Mode::Engrave | Mode::Cut | Mode::Trace) {
                self.work_mode = mode;
            }
            self.mark_stale();
        }
    }

    pub fn set_kind(&mut self, kind: Kind) {
        if self.kind != kind {
            self.kind = kind;
            self.mark_stale();
        }
    }

    pub fn set_tone_invert(&mut self, on: bool) {
        if self.tone_invert != on {
            self.tone_invert = on;
            self.mark_stale();
        }
    }

    pub fn set_tone_auto_level(&mut self, on: bool) {
        if self.tone_auto_level != on {
            self.tone_auto_level = on;
            self.mark_stale();
        }
    }

    pub fn set_trace_invert(&mut self, on: bool) {
        if self.trace_invert != on {
            self.trace_invert = on;
            self.mark_stale();
        }
    }

    pub fn set_trace_engrave(&mut self, on: bool) {
        if self.trace_engrave != on {
            self.trace_engrave = on;
            self.mark_stale();
        }
    }

    pub fn set_overscan_auto(&mut self, on: bool) {
        if self.overscan_auto != on {
            self.overscan_auto = on;
            self.mark_stale();
        }
    }

    fn field(&self, key: Key) -> &Field {
        // every key has its field: `default_fields` makes one for each
        self.fields
            .iter()
            .find(|f| f.key == key)
            .unwrap_or(&self.fields[0])
    }

    fn number(&self, key: Key) -> Result<f64, Status> {
        let f = self.field(key);
        f.text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(Status::BadValue(f.label))
    }

    fn whole(&self, key: Key, lo: f64, hi: f64) -> Result<u32, Status> {
        let v = self.number(key)?;
        if v.fract() == 0.0 && (lo..=hi).contains(&v) {
            Ok(v as u32)
        } else {
            Err(Status::BadValue(self.field(key).label))
        }
    }

    /// A list of numbers separated by commas.
    fn list(&self, key: Key) -> Result<Vec<f64>, Status> {
        let f = self.field(key);
        f.text
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or(Status::BadValue(f.label))
            })
            .collect()
    }

    /// The tone curve of the boxes and the two switches, checked the way the command line checks it.
    pub fn tone(&self) -> Result<Tone, Status> {
        let t = Tone {
            brightness: self.number(Key::Brightness)?,
            contrast: self.number(Key::Contrast)?,
            gamma: self.number(Key::Gamma)?,
            invert: self.tone_invert,
            auto_level: self.tone_auto_level,
        };
        t.validate().map_err(Status::Failed)?;
        Ok(t)
    }

    /// The limit of power (a percent of full power) and the limit of speed (mm/min) of the boxes.
    pub fn limits(&self) -> Result<(f64, f64), Status> {
        let power = self.number(Key::MaxPower)?;
        safety::validate_max(power).map_err(|_| Status::BadValue(Msg::MaxPower))?;
        let speed = self.number(Key::MaxSpeed)?;
        safety::validate_max_speed(speed).map_err(|_| Status::BadValue(Msg::MaxSpeed))?;
        Ok((power, speed))
    }

    /// The parameters of the laser from the settings, checked the way the command line checks them.
    pub fn params(&self) -> Result<Params, Status> {
        let feed = self.number(Key::Feed)?;
        let overscan = if self.overscan_auto {
            overscan_for(feed, self.number(Key::Accel)?).map_err(Status::Failed)?
        } else {
            self.number(Key::Overscan)?
        };
        let p = Params {
            pixel_mm: self.number(Key::DotMm)?,
            feed_mm_min: feed,
            s_max: self.whole(Key::SMax, 1.0, 100_000.0)?,
            power_percent: self.number(Key::Power)?,
            overscan_mm: overscan,
            origin: (self.number(Key::OriginX)?, self.number(Key::OriginY)?),
        };
        p.validate().map_err(Status::Failed)?;
        // a power or a speed over the limit is refused, never lowered
        let (max_power, max_speed) = self.limits()?;
        safety::check_power("the power", p.power_percent, max_power).map_err(Status::Failed)?;
        safety::check_speed("the speed", p.feed_mm_min, max_speed).map_err(Status::Failed)?;
        Ok(p)
    }

    /// The way the picture becomes dots.
    pub fn method(&self) -> Result<Method, Status> {
        Ok(match self.kind {
            Kind::FloydSteinberg => Method::FloydSteinberg,
            Kind::Atkinson => Method::Atkinson,
            Kind::Bayer => Method::Bayer,
            Kind::Threshold => {
                Method::Threshold(self.number(Key::Threshold)?.clamp(0.0, 255.0) as u8)
            }
        })
    }

    /// The name of the picture that is open.
    pub fn picture_name(&self) -> Option<String> {
        self.picture.as_ref().map(|p| p.name.clone())
    }

    /// The name of the file the work is made from: the drawing when the program is cutting, the picture otherwise.
    pub fn current_name(&self) -> Option<String> {
        match self.mode {
            Mode::Cut => self.vector.as_ref().map(|v| v.name.clone()),
            Mode::Preview => self.program.as_ref().map(|p| p.name.clone()),
            _ => self.picture_name(),
        }
    }

    /// The folder of the file that is open, where the window of files starts.
    pub fn work_folder(&self) -> Option<PathBuf> {
        let path = match self.mode {
            Mode::Cut => self.vector.as_ref().map(|v| &v.path),
            Mode::Preview => self.program.as_ref().map(|p| &p.path),
            _ => self.picture.as_ref().map(|p| &p.path),
        };
        path.and_then(|p| p.parent().map(Path::to_path_buf))
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// The name offered for the G-code to write.
    pub fn suggested_output_for(&self, job: Job) -> Option<PathBuf> {
        if job != Job::ExportGcode || self.mode == Mode::Preview {
            return None;
        }
        match (&self.picture, self.mode) {
            (_, Mode::Cut) if self.vector.is_some() => {
                let v = self.vector.as_ref()?;
                let stem = v.path.file_stem()?.to_string_lossy().into_owned();
                Some(v.path.with_file_name(format!("{stem}.nc")))
            }
            (Some(p), Mode::Engrave | Mode::Trace) => {
                let stem = p.path.file_stem()?.to_string_lossy().into_owned();
                Some(p.path.with_file_name(format!("{stem}.nc")))
            }
            (_, Mode::Focus) => Some(PathBuf::from("focus.nc")),
            (_, Mode::Frame) => Some(PathBuf::from("frame.nc")),
            _ => Some(PathBuf::from("calibration.nc")),
        }
    }

    /// Opens a PNG or PGM picture.
    pub fn open_image(&mut self, path: &Path) -> bool {
        let loaded = read_file(path).and_then(|bytes| {
            let gray = from_bytes(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok((bytes, gray))
        });
        match loaded {
            Ok((bytes, gray)) => {
                self.status = Status::ImageOpened {
                    name: name_of(path),
                    w: gray.w,
                    h: gray.h,
                };
                self.picture = Some(Picture {
                    path: path.to_path_buf(),
                    name: name_of(path),
                    bytes,
                    gray,
                });
                // what was worked out from the old picture is not valid any more; the outline stays the way of working when it was chosen
                let tracing = self.mode == Mode::Trace;
                self.work_mode = if tracing { Mode::Trace } else { Mode::Engrave };
                if matches!(
                    self.mode,
                    Mode::Engrave | Mode::Cut | Mode::Preview | Mode::Trace
                ) {
                    if !tracing {
                        self.mode = Mode::Engrave;
                    }
                    self.warnings.clear();
                    self.done = None;
                    self.segs.clear();
                    self.stale = false;
                    self.truncated = false;
                }
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Opens a G-code program (this program's or another's) to see: its path is drawn and the preview says its time and what looks dangerous. It is read as text and never run, and the
    /// window never writes it again.
    pub fn open_gcode(&mut self, path: &Path) -> bool {
        let loaded = read_file(path).map(|bytes| ProgramFile {
            path: path.to_path_buf(),
            name: name_of(path),
            text: String::from_utf8_lossy(&bytes).into_owned(),
        });
        let program = match loaded {
            Ok(p) => p,
            Err(e) => {
                self.status = Status::Failed(e);
                return false;
            }
        };
        // the program is checked before it replaces what was open
        let before = (self.program.take(), self.mode);
        self.program = Some(program);
        self.mode = Mode::Preview;
        if self.compute() {
            self.stale = false;
            true
        } else {
            let failure = self.status.clone();
            self.program = before.0;
            self.mode = before.1;
            self.status = failure;
            false
        }
    }

    /// Opens a file chosen or dropped on the window: a drawing (SVG, DXF) or a picture (anything else, which the reader of pictures refuses when it is not one).
    pub fn open_dropped(&mut self, path: &Path) -> bool {
        match extension_of(path).as_str() {
            "svg" | "dxf" => self.open_vector(path),
            "png" | "pgm" => self.open_image(path),
            "nc" | "ngc" | "gcode" | "gc" | "tap" => self.open_gcode(path),
            _ => {
                self.status = Status::Failed(format!(
                    "{}: not a file this program reads (PNG, PGM, SVG, DXF, G-code)",
                    path.display()
                ));
                false
            }
        }
    }

    /// How the drawing is read, from the boxes of the window.
    fn read_options(&self) -> Result<ReadOptions, Status> {
        let mut o = self.settings.read;
        o.dpi = self.number(Key::Dpi)?;
        o.tolerance_mm = self.number(Key::Tolerance)?;
        o.validate().map_err(Status::Failed)?;
        Ok(o)
    }

    /// The settings of a layer from its row: what the settings file says for it with what is typed in the table over it.
    fn layer_params(&self, row: &LayerRow) -> Result<LayerParams, Status> {
        let num = |f: LayerField| {
            row.fields[f as usize]
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or(Status::BadValue(f.label()))
        };
        let whole = |f: LayerField| {
            let v = num(f)?;
            if v.fract() == 0.0 && (0.0..=1_000_000.0).contains(&v) {
                Ok(v as u32)
            } else {
                Err(Status::BadValue(f.label()))
            }
        };
        let mut p = self.settings.params_for(&row.key);
        p.mode = row.mode;
        p.kerf_side = row.kerf_side;
        p.power_percent = num(LayerField::Power)?;
        p.speed_mm_min = num(LayerField::Speed)?;
        p.passes = whole(LayerField::Passes)?;
        p.focus_step_mm = num(LayerField::Focus)?;
        p.kerf_mm = num(LayerField::Kerf)?;
        p.tabs = whole(LayerField::Tabs)?;
        p.tab_width_mm = num(LayerField::TabWidth)?;
        p.hatch_angle_deg = num(LayerField::HatchAngle)?;
        p.hatch_spacing_mm = num(LayerField::HatchSpacing)?;
        Ok(p)
    }

    /// All the settings of the vector work: the file of settings under the boxes of the window and the table of layers.
    pub fn vector_settings(&self) -> Result<Settings, Status> {
        let mut s = self.settings.clone();
        s.read = self.read_options()?;
        s.machine.s_max = self.whole(Key::SMax, 1.0, 100_000.0)?;
        (s.machine.max_power_percent, s.machine.max_speed_mm_min) = self.limits()?;
        s.machine.overscan_mm = self.number(Key::Overscan)?;
        s.machine.origin = (self.number(Key::OriginX)?, self.number(Key::OriginY)?);
        s.layers = self
            .rows
            .iter()
            .map(|r| Ok((r.key.clone(), self.layer_params(r)?)))
            .collect::<Result<_, Status>>()?;
        s.validate().map_err(Status::Failed)?;
        Ok(s)
    }

    /// Opens an SVG or DXF drawing and builds the table of its layers; the program then cuts instead of engraving.
    pub fn open_vector(&mut self, path: &Path) -> bool {
        let opened = self.read_options().and_then(|opts| {
            let ext = extension_of(path);
            let bytes =
                read_file_up_to(path, opts.limits.max_file_bytes as u64).map_err(Status::Failed)?;
            let drawing = parse_drawing(&bytes, &ext, &opts)
                .map_err(|e| Status::Failed(format!("{}: {e}", path.display())))?;
            Ok((ext, bytes, drawing))
        });
        match opened {
            Ok((ext, bytes, drawing)) => {
                self.warnings = drawing.warnings.clone();
                self.vector = Some(VectorFile {
                    path: path.to_path_buf(),
                    name: name_of(path),
                    ext,
                    bytes,
                    drawing,
                });
                self.rebuild_rows();
                self.mode = Mode::Cut;
                self.work_mode = Mode::Cut;
                self.done = None;
                self.segs.clear();
                self.stale = false;
                self.truncated = false;
                let shapes = self.vector.as_ref().map_or(0, |v| v.drawing.shapes.len());
                self.status = Status::VectorOpened {
                    name: name_of(path),
                    shapes,
                    layers: self.rows.len(),
                };
                true
            }
            Err(s) => {
                self.status = s;
                false
            }
        }
    }

    /// The table of layers of the drawing that is open, with what the settings say for each.
    fn rebuild_rows(&mut self) {
        let Some(v) = &self.vector else {
            self.rows.clear();
            return;
        };
        self.rows = layer_list(&v.drawing, &self.settings)
            .iter()
            .map(|l| LayerRow::new(&l.key, l.shapes, &self.settings.params_for(&l.key)))
            .collect();
    }

    fn put(&mut self, key: Key, text: String) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.key == key) {
            f.text = text;
        }
    }

    /// Reads a file of settings: the boxes of the window and the table of layers take what it says.
    pub fn load_config(&mut self, path: &Path) -> bool {
        match vconfig::load(path) {
            Ok(s) => {
                let t = |v: f64| format!("{v}");
                self.put(Key::Dpi, t(s.read.dpi));
                self.put(Key::Tolerance, t(s.read.tolerance_mm));
                self.put(Key::SMax, s.machine.s_max.to_string());
                self.put(Key::Overscan, t(s.machine.overscan_mm));
                self.put(Key::OriginX, t(s.machine.origin.0));
                self.put(Key::OriginY, t(s.machine.origin.1));
                self.put(Key::ZStart, t(s.focus.z_start_mm));
                self.put(Key::ZStep, t(s.focus.z_step_mm));
                self.put(Key::FocusLines, s.focus.steps.to_string());
                self.put(Key::FocusLength, t(s.focus.length_mm));
                self.put(Key::FocusSpacing, t(s.focus.spacing_mm));
                self.put(Key::FocusFeed, t(s.focus.feed_mm_min));
                self.put(Key::FocusPower, t(s.focus.power_percent));
                self.put(Key::FrameMargin, t(s.frame.margin_mm));
                // a pilot power in the file is only a value to show: the laser still needs the confirmation of the person
                self.put(Key::FramePilot, t(s.frame.pilot_percent));
                self.put(Key::FrameLaps, s.frame.laps.to_string());
                self.put(Key::FrameFeed, t(s.frame.feed_mm_min));
                self.put(Key::MaxPower, t(s.machine.max_power_percent));
                self.put(Key::MaxSpeed, t(s.machine.max_speed_mm_min));
                self.put(Key::Gamma, t(s.tone.gamma));
                self.put(Key::Contrast, t(s.tone.contrast));
                self.put(Key::Brightness, t(s.tone.brightness));
                self.tone_invert = s.tone.invert;
                self.tone_auto_level = s.tone.auto_level;
                self.put(Key::TraceThreshold, s.trace.threshold.to_string());
                self.put(Key::TraceSimplify, t(s.trace.simplify_mm));
                self.put(Key::TraceMinArea, t(s.trace.min_area_mm2));
                self.trace_invert = s.trace.invert;
                self.settings = s;
                self.config_path = Some(path.to_path_buf());
                self.rebuild_rows();
                self.mark_stale();
                self.status = Status::ConfigLoaded(name_of(path));
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Reads the settings file that is named by `PHOENIX_LASER_CONFIG`, or `phoenix-laser.cfg` in the folder the program runs in, when there is one.
    pub fn load_default_config(&mut self) {
        let path = std::env::var_os("PHOENIX_LASER_CONFIG")
            .map(PathBuf::from)
            .or_else(|| Some(PathBuf::from("phoenix-laser.cfg")).filter(|p| p.is_file()));
        if let Some(p) = path {
            self.load_config(&p);
        }
        self.load_default_materials();
    }

    /// Reads a library of materials; the materials are then offered by name.
    pub fn load_materials(&mut self, path: &Path) -> bool {
        match materials::load(path) {
            Ok(lib) => {
                self.status = Status::MaterialsLoaded(name_of(path), lib.materials.len());
                self.materials = Some(lib);
                self.materials_path = Some(path.to_path_buf());
                self.material = None;
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Reads the library named by `PHOENIX_LASER_MATERIALS`, the way the command line does, when there is one.
    pub fn load_default_materials(&mut self) {
        if let Some(p) = std::env::var_os("PHOENIX_LASER_MATERIALS")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
        {
            self.load_materials(&p);
        }
    }

    /// The names of the machine profiles of the settings file, in the order of the file.
    pub fn profile_names(&self) -> Vec<String> {
        self.settings
            .profiles
            .iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// Makes a machine profile of the settings file the machine, like `--profile` does: its values win over `[machine]`, and the boxes of the full-power value and of the two limits take
    /// them (the person can still change a box afterwards, and the limits hold over what the boxes say). A profile that the file does not have changes nothing.
    pub fn apply_profile(&mut self, name: &str) -> bool {
        let mut s = self.settings.clone();
        if let Err(e) = s.apply_profile(name).and_then(|()| s.validate()) {
            self.status = Status::Failed(e);
            return false;
        }
        let m = s.machine;
        self.put(Key::SMax, m.s_max.to_string());
        self.put(Key::MaxPower, format!("{}", m.max_power_percent));
        self.put(Key::MaxSpeed, format!("{}", m.max_speed_mm_min));
        self.status = Status::ProfileApplied {
            name: s.profile.clone().unwrap_or_default(),
            s_max: m.s_max,
            max_power: m.max_power_percent,
            max_speed: m.max_speed_mm_min,
        };
        self.settings = s;
        self.mark_stale();
        true
    }

    /// The names of the materials of the library, in the order of the file.
    pub fn material_names(&self) -> Vec<String> {
        self.materials
            .as_ref()
            .map(|l| l.materials.iter().map(|m| m.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Puts what a material says in the boxes of the window (what it leaves out stays as it is), like `--material` does before the options: the power, the speed, the dot, the dithering,
    /// the tone curve and the settings of every layer of the drawing. The limits still hold when the work is made.
    pub fn apply_material(&mut self, name: &str) -> bool {
        let Some(m) = self.materials.as_ref().and_then(|l| l.find(name)).cloned() else {
            self.status = Status::Failed(format!("there is no material '{}'", name.trim()));
            return false;
        };
        self.apply_material_values(&m);
        self.material = Some(m.name.clone());
        self.mark_stale();
        self.status = Status::MaterialApplied(m.name);
        true
    }

    fn apply_material_values(&mut self, m: &Material) {
        let t = |v: f64| format!("{v}");
        if let Some(v) = m.power {
            self.put(Key::Power, t(v));
        }
        if let Some(v) = m.speed {
            self.put(Key::Feed, t(v));
        }
        if let Some(v) = m.dot {
            self.put(Key::DotMm, t(v));
        }
        if let Some(v) = m.overscan {
            self.put(Key::Overscan, t(v));
            self.overscan_auto = false;
        }
        if let Some(v) = m.gamma {
            self.put(Key::Gamma, t(v));
        }
        if let Some(v) = m.contrast {
            self.put(Key::Contrast, t(v));
        }
        if let Some(v) = m.brightness {
            self.put(Key::Brightness, t(v));
        }
        if let Some(v) = m.invert {
            self.tone_invert = v;
        }
        if let Some(v) = m.auto_level {
            self.tone_auto_level = v;
        }
        if let Some(v) = m.threshold {
            self.put(Key::Threshold, v.to_string());
        }
        if let Some(word) = &m.dither {
            if let Some(kind) = Kind::from_word(word) {
                self.kind = kind;
            }
        }
        for i in 0..self.rows.len() {
            let key = self.rows[i].key.clone();
            let shapes = self.rows[i].shapes;
            let mut p = self
                .layer_params(&self.rows[i])
                .unwrap_or_else(|_| self.settings.params_for(&key));
            m.apply_to_layer(&mut p);
            self.rows[i] = LayerRow::new(&key, shapes, &p);
        }
    }

    pub fn set_layer_field(&mut self, row: usize, field: LayerField, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            if r.fields[field as usize] != text {
                r.fields[field as usize] = text;
                self.mark_stale();
            }
        }
    }

    pub fn set_layer_mode(&mut self, row: usize, mode: LayerMode) {
        if let Some(r) = self.rows.get_mut(row) {
            if r.mode != mode {
                r.mode = mode;
                self.mark_stale();
            }
        }
    }

    pub fn set_kerf_side(&mut self, row: usize, side: KerfSide) {
        if let Some(r) = self.rows.get_mut(row) {
            if r.kerf_side != side {
                r.kerf_side = side;
                self.mark_stale();
            }
        }
    }

    /// Works the G-code out for what is chosen (the engraving or the grid), like the command line, and the passes to draw.
    pub fn compute(&mut self) -> bool {
        let made = match self.mode {
            Mode::Engrave => self.engrave_job(),
            Mode::Calibrate => self.calibrate_job(),
            Mode::Cut => self.cut_job(),
            Mode::Focus => self.focus_job(),
            Mode::Frame => self.frame_job(),
            Mode::Preview => self.preview_job(),
            Mode::Trace => self.trace_job(),
        };
        match made {
            Ok(done) => {
                self.warnings = done.notes.clone();
                self.report = preview::analyse(
                    &preview::read(&done.gcode),
                    &self.preview_options(done.s_max),
                )
                .ok();
                let (segs, cut) = passes_of(
                    &done.gcode,
                    done.s_max,
                    matches!(
                        self.mode,
                        Mode::Cut | Mode::Focus | Mode::Frame | Mode::Preview | Mode::Trace
                    ),
                );
                self.segs = segs;
                self.truncated = cut;
                self.status = Status::Done(done.summary);
                self.done = Some(done);
                self.stale = false;
                true
            }
            Err(s) => {
                self.status = s;
                false
            }
        }
    }

    fn engrave_job(&self) -> Result<Done, Status> {
        let pic = self.picture.as_ref().ok_or(Status::NeedImage)?;
        let width = self.number(Key::WidthMm)?;
        if !(width > 0.0 && width <= 2000.0) {
            return Err(Status::BadValue(Msg::WidthMm));
        }
        let p = self.params()?;
        let method = self.method()?;
        let tone = self.tone()?;
        let w = (width / p.pixel_mm).round().max(1.0) as usize;
        let h = ((w as f64) * pic.gray.h as f64 / pic.gray.w as f64)
            .round()
            .max(1.0) as usize;
        let resized = pic.gray.resize(w, h).map_err(Status::Failed)?;
        let dots = dither(&tone.apply(&resized).map_err(Status::Failed)?, method);
        let e = engrave(&dots, &p).map_err(Status::Failed)?;
        Ok(Done {
            gcode: e.gcode,
            source: Some((pic.name.clone(), sha256_hex(&pic.bytes))),
            tools: format!("laser, S 0-{} ({} %)", p.s_max, p.power_percent),
            s_max: p.s_max,
            line_mm: p.pixel_mm,
            summary: Summary::Engraved {
                dots_w: dots.w,
                dots_h: dots.h,
                width_mm: dots.w as f64 * p.pixel_mm,
                height_mm: dots.h as f64 * p.pixel_mm,
                rows: e.rows,
                runs: e.runs,
                burn_mm: e.burn_mm,
                seconds: e.seconds,
            },
            notes: Vec::new(),
        })
    }

    /// The G-code that cuts and engraves the drawing that is open, with the settings of the boxes and of the table of layers.
    fn cut_job(&self) -> Result<Done, Status> {
        let file = self.vector.as_ref().ok_or(Status::NeedVector)?;
        let opts = self.read_options()?;
        let drawing = parse_drawing(&file.bytes, &file.ext, &opts).map_err(Status::Failed)?;
        let settings = self.vector_settings()?;
        let plan = plan_cut(&drawing, &settings).map_err(Status::Failed)?;
        Ok(Done {
            gcode: plan.gcode,
            source: Some((file.name.clone(), sha256_hex(&file.bytes))),
            tools: format!("laser, S 0-{}, vector layers", settings.machine.s_max),
            s_max: settings.machine.s_max,
            line_mm: 0.1,
            summary: Summary::Cut {
                layers: plan.stats.layers,
                paths: plan.stats.contours,
                cut_mm: plan.stats.cut_mm,
                engrave_mm: plan.stats.engrave_mm,
                travel_mm: plan.stats.travel_mm,
                seconds: plan.stats.seconds,
            },
            notes: plan.warnings,
        })
    }

    /// The outline of the picture that is open, followed with the boxes of its group and the limits of the settings file (`[trace]`), and the settings of the one layer it makes (`trace`):
    /// the power and the speed are the ones of the laser boxes and obey the same limits, and the layer cuts unless the person asked to fill it with lines.
    fn trace_job(&self) -> Result<Done, Status> {
        let pic = self.picture.as_ref().ok_or(Status::NeedImage)?;
        let width = self.number(Key::WidthMm)?;
        if !(width > 0.0 && width <= trace::MAX_WIDTH_MM) {
            return Err(Status::BadValue(Msg::WidthMm));
        }
        let t = TraceSettings {
            threshold: self.whole(Key::TraceThreshold, 0.0, 255.0)? as u8,
            invert: self.trace_invert,
            simplify_mm: self.number(Key::TraceSimplify)?,
            min_area_mm2: self.number(Key::TraceMinArea)?,
            ..self.settings.trace
        };
        t.validate().map_err(Status::Failed)?;
        let drawing = trace::trace(&pic.gray, width, &t, self.settings.read.limits)
            .map_err(Status::Failed)?;
        let feed = self.number(Key::Feed)?;
        let mut s = self.settings.clone();
        s.machine.s_max = self.whole(Key::SMax, 1.0, 100_000.0)?;
        (s.machine.max_power_percent, s.machine.max_speed_mm_min) = self.limits()?;
        s.machine.overscan_mm = if self.overscan_auto {
            overscan_for(feed, self.number(Key::Accel)?).map_err(Status::Failed)?
        } else {
            self.number(Key::Overscan)?
        };
        s.machine.origin = (self.number(Key::OriginX)?, self.number(Key::OriginY)?);
        let mut p = self.settings.params_for(trace::LAYER);
        p.mode = if self.trace_engrave {
            LayerMode::Engrave
        } else {
            LayerMode::Cut
        };
        p.power_percent = self.number(Key::Power)?;
        p.speed_mm_min = feed;
        s.layers = vec![(trace::LAYER.to_string(), p)];
        s.validate().map_err(Status::Failed)?;
        let plan = plan_cut(&drawing, &s).map_err(Status::Failed)?;
        let points: usize = drawing.shapes.iter().map(|d| d.pts.len()).sum();
        Ok(Done {
            gcode: plan.gcode,
            source: Some((pic.name.clone(), sha256_hex(&pic.bytes))),
            tools: format!("laser, S 0-{}, outline of a picture", s.machine.s_max),
            s_max: s.machine.s_max,
            line_mm: 0.1,
            summary: Summary::Traced {
                contours: drawing.shapes.len(),
                points,
                cut_mm: plan.stats.cut_mm,
                engrave_mm: plan.stats.engrave_mm,
                seconds: plan.stats.seconds,
            },
            notes: plan.warnings,
        })
    }

    /// What the preview is told about the machine: the acceleration of the box, the speed of the moves with the laser off and the bed of the settings file.
    fn preview_options(&self, s_max: u32) -> preview::Options {
        let mut o = preview::Options {
            rapid_mm_min: self.settings.machine.rapid_mm_min,
            s_max: Some(s_max),
            bed_mm: self.settings.machine.bed_mm,
            ..preview::Options::default()
        };
        if let Ok(a) = self.number(Key::Accel) {
            if (1.0..=100_000.0).contains(&a) {
                o.accel_mm_s2 = a;
            }
        }
        o
    }

    /// The lines of the preview of the G-code that was worked out or opened: what it does and the notes about what looks dangerous.
    pub fn preview_lines(&self) -> Vec<String> {
        let (Some(r), Some(_)) = (&self.report, &self.done) else {
            return Vec::new();
        };
        let mut lines = vec![preview_line(
            self.lang,
            r.moves,
            r.burn_mm,
            r.travel_mm,
            r.seconds,
        )];
        lines.extend(r.notes.iter().take(8).cloned());
        lines
    }

    /// The G-code program that is open, counted and checked by the preview.
    fn preview_job(&self) -> Result<Done, Status> {
        let program = self.program.as_ref().ok_or(Status::NeedImage)?;
        let s_max = self.whole(Key::SMax, 1.0, 100_000.0)?;
        let r = preview::analyse(&preview::read(&program.text), &self.preview_options(s_max))
            .map_err(Status::Failed)?;
        if r.moves == 0 {
            return Err(Status::Failed(format!(
                "{}: the program has no move in the plane",
                program.name
            )));
        }
        Ok(Done {
            gcode: program.text.clone(),
            source: None,
            tools: String::new(),
            s_max,
            line_mm: 0.1,
            summary: Summary::Previewed {
                moves: r.moves,
                burn_mm: r.burn_mm,
                travel_mm: r.travel_mm,
                seconds: r.seconds,
            },
            notes: r.notes,
        })
    }

    /// The work that the frame is made of: the drawing when it was the last thing opened or chosen (or the only thing open), the picture otherwise.
    pub fn framed_mode(&self) -> Option<Mode> {
        match (
            self.work_mode,
            self.picture.is_some(),
            self.vector.is_some(),
        ) {
            (Mode::Trace, true, _) => Some(Mode::Trace),
            (Mode::Cut, _, true) | (_, false, true) => Some(Mode::Cut),
            (_, true, _) => Some(Mode::Engrave),
            _ => None,
        }
    }

    /// The pilot power of the boxes of the frame (a percent), when the box holds a number.
    pub fn pilot_percent(&self) -> Option<f64> {
        self.number(Key::FramePilot).ok()
    }

    /// The frame would turn the laser on and the person has not confirmed it yet: the window asks, and nothing is worked out or written until the answer is yes.
    pub fn needs_confirmation(&self) -> bool {
        self.mode == Mode::Frame
            && !self.pilot_confirmed
            && self.pilot_percent().is_some_and(|p| p > 0.0)
    }

    /// The frame of the work that is open, worked out like `phoenix-laser frame --gcode`: the box of what burns, `margin` wider, traced `laps` times. The laser stays off unless the
    /// pilot power is more than 0 *and* the person has confirmed it (`pilot_confirmed`); the pilot power obeys the cap of `[frame]` and the limit of power, and the frame must fit the bed
    /// of the settings file when it has one.
    fn frame_job(&self) -> Result<Done, Status> {
        let work = match self.framed_mode() {
            Some(Mode::Cut) => self.cut_job()?,
            Some(Mode::Trace) => self.trace_job()?,
            Some(_) => self.engrave_job()?,
            None => return Err(Status::NeedImage),
        };
        let report = preview::analyse(&preview::read(&work.gcode), &preview::Options::default())
            .map_err(Status::Failed)?;
        let (lo, hi) = report
            .burn_bounds
            .or(report.bounds)
            .ok_or_else(|| Status::Failed("the work has no move in the plane".to_string()))?;
        let pilot_requested = self.number(Key::FramePilot)?;
        let feed_mm_min = self.number(Key::FrameFeed)?;
        let laps = self.whole(Key::FrameLaps, 0.0, 1000.0)?;
        let margin_mm = self.number(Key::FrameMargin)?;
        let (max_power, max_speed) = self.limits()?;
        // the laser is not lit unless the person said so, and then only up to the cap and the limit of power
        if pilot_requested > 0.0 && !self.pilot_confirmed {
            return Err(Status::NeedConfirm);
        }
        let max_pilot_percent = self.settings.frame.max_pilot_percent;
        let pilot = safety::pilot_percent(
            pilot_requested,
            self.pilot_confirmed,
            max_power,
            max_pilot_percent,
        )
        .map_err(Status::Failed)?;
        let settings = FrameSettings {
            feed_mm_min,
            laps,
            margin_mm,
            pilot_percent: pilot,
            max_pilot_percent,
        };
        settings.validate(max_power).map_err(Status::Failed)?;
        safety::check_speed("the speed of the frame", settings.feed_mm_min, max_speed)
            .map_err(Status::Failed)?;
        let pilot_s = frame::pilot_value(work.s_max, pilot).map_err(Status::Failed)?;
        let (lo, hi) = frame::grow(lo, hi, settings.margin_mm);
        let gcode = frame::gcode(
            &Frame {
                lo,
                hi,
                feed_mm_min: settings.feed_mm_min,
                laps: settings.laps,
                s_max: work.s_max,
                pilot_s,
            },
            self.settings.machine.bed_mm,
        )
        .map_err(Status::Failed)?;
        let tools = if pilot_s > 0 {
            format!(
                "laser, S 0-{}, frame of the work with a pilot power of S{pilot_s}",
                work.s_max
            )
        } else {
            "laser OFF, frame of the work".to_string()
        };
        Ok(Done {
            gcode,
            source: None,
            tools,
            s_max: work.s_max,
            line_mm: 0.1,
            summary: Summary::Framed {
                width_mm: hi.0 - lo.0,
                height_mm: hi.1 - lo.1,
                laps: settings.laps,
                pilot_percent: pilot,
            },
            notes: work.notes,
        })
    }

    /// The focus test of the boxes of its group, worked out like `phoenix-laser focus`: its power and its speed obey the limits, and the step of the height has no default.
    fn focus_job(&self) -> Result<Done, Status> {
        let f = FocusSettings {
            z_start_mm: self.number(Key::ZStart)?,
            z_step_mm: self.number(Key::ZStep)?,
            steps: self.whole(Key::FocusLines, 0.0, 1000.0)?,
            length_mm: self.number(Key::FocusLength)?,
            spacing_mm: self.number(Key::FocusSpacing)?,
            feed_mm_min: self.number(Key::FocusFeed)?,
            power_percent: self.number(Key::FocusPower)?,
        };
        let s_max = self.whole(Key::SMax, 1.0, 100_000.0)?;
        let (max_power, max_speed) = self.limits()?;
        safety::check_speed("the speed of the focus test", f.feed_mm_min, max_speed)
            .map_err(Status::Failed)?;
        let gcode = focus_test(&f, s_max, max_power).map_err(Status::Failed)?;
        Ok(Done {
            gcode,
            source: None,
            tools: format!("laser, S 0-{s_max}, focus test (the head moves in Z)"),
            s_max,
            line_mm: 0.1,
            summary: Summary::Focused {
                lines: f.steps,
                length_mm: f.length_mm,
            },
            notes: vec![Msg::FocusWarning.tr(self.lang).to_string()],
        })
    }

    fn calibrate_job(&self) -> Result<Done, Status> {
        let powers = self.list(Key::Powers)?;
        let feeds = self.list(Key::Feeds)?;
        let (max_power, max_speed) = self.limits()?;
        for p in &powers {
            safety::check_power("a power of the grid", *p, max_power).map_err(Status::Failed)?;
        }
        for f in &feeds {
            safety::check_speed("a speed of the grid", *f, max_speed).map_err(Status::Failed)?;
        }
        let g = Grid {
            s_max: self.whole(Key::SMax, 1.0, 100_000.0)?,
            ..Grid::default()
        };
        let gcode = grid(&powers, &feeds, &g).map_err(Status::Failed)?;
        Ok(Done {
            gcode,
            source: None,
            tools: format!("laser, S 0-{}, grid of power against feed", g.s_max),
            s_max: g.s_max,
            line_mm: g.line_spacing_mm,
            summary: Summary::Calibrated {
                squares: powers.len() * feeds.len(),
            },
            notes: Vec::new(),
        })
    }

    /// Writes the G-code with the standard header (the G-code is worked out again first when a setting changed). It is never written over the picture.
    pub fn export(&mut self, path: &Path) -> bool {
        // a program that was opened to be seen is not written again
        if self.mode == Mode::Preview {
            self.status = Status::NothingToWrite;
            return false;
        }
        if (self.done.is_none() || self.stale) && !self.compute() {
            return false;
        }
        // a frame that lights the laser is written only with the confirmation of the person, every time
        if self.needs_confirmation() {
            self.status = Status::NeedConfirm;
            return false;
        }
        let inputs = [
            self.picture.as_ref().map(|p| &p.path),
            self.vector.as_ref().map(|v| &v.path),
            self.config_path.as_ref(),
        ];
        if inputs.into_iter().flatten().any(|p| same_file(p, path)) {
            self.status = Status::SameFile(path.display().to_string());
            return false;
        }
        let Some(done) = &self.done else {
            self.status = Status::NeedImage;
            return false;
        };
        let head = header(&Header {
            generator: "phoenix-laser",
            version: env!("CARGO_PKG_VERSION"),
            source: done.source.as_ref().map(|(n, h)| (n.as_str(), h.clone())),
            tools: &done.tools,
        });
        match write_atomic(path, &format!("{head}{}", done.gcode)) {
            Ok(()) => {
                self.status = Status::Written(name_of(path));
                self.pilot_confirmed = false;
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Does what a chosen file is for.
    pub fn run_job(&mut self, job: Job, path: &Path) -> bool {
        match job {
            Job::OpenImage => self.open_dropped(path),
            Job::OpenConfig => self.load_config(path),
            Job::OpenMaterials => self.load_materials(path),
            Job::OpenGcode => self.open_gcode(path),
            Job::ExportGcode => self.export(path),
        }
    }

    /// The area the work covers, from the corner of the origin: the passes, or the size the picture will have.
    pub fn bounds(&self) -> Option<((f64, f64), (f64, f64))> {
        if !self.segs.is_empty() {
            let mut min = (f64::INFINITY, f64::INFINITY);
            let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for s in &self.segs {
                for (x, y) in [(s.x0, s.y0), (s.x1, s.y1)] {
                    min = (min.0.min(x), min.1.min(y));
                    max = (max.0.max(x), max.1.max(y));
                }
            }
            return Some((min, max));
        }
        let pic = self.picture.as_ref()?;
        let width = self.number(Key::WidthMm).ok()?;
        let origin = (
            self.number(Key::OriginX).ok()?,
            self.number(Key::OriginY).ok()?,
        );
        let height = width * pic.gray.h as f64 / pic.gray.w as f64;
        Some((origin, (origin.0 + width, origin.1 + height)))
    }

    /// Shows everything in a picture of the size given.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((min, max)) = self.bounds() {
            self.viewport = Viewport::fit(min, max, width_px, height_px, 24.0);
        }
    }

    /// What the pointer is over.
    pub fn cursor_text(&self, x: f64, y: f64) -> String {
        fill(
            Msg::CursorPlain.tr(self.lang),
            &[&format!("{x:.2}"), &format!("{y:.2}")],
        )
    }

    /// The facts about the picture (what the `info` command prints).
    pub fn info_line(&self) -> Option<String> {
        if self.mode == Mode::Preview {
            return None;
        }
        if self.mode == Mode::Cut {
            let v = self.vector.as_ref()?;
            let ((x0, y0), (x1, y1)) = v.drawing.bounds()?;
            return Some(fill(
                Msg::InfoVector.tr(self.lang),
                &[
                    &v.drawing.shapes.len().to_string(),
                    &self.rows.len().to_string(),
                    &format!("{:.1}", x1 - x0),
                    &format!("{:.1}", y1 - y0),
                ],
            ));
        }
        let g = &self.picture.as_ref()?.gray;
        let mean = g.data.iter().map(|&v| u64::from(v)).sum::<u64>() as f64 / g.data.len() as f64;
        Some(fill(
            Msg::InfoImage.tr(self.lang),
            &[&g.w.to_string(), &g.h.to_string(), &format!("{mean:.1}")],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Four rows of eight dots: the left half black, the right half white.
    const PICTURE: &str = "P2\n8 4\n255\n0 0 0 0 255 255 255 255\n0 0 0 0 255 255 255 255\n0 0 0 0 255 255 255 255\n0 0 0 0 255 255 255 255\n";

    /// A folder of its own for each test, removed at the end.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("eh3d-laser-window-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let p = self.0.join(name);
            fs::write(&p, text).unwrap();
            p
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn set(w: &mut Window, key: Key, text: &str) {
        let i = w.fields.iter().position(|f| f.key == key).unwrap();
        w.set_field(i, text.to_string());
    }

    /// A window with the picture open, a width of 0.8 mm (eight dots) and the threshold kind of dots.
    fn ready(tag: &str) -> (Scratch, Window) {
        let s = Scratch::new(tag);
        let p = s.write("pic.pgm", PICTURE);
        let mut w = Window::new(Lang::En);
        assert!(w.open_image(&p));
        set(&mut w, Key::WidthMm, "0.8");
        w.set_kind(Kind::Threshold);
        (s, w)
    }

    #[test]
    fn opening_a_picture_keeps_its_bytes_and_says_its_size() {
        let s = Scratch::new("open");
        let p = s.write("pic.pgm", PICTURE);
        let mut w = Window::new(Lang::En);
        assert!(w.open_image(&p));
        assert_eq!(w.picture_name().as_deref(), Some("pic.pgm"));
        assert_eq!(w.picture.as_ref().unwrap().bytes, PICTURE.as_bytes());
        assert!(w.status.text(Lang::En).contains("pic.pgm: 8 x 4 pixels"));
        assert!(w.has_work());
        assert_eq!(
            w.info_line().as_deref(),
            Some("8 x 4 pixels, mean grey 127.5 of 255")
        );
    }

    #[test]
    fn a_file_that_is_not_a_picture_is_refused_with_its_name() {
        let s = Scratch::new("bad");
        let p = s.write("notes.png", "this is not a picture");
        let mut w = Window::new(Lang::En);
        assert!(!w.open_image(&p));
        assert!(w.picture.is_none());
        assert!(w.status.is_problem());
        assert!(w.status.text(Lang::En).contains("notes.png"));
        assert!(!w.open_dropped(Path::new("/no/such/folder/pic.png")));
    }

    #[test]
    fn the_window_engraves_like_the_command_line() {
        let (_s, mut w) = ready("same");
        assert!(w.compute());
        let img = from_bytes(PICTURE.as_bytes()).unwrap();
        let dots = dither(&img.resize(8, 4).unwrap(), Method::Threshold(128));
        let expected = engrave(&dots, &Params::default()).unwrap();
        let done = w.done.as_ref().unwrap();
        assert_eq!(done.gcode, expected.gcode);
        assert_eq!(
            done.summary,
            Summary::Engraved {
                dots_w: 8,
                dots_h: 4,
                width_mm: 0.8,
                height_mm: 0.4,
                rows: expected.rows,
                runs: expected.runs,
                burn_mm: expected.burn_mm,
                seconds: expected.seconds,
            }
        );
        assert_eq!((expected.rows, expected.runs), (4, 4));
        assert!(!w.stale);
    }

    #[test]
    fn the_passes_drawn_are_the_ones_of_the_gcode() {
        let (_s, mut w) = ready("passes");
        assert!(w.compute());
        let burning = w.segs.iter().filter(|s| s.burn).count();
        let travelling = w.segs.iter().filter(|s| !s.burn).count();
        assert_eq!(burning, 4);
        assert!(travelling >= 4);
        // half of the full power
        assert!(w
            .segs
            .iter()
            .filter(|s| s.burn)
            .all(|s| (s.level - 0.5).abs() < 1e-9));
        assert!(!w.truncated);
        let ((x0, _), (x1, _)) = w.bounds().unwrap();
        assert!(x0 < 0.0 && x1 > 0.8, "the overscan runs past the work");
    }

    #[test]
    fn a_pass_burns_only_where_the_power_is_on() {
        let gcode = "G21\nM4 S0\nG0 X0 Y1\nG1 X5 S0\nG1 X8 S500\nG1 X10 S0\n";
        let (segs, cut) = passes(gcode, 1000);
        assert!(!cut);
        assert_eq!(segs.len(), 3);
        assert!(!segs[0].burn && segs[1].burn && !segs[2].burn);
        assert_eq!((segs[1].x0, segs[1].x1, segs[1].y0), (5.0, 8.0, 1.0));
        assert_eq!(segs[1].level, 0.5);
    }

    #[test]
    fn comments_and_words_that_are_not_numbers_are_left_out_of_the_picture() {
        let (segs, _) = passes("; header\n(a note)\nG1 Xabc\nG1 X1 Sxyz\nF3000\n", 1000);
        assert_eq!(segs.len(), 1);
        assert!(!segs[0].burn);
    }

    #[test]
    fn a_huge_gcode_is_cut_short_for_the_picture() {
        let mut text = String::from("G0 X0 Y0\n");
        for k in 1..=(MAX_SEGMENTS + 5) {
            text.push_str(&format!("G1 X{k} S0\n"));
        }
        let (segs, cut) = passes(&text, 1000);
        assert!(cut);
        assert_eq!(segs.len(), MAX_SEGMENTS);
    }

    #[test]
    fn the_colours_go_from_dark_red_to_yellow() {
        let (low, high) = (fire(0.0), fire(1.0));
        assert!(low[0] > low[1] && low[0] > low[2], "the lowest is red");
        assert!(high[1] > low[1], "the highest is lighter");
        assert_eq!(fire(f64::NAN), fire(0.0));
        assert_eq!(fire(-4.0), fire(0.0));
        assert_eq!(fire(9.0), fire(1.0));
    }

    #[test]
    fn a_setting_that_changes_after_the_work_makes_it_stale() {
        let (_s, mut w) = ready("stale");
        assert!(w.compute());
        set(&mut w, Key::Power, "70");
        assert!(w.stale);
        assert!(w.compute());
        assert!(!w.stale);
        w.set_kind(Kind::Atkinson);
        assert!(w.stale);
        assert!(w.compute());
        w.set_overscan_auto(true);
        assert!(w.stale);
        assert!(w.compute());
        w.set_mode(Mode::Calibrate);
        assert!(w.stale);
    }

    #[test]
    fn the_overscan_can_be_worked_out_from_the_acceleration() {
        let (_s, mut w) = ready("auto");
        w.set_overscan_auto(true);
        let p = w.params().unwrap();
        assert_eq!(p.overscan_mm, overscan_for(3000.0, 1000.0).unwrap());
        set(&mut w, Key::Accel, "0");
        assert!(matches!(w.params().unwrap_err(), Status::Failed(_)));
    }

    #[test]
    fn the_settings_are_checked_like_the_command_line_checks_them() {
        let mut w = Window::new(Lang::En);
        assert!(w.params().is_ok());
        set(&mut w, Key::Power, "150");
        assert!(matches!(w.params().unwrap_err(), Status::Failed(_)));
        set(&mut w, Key::Power, "50");
        set(&mut w, Key::SMax, "0");
        assert_eq!(w.params().unwrap_err(), Status::BadValue(Msg::SMax));
        set(&mut w, Key::SMax, "1000.5");
        assert_eq!(w.params().unwrap_err(), Status::BadValue(Msg::SMax));
        set(&mut w, Key::SMax, "1000");
        set(&mut w, Key::Feed, "fast");
        assert_eq!(w.params().unwrap_err(), Status::BadValue(Msg::Feed));
    }

    #[test]
    fn the_threshold_is_held_between_0_and_255() {
        let mut w = Window::new(Lang::En);
        w.set_kind(Kind::Threshold);
        set(&mut w, Key::Threshold, "999");
        assert_eq!(w.method().unwrap(), Method::Threshold(255));
        set(&mut w, Key::Threshold, "-4");
        assert_eq!(w.method().unwrap(), Method::Threshold(0));
        set(&mut w, Key::Threshold, "dark");
        assert_eq!(w.method().unwrap_err(), Status::BadValue(Msg::Threshold));
        w.set_kind(Kind::Atkinson);
        assert_eq!(w.method().unwrap(), Method::Atkinson);
    }

    #[test]
    fn engraving_needs_a_picture_and_a_sensible_width() {
        let mut w = Window::new(Lang::En);
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedImage);
        let (_s, mut w) = ready("width");
        set(&mut w, Key::WidthMm, "0");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::WidthMm));
        set(&mut w, Key::WidthMm, "5000");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::WidthMm));
    }

    #[test]
    fn a_picture_that_would_need_too_many_dots_is_refused_by_the_reader() {
        let (_s, mut w) = ready("toomany");
        set(&mut w, Key::WidthMm, "2000");
        set(&mut w, Key::DotMm, "0.01");
        assert!(!w.compute());
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn the_calibration_grid_is_the_one_of_the_command_line() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Calibrate);
        assert!(w.can_compute());
        assert!(w.compute());
        let expected = grid(
            &[20.0, 40.0, 60.0, 80.0],
            &[1000.0, 2000.0, 3000.0],
            &Grid::default(),
        )
        .unwrap();
        assert_eq!(w.done.as_ref().unwrap().gcode, expected);
        assert_eq!(
            w.done.as_ref().unwrap().summary,
            Summary::Calibrated { squares: 12 }
        );
        assert!(w.segs.iter().filter(|s| s.burn).count() >= 12 * 80);
        // the squares of higher power are lighter
        let levels: Vec<f64> = w.segs.iter().filter(|s| s.burn).map(|s| s.level).collect();
        assert!(levels.iter().any(|&l| (l - 0.2).abs() < 1e-9));
        assert!(levels.iter().any(|&l| (l - 0.8).abs() < 1e-9));
    }

    #[test]
    fn a_calibration_with_a_list_that_is_not_numbers_says_which_one() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Calibrate);
        set(&mut w, Key::Feeds, "1000,,3000");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Feeds));
        set(&mut w, Key::Feeds, "1000");
        set(&mut w, Key::Powers, "20,x");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Powers));
        set(&mut w, Key::Powers, "200");
        assert!(!w.compute());
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn the_exported_file_has_the_header_of_the_command_line() {
        let (s, mut w) = ready("export");
        let out = s.0.join("pic.nc");
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("; generator: phoenix-laser "));
        assert!(text.contains("; units: mm (G21)\n"));
        let hash = sha256_hex(PICTURE.as_bytes());
        assert!(text.contains(&format!("; source: pic.pgm sha256:{hash}\n")));
        assert!(text.contains("; tools: laser, S 0-1000 (50 %)\n"));
        assert!(text.ends_with("M5\nM2\n"));
        assert!(!s.0.join("pic.nc.tmp").exists());
        assert!(w.status.text(Lang::En).contains("pic.nc"));
    }

    #[test]
    fn the_grid_is_written_with_no_source() {
        let s = Scratch::new("gridfile");
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Calibrate);
        let out = s.0.join("grid.nc");
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("; source: none\n"));
        assert!(text.contains("grid of power against feed"));
    }

    #[test]
    fn exporting_works_the_gcode_out_again_when_a_setting_changed() {
        let (s, mut w) = ready("again");
        assert!(w.compute());
        set(&mut w, Key::Power, "100");
        assert!(w.export(&s.0.join("hot.nc")));
        assert!(!w.stale);
        assert!(fs::read_to_string(s.0.join("hot.nc"))
            .unwrap()
            .contains("S1000"));
    }

    #[test]
    fn the_picture_is_never_written_over() {
        let (s, mut w) = ready("overpic");
        let picture = s.0.join("pic.pgm");
        assert!(!w.export(&picture));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(fs::read_to_string(&picture).unwrap(), PICTURE);
    }

    #[test]
    fn nothing_is_written_when_there_is_nothing_to_engrave() {
        let s = Scratch::new("nothing");
        let mut w = Window::new(Lang::En);
        assert!(!w.export(&s.0.join("x.nc")));
        assert!(!s.0.join("x.nc").exists());
    }

    #[test]
    fn the_name_offered_follows_the_picture() {
        let (s, mut w) = ready("name");
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(s.0.join("pic.nc"))
        );
        assert_eq!(w.suggested_output_for(Job::OpenImage), None);
        assert_eq!(w.work_folder(), Some(s.0.clone()));
        w.set_mode(Mode::Calibrate);
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(PathBuf::from("calibration.nc"))
        );
    }

    #[test]
    fn the_picture_fits_the_work_before_and_after_it_is_worked_out() {
        let (_s, mut w) = ready("fit");
        // before: the size the picture will have
        assert_eq!(w.bounds(), Some(((0.0, 0.0), (0.8, 0.4))));
        w.fit(800.0, 600.0);
        let (x, y) = w.viewport.to_screen(0.4, 0.2, 800.0, 600.0);
        assert!((x - 400.0).abs() < 1e-6 && (y - 300.0).abs() < 1e-6);
        assert!(w.compute());
        assert!(w.bounds().is_some());
        assert!(Window::new(Lang::En).bounds().is_none());
    }

    #[test]
    fn a_new_picture_drops_the_work_made_from_the_old_one() {
        let (s, mut w) = ready("newpic");
        assert!(w.compute());
        let other = s.write("other.pgm", PICTURE);
        assert!(w.open_image(&other));
        assert!(w.done.is_none() && w.segs.is_empty());
        assert_eq!(w.picture_name().as_deref(), Some("other.pgm"));
    }

    #[test]
    fn the_status_speaks_the_language_chosen() {
        let s = Status::Written("a.nc".into());
        assert_eq!(s.text(Lang::En), "Written: a.nc");
        assert_eq!(s.text(Lang::Es), "Escrito: a.nc");
        assert!(Status::BadValue(Msg::Feed)
            .text(Lang::De)
            .contains("Vorschub"));
        let done = Status::Done(Summary::Calibrated { squares: 6 });
        assert_eq!(done.text(Lang::En), "6 squares of the grid");
        assert_eq!(done.text(Lang::Pt), "6 quadrados da grelha");
    }

    /// A square of 20 mm drawn in red and a circle of radius 3 drawn in blue (one user unit is one mm).
    const DRAWING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20" stroke="#ff0000" fill="none"/><circle cx="10" cy="10" r="3" stroke="#0000ff" fill="none"/></svg>"##;

    /// A window with the drawing open.
    fn drawn(tag: &str) -> (Scratch, Window, PathBuf) {
        let s = Scratch::new(tag);
        let p = s.write("part.svg", DRAWING);
        let mut w = Window::new(Lang::En);
        assert!(w.open_vector(&p));
        (s, w, p)
    }

    fn cut_summary(w: &Window) -> (usize, usize, f64, f64) {
        match w.done.as_ref().unwrap().summary {
            Summary::Cut {
                layers,
                paths,
                cut_mm,
                engrave_mm,
                ..
            } => (layers, paths, cut_mm, engrave_mm),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn opening_a_drawing_builds_the_table_of_layers_and_switches_to_cutting() {
        let (_s, w, _p) = drawn("vec-open");
        assert_eq!(w.mode, Mode::Cut);
        assert_eq!(
            w.status,
            Status::VectorOpened {
                name: "part.svg".into(),
                shapes: 2,
                layers: 2
            }
        );
        let keys: Vec<&str> = w.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["#ff0000", "#0000ff"]);
        // the defaults of a layer
        assert_eq!(w.rows[0].mode, LayerMode::Cut);
        assert_eq!(w.rows[0].fields[LayerField::Power as usize], "50");
        assert_eq!(w.rows[0].fields[LayerField::Speed as usize], "600");
        assert_eq!(w.rows[0].fields[LayerField::Passes as usize], "1");
        assert!(w.can_compute() && w.has_work());
        assert_eq!(w.current_name().as_deref(), Some("part.svg"));
        // 2 shapes in 2 layers, 20 x 20 mm
        assert_eq!(w.info_line().unwrap(), "2 shapes, 2 layers, 20.0 x 20.0 mm");
    }

    #[test]
    fn the_drawing_is_cut_with_the_settings_of_its_table() {
        let (_s, mut w, _p) = drawn("vec-cut");
        assert!(w.compute());
        let (layers, paths, cut_mm, engrave_mm) = cut_summary(&w);
        assert_eq!((layers, paths), (2, 2));
        assert_eq!(engrave_mm, 0.0);
        // the square is 80 mm; the circle has a perimeter of 18.85 mm and its flattened form a little less
        assert!(cut_mm > 98.7 && cut_mm < 98.86, "{cut_mm}");
        // both layers are cut (the blue one first: same order, lower name), the passes of the picture are there
        let g = &w.done.as_ref().unwrap().gcode;
        let first_layer = g.lines().find(|l| l.starts_with("; layer")).unwrap();
        assert!(
            first_layer.starts_with("; layer #0000ff: cut, 50 %, 600 mm/min"),
            "{first_layer}"
        );
        assert!(w.segs.iter().any(|s| s.burn) && w.segs.iter().any(|s| !s.burn));
        assert!(!w.stale);
        // a layer that is skipped is not cut: only the square is left, 80 mm
        w.set_layer_mode(1, LayerMode::Skip);
        assert!(w.stale);
        assert!(w.compute());
        assert_eq!(cut_summary(&w), (1, 1, 80.0, 0.0));
    }

    #[test]
    fn the_beam_width_of_a_layer_moves_the_cut_outwards() {
        let (_s, mut w, _p) = drawn("vec-kerf");
        w.set_layer_mode(1, LayerMode::Skip);
        w.set_layer_field(0, LayerField::Kerf, "0.2".into());
        w.set_kerf_side(0, KerfSide::Outside);
        assert!(w.compute());
        let ((x0, y0), (x1, y1)) = w.bounds().unwrap();
        assert!(
            (x0 + 0.1).abs() < 1e-9 && (y0 + 0.1).abs() < 1e-9,
            "{x0} {y0}"
        );
        assert!(
            (x1 - 20.1).abs() < 1e-9 && (y1 - 20.1).abs() < 1e-9,
            "{x1} {y1}"
        );
    }

    #[test]
    fn a_box_of_the_table_that_is_not_valid_says_which() {
        let (_s, mut w, _p) = drawn("vec-bad");
        let try_value = |w: &mut Window, f: LayerField, text: &str| {
            w.set_layer_field(0, f, text.into());
            let ok = w.compute();
            let status = w.status.clone();
            w.set_layer_field(0, f, "1".into());
            (ok, status)
        };
        assert_eq!(
            try_value(&mut w, LayerField::Power, "0"),
            (
                false,
                Status::Failed("layer #ff0000: power is 0: it must be from 0.001 to 100".into())
            )
        );
        assert_eq!(
            try_value(&mut w, LayerField::Speed, "abc"),
            (false, Status::BadValue(Msg::LayerSpeed))
        );
        assert_eq!(
            try_value(&mut w, LayerField::Passes, "1.5"),
            (false, Status::BadValue(Msg::Passes))
        );
        assert_eq!(
            try_value(&mut w, LayerField::Tabs, "-1"),
            (false, Status::BadValue(Msg::Tabs))
        );
        assert_eq!(
            try_value(&mut w, LayerField::HatchAngle, "x"),
            (false, Status::BadValue(Msg::HatchAngle))
        );
        // a box of the window that is not a number
        set(&mut w, Key::Dpi, "abc");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Dpi));
        set(&mut w, Key::Dpi, "96");
        set(&mut w, Key::SMax, "0");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::SMax));
        // all the layers skipped: there is nothing to cut
        set(&mut w, Key::SMax, "1000");
        w.set_layer_mode(0, LayerMode::Skip);
        w.set_layer_mode(1, LayerMode::Skip);
        assert!(!w.compute());
        assert!(w.status.is_problem());
    }

    #[test]
    fn the_drawing_is_exported_with_the_header_of_the_picture_and_never_over_the_input() {
        let (s, mut w, p) = drawn("vec-export");
        let out = s.0.join("part.nc");
        assert_eq!(w.suggested_output_for(Job::ExportGcode), Some(out.clone()));
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("; generator: phoenix-laser "));
        assert!(text.contains(&format!(
            "; source: part.svg sha256:{}\n",
            sha256_hex(DRAWING.as_bytes())
        )));
        assert!(text.ends_with("M5\nM2\n"));
        crate::cut::verify_gcode(&text, 1000, None).unwrap();
        // never over the drawing
        assert!(!w.export(&p));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(fs::read_to_string(&p).unwrap(), DRAWING);
        // nor over the settings file that was read
        let cfg = s.write("a.cfg", "[machine]\ns_max = 1000\n");
        assert!(w.load_config(&cfg));
        assert!(!w.export(&cfg));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(
            fs::read_to_string(&cfg).unwrap(),
            "[machine]\ns_max = 1000\n"
        );
    }

    #[test]
    fn a_file_of_settings_fills_the_boxes_and_the_table() {
        let (s, mut w, _p) = drawn("vec-config");
        let cfg = s.write(
            "a.cfg",
            "[machine]\ns_max = 255\noverscan_mm = 1.5\n[read]\ndpi = 72\n[layer #ff0000]\npower = 80\nmode = engrave\n",
        );
        assert!(w.load_config(&cfg));
        assert_eq!(w.status, Status::ConfigLoaded("a.cfg".into()));
        let field = |w: &Window, k: Key| w.fields.iter().find(|f| f.key == k).unwrap().text.clone();
        assert_eq!(field(&w, Key::SMax), "255");
        assert_eq!(field(&w, Key::Overscan), "1.5");
        assert_eq!(field(&w, Key::Dpi), "72");
        assert_eq!(w.rows[0].mode, LayerMode::Engrave);
        assert_eq!(w.rows[0].fields[LayerField::Power as usize], "80");
        assert_eq!(w.rows[1].mode, LayerMode::Cut);
        assert_eq!(w.rows[1].fields[LayerField::Power as usize], "50");
        // the engraving of the square: lines 0.1 apart across 20 mm are 200 lines of 20 mm; the circle is cut
        assert!(w.compute());
        let (layers, _, cut_mm, engrave_mm) = cut_summary(&w);
        assert_eq!(layers, 2);
        assert!((engrave_mm - 4000.0).abs() < 1e-6, "{engrave_mm}");
        assert!(cut_mm > 18.7 && cut_mm < 18.86, "{cut_mm}");
        // 80 % of 255 is 204 and 50 % is 127.5, which rounds to 128
        let g = &w.done.as_ref().unwrap().gcode;
        assert!(g.contains(" S204\n") && g.contains(" S128\n"));
        assert!(!g.contains("S1000"));
    }

    #[test]
    fn a_file_of_settings_with_a_mistake_changes_nothing() {
        let (s, mut w, _p) = drawn("vec-config-bad");
        let cfg = s.write("b.cfg", "[machine]\ns_max = 255\n[defaults]\npower = 500\n");
        assert!(!w.load_config(&cfg));
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("power") && e.contains("b.cfg"))
        );
        let smax = w
            .fields
            .iter()
            .find(|f| f.key == Key::SMax)
            .unwrap()
            .text
            .clone();
        assert_eq!(smax, "1000");
        assert!(w.config_path.is_none());
        assert!(!w.load_config(&s.0.join("missing.cfg")));
    }

    #[test]
    fn a_dropped_file_is_opened_by_what_it_is() {
        let s = Scratch::new("vec-drop");
        let svg = s.write("part.svg", DRAWING);
        let pgm = s.write("pic.pgm", PICTURE);
        let txt = s.write("notes.txt", "hello");
        let mut w = Window::new(Lang::En);
        assert!(w.open_dropped(&svg));
        assert_eq!(w.mode, Mode::Cut);
        // a file that is not one of the four kinds leaves everything as it is
        assert!(!w.open_dropped(&txt));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("notes.txt")));
        assert_eq!(w.mode, Mode::Cut);
        assert_eq!(w.rows.len(), 2);
        // a picture takes the program back to engraving
        assert!(w.open_dropped(&pgm));
        assert_eq!(w.mode, Mode::Engrave);
        assert!(
            w.vector.is_some(),
            "the drawing stays open for when the person comes back to cutting"
        );
        assert!(w.can_compute());
        w.set_mode(Mode::Cut);
        assert_eq!(w.current_name().as_deref(), Some("part.svg"));
        // the extension is read in any case
        let upper = s.write("PART2.SVG", DRAWING);
        assert!(w.open_dropped(&upper));
    }

    #[test]
    fn a_drawing_that_cannot_be_read_is_refused_and_the_one_that_was_open_stays() {
        let (s, mut w, _p) = drawn("vec-refuse");
        let bad = s.write("bad.svg", "<svg");
        assert!(!w.open_vector(&bad));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("bad.svg")));
        assert_eq!(w.rows.len(), 2);
        assert_eq!(w.current_name().as_deref(), Some("part.svg"));
        let entity = s.write("entity.svg", r#"<!DOCTYPE svg [<!ENTITY a "b">]><svg/>"#);
        assert!(!w.open_vector(&entity));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("entities")));
        // larger than the limit of the settings
        w.settings.read.limits.max_file_bytes = 10;
        let again = s.write("again.svg", DRAWING);
        assert!(!w.open_vector(&again));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("larger")));
        assert!(!w.open_vector(&s.0.join("missing.svg")));
    }

    #[test]
    fn cutting_needs_a_drawing() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Cut);
        assert!(!w.can_compute());
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedVector);
        assert!(w.status.is_problem());
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(PathBuf::from("calibration.nc"))
        );
    }

    #[test]
    fn a_dxf_drawing_is_cut_by_layer() {
        let s = Scratch::new("vec-dxf");
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\nHoles\n10\n10\n20\n10\n40\n2\n0\nLINE\n8\nScore\n10\n0\n20\n0\n11\n20\n21\n0\n0\nENDSEC\n0\nEOF\n";
        let p = s.write("plate.dxf", dxf);
        let mut w = Window::new(Lang::En);
        assert!(w.open_vector(&p));
        let keys: Vec<&str> = w.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["holes", "score"]);
        w.set_layer_mode(1, LayerMode::Engrave);
        assert!(w.compute());
        let (layers, paths, cut_mm, engrave_mm) = cut_summary(&w);
        assert_eq!((layers, paths), (2, 2));
        // the line is engraved at its length, the hole is a circle of radius 2
        assert_eq!(engrave_mm, 20.0);
        assert!(cut_mm > 12.4 && cut_mm < 12.57, "{cut_mm}");
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(s.0.join("plate.nc"))
        );
    }

    #[test]
    fn the_notes_of_the_reading_are_kept_for_the_window() {
        let s = Scratch::new("vec-notes");
        let p = s.write(
            "n.svg",
            r##"<svg><text x="0" y="0">hello</text><path d="M0 0 L10 0" stroke="#f00"/></svg>"##,
        );
        let mut w = Window::new(Lang::En);
        assert!(w.open_vector(&p));
        assert_eq!(w.warnings.len(), 1);
        assert!(w.compute());
        assert_eq!(
            w.warnings.len(),
            1,
            "the note of the reading is still there after the plan"
        );
        assert!(w.warnings[0].contains("text"));
    }

    #[test]
    fn the_status_of_the_vector_work_speaks_every_language() {
        let opened = Status::VectorOpened {
            name: "a.svg".into(),
            shapes: 3,
            layers: 2,
        };
        assert_eq!(opened.text(Lang::En), "a.svg: 3 shapes in 2 layers");
        assert_eq!(opened.text(Lang::Es), "a.svg: 3 formas en 2 capas");
        assert_eq!(opened.text(Lang::Pt), "a.svg: 3 formas em 2 camadas");
        let done = Status::Done(Summary::Cut {
            layers: 2,
            paths: 3,
            cut_mm: 98.8,
            engrave_mm: 0.0,
            travel_mm: 12.4,
            seconds: 9.6,
        });
        assert_eq!(
            done.text(Lang::En),
            "2 layers, 3 paths, 99 mm cut, 0 mm engraved, 12 mm of travel, about 10 s"
        );
        assert_eq!(
            Status::ConfigLoaded("x.cfg".into()).text(Lang::En),
            "Settings read from x.cfg"
        );
        assert!(Status::NeedVector.text(Lang::De).contains("SVG"));
        for l in Lang::ALL {
            assert!(!done.text(l).contains('{'), "{}", l.code());
            assert!(!opened.text(l).contains('{'), "{}", l.code());
        }
    }

    #[test]
    fn the_travel_between_contours_is_drawn_only_when_it_is_asked_for() {
        let g = "G21\nM4 S0\nG0 X10 Y0 S0\nG1 X10 Y5 S500\nS0\nG0 X0 Y0\n";
        let (plain, _) = passes(g, 1000);
        assert_eq!(plain.len(), 1, "only the G1 move is a pass");
        let (all, _) = passes_of(g, 1000, true);
        assert_eq!(all.len(), 3);
        assert!(!all[0].burn && all[1].burn && !all[2].burn);
        assert_eq!(
            (all[0].x0, all[0].y0, all[0].x1, all[0].y1),
            (0.0, 0.0, 10.0, 0.0)
        );
        assert_eq!(
            (all[2].x0, all[2].y0, all[2].x1, all[2].y1),
            (10.0, 5.0, 0.0, 0.0)
        );
        assert!((all[1].level - 0.5).abs() < 1e-12);
    }

    #[test]
    fn each_job_lists_the_files_it_can_open() {
        assert!(Job::OpenImage.extensions().contains(&"png"));
        assert!(Job::OpenImage.extensions().contains(&"pgm"));
        assert!(Job::OpenImage.extensions().contains(&"svg"));
        assert!(Job::OpenImage.extensions().contains(&"dxf"));
        assert_eq!(Job::OpenConfig.extensions(), &["cfg"]);
        assert!(
            Job::ExportGcode.is_export()
                && !Job::OpenImage.is_export()
                && !Job::OpenConfig.is_export()
        );
        assert_eq!(Kind::ALL.len(), 4);
    }

    fn text_of(w: &Window, key: Key) -> String {
        w.fields.iter().find(|f| f.key == key).unwrap().text.clone()
    }

    /// A library with a way of working for a picture and one for cutting.
    const LIBRARY: &str = "[material photo]\npower = 30\nspeed = 2500\ndot = 0.2\noverscan = 1\ndither = bayer\ngamma = 1.4\ncontrast = 5\nbrightness = -3\ninvert = on\nauto_level = on\nthreshold = 90\n\n[material Cut 3 mm]\npower = 90\nspeed = 300\npasses = 2\nkerf = 0.15\n";

    fn engraved_gcode(w: &Window) -> String {
        w.done.as_ref().unwrap().gcode.clone()
    }

    #[test]
    fn the_tone_curve_of_the_boxes_is_applied_like_the_command_line_applies_it() {
        let (_s, mut w) = ready("tone");
        assert!(w.tone().unwrap().is_identity());
        assert!(w.compute());
        let plain = engraved_gcode(&w);
        // inverted: the right half burns and the left half does not
        w.set_tone_invert(true);
        assert!(w.stale);
        assert!(w.compute());
        let img = from_bytes(PICTURE.as_bytes()).unwrap();
        let tone = Tone {
            invert: true,
            ..Tone::default()
        };
        let dots = dither(
            &tone.apply(&img.resize(8, 4).unwrap()).unwrap(),
            Method::Threshold(128),
        );
        let expected = engrave(&dots, &Params::default()).unwrap();
        assert_eq!(engraved_gcode(&w), expected.gcode);
        assert_ne!(engraved_gcode(&w), plain);
        // the other switch and the three boxes
        w.set_tone_invert(false);
        w.set_tone_auto_level(true);
        set(&mut w, Key::Gamma, "1.5");
        set(&mut w, Key::Contrast, "20");
        set(&mut w, Key::Brightness, "-10");
        let t = w.tone().unwrap();
        assert_eq!(
            (t.gamma, t.contrast, t.brightness, t.invert, t.auto_level),
            (1.5, 20.0, -10.0, false, true)
        );
        // a brightness of 100 leaves nothing to burn
        set(&mut w, Key::Brightness, "100");
        w.set_tone_auto_level(false);
        assert!(w.compute());
        assert!(matches!(
            w.done.as_ref().unwrap().summary,
            Summary::Engraved { runs: 0, .. }
        ));
    }

    #[test]
    fn a_tone_box_that_is_not_valid_says_which() {
        let (_s, mut w) = ready("tone-bad");
        set(&mut w, Key::Gamma, "fast");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Gamma));
        set(&mut w, Key::Gamma, "0");
        assert!(!w.compute());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("gamma")),
            "{:?}",
            w.status
        );
        set(&mut w, Key::Gamma, "1");
        set(&mut w, Key::Contrast, "101");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("contrast")));
        set(&mut w, Key::Contrast, "0");
        set(&mut w, Key::Brightness, "");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Brightness));
    }

    #[test]
    fn a_power_or_a_speed_over_the_limit_is_refused_and_never_lowered() {
        let (_s, mut w) = ready("limits");
        assert_eq!(w.limits().unwrap(), (100.0, 30_000.0));
        set(&mut w, Key::MaxPower, "40");
        assert!(!w.compute());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")),
            "{:?}",
            w.status
        );
        // the box of the power is left as it was typed
        assert_eq!(text_of(&w, Key::Power), "50");
        set(&mut w, Key::Power, "40");
        assert!(w.compute());
        set(&mut w, Key::MaxSpeed, "2000");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_speed_mm_min")));
        set(&mut w, Key::Feed, "2000");
        assert!(w.compute());
        // a limit that is not a limit is a bad box
        for (key, bad, label) in [
            (Key::MaxPower, "0", Msg::MaxPower),
            (Key::MaxPower, "100.5", Msg::MaxPower),
            (Key::MaxPower, "x", Msg::MaxPower),
            (Key::MaxSpeed, "0", Msg::MaxSpeed),
            (Key::MaxSpeed, "30001", Msg::MaxSpeed),
        ] {
            let (_s2, mut v) = ready("limits-bad");
            set(&mut v, key, bad);
            assert!(!v.compute(), "{bad}");
            assert_eq!(v.status, Status::BadValue(label), "{bad}");
        }
    }

    #[test]
    fn the_limits_hold_over_the_layers_of_a_drawing_and_over_the_grid() {
        let (_s, mut w, _p) = drawn("limits-cut");
        set(&mut w, Key::MaxPower, "30");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")));
        set(&mut w, Key::MaxPower, "100");
        set(&mut w, Key::MaxSpeed, "500");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_speed_mm_min")));
        set(&mut w, Key::MaxSpeed, "600");
        assert!(w.compute());
        // the grid: 20, 40, 60 and 80 % at 1000, 2000 and 3000 mm/min
        w.set_mode(Mode::Calibrate);
        set(&mut w, Key::MaxPower, "50");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("a power of the grid")));
        set(&mut w, Key::MaxPower, "100");
        set(&mut w, Key::MaxSpeed, "2500");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("a speed of the grid")));
        set(&mut w, Key::MaxSpeed, "3000");
        assert!(w.compute());
    }

    #[test]
    fn a_file_of_settings_puts_its_tone_and_its_limits_in_the_boxes() {
        let (s, mut w) = ready("tone-config");
        let cfg = s.write(
            "t.cfg",
            "[tone]\ngamma = 1.4\ncontrast = 10\nbrightness = -5\ninvert = on\nauto_level = on\n[machine]\nmax_power_percent = 70\nmax_speed_mm_min = 5000\n",
        );
        assert!(w.load_config(&cfg));
        assert_eq!(text_of(&w, Key::Gamma), "1.4");
        assert_eq!(text_of(&w, Key::Contrast), "10");
        assert_eq!(text_of(&w, Key::Brightness), "-5");
        assert!(w.tone_invert && w.tone_auto_level);
        assert_eq!(w.limits().unwrap(), (70.0, 5000.0));
        // a file with a mistake in the tone changes nothing
        let bad = s.write("b.cfg", "[tone]\ngamma = 0\n");
        assert!(!w.load_config(&bad));
        assert_eq!(text_of(&w, Key::Gamma), "1.4");
        // a file without the section brings the boxes back to the default curve
        let plain = s.write("p.cfg", "[machine]\ns_max = 1000\n");
        assert!(w.load_config(&plain));
        assert!(w.tone().unwrap().is_identity());
        assert_eq!(w.limits().unwrap(), (100.0, 30_000.0));
    }

    #[test]
    fn a_material_fills_the_boxes_and_the_limits_still_hold_over_it() {
        let (s, mut w) = ready("material");
        let lib = s.write("lib.cfg", LIBRARY);
        assert!(w.load_materials(&lib));
        assert_eq!(w.status, Status::MaterialsLoaded("lib.cfg".into(), 2));
        assert_eq!(w.material_names(), vec!["photo", "Cut 3 mm"]);
        assert!(w.apply_material(" PHOTO "));
        assert_eq!(w.status, Status::MaterialApplied("photo".into()));
        assert_eq!(w.material.as_deref(), Some("photo"));
        for (key, text) in [
            (Key::Power, "30"),
            (Key::Feed, "2500"),
            (Key::DotMm, "0.2"),
            (Key::Overscan, "1"),
            (Key::Gamma, "1.4"),
            (Key::Contrast, "5"),
            (Key::Brightness, "-3"),
            (Key::Threshold, "90"),
            // what the material does not say stays as it was
            (Key::SMax, "1000"),
            (Key::WidthMm, "0.8"),
        ] {
            assert_eq!(text_of(&w, key), text, "{key:?}");
        }
        assert_eq!(w.kind, Kind::Bayer);
        assert!(w.tone_invert && w.tone_auto_level && !w.overscan_auto);
        assert!(w.stale || w.done.is_none());
        // the power of the material is over the limit of the machine: refused, and the box keeps what the material said
        set(&mut w, Key::MaxPower, "20");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")));
        assert_eq!(text_of(&w, Key::Power), "30");
        set(&mut w, Key::MaxPower, "100");
        assert!(w.compute());
        // a material that is not in the library, and a library that cannot be read, change nothing
        assert!(!w.apply_material("plasma"));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("plasma")));
        assert_eq!(w.material.as_deref(), Some("photo"));
        let bad = s.write("bad.cfg", "[material x]\npower = 500\n");
        assert!(!w.load_materials(&bad));
        assert!(matches!(&w.status, Status::Failed(_)));
        assert_eq!(w.material_names().len(), 2);
        assert!(!w.load_materials(Path::new("/no/such/folder/lib.cfg")));
    }

    #[test]
    fn a_material_fills_the_table_of_layers_of_a_drawing() {
        let (s, mut w, _p) = drawn("material-rows");
        let lib = s.write("lib.cfg", LIBRARY);
        assert!(w.load_materials(&lib));
        w.set_layer_mode(0, LayerMode::Engrave);
        assert!(w.apply_material("cut 3 mm"));
        for r in &w.rows {
            assert_eq!(r.fields[LayerField::Power as usize], "90");
            assert_eq!(r.fields[LayerField::Speed as usize], "300");
            assert_eq!(r.fields[LayerField::Passes as usize], "2");
            assert_eq!(r.fields[LayerField::Kerf as usize], "0.15");
        }
        // the mode of a layer is the person's, not the material's
        assert_eq!(w.rows[0].mode, LayerMode::Engrave);
        assert_eq!(w.rows[1].mode, LayerMode::Cut);
        assert!(w.compute());
        // 90 % of 1000 is S900
        assert!(engraved_gcode(&w).contains(" S900\n"));
        // the limit of power holds over it
        set(&mut w, Key::MaxPower, "80");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")));
    }

    #[test]
    fn the_words_of_a_material_are_the_kinds_of_dots_and_the_job_lists_its_files() {
        assert_eq!(Kind::from_word("FS"), Some(Kind::FloydSteinberg));
        assert_eq!(
            Kind::from_word(" floyd-steinberg "),
            Some(Kind::FloydSteinberg)
        );
        assert_eq!(Kind::from_word("atkinson"), Some(Kind::Atkinson));
        assert_eq!(Kind::from_word("ordered"), Some(Kind::Bayer));
        assert_eq!(Kind::from_word("threshold"), Some(Kind::Threshold));
        assert_eq!(Kind::from_word("halftone"), None);
        assert_eq!(Job::OpenMaterials.extensions(), &["cfg"]);
        assert!(!Job::OpenMaterials.is_export());
        for l in Lang::ALL {
            assert!(!Job::OpenMaterials.title().tr(l).is_empty());
            assert!(Status::MaterialsLoaded("a.cfg".into(), 3)
                .text(l)
                .contains("a.cfg"));
            assert!(Status::MaterialApplied("birch".into())
                .text(l)
                .contains("birch"));
        }
        assert_eq!(
            Status::MaterialsLoaded("a.cfg".into(), 3).text(Lang::Es),
            "a.cfg: 3 material(es) cargado(s)"
        );
    }

    #[test]
    fn the_focus_test_needs_the_step_of_the_height_and_is_the_one_of_the_command_line() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Focus);
        assert!(w.can_compute());
        // which way is down depends on the machine: there is no default
        assert!(!w.compute());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("focus step")),
            "{:?}",
            w.status
        );
        set(&mut w, Key::ZStep, "0.5");
        assert!(w.compute());
        let f = FocusSettings {
            z_step_mm: 0.5,
            ..FocusSettings::default()
        };
        let done = w.done.as_ref().unwrap();
        assert_eq!(done.gcode, focus_test(&f, 1000, 100.0).unwrap());
        assert_eq!(
            done.summary,
            Summary::Focused {
                lines: 9,
                length_mm: 10.0
            }
        );
        assert_eq!(w.status.text(Lang::En), "Focus test: 9 lines of 10 mm");
        assert_eq!(
            w.status.text(Lang::Es),
            "Prueba de enfoque: 9 líneas de 10 mm"
        );
        // the warning that the test moves Z is in the notes, in the language of the window
        assert_eq!(w.warnings, vec![Msg::FocusWarning.tr(Lang::En).to_string()]);
        // the lines are drawn: nine lines of 10 mm with the laser on
        assert_eq!(w.segs.iter().filter(|s| s.burn).count(), 9);
        assert!(w.bounds().is_some());
        // it is written with the header of the command line
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(PathBuf::from("focus.nc"))
        );
        let s = Scratch::new("focus-export");
        let out = s.0.join("focus.nc");
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("; tools: laser, S 0-1000, focus test (the head moves in Z)"));
        assert!(text.ends_with("M2\n"));
    }

    #[test]
    fn the_focus_test_obeys_the_limits_and_says_which_box_is_wrong() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Focus);
        set(&mut w, Key::ZStep, "0.5");
        set(&mut w, Key::MaxPower, "20");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")));
        set(&mut w, Key::MaxPower, "100");
        set(&mut w, Key::MaxSpeed, "500");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_speed_mm_min")));
        set(&mut w, Key::MaxSpeed, "30000");
        set(&mut w, Key::ZStep, "far");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::ZStep));
        set(&mut w, Key::ZStep, "0.5");
        set(&mut w, Key::FocusLines, "1");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("steps")));
        set(&mut w, Key::FocusLines, "2.5");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::FocusLines));
        set(&mut w, Key::FocusLines, "9");
        set(&mut w, Key::FocusLength, "");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::FocusLength));
        // a step that would take the head too far is refused by the same check as the command line
        set(&mut w, Key::FocusLength, "10");
        set(&mut w, Key::ZStep, "9");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("50 mm")));
    }

    #[test]
    fn a_file_of_settings_fills_the_boxes_of_the_focus_test() {
        let s = Scratch::new("focus-config");
        let cfg = s.write(
            "f.cfg",
            "[focus]\nz_start_mm = 2\nz_step_mm = -0.25\nsteps = 5\nlength_mm = 8\nspacing_mm = 3\nfeed_mm_min = 800\npower_percent = 12\n",
        );
        let mut w = Window::new(Lang::En);
        assert!(w.load_config(&cfg));
        for (key, text) in [
            (Key::ZStart, "2"),
            (Key::ZStep, "-0.25"),
            (Key::FocusLines, "5"),
            (Key::FocusLength, "8"),
            (Key::FocusSpacing, "3"),
            (Key::FocusFeed, "800"),
            (Key::FocusPower, "12"),
        ] {
            assert_eq!(text_of(&w, key), text, "{key:?}");
        }
        w.set_mode(Mode::Focus);
        assert!(w.compute());
        assert!(matches!(
            w.done.as_ref().unwrap().summary,
            Summary::Focused { lines: 5, .. }
        ));
        // the boxes start as the defaults of the settings
        let plain = Window::new(Lang::En);
        let d = FocusSettings::default();
        assert_eq!(text_of(&plain, Key::ZStep), format!("{}", d.z_step_mm));
        assert_eq!(text_of(&plain, Key::FocusLines), d.steps.to_string());
        assert_eq!(
            text_of(&plain, Key::FocusFeed),
            format!("{}", d.feed_mm_min)
        );
        assert_eq!(
            text_of(&plain, Key::FocusPower),
            format!("{}", d.power_percent)
        );
    }

    /// The frame that the command line writes for the same work, to compare with.
    fn frame_of(w: &Window, margin: f64, laps: u32, pilot_s: u32) -> String {
        let work = w.engrave_job().unwrap();
        let r =
            preview::analyse(&preview::read(&work.gcode), &preview::Options::default()).unwrap();
        let (lo, hi) = r.burn_bounds.or(r.bounds).unwrap();
        let (lo, hi) = frame::grow(lo, hi, margin);
        frame::gcode(
            &Frame {
                lo,
                hi,
                feed_mm_min: 3000.0,
                laps,
                s_max: 1000,
                pilot_s,
            },
            None,
        )
        .unwrap()
    }

    #[test]
    fn the_frame_of_the_work_is_traced_with_the_laser_off_like_the_command_line_does() {
        let (_s, mut w) = ready("frame");
        w.set_mode(Mode::Frame);
        assert!(w.can_compute());
        assert!(!w.needs_confirmation());
        assert!(w.compute());
        let done = w.done.as_ref().unwrap();
        assert_eq!(done.gcode, frame_of(&w, 0.0, 1, 0));
        // nothing sets a power or turns the spindle on
        assert!(!done.gcode.contains(" S") && !done.gcode.contains("M3"));
        assert_eq!(done.tools, "laser OFF, frame of the work");
        let Summary::Framed {
            width_mm,
            height_mm,
            laps,
            pilot_percent,
        } = done.summary
        else {
            panic!("{:?}", done.summary)
        };
        assert!(width_mm > 0.0 && height_mm > 0.0);
        assert_eq!((laps, pilot_percent), (1, 0.0));
        assert!(w.status.text(Lang::En).ends_with("1 lap(s), laser off"));
        assert!(w.status.text(Lang::De).ends_with("1 Runde(n), Laser aus"));
        // the margin and the laps change the rectangle and the trace
        set(&mut w, Key::FrameMargin, "2");
        set(&mut w, Key::FrameLaps, "2");
        assert!(w.compute());
        assert_eq!(w.done.as_ref().unwrap().gcode, frame_of(&w, 2.0, 2, 0));
        // the name offered and the header
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(PathBuf::from("frame.nc"))
        );
    }

    #[test]
    fn a_frame_that_lights_the_laser_needs_the_confirmation_of_the_person_every_time() {
        let (s, mut w) = ready("frame-pilot");
        w.set_mode(Mode::Frame);
        set(&mut w, Key::FramePilot, "2");
        // asking for a light is not enough: nothing is worked out, nothing is written
        assert!(w.needs_confirmation());
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedConfirm);
        assert!(w.status.is_problem());
        assert!(w.done.is_none());
        let out = s.0.join("frame.nc");
        assert!(!w.export(&out));
        assert!(!out.exists());
        // the person confirms: the laser is lit weakly with M3 and turned off with M5 before the end
        w.pilot_confirmed = true;
        assert!(!w.needs_confirmation());
        assert!(w.compute());
        let done = w.done.as_ref().unwrap();
        assert_eq!(done.gcode, frame_of(&w, 0.0, 1, 20));
        assert!(done.gcode.contains("M3 S20\n") && done.gcode.contains("M5\nM2\n"));
        assert!(done.tools.contains("pilot power of S20"));
        assert!(w.status.text(Lang::En).ends_with("laser ON at 2 %"));
        assert!(w.status.text(Lang::Pt).ends_with("laser LIGADO a 2 %"));
        // the confirmation does not survive a change of any box...
        set(&mut w, Key::FrameMargin, "1");
        assert!(!w.pilot_confirmed && w.needs_confirmation() && w.stale);
        w.pilot_confirmed = true;
        assert!(w.compute());
        // ...nor a change of mode...
        w.set_mode(Mode::Engrave);
        w.set_mode(Mode::Frame);
        assert!(!w.pilot_confirmed && w.needs_confirmation());
        // ...nor writing the file: the next one is confirmed again
        w.pilot_confirmed = true;
        assert!(w.export(&out));
        assert!(!w.pilot_confirmed);
        assert!(fs::read_to_string(&out).unwrap().contains("M3 S20\n"));
        let again = s.0.join("again.nc");
        assert!(!w.export(&again));
        assert_eq!(w.status, Status::NeedConfirm);
        assert!(!again.exists());
    }

    #[test]
    fn the_pilot_power_of_the_frame_obeys_the_cap_the_limit_and_the_bed() {
        let (s, mut w) = ready("frame-caps");
        w.set_mode(Mode::Frame);
        let confirm = |w: &mut Window| w.pilot_confirmed = true;
        // the cap of [frame] is 5 % unless the settings file says another
        set(&mut w, Key::FramePilot, "6");
        confirm(&mut w);
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_pilot_percent")));
        let cfg = s.write(
            "cap.cfg",
            "[frame]\nmax_pilot_percent = 10\npilot_percent = 8\n",
        );
        assert!(w.load_config(&cfg));
        // a pilot power in the file is only shown in the box: the laser still needs the confirmation
        assert_eq!(text_of(&w, Key::FramePilot), "8");
        assert!(w.needs_confirmation());
        confirm(&mut w);
        assert!(w.compute());
        // the limit of power of the machine is over the cap
        set(&mut w, Key::MaxPower, "5");
        confirm(&mut w);
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")));
        set(&mut w, Key::MaxPower, "100");
        // a pilot power that rounds to nothing is an error and not a laser that stays off
        set(&mut w, Key::FramePilot, "0.01");
        confirm(&mut w);
        assert!(!w.compute());
        // a negative pilot power, a pilot power that is not a number
        set(&mut w, Key::FramePilot, "-1");
        confirm(&mut w);
        assert!(!w.compute());
        set(&mut w, Key::FramePilot, "dim");
        assert!(!w.needs_confirmation());
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::PilotPower));
        // the speed obeys the limit of speed
        set(&mut w, Key::FramePilot, "0");
        set(&mut w, Key::MaxSpeed, "1000");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_speed_mm_min")));
        set(&mut w, Key::MaxSpeed, "30000");
        // a frame that does not fit the bed of the settings file is refused
        let small = s.write("bed.cfg", "[machine]\nbed_x_mm = 1\nbed_y_mm = 1\n");
        assert!(w.load_config(&small));
        set(&mut w, Key::FrameMargin, "2");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("outside of the bed")));
        // laps and margin out of range
        let (_s2, mut v) = ready("frame-laps");
        v.set_mode(Mode::Frame);
        set(&mut v, Key::FrameLaps, "0");
        assert!(!v.compute());
        set(&mut v, Key::FrameLaps, "21");
        assert!(!v.compute());
        set(&mut v, Key::FrameLaps, "1");
        set(&mut v, Key::FrameMargin, "101");
        assert!(!v.compute());
    }

    #[test]
    fn the_frame_follows_the_picture_or_the_drawing_that_was_opened_last() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Frame);
        assert!(!w.can_compute());
        assert_eq!(w.framed_mode(), None);
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedImage);
        let s = Scratch::new("frame-which");
        let pic = s.write("pic.pgm", PICTURE);
        let part = s.write("part.svg", DRAWING);
        // a drawing alone is framed as a drawing: the square of 20 mm
        assert!(w.open_vector(&part));
        w.set_mode(Mode::Frame);
        assert_eq!(w.framed_mode(), Some(Mode::Cut));
        assert!(w.compute());
        let Summary::Framed {
            width_mm,
            height_mm,
            ..
        } = w.done.as_ref().unwrap().summary
        else {
            panic!()
        };
        assert!((width_mm - 20.0).abs() < 1e-6 && (height_mm - 20.0).abs() < 1e-6);
        // a picture opened after it is the work now
        assert!(w.open_image(&pic));
        set(&mut w, Key::WidthMm, "0.8");
        w.set_mode(Mode::Frame);
        assert_eq!(w.framed_mode(), Some(Mode::Engrave));
        assert!(w.compute());
        let Summary::Framed { width_mm, .. } = w.done.as_ref().unwrap().summary else {
            panic!()
        };
        assert!(width_mm < 2.0);
        // choosing cutting again goes back to the drawing
        w.set_mode(Mode::Cut);
        w.set_mode(Mode::Frame);
        assert_eq!(w.framed_mode(), Some(Mode::Cut));
    }

    #[test]
    fn a_program_is_opened_to_be_seen_and_is_never_written_again() {
        let (s, mut w) = ready("gcode-open");
        let out = s.0.join("work.nc");
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        let mut v = Window::new(Lang::En);
        assert!(v.open_gcode(&out));
        assert_eq!(v.mode, Mode::Preview);
        assert_eq!(v.current_name().as_deref(), Some("work.nc"));
        assert!(v.has_work() && v.can_compute() && !v.can_export());
        let done = v.done.as_ref().unwrap();
        assert_eq!(done.gcode, text);
        let r = preview::analyse(
            &preview::read(&text),
            &preview::Options {
                s_max: Some(1000),
                ..preview::Options::default()
            },
        )
        .unwrap();
        assert_eq!(
            done.summary,
            Summary::Previewed {
                moves: r.moves,
                burn_mm: r.burn_mm,
                travel_mm: r.travel_mm,
                seconds: r.seconds
            }
        );
        // the path is drawn, with the burning passes in it
        assert!(v.segs.iter().any(|s| s.burn));
        assert!(v.bounds().is_some());
        // the lines of the preview: what the program does
        let lines = v.preview_lines();
        assert_eq!(lines.len(), 1 + r.notes.len());
        assert!(
            lines[0].starts_with(&format!("{} moves, ", r.moves)),
            "{lines:?}"
        );
        assert!(lines[0].contains(&preview::clock(r.seconds)));
        assert_eq!(v.status.text(Lang::En), lines[0]);
        // it is not written again, and no name is offered
        let copy = s.0.join("copy.nc");
        assert!(!v.export(&copy));
        assert_eq!(v.status, Status::NothingToWrite);
        assert!(!copy.exists());
        assert_eq!(v.suggested_output_for(Job::ExportGcode), None);
        // the folder of the program is where the window of files starts
        assert_eq!(v.work_folder().as_deref(), Some(s.0.as_path()));
        // opened by dropping it, by the extension
        let mut d = Window::new(Lang::En);
        assert!(d.open_dropped(&out));
        assert_eq!(d.mode, Mode::Preview);
        // a picture opened after it takes the window back to engraving
        let pic = s.0.join("pic.pgm");
        assert!(d.open_dropped(&pic));
        assert_eq!(d.mode, Mode::Engrave);
        // the program stays open for when the person comes back to it
        assert!(d.program.is_some());
    }

    #[test]
    fn a_program_that_looks_dangerous_is_opened_with_its_notes() {
        let s = Scratch::new("gcode-danger");
        let p = s.write(
            "bad.nc",
            "G21\nG90\nG0 X0 Y0\nM3 S500\nG0 X10 Y0\nG1 X20 Y0\nG1 X30 Y0 F100 S2000\nM2\n",
        );
        let mut w = Window::new(Lang::En);
        assert!(w.open_gcode(&p));
        assert!(
            w.warnings.iter().any(|n| n.contains("rapid move")),
            "{:?}",
            w.warnings
        );
        assert!(w.warnings.iter().any(|n| n.contains("no feed")));
        assert!(w
            .warnings
            .iter()
            .any(|n| n.contains("above the full power")));
        let lines = w.preview_lines();
        assert_eq!(lines.len(), 1 + w.warnings.len());
        // the full power of the box decides what is above it
        set(&mut w, Key::SMax, "5000");
        assert!(w.compute());
        assert!(!w
            .warnings
            .iter()
            .any(|n| n.contains("above the full power")));
    }

    #[test]
    fn a_file_that_is_not_a_program_is_refused_and_what_was_open_stays() {
        let s = Scratch::new("gcode-junk");
        let good = s.write(
            "good.nc",
            "G21\nG90\nF1000\nG0 X0 Y0\nM3 S100\nG1 X10 Y0\nM5\nM2\n",
        );
        let junk = s.write("junk.nc", "this is not a program\n");
        let mut w = Window::new(Lang::En);
        // nothing open: the window stays as it was
        assert!(!w.open_gcode(&junk));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("no move")));
        assert_eq!(w.mode, Mode::Engrave);
        assert!(w.program.is_none());
        assert!(!w.open_gcode(Path::new("/no/such/folder/a.nc")));
        // a program open: it stays
        assert!(w.open_gcode(&good));
        assert!(!w.open_gcode(&junk));
        assert_eq!(w.mode, Mode::Preview);
        assert_eq!(w.program.as_ref().unwrap().name, "good.nc");
        assert!(w.done.is_some() && !w.segs.is_empty());
        // a picture is open, then a program fails: the window goes back to the picture
        let (_s2, mut v) = ready("gcode-junk-pic");
        assert!(!v.open_gcode(&junk));
        assert_eq!(v.mode, Mode::Engrave);
        assert!(v.compute());
    }

    #[test]
    fn the_preview_of_the_work_counts_it_and_warns_when_it_goes_outside_the_bed() {
        let (s, mut w) = ready("preview-work");
        assert!(w.preview_lines().is_empty());
        assert!(w.compute());
        let lines = w.preview_lines();
        assert!(
            lines[0].contains(" moves, ") && lines[0].contains("burnt"),
            "{lines:?}"
        );
        assert_eq!(lines.len(), 1);
        // the time counts the corners: a lower acceleration takes longer
        let quick = w.report.as_ref().unwrap().seconds;
        set(&mut w, Key::Accel, "10");
        assert!(w.compute());
        assert!(w.report.as_ref().unwrap().seconds > quick);
        // a bed that is smaller than the work
        let cfg = s.write("bed.cfg", "[machine]\nbed_x_mm = 1\nbed_y_mm = 1\n");
        assert!(w.load_config(&cfg));
        set(&mut w, Key::WidthMm, "5");
        assert!(w.compute());
        assert!(
            w.preview_lines()
                .iter()
                .any(|n| n.contains("outside of the bed")),
            "{:?}",
            w.preview_lines()
        );
        // without a result there is nothing to say
        let fresh = Window::new(Lang::En);
        assert!(fresh.preview_lines().is_empty());
    }

    #[test]
    fn the_words_of_the_preview_speak_every_language() {
        let done = Status::Done(Summary::Previewed {
            moves: 12,
            burn_mm: 34.4,
            travel_mm: 5.0,
            seconds: 65.0,
        });
        assert_eq!(
            done.text(Lang::En),
            "12 moves, 34 mm burnt, 5 mm of travel, about 1 min 05 s"
        );
        assert_eq!(
            done.text(Lang::De),
            "12 Bewegungen, 34 mm gebrannt, 5 mm Leerfahrt, etwa 1 min 05 s"
        );
        for l in Lang::ALL {
            assert!(done.text(l).contains("12") && done.text(l).contains("1 min 05 s"));
            assert!(!Status::NothingToWrite.text(l).is_empty());
            assert!(!Job::OpenGcode.title().tr(l).is_empty());
            assert!(Status::NothingToWrite.is_problem());
        }
        assert!(Job::OpenGcode.extensions().contains(&"nc"));
        assert!(!Job::OpenGcode.is_export());
    }

    /// Two dark dots in a white picture: two contours.
    const TWO_DOTS: &str =
        "P2\n5 3\n255\n255 255 255 255 255\n255 0 255 0 255\n255 255 255 255 255\n";

    /// A window with the picture of the two dots open, 5 mm wide, in the mode of the outline.
    fn tracing(tag: &str) -> (Scratch, Window) {
        let s = Scratch::new(tag);
        let p = s.write("dots.pgm", TWO_DOTS);
        let mut w = Window::new(Lang::En);
        assert!(w.open_image(&p));
        set(&mut w, Key::WidthMm, "5");
        w.set_mode(Mode::Trace);
        (s, w)
    }

    #[test]
    fn the_outline_is_cut_like_the_command_line_cuts_it() {
        let (_s, mut w) = tracing("trace-same");
        assert!(w.can_compute() && w.can_export());
        assert!(w.compute());
        let img = from_bytes(TWO_DOTS.as_bytes()).unwrap();
        let drawing =
            trace::trace(&img, 5.0, &TraceSettings::default(), w.settings.read.limits).unwrap();
        let mut s = Settings::default();
        s.machine.s_max = 1000;
        let mut p = s.params_for(trace::LAYER);
        p.mode = LayerMode::Cut;
        p.power_percent = 50.0;
        p.speed_mm_min = 3000.0;
        s.layers = vec![(trace::LAYER.to_string(), p)];
        let expected = plan_cut(&drawing, &s).unwrap();
        let done = w.done.as_ref().unwrap();
        assert_eq!(done.gcode, expected.gcode);
        let Summary::Traced {
            contours,
            points,
            cut_mm,
            engrave_mm,
            ..
        } = done.summary
        else {
            panic!("{:?}", done.summary)
        };
        assert_eq!(contours, 2);
        assert!(points >= 6 && cut_mm > 0.0 && engrave_mm == 0.0);
        assert!(!w.segs.is_empty() && !w.stale);
        assert_eq!(w.current_name().as_deref(), Some("dots.pgm"));
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode)
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())),
            Some("dots.nc".to_string())
        );
        assert!(done.source.as_ref().is_some_and(|(n, _)| n == "dots.pgm"));
    }

    #[test]
    fn the_outline_can_be_filled_with_lines_and_the_light_part_can_be_the_ink() {
        let (_s, mut w) = tracing("trace-fill");
        assert!(w.compute());
        let cut = w.done.as_ref().unwrap().gcode.clone();
        w.set_trace_engrave(true);
        assert!(w.stale);
        assert!(w.compute());
        let Summary::Traced {
            cut_mm, engrave_mm, ..
        } = w.done.as_ref().unwrap().summary
        else {
            panic!()
        };
        assert!(engrave_mm > 0.0 && cut_mm == 0.0);
        assert_ne!(w.done.as_ref().unwrap().gcode, cut);
        // the light part of the picture is the ink: one contour round the paper instead of two round the dots
        w.set_trace_engrave(false);
        w.set_trace_invert(true);
        assert!(w.compute());
        let Summary::Traced { contours, .. } = w.done.as_ref().unwrap().summary else {
            panic!()
        };
        assert_ne!(contours, 2);
        // a threshold under every grey level leaves no ink at all
        w.set_trace_invert(false);
        set(&mut w, Key::TraceThreshold, "0");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(_)), "{:?}", w.status);
    }

    #[test]
    fn the_outline_obeys_the_limits_of_power_and_speed_and_of_the_picture() {
        let (_s, mut w) = tracing("trace-limits");
        set(&mut w, Key::MaxPower, "40");
        assert!(!w.compute());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")),
            "{:?}",
            w.status
        );
        assert_eq!(text_of(&w, Key::Power), "50");
        set(&mut w, Key::MaxPower, "100");
        set(&mut w, Key::MaxSpeed, "2000");
        assert!(!w.compute());
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("max_speed_mm_min")));
        set(&mut w, Key::MaxSpeed, "30000");
        // the limits of the picture are the ones of [trace]: more contours than allowed is an error
        w.settings.trace.max_contours = 1;
        assert!(!w.compute());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("max_contours")),
            "{:?}",
            w.status
        );
        w.settings.trace.max_contours = TraceSettings::default().max_contours;
        assert!(w.compute());
        // a box that is not valid says which
        for (key, bad, label) in [
            (Key::TraceThreshold, "256", Msg::Threshold),
            (Key::TraceThreshold, "1.5", Msg::Threshold),
            (Key::TraceSimplify, "x", Msg::TraceSimplify),
            (Key::TraceMinArea, "", Msg::TraceMinArea),
            (Key::WidthMm, "0", Msg::WidthMm),
            (Key::WidthMm, "5001", Msg::WidthMm),
        ] {
            let (_s2, mut v) = tracing("trace-bad");
            set(&mut v, key, bad);
            assert!(!v.compute(), "{bad}");
            assert_eq!(v.status, Status::BadValue(label), "{bad}");
        }
        // a value out of its range is refused by the same rule as the file of settings
        let (_s3, mut v) = tracing("trace-range");
        set(&mut v, Key::TraceSimplify, "9");
        assert!(!v.compute());
        assert!(matches!(&v.status, Status::Failed(e) if e.contains("simplify_mm")));
    }

    #[test]
    fn the_outline_needs_a_picture_and_keeps_its_mode_when_another_picture_is_opened() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Trace);
        assert!(!w.can_compute());
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedImage);
        let (s, mut v) = tracing("trace-keep");
        assert!(v.compute());
        let again = s.write("again.pgm", TWO_DOTS);
        assert!(v.open_image(&again));
        assert_eq!(v.mode, Mode::Trace);
        assert_eq!(v.work_mode, Mode::Trace);
        assert!(v.done.is_none() && v.segs.is_empty());
        assert!(v.compute());
        // a picture opened in the other modes still goes to engraving
        let (_s4, mut e) = ready("trace-other");
        assert_eq!(e.mode, Mode::Engrave);
        assert_eq!(e.work_mode, Mode::Engrave);
        e.set_mode(Mode::Trace);
        assert_eq!(e.work_mode, Mode::Trace);
    }

    #[test]
    fn the_frame_follows_the_outline_when_that_is_the_work() {
        let (_s, mut w) = tracing("trace-frame");
        w.set_mode(Mode::Frame);
        assert_eq!(w.framed_mode(), Some(Mode::Trace));
        assert!(w.compute());
        let Summary::Framed {
            width_mm,
            height_mm,
            pilot_percent,
            ..
        } = w.done.as_ref().unwrap().summary
        else {
            panic!()
        };
        // the box of the two dots, not the box of the whole picture
        assert!(width_mm > 2.0 && width_mm < 5.0, "{width_mm}");
        assert!(height_mm > 0.0 && height_mm < 3.0);
        assert_eq!(pilot_percent, 0.0);
        assert!(!w.done.as_ref().unwrap().gcode.contains("M3"));
    }

    #[test]
    fn a_file_of_settings_fills_the_boxes_of_the_outline() {
        let s = Scratch::new("trace-cfg");
        let cfg = s.write(
            "t.cfg",
            "[trace]\nthreshold = 90\ninvert = on\nsimplify_mm = 0.1\nmin_area_mm2 = 1\nmax_contours = 7\n",
        );
        let mut w = Window::new(Lang::En);
        assert!(w.load_config(&cfg));
        assert_eq!(text_of(&w, Key::TraceThreshold), "90");
        assert_eq!(text_of(&w, Key::TraceSimplify), "0.1");
        assert_eq!(text_of(&w, Key::TraceMinArea), "1");
        assert!(w.trace_invert);
        assert_eq!(w.settings.trace.max_contours, 7);
        // the defaults of the boxes are the ones of the settings
        let plain = Window::new(Lang::En);
        let d = TraceSettings::default();
        assert_eq!(
            text_of(&plain, Key::TraceThreshold),
            d.threshold.to_string()
        );
        assert_eq!(
            text_of(&plain, Key::TraceSimplify),
            format!("{}", d.simplify_mm)
        );
        assert_eq!(
            text_of(&plain, Key::TraceMinArea),
            format!("{}", d.min_area_mm2)
        );
        assert!(!plain.trace_invert && !plain.trace_engrave);
    }

    #[test]
    fn the_words_of_the_outline_speak_every_language() {
        let done = Status::Done(Summary::Traced {
            contours: 2,
            points: 17,
            cut_mm: 12.4,
            engrave_mm: 0.0,
            seconds: 9.0,
        });
        assert_eq!(
            done.text(Lang::En),
            "Outline: 2 contours of 17 points, 12 mm cut, 0 mm engraved, 9 s"
        );
        for l in Lang::ALL {
            let text = done.text(l);
            assert!(text.contains("17") && text.contains("12"), "{text}");
            assert!(!Msg::ModeTrace.tr(l).is_empty() && !Msg::SectionTrace.tr(l).is_empty());
        }
        assert_ne!(done.text(Lang::De), done.text(Lang::En));
    }

    /// Two machines in one file: a diode laser that is chosen by the file, and a CO2 laser that leaves out the travel and the speed.
    const TWO_MACHINES: &str = "[machine]\ns_max = 255\nbed_x_mm = 100\nbed_y_mm = 100\nprofile = Diode\n\
        [profile diode]\ntravel_x_mm = 400\ntravel_y_mm = 300\nmax_power_percent = 80\nmax_speed_mm_min = 6000\ns_max = 1000\nlaser_watts = 10\n\
        [profile CO2]\ns_max = 100\nmax_power_percent = 90\n";

    #[test]
    fn the_profile_of_the_settings_file_is_the_machine_and_the_boxes_show_it() {
        let s = Scratch::new("profile-file");
        let cfg = s.write("m.cfg", TWO_MACHINES);
        let mut w = Window::new(Lang::En);
        assert!(w.profile_names().is_empty());
        assert!(w.load_config(&cfg));
        assert_eq!(w.profile_names(), vec!["diode", "co2"]);
        assert_eq!(w.settings.profile.as_deref(), Some("diode"));
        assert_eq!(text_of(&w, Key::SMax), "1000");
        assert_eq!(text_of(&w, Key::MaxPower), "80");
        assert_eq!(text_of(&w, Key::MaxSpeed), "6000");
        assert_eq!(w.settings.machine.bed_mm, Some((400.0, 300.0)));
    }

    #[test]
    fn choosing_a_profile_sets_the_boxes_and_the_limits_hold_over_them() {
        let s = Scratch::new("profile-choose");
        let cfg = s.write("m.cfg", TWO_MACHINES);
        let (_s2, mut w) = ready("profile-choose-w");
        assert!(w.load_config(&cfg));
        set(&mut w, Key::Power, "90");
        // the diode laser stops at 80 %
        assert!(!w.compute());
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("max_power_percent")),
            "{:?}",
            w.status
        );
        // the CO2 laser leaves out the travel and the speed: the machine section decides, never the diode before it
        assert!(w.apply_profile(" CO2 "));
        assert_eq!(
            w.status,
            Status::ProfileApplied {
                name: "co2".to_string(),
                s_max: 100,
                max_power: 90.0,
                max_speed: 30_000.0,
            }
        );
        assert_eq!(text_of(&w, Key::SMax), "100");
        assert_eq!(text_of(&w, Key::MaxPower), "90");
        assert_eq!(text_of(&w, Key::MaxSpeed), "30000");
        assert_eq!(w.settings.machine.bed_mm, Some((100.0, 100.0)));
        assert!(w.compute());
        // and back: the limits of the diode hold again
        assert!(w.apply_profile("diode"));
        assert_eq!(text_of(&w, Key::MaxPower), "80");
        assert!(!w.compute());
        set(&mut w, Key::Power, "70");
        assert!(w.compute());
    }

    #[test]
    fn a_profile_that_is_not_in_the_file_changes_nothing_and_says_which_there_are() {
        let s = Scratch::new("profile-none");
        let cfg = s.write("m.cfg", TWO_MACHINES);
        let mut w = Window::new(Lang::En);
        assert!(w.load_config(&cfg));
        let before = (
            text_of(&w, Key::SMax),
            text_of(&w, Key::MaxPower),
            w.settings.profile.clone(),
        );
        assert!(!w.apply_profile("plasma"));
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("plasma") && e.contains("diode") && e.contains("co2")),
            "{:?}",
            w.status
        );
        assert_eq!(
            before,
            (
                text_of(&w, Key::SMax),
                text_of(&w, Key::MaxPower),
                w.settings.profile.clone()
            )
        );
        // a window with no file has no profile to choose
        let mut bare = Window::new(Lang::En);
        assert!(!bare.apply_profile("diode"));
        assert!(bare.status.is_problem());
    }

    #[test]
    fn choosing_a_profile_makes_the_work_stale_and_takes_the_confirmation_back() {
        let s = Scratch::new("profile-stale");
        let cfg = s.write("m.cfg", TWO_MACHINES);
        let (_s2, mut w) = ready("profile-stale-w");
        assert!(w.load_config(&cfg));
        assert!(w.compute());
        w.pilot_confirmed = true;
        assert!(w.apply_profile("co2"));
        assert!(w.stale);
        assert!(!w.pilot_confirmed);
    }

    #[test]
    fn the_words_of_the_machine_profile_speak_every_language() {
        let done = Status::ProfileApplied {
            name: "diode".to_string(),
            s_max: 1000,
            max_power: 80.0,
            max_speed: 6000.0,
        };
        assert_eq!(
            done.text(Lang::En),
            "Machine profile diode: S 1000, power up to 80 %, speed up to 6000 mm/min"
        );
        for l in Lang::ALL {
            let text = done.text(l);
            assert!(text.contains("diode") && text.contains("1000") && text.contains("6000"));
            assert!(!Msg::SectionMachine.tr(l).is_empty() && !Msg::NoProfile.tr(l).is_empty());
        }
    }
}
