// EH3D-PHOENIX-LASER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_phoenix_laser::calibrate::{focus_test, grid, labelled_grid, ladder, Grid, Labels};
use eh3d_phoenix_laser::cut::{layers as layer_list, plan, Plan};
use eh3d_phoenix_laser::dither::{dither, Method};
use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::energy::describe;
use eh3d_phoenix_laser::frame::{self, Frame};
use eh3d_phoenix_laser::gcode_out::{header, sha256_hex, Header};
use eh3d_phoenix_laser::image::from_bytes;
use eh3d_phoenix_laser::materials::{self, Material};
use eh3d_phoenix_laser::preview;
use eh3d_phoenix_laser::qr;
use eh3d_phoenix_laser::raster::{engrave, overscan_for, Params};
use eh3d_phoenix_laser::safety;
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::text as laser_text;
use eh3d_phoenix_laser::tone::Tone;
use eh3d_phoenix_laser::trace;
use eh3d_phoenix_laser::vconfig::{self, KerfSide, LayerMode, LayerParams, Settings};
use eh3d_phoenix_laser::vector::Drawing;
use eh3d_phoenix_laser::verify;
use eh3d_phoenix_laser::window::{same_file, write_atomic};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

/// A program larger than this is not read by `preview`.
const MAX_PROGRAM_BYTES: u64 = 256 << 20;

const HELP: &str = "phoenix-laser: laser engraving from an image

  phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
                [--dither fs|atkinson|threshold|bayer] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
                [--gamma 1] [--contrast 0] [--brightness 0] [--invert] [--auto-level] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
                [--material NAME --materials FILE]
  phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
  phoenix-laser calibrate OUT.nc --material NAME --materials FILE [--steps 5] [--spread 40] [--labels|--no-labels] [--label-mm 3] [--smax 1000] [--max-power 100] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
  phoenix-laser info IMAGE
  phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
                [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--max-power P] [--max-speed MM/MIN] [--profile NAME] [--material NAME --materials FILE] [--dpi N] [--tolerance MM] [--dry-run]
  phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM] [--cross|--no-cross]
  phoenix-laser layers DRAWING [--config FILE] [--dpi N]
  phoenix-laser trace IMAGE OUT.nc --width-mm W [--threshold 128] [--invert] [--simplify MM] [--min-area MM2] [--engrave] [the options of cut]
  phoenix-laser materials FILE
  phoenix-laser focus OUT.nc --z-step MM [--z-start MM] [--steps 9] [--length 10] [--spacing 2] [--feed 1000] [--power 30] [--smax 1000] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
  phoenix-laser text OUT.nc --text TEXT [--height-mm 10] [--spacing 25] [--line-spacing 50] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--align left|center|right] [--mirror] [--origin X,Y] [--bed X,Y] [--smax N]
                [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
  phoenix-laser qr OUT.nc --text DATA [--ecc L|M|Q|H] [--module-mm 1] [--line-mm 0.1] [--power 20] [--feed 1000] [--passes 1] [--max-minutes 240] [--invert] [--border-modules 0] [--overscan-mm 0] [--mirror] [--origin X,Y] [--bed X,Y] [--smax N] [--print]
                [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
  phoenix-laser frame OUT.nc --size W,H | --gcode PROGRAM.nc [--origin X,Y] [--margin MM] [--feed 3000] [--laps 1] [--bed X,Y] [--smax N]
                [--pilot P --confirm-laser-on] [--max-power P] [--max-speed MM/MIN] [--config FILE] [--profile NAME]
  phoenix-laser preview PROGRAM.nc [OUT.svg] [--accel 1000] [--rapid 6000] [--smax N] [--bed X,Y] [--width-px 900] [--config FILE] [--profile NAME]
  phoenix-laser verify PROGRAM.nc [--config FILE] [--profile NAME] [--smax N] [--max-power P] [--max-speed MM/MIN] [--bed X,Y] [--overscan-mm MM] [--accel 1000] [--expect-sha256 HASH] [--json]
  phoenix-laser [gui] [FILE]                     the window (a build with --features gui): with no arguments, with a file or with gui it opens; the commands above never open it
  phoenix-laser gui [FILE] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

IMAGE is a PNG or a PGM file. --dry-run works the engraving out and counts it but writes nothing (OUT.nc may then be left out). --power is a percent of full power; --smax is the spindle value that is full power for the controller (GRBL: $30).
The engraving is made of dots of --dot-mm on a side, so the image is scaled to --width-mm / --dot-mm dots wide.
The tone options change the grey levels before the dots are made: --gamma (0.1 to 10, more than 1 lightens the middle greys), --contrast and --brightness (-100 to 100), --invert (black burns least) and --auto-level (the darkest level becomes black and the lightest white).
DRAWING is an SVG or a DXF file. cut follows the contours (the ones inside first), vector fills the closed shapes with lines and follows the open ones. Every layer (or colour, for an SVG) has its
own settings in the file of --config (see the documentation); an option of the command line changes them for all the layers. Without --config, cut cuts everything and vector engraves everything.
layers lists the layers of a drawing with what would be done with each.
trace follows the outline of a picture (a PNG or a PGM of a logo, a silhouette or a stencil) instead of burning it dot by dot: grey levels under --threshold are ink (over it with --invert), the border is found with sub-pixel precision, simplified until no point is farther than --simplify mm from the line, and specks smaller than --min-area mm2 are dropped. The result is a layer called trace that is cut, or filled with lines with --engrave, with the settings of --config, the options of cut and every limit of the file ([trace] holds max_pixels, max_contours and max_segments).
materials FILE lists the materials of a library: a plain text file with a [material NAME] section for each material and way of working (see the documentation). --material NAME takes the power, the speed, the passes and the rest that the library says for it, before the other options, which win; the library is --materials FILE or the file named by PHOENIX_LASER_MATERIALS.
focus writes a focus test: short lines side by side, each burnt with the head at another height (Z); the sharpest and thinnest line shows where the beam is in focus. --z-step has no default because which way is down depends on the machine: check it before running the program, which moves Z. The usual values live in the [focus] section of --config.
text burns TEXT with a font of strokes of the program (capitals, figures, space and . , - + / : ! ? ( ) = _ '; lower case is burnt as capitals, a new line starts a row, and a character the font has not is refused): --height-mm is the height of a capital (at most max_height_mm, 100 unless the file says another), --spacing and --line-spacing are the gaps between letters and rows as a percent of the height, --align left, center or right sets where the rows of a text of several lines start, --passes burns the whole text that many times (a burning that would take more than --max-minutes, 240 unless the file says another, is refused), --mirror burns its mirror image (for the back of glass or acrylic), and --origin X,Y is the lower left corner. A text of more than max_chars characters, a power or a speed over the limit, or work that would go outside of the bed (--bed X,Y or the machine of --config) is refused, never cut down; the [text] section of --config holds the usual values.
qr burns DATA as a QR code of its own (versions 1 to 4, byte mode, with the level of error correction --ecc L, M, Q or H and the mask of the lowest penalty), filling each dark module with lines --line-mm apart (--invert fills the light ones instead, so the symbol is left bare in a burnt plate; --no-invert undoes invert = on of the settings; --border-modules N, only with --invert, also burns a border N modules wide around it, and the square counts as the symbol for --origin, the bed and max_size_mm; --overscan-mm runs the head up that far before each line and out past it with the laser off, and is refused when it would leave the bed): --mirror turns the symbol into its mirror image, to be burnt on the back of glass or acrylic; --module-mm is the side of a module and --origin X,Y the lower left corner; leave a light margin of four modules around the symbol on the material. --print also shows the symbol as text. Data of more than max_bytes bytes, a symbol that needs a version over max_version or is larger than max_size_mm, a fill of more than max_lines moves, a burning that would take more than --max-minutes (240 unless the file says another), a power or a speed over the limit, or work that would go outside of the bed (--bed X,Y or the machine of --config) is refused, never cut down; the [qr] section of --config holds the usual values.
frame writes a program that traces the rectangle of the work with the laser OFF, to see where it will land: --size W,H from the corner --origin, or the box of what burns in --gcode PROGRAM.nc (its real place). --margin widens it. A --pilot power (a percent, at most max_pilot_percent of [frame], 5 unless the file says another, and at most the limit of power) turns the laser on weakly and only with --confirm-laser-on; the [frame] section of --config holds the usual values.
preview reads a G-code program (this program's or another's) and says what it would do: moves, burnt and travelled length, the box, the powers, a time that counts the corners (--accel in mm/s2) and what looks dangerous (the laser on during a rapid move, a burning move with no feed, S above --smax, work outside --bed, no end); with OUT.svg it also draws the path. It only reads text.
verify reads a G-code program as text (it never runs it) and says its SHA-256, what it does and whether it passes: under the limit of power and of speed, inside the bed (--overscan-mm lets the head go that far past the bed with the laser off, as the overscan of another program does; burning outside the bed always fails, and [verify] overscan_mm of --config sets the same), with the laser off during every rapid move and a feed on every burning move; --expect-sha256 HASH ties the check to a program that was authorised: when the SHA-256 of the file is not that one (64 hex digits) the program fails, so a program that was changed after it was looked at is not run by mistake; --json gives the report for another program to keep, and the exit code is 1 when it fails.
calibrate with --material takes the powers and the feeds from the library: a ladder of --steps values (5) from --spread percent (40) below to the same above the power and the speed of the material, held between the limits and given once each, unless --powers or --feeds give a list; the power of each column is burnt below it and the feed of each row on its left, at the power and the speed of the material, so the squares can be read on the material: with --material they come by default and --no-labels takes them away, and --labels asks for them with lists too (--label-mm is the height of the figures; [calibrate] of --config holds the usual values, and also the lowest power and feed of the ladder, min_power_percent and min_feed_mm_min).
--max-power is the limit of power, a percent of full power (100 when nothing says another): a program whose power is more than the limit is refused, never lowered. engrave and calibrate also read the limit and --smax from the [machine] section of the file of --config, and engrave the tone curve from its [tone] section (a material and the tone options win over it). --max-speed is the limit of speed (mm/min, 30000 when nothing says another) and works the same way: a feed over it is refused, never lowered.
A [profile NAME] section of the file of --config describes a machine and laser (its travel, its limit of power and of speed, its S and its watts); --profile NAME (or profile = NAME in [machine]) makes it the machine, over [machine] and under the options of the command line.";

/// The tone options of the command line over `base`: the curve that is applied to the grey levels before the dots are made.
fn take_tone(args: &mut Vec<String>, base: Tone) -> Result<Tone, String> {
    let mut t = base;
    if let Some(v) = flag(args, "--brightness")? {
        t.brightness = num(&v, "--brightness")?;
    }
    if let Some(v) = flag(args, "--contrast")? {
        t.contrast = num(&v, "--contrast")?;
    }
    if let Some(v) = flag(args, "--gamma")? {
        t.gamma = num(&v, "--gamma")?;
    }
    if switch(args, "--invert") {
        t.invert = true;
    }
    if switch(args, "--auto-level") {
        t.auto_level = true;
    }
    t.validate()?;
    Ok(t)
}

/// The profile that `--profile` names, made the machine of the settings. The profiles live in the file of `--config`, so the option without it is an error.
fn take_profile(
    args: &mut Vec<String>,
    settings: &mut Settings,
    has_config: bool,
) -> Result<(), String> {
    if let Some(name) = flag(args, "--profile")? {
        if !has_config {
            return Err(
                "--profile needs --config FILE: the profiles live in the settings file".into(),
            );
        }
        settings.apply_profile(&name)?;
        settings.validate()?;
    }
    Ok(())
}

/// What `engrave` and `calibrate` take from the machine.
struct MachineLimits {
    /// The limit of power, a percent of full power.
    max_power: f64,
    /// The limit of speed, mm/min.
    max_speed: f64,
    /// The spindle value that is full power, when the settings file gave it.
    s_max: Option<u32>,
    /// The tone curve of the settings file (the default curve when there is no file).
    tone: Tone,
    /// The settings file, when there is one.
    config: Option<String>,
    /// The test grid made from a material and its figures (the defaults when there is no file).
    calibrate: vconfig::CalibrateSettings,
}

/// The limits and the spindle value that `--config`, `--profile`, `--max-power` and `--max-speed` give to `engrave` and `calibrate`: the file is read as a whole (so it is checked), and
/// the options win.
fn machine_limits(args: &mut Vec<String>) -> Result<MachineLimits, String> {
    let config = flag(args, "--config")?;
    let mut s = match &config {
        Some(path) => vconfig::load(Path::new(path))?,
        None => Settings::default(),
    };
    take_profile(args, &mut s, config.is_some())?;
    let mut max_power = s.machine.max_power_percent;
    let mut max_speed = s.machine.max_speed_mm_min;
    let s_max = config.as_ref().map(|_| s.machine.s_max);
    let tone = s.tone;
    let calibrate = s.calibrate;
    if let Some(v) = flag(args, "--max-power")? {
        max_power = num(&v, "--max-power")?;
    }
    if let Some(v) = flag(args, "--max-speed")? {
        max_speed = num(&v, "--max-speed")?;
    }
    safety::validate_max(max_power)?;
    safety::validate_max_speed(max_speed)?;
    Ok(MachineLimits {
        max_power,
        max_speed,
        s_max,
        tone,
        config,
        calibrate,
    })
}

/// The material that `--material` names, taken from the library of `--materials` (or of the file named by `PHOENIX_LASER_MATERIALS`), with the file it comes from.
fn take_material(args: &mut Vec<String>) -> Result<Option<(Material, String)>, String> {
    let name = flag(args, "--material")?;
    let given = flag(args, "--materials")?;
    let Some(name) = name else {
        return if given.is_some() {
            Err("--materials needs --material NAME".into())
        } else {
            Ok(None)
        };
    };
    let file = given
        .or_else(|| env::var("PHOENIX_LASER_MATERIALS").ok())
        .filter(|f| !f.trim().is_empty())
        .ok_or("--material needs the library: --materials FILE or PHOENIX_LASER_MATERIALS")?;
    let lib = materials::load(Path::new(&file))?;
    let found = lib.find(&name).ok_or_else(|| {
        format!(
            "{file}: there is no material '{}' (it has: {})",
            name.trim(),
            lib.names().join(", ")
        )
    })?;
    Ok(Some((found.clone(), file)))
}

fn list(text: &str, what: &str) -> Result<Vec<f64>, String> {
    text.split(',').map(|p| num(p, what)).collect()
}

/// A whole number from 0 up to a million.
fn whole(text: &str, what: &str) -> Result<u32, String> {
    let v = num(text, what)?;
    if v.fract() == 0.0 && (0.0..=1_000_000.0).contains(&v) {
        Ok(v as u32)
    } else {
        Err(format!(
            "{what} '{text}' is not a whole number from 0 to 1000000"
        ))
    }
}

/// What the options of a drawing command change in the settings of every layer.
#[derive(Default)]
struct Overrides {
    power: Option<f64>,
    speed: Option<f64>,
    passes: Option<u32>,
    focus_step: Option<f64>,
    kerf: Option<f64>,
    side: Option<KerfSide>,
    tabs: Option<u32>,
    tab_width: Option<f64>,
    angle: Option<f64>,
    spacing: Option<f64>,
    cross: Option<bool>,
}

impl Overrides {
    fn take(args: &mut Vec<String>) -> Result<Overrides, String> {
        let mut o = Overrides::default();
        let mut number = |name: &str| -> Result<Option<f64>, String> {
            flag(args, name)?.map(|v| num(&v, name)).transpose()
        };
        o.power = number("--power")?;
        o.speed = number("--speed")?;
        o.focus_step = number("--focus-step")?;
        o.kerf = number("--kerf")?;
        o.tab_width = number("--tab-width")?;
        o.angle = number("--angle")?;
        o.spacing = number("--spacing")?;
        o.passes = flag(args, "--passes")?
            .map(|v| whole(&v, "--passes"))
            .transpose()?;
        o.tabs = flag(args, "--tabs")?
            .map(|v| whole(&v, "--tabs"))
            .transpose()?;
        let (cross, no_cross) = (switch(args, "--cross"), switch(args, "--no-cross"));
        if cross && no_cross {
            return Err("--cross and --no-cross cannot be given together".into());
        }
        o.cross = if cross {
            Some(true)
        } else if no_cross {
            Some(false)
        } else {
            None
        };
        o.side = flag(args, "--kerf-side")?
            .map(|v| {
                KerfSide::from_word(&v).ok_or_else(|| {
                    format!("--kerf-side '{v}' is not none, auto, inside or outside")
                })
            })
            .transpose()?;
        Ok(o)
    }

    fn apply(&self, p: &mut LayerParams) {
        p.power_percent = self.power.unwrap_or(p.power_percent);
        p.speed_mm_min = self.speed.unwrap_or(p.speed_mm_min);
        p.passes = self.passes.unwrap_or(p.passes);
        p.focus_step_mm = self.focus_step.unwrap_or(p.focus_step_mm);
        p.kerf_mm = self.kerf.unwrap_or(p.kerf_mm);
        p.kerf_side = self.side.unwrap_or(p.kerf_side);
        p.tabs = self.tabs.unwrap_or(p.tabs);
        p.tab_width_mm = self.tab_width.unwrap_or(p.tab_width_mm);
        p.hatch_angle_deg = self.angle.unwrap_or(p.hatch_angle_deg);
        p.hatch_spacing_mm = self.spacing.unwrap_or(p.hatch_spacing_mm);
        p.hatch_cross = self.cross.unwrap_or(p.hatch_cross);
    }
}

/// The settings of a drawing command: the file of `--config` (or the defaults, with `default_mode` for every layer) and the options of the command line over it. Gives the file of
/// the files it was made from (the settings and the library of materials), so that none of them is ever written over.
fn drawing_settings(
    args: &mut Vec<String>,
    default_mode: LayerMode,
) -> Result<(Settings, Vec<String>), String> {
    let config = flag(args, "--config")?;
    let material = take_material(args)?;
    let mut s = match &config {
        Some(path) => vconfig::load(Path::new(path))?,
        None => {
            let mut s = Settings::default();
            s.defaults.mode = default_mode;
            s
        }
    };
    take_profile(args, &mut s, config.is_some())?;
    if let Some(v) = flag(args, "--dpi")? {
        s.read.dpi = num(&v, "--dpi")?;
    }
    if let Some(v) = flag(args, "--tolerance")? {
        s.read.tolerance_mm = num(&v, "--tolerance")?;
    }
    if let Some(v) = flag(args, "--smax")? {
        s.machine.s_max = whole(&v, "--smax")?;
    }
    if let Some(v) = flag(args, "--overscan")? {
        s.machine.overscan_mm = num(&v, "--overscan")?;
    }
    if let Some(v) = flag(args, "--max-power")? {
        s.machine.max_power_percent = num(&v, "--max-power")?;
    }
    if let Some(v) = flag(args, "--max-speed")? {
        s.machine.max_speed_mm_min = num(&v, "--max-speed")?;
    }
    if let Some(v) = flag(args, "--origin")? {
        let o = list(&v, "--origin")?;
        if o.len() != 2 {
            return Err("--origin needs X,Y".into());
        }
        s.machine.origin = (o[0], o[1]);
    }
    if let Some(v) = flag(args, "--bed")? {
        let b = list(&v, "--bed")?;
        if b.len() != 2 {
            return Err("--bed needs X,Y".into());
        }
        s.machine.bed_mm = Some((b[0], b[1]));
    }
    // the material is applied to every layer like the options of the command line, and under them
    if let Some((m, _)) = &material {
        m.apply_to_layer(&mut s.defaults);
        for (_, p) in s.layers.iter_mut() {
            m.apply_to_layer(p);
        }
    }
    let over = Overrides::take(args)?;
    over.apply(&mut s.defaults);
    for (_, p) in s.layers.iter_mut() {
        over.apply(p);
    }
    s.validate()?;
    let sources = config.into_iter().chain(material.map(|(_, f)| f)).collect();
    Ok((s, sources))
}

/// Reads an SVG or DXF file, with the limit on its size that the settings give.
fn read_drawing(path: &str, s: &Settings) -> Result<(Vec<u8>, Drawing), String> {
    let meta = fs::metadata(path).map_err(|e| format!("{path}: {e}"))?;
    if meta.len() > s.read.limits.max_file_bytes as u64 {
        return Err(format!(
            "{path}: the file is larger than {} bytes (max_file_bytes)",
            s.read.limits.max_file_bytes
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let drawing = match ext.as_str() {
        "svg" => read_svg(&bytes, &s.read),
        "dxf" => read_dxf(&bytes, &s.read),
        _ => Err("it is not an SVG or a DXF file".to_string()),
    }
    .map_err(|e| format!("{path}: {e}"))?;
    Ok((bytes, drawing))
}

/// Writes the plan of a drawing made from the file `input` (nothing is written when `out` is `None`, a dry run) and says what it holds. The output is never written over the file it is
/// made from or over the settings, and the header names the file with its hash.
fn emit_plan(
    input: &str,
    out: Option<&str>,
    bytes: &[u8],
    planned: &Plan,
    settings: &Settings,
    sources: &[String],
) -> Result<(), String> {
    match out {
        None => print!("dry run, nothing written: "),
        Some(out) => {
            let out_path = Path::new(out);
            if same_file(Path::new(input), out_path)
                || sources.iter().any(|c| same_file(Path::new(c), out_path))
            {
                return Err(format!(
                    "{out}: the output would be written over a file it is made from"
                ));
            }
            let name = Path::new(input)
                .file_name()
                .map_or(input.to_string(), |n| n.to_string_lossy().into_owned());
            let tools = format!("laser, S 0-{}, vector layers", settings.machine.s_max);
            let head = header(&Header {
                generator: "phoenix-laser",
                version: env!("CARGO_PKG_VERSION"),
                source: Some((&name, sha256_hex(bytes))),
                tools: &tools,
            });
            write_atomic(out_path, &format!("{head}{}", planned.gcode))?;
        }
    }
    for note in &planned.warnings {
        eprintln!("note: {note}");
    }
    let st = &planned.stats;
    println!(
        "{} layers, {} paths, {:.0} mm cut, {:.0} mm engraved, {:.0} mm of travel, about {:.0} s",
        st.layers, st.contours, st.cut_mm, st.engrave_mm, st.travel_mm, st.seconds
    );
    Ok(())
}

/// `cut` and `vector`: a drawing becomes the G-code of the laser.
fn drawing_job(mut args: Vec<String>, default_mode: LayerMode) -> Result<(), String> {
    let dry_run = switch(&mut args, "--dry-run");
    let (settings, sources) = drawing_settings(&mut args, default_mode)?;
    let needed = if dry_run { 1 } else { 2 };
    if args.len() < needed {
        return Err(if dry_run {
            "a drawing is needed".into()
        } else {
            "a drawing and an output file are needed".into()
        });
    }
    finish(&args, 2)?;
    let (bytes, drawing) = read_drawing(&args[0], &settings)?;
    let planned = plan(&drawing, &settings)?;
    emit_plan(
        &args[0],
        (!dry_run).then(|| args[1].as_str()),
        &bytes,
        &planned,
        &settings,
        &sources,
    )
}

/// `trace`: the contours of a picture become the G-code of the laser, cut (or engraved with `--engrave`) like the shapes of a drawing.
fn trace_job(mut args: Vec<String>) -> Result<(), String> {
    let dry_run = switch(&mut args, "--dry-run");
    let engrave = switch(&mut args, "--engrave");
    let width = flag(&mut args, "--width-mm")?
        .map(|v| num(&v, "--width-mm"))
        .transpose()?
        .ok_or("--width-mm is needed")?;
    let (mut settings, sources) = drawing_settings(
        &mut args,
        if engrave {
            LayerMode::Engrave
        } else {
            LayerMode::Cut
        },
    )?;
    if let Some(v) = flag(&mut args, "--threshold")? {
        let t = num(&v, "--threshold")?;
        if !(t.fract() == 0.0 && (0.0..=255.0).contains(&t)) {
            return Err(format!(
                "--threshold '{v}' is not a whole number from 0 to 255"
            ));
        }
        settings.trace.threshold = t as u8;
    }
    if switch(&mut args, "--invert") {
        settings.trace.invert = true;
    }
    if let Some(v) = flag(&mut args, "--simplify")? {
        settings.trace.simplify_mm = num(&v, "--simplify")?;
    }
    if let Some(v) = flag(&mut args, "--min-area")? {
        settings.trace.min_area_mm2 = num(&v, "--min-area")?;
    }
    settings.validate()?;
    let needed = if dry_run { 1 } else { 2 };
    if args.len() < needed {
        return Err(if dry_run {
            "trace needs a picture".into()
        } else {
            "trace needs a picture and an output file".into()
        });
    }
    finish(&args, 2)?;
    let meta = fs::metadata(&args[0]).map_err(|e| format!("{}: {e}", args[0]))?;
    if meta.len() > settings.read.limits.max_file_bytes as u64 {
        return Err(format!(
            "{}: the file is larger than {} bytes (max_file_bytes)",
            args[0], settings.read.limits.max_file_bytes
        ));
    }
    let bytes = fs::read(&args[0]).map_err(|e| format!("{}: {e}", args[0]))?;
    let img = from_bytes(&bytes).map_err(|e| format!("{}: {e}", args[0]))?;
    let drawing = trace::trace(&img, width, &settings.trace, settings.read.limits)?;
    let planned = plan(&drawing, &settings)?;
    let points: usize = drawing.shapes.iter().map(|s| s.pts.len()).sum();
    print!(
        "{} contours of {} points from {} x {} pixels; ",
        drawing.shapes.len(),
        points,
        img.w,
        img.h
    );
    emit_plan(
        &args[0],
        (!dry_run).then(|| args[1].as_str()),
        &bytes,
        &planned,
        &settings,
        &sources,
    )
}

/// `layers`: what is in a drawing and what would be done with it.
fn layers_job(mut args: Vec<String>) -> Result<(), String> {
    let (settings, _) = drawing_settings(&mut args, LayerMode::Cut)?;
    finish(&args, 1)?;
    let path = args.first().ok_or("layers needs a drawing")?;
    let (_, drawing) = read_drawing(path, &settings)?;
    for l in layer_list(&drawing, &settings) {
        let p = settings.params_for(&l.key);
        println!(
            "{}: {} shapes ({} closed), {:.1} mm, {} at {} % and {} mm/min, {} pass(es){}",
            l.key,
            l.shapes,
            l.closed,
            l.length_mm,
            p.mode.word(),
            p.power_percent,
            p.speed_mm_min,
            p.passes,
            describe(settings.machine.laser_watts, &p)
        );
    }
    for note in &drawing.warnings {
        eprintln!("note: {note}");
    }
    Ok(())
}

/// `text`: a line of text, or several, burnt with the font of strokes of the program.
fn text_job(mut args: Vec<String>) -> Result<(), String> {
    let config = flag(&mut args, "--config")?;
    let mut settings = match &config {
        Some(path) => vconfig::load(Path::new(path))?,
        None => Settings::default(),
    };
    take_profile(&mut args, &mut settings, config.is_some())?;
    let mut machine = settings.machine;
    let mut t = settings.text;
    let words = flag(&mut args, "--text")?.ok_or("text needs --text TEXT")?;
    if let Some(v) = flag(&mut args, "--height-mm")? {
        t.height_mm = num(&v, "--height-mm")?;
    }
    if let Some(v) = flag(&mut args, "--spacing")? {
        t.spacing_percent = num(&v, "--spacing")?;
    }
    if let Some(v) = flag(&mut args, "--line-spacing")? {
        t.line_spacing_percent = num(&v, "--line-spacing")?;
    }
    if let Some(v) = flag(&mut args, "--power")? {
        t.power_percent = num(&v, "--power")?;
    }
    if let Some(v) = flag(&mut args, "--feed")? {
        t.speed_mm_min = num(&v, "--feed")?;
    }
    if let Some(v) = flag(&mut args, "--passes")? {
        t.passes = whole(&v, "--passes")?;
    }
    if let Some(v) = flag(&mut args, "--max-minutes")? {
        t.max_minutes = num(&v, "--max-minutes")?;
    }
    if let Some(v) = flag(&mut args, "--smax")? {
        machine.s_max = whole(&v, "--smax")?;
    }
    if let Some(v) = flag(&mut args, "--max-power")? {
        machine.max_power_percent = num(&v, "--max-power")?;
    }
    if let Some(v) = flag(&mut args, "--max-speed")? {
        machine.max_speed_mm_min = num(&v, "--max-speed")?;
    }
    if let Some(v) = flag(&mut args, "--origin")? {
        let o = list(&v, "--origin")?;
        if o.len() != 2 {
            return Err("--origin needs X,Y".into());
        }
        machine.origin = (o[0], o[1]);
    }
    if let Some(v) = flag(&mut args, "--bed")? {
        let b = list(&v, "--bed")?;
        if b.len() != 2 {
            return Err("--bed needs X,Y".into());
        }
        machine.bed_mm = Some((b[0], b[1]));
    }
    if let Some(v) = flag(&mut args, "--align")? {
        t.align = vconfig::TextAlign::from_word(&v)
            .ok_or_else(|| format!("--align is '{v}': it must be left, center or right"))?;
    }
    mirror_flags(&mut args, &mut t.mirror)?;
    finish(&args, 1)?;
    let out = args.first().ok_or("text needs an output file")?;
    if config
        .as_deref()
        .is_some_and(|c| same_file(Path::new(c), Path::new(out)))
    {
        return Err(format!(
            "{out}: the output would be written over the settings it is made from"
        ));
    }
    let m = laser_text::Machine {
        s_max: machine.s_max,
        max_power_percent: machine.max_power_percent,
        max_speed_mm_min: machine.max_speed_mm_min,
        bed_mm: machine.bed_mm,
    };
    let (gcode, l) = laser_text::program(&words, machine.origin, &t, &m)?;
    let head = header(&Header {
        generator: "phoenix-laser",
        version: env!("CARGO_PKG_VERSION"),
        source: None,
        tools: &format!("laser, S 0-{}, text with a font of strokes", machine.s_max),
    });
    write_atomic(Path::new(out), &format!("{head}{gcode}"))?;
    println!(
        "{} characters, {:.1} x {:.1} mm, {} strokes, at {} % and {} mm/min, written to {out}",
        words.chars().count(),
        l.width_mm,
        l.height_mm,
        l.strokes.len(),
        t.power_percent,
        t.speed_mm_min
    );
    Ok(())
}

/// `--mirror` and `--no-mirror` of `text` and `qr`: the option decides over the settings file, and the two together are an error.
fn mirror_flags(args: &mut Vec<String>, mirror: &mut bool) -> Result<(), String> {
    let (on, off) = (switch(args, "--mirror"), switch(args, "--no-mirror"));
    if on && off {
        return Err("--mirror and --no-mirror cannot be given together".into());
    }
    if on {
        *mirror = true;
    } else if off {
        *mirror = false;
    }
    Ok(())
}

/// `qr`: some data burnt as a QR code, filling its dark modules.
fn qr_job(mut args: Vec<String>) -> Result<(), String> {
    let config = flag(&mut args, "--config")?;
    let mut settings = match &config {
        Some(path) => vconfig::load(Path::new(path))?,
        None => Settings::default(),
    };
    take_profile(&mut args, &mut settings, config.is_some())?;
    let mut machine = settings.machine;
    let mut q = settings.qr;
    let data = flag(&mut args, "--text")?.ok_or("qr needs --text DATA")?;
    if let Some(v) = flag(&mut args, "--ecc")? {
        q.ecc = vconfig::QrEcc::from_word(&v)
            .ok_or_else(|| format!("--ecc is '{v}': it must be L, M, Q or H"))?;
    }
    if let Some(v) = flag(&mut args, "--module-mm")? {
        q.module_mm = num(&v, "--module-mm")?;
    }
    if let Some(v) = flag(&mut args, "--line-mm")? {
        q.line_mm = num(&v, "--line-mm")?;
    }
    if let Some(v) = flag(&mut args, "--power")? {
        q.power_percent = num(&v, "--power")?;
    }
    if let Some(v) = flag(&mut args, "--feed")? {
        q.speed_mm_min = num(&v, "--feed")?;
    }
    if let Some(v) = flag(&mut args, "--passes")? {
        q.passes = whole(&v, "--passes")?;
    }
    if let Some(v) = flag(&mut args, "--max-minutes")? {
        q.max_minutes = num(&v, "--max-minutes")?;
    }
    if let Some(v) = flag(&mut args, "--border-modules")? {
        q.border_modules = whole(&v, "--border-modules")?;
    }
    if let Some(v) = flag(&mut args, "--smax")? {
        machine.s_max = whole(&v, "--smax")?;
    }
    if let Some(v) = flag(&mut args, "--max-power")? {
        machine.max_power_percent = num(&v, "--max-power")?;
    }
    if let Some(v) = flag(&mut args, "--max-speed")? {
        machine.max_speed_mm_min = num(&v, "--max-speed")?;
    }
    if let Some(v) = flag(&mut args, "--origin")? {
        let o = list(&v, "--origin")?;
        if o.len() != 2 {
            return Err("--origin needs X,Y".into());
        }
        machine.origin = (o[0], o[1]);
    }
    if let Some(v) = flag(&mut args, "--bed")? {
        let b = list(&v, "--bed")?;
        if b.len() != 2 {
            return Err("--bed needs X,Y".into());
        }
        machine.bed_mm = Some((b[0], b[1]));
    }
    if let Some(v) = flag(&mut args, "--overscan-mm")? {
        q.overscan_mm = num(&v, "--overscan-mm")?;
    }
    mirror_flags(&mut args, &mut q.mirror)?;
    let (invert, plain) = (
        switch(&mut args, "--invert"),
        switch(&mut args, "--no-invert"),
    );
    if invert && plain {
        return Err("--invert and --no-invert cannot be given together".into());
    }
    if invert {
        q.invert = true;
    } else if plain {
        q.invert = false;
    }
    let show = switch(&mut args, "--print");
    finish(&args, 1)?;
    let out = args.first().ok_or("qr needs an output file")?;
    if config
        .as_deref()
        .is_some_and(|c| same_file(Path::new(c), Path::new(out)))
    {
        return Err(format!(
            "{out}: the output would be written over the settings it is made from"
        ));
    }
    let m = qr::Machine {
        s_max: machine.s_max,
        max_power_percent: machine.max_power_percent,
        max_speed_mm_min: machine.max_speed_mm_min,
        bed_mm: machine.bed_mm,
    };
    let (gcode, sym) = qr::program(&data, machine.origin, &q, &m)?;
    let head = header(&Header {
        generator: "phoenix-laser",
        version: env!("CARGO_PKG_VERSION"),
        source: None,
        tools: &format!("laser, S 0-{}, QR code filled with lines", machine.s_max),
    });
    write_atomic(Path::new(out), &format!("{head}{gcode}"))?;
    println!(
        "{} bytes, version {} ({} x {} modules, level {}), {:.1} x {:.1} mm, at {} % and {} mm/min, written to {out}",
        data.len(),
        sym.version,
        sym.size,
        sym.size,
        sym.ecc.word(),
        qr::side_modules(&sym, &q) as f64 * q.module_mm,
        qr::side_modules(&sym, &q) as f64 * q.module_mm,
        q.power_percent,
        q.speed_mm_min
    );
    if show {
        print!("{}", sym.to_text());
    }
    Ok(())
}

/// `calibrate`: the grid of squares of power against feed, from lists or spread around what a material of the library says, with the figures of each column and row burnt next to it when
/// they are asked for.
fn calibrate_job(mut args: Vec<String>) -> Result<(), String> {
    let material = take_material(&mut args)?;
    let powers_arg = flag(&mut args, "--powers")?;
    let feeds_arg = flag(&mut args, "--feeds")?;
    let steps_arg = flag(&mut args, "--steps")?;
    let spread_arg = flag(&mut args, "--spread")?;
    let label_mm_arg = flag(&mut args, "--label-mm")?;
    let (labels_on, labels_off) = (
        switch(&mut args, "--labels"),
        switch(&mut args, "--no-labels"),
    );
    if labels_on && labels_off {
        return Err("--labels and --no-labels cannot be given together".into());
    }
    let limits = machine_limits(&mut args)?;
    let (max_power, max_speed, config) =
        (limits.max_power, limits.max_speed, limits.config.clone());
    let mut c = limits.calibrate;
    if let Some(v) = &steps_arg {
        c.steps = whole(v, "--steps")?;
    }
    if let Some(v) = &spread_arg {
        c.spread_percent = num(v, "--spread")?;
    }
    if let Some(v) = &label_mm_arg {
        c.label_mm = num(v, "--label-mm")?;
    }
    // the figures come by default with a material, and with nothing else unless they are asked for
    c.labels = labels_on || (c.labels && material.is_some() && !labels_off);
    c.validate()?;
    let mut g = Grid::default();
    if let Some(s_max) = limits.s_max {
        g.s_max = s_max;
    }
    if let Some(v) = flag(&mut args, "--smax")? {
        g.s_max = whole(&v, "--smax")?;
    }
    // the powers and the feeds: the lists that were given, or a ladder around what the material says
    let mut spread_used = false;
    let mut mid: (Option<f64>, Option<f64>) = (None, None);
    let powers = match (&powers_arg, &material) {
        (Some(v), _) => list(v, "--powers")?,
        (None, Some((m, _))) => {
            let centre = m
                .power
                .ok_or_else(|| format!("the material {} has no power: give --powers", m.name))?;
            safety::check_power("the power of the material", centre, max_power)?;
            spread_used = true;
            mid.0 = Some(centre);
            ladder(
                centre,
                c.steps as usize,
                c.spread_percent,
                1,
                c.min_power_percent.min(max_power),
                max_power,
            )?
        }
        (None, None) => {
            return Err(
                "--powers is needed (or --material, to spread them around its power)".into(),
            )
        }
    };
    let feeds = match (&feeds_arg, &material) {
        (Some(v), _) => list(v, "--feeds")?,
        (None, Some((m, _))) => {
            let centre = m
                .speed
                .ok_or_else(|| format!("the material {} has no speed: give --feeds", m.name))?;
            safety::check_speed("the speed of the material", centre, max_speed)?;
            spread_used = true;
            mid.1 = Some(centre);
            ladder(
                centre,
                c.steps as usize,
                c.spread_percent,
                0,
                c.min_feed_mm_min.min(max_speed),
                max_speed,
            )?
        }
        (None, None) => {
            return Err("--feeds is needed (or --material, to spread them around its speed)".into())
        }
    };
    finish(&args, 1)?;
    let out = args.first().ok_or("calibrate needs an output file")?;
    if (steps_arg.is_some() || spread_arg.is_some()) && !spread_used {
        return Err("--steps and --spread need --material: they spread the powers and the feeds around what it says".into());
    }
    if powers_arg.is_some() {
        for p in &powers {
            safety::check_power("a power of --powers", *p, max_power)?;
        }
    }
    if feeds_arg.is_some() {
        for f in &feeds {
            safety::check_speed("a speed of --feeds", *f, max_speed)?;
        }
    }
    let inputs = config.iter().chain(material.iter().map(|(_, f)| f));
    for input in inputs {
        if same_file(Path::new(input), Path::new(out)) {
            return Err(format!(
                "{out}: the output would be written over the file it is made from"
            ));
        }
    }
    let program = if c.labels {
        // the figures are burnt with what the material says, or with the middle of the lists
        let labels = Labels {
            height_mm: c.label_mm,
            power_percent: mid.0.unwrap_or(powers[powers.len() / 2]),
            feed_mm_min: mid.1.unwrap_or(feeds[feeds.len() / 2]),
        };
        safety::check_power("the power of the figures", labels.power_percent, max_power)?;
        safety::check_speed("the speed of the figures", labels.feed_mm_min, max_speed)?;
        labelled_grid(&powers, &feeds, &g, &labels)?
    } else {
        grid(&powers, &feeds, &g)?
    };
    let head = header(&Header {
        generator: "phoenix-laser",
        version: env!("CARGO_PKG_VERSION"),
        source: None,
        tools: &format!(
            "laser, S 0-{}, grid of power against feed{}",
            g.s_max,
            if c.labels { " with figures" } else { "" }
        ),
    });
    write_atomic(Path::new(out), &format!("{head}{program}"))?;
    println!("{} squares written to {out}", powers.len() * feeds.len());
    if let Some((m, _)) = &material {
        let join = |v: &[f64]| {
            v.iter()
                .map(|x| format!("{x}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        if spread_used {
            println!(
                "material {}: powers {} %, feeds {} mm/min",
                m.name,
                join(&powers),
                join(&feeds)
            );
        }
    }
    Ok(())
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err("this build has no window: build it with --features gui".into());
    }
    match cmd.as_str() {
        "engrave" => {
            let width = flag(&mut args, "--width-mm")?
                .map(|v| num(&v, "--width-mm"))
                .transpose()?
                .ok_or("--width-mm is needed")?;
            let limits = machine_limits(&mut args)?;
            let (max_power, config) = (limits.max_power, limits.config.clone());
            let material = take_material(&mut args)?;
            let mut p = Params::default();
            if let Some(s_max) = limits.s_max {
                p.s_max = s_max;
            }
            // the material goes under the options of the command line
            let mut tone_base = limits.tone;
            if let Some((m, _)) = &material {
                m.apply_to_engraving(&mut p);
                m.apply_to_tone(&mut tone_base);
            }
            if let Some(v) = flag(&mut args, "--dot-mm")? {
                p.pixel_mm = num(&v, "--dot-mm")?;
            }
            if let Some(v) = flag(&mut args, "--feed")? {
                p.feed_mm_min = num(&v, "--feed")?;
            }
            if let Some(v) = flag(&mut args, "--power")? {
                p.power_percent = num(&v, "--power")?;
            }
            if let Some(v) = flag(&mut args, "--smax")? {
                p.s_max = whole(&v, "--smax")?;
            }
            if let Some(v) = flag(&mut args, "--origin")? {
                let o = list(&v, "--origin")?;
                if o.len() != 2 {
                    return Err("--origin needs X,Y".into());
                }
                p.origin = (o[0], o[1]);
            }
            let accel = flag(&mut args, "--accel")?
                .map(|v| num(&v, "--accel"))
                .transpose()?
                .unwrap_or(1000.0);
            match flag(&mut args, "--overscan")?.as_deref() {
                Some("auto") => p.overscan_mm = overscan_for(p.feed_mm_min, accel)?,
                Some(v) => p.overscan_mm = num(v, "--overscan")?,
                None => {}
            }
            let material_threshold = material
                .as_ref()
                .and_then(|(m, _)| m.threshold)
                .map_or(128.0, f64::from);
            let threshold = flag(&mut args, "--threshold")?
                .map(|v| num(&v, "--threshold"))
                .transpose()?
                .unwrap_or(material_threshold)
                .clamp(0.0, 255.0) as u8;
            let word = flag(&mut args, "--dither")?
                .or_else(|| material.as_ref().and_then(|(m, _)| m.dither.clone()))
                .unwrap_or_else(|| "fs".to_string());
            let method = Method::from_word(&word, threshold).ok_or_else(|| {
                format!("--dither '{word}' is not fs, atkinson, threshold or bayer")
            })?;
            let tone = take_tone(&mut args, tone_base)?;
            let dry_run = switch(&mut args, "--dry-run");
            let needed = if dry_run { 1 } else { 2 };
            if args.len() < needed {
                return Err(if dry_run {
                    "engrave needs an image".into()
                } else {
                    "engrave needs an image and an output file".into()
                });
            }
            finish(&args, 2)?;
            if !dry_run && same_file(Path::new(&args[0]), Path::new(&args[1])) {
                return Err(format!(
                    "{}: the output would be written over the image it is made from",
                    args[1]
                ));
            }
            let bytes = fs::read(&args[0]).map_err(|e| format!("{}: {e}", args[0]))?;
            let img = from_bytes(&bytes)?;
            if !(width.is_finite() && width > 0.0 && width <= 2000.0) {
                return Err("--width-mm must be from 0 to 2000".into());
            }
            p.validate()?;
            safety::check_power("--power", p.power_percent, max_power)?;
            safety::check_speed("--feed", p.feed_mm_min, limits.max_speed)?;
            if !dry_run
                && config
                    .iter()
                    .chain(material.iter().map(|(_, f)| f))
                    .any(|c| same_file(Path::new(c), Path::new(&args[1])))
            {
                return Err(format!(
                    "{}: the output would be written over the settings it is made from",
                    args[1]
                ));
            }
            let w = (width / p.pixel_mm).round().max(1.0) as usize;
            let h = ((w as f64) * img.h as f64 / img.w as f64).round().max(1.0) as usize;
            let dots = dither(&tone.apply(&img.resize(w, h)?)?, method);
            let e = engrave(&dots, &p)?;
            // a dry run works the engraving out and counts it, and writes nothing
            if dry_run {
                print!("dry run, nothing written: ");
            } else {
                // the standard header: the generator, the units, the picture it was made from with its hash, and the laser
                let name = std::path::Path::new(&args[0])
                    .file_name()
                    .map_or(args[0].clone(), |n| n.to_string_lossy().into_owned());
                let tools = format!("laser, S 0-{} ({} %)", p.s_max, p.power_percent);
                let head = header(&Header {
                    generator: "phoenix-laser",
                    version: env!("CARGO_PKG_VERSION"),
                    source: Some((&name, sha256_hex(&bytes))),
                    tools: &tools,
                });
                write_atomic(Path::new(&args[1]), &format!("{head}{}", e.gcode))?;
            }
            println!("{} x {} dots ({:.1} x {:.1} mm), {} lines, {} runs, {:.0} mm burned, about {:.0} s", dots.w, dots.h, dots.w as f64 * p.pixel_mm, dots.h as f64 * p.pixel_mm, e.rows, e.runs, e.burn_mm, e.seconds);
            Ok(())
        }
        "calibrate" => calibrate_job(args),
        "text" => text_job(args),
        "qr" => qr_job(args),
        "cut" => drawing_job(args, LayerMode::Cut),
        "vector" => drawing_job(args, LayerMode::Engrave),
        "layers" => layers_job(args),
        "trace" => trace_job(args),
        "focus" => {
            let config = flag(&mut args, "--config")?;
            let mut settings = Settings::default();
            if let Some(path) = &config {
                settings = vconfig::load(Path::new(path))?;
            }
            take_profile(&mut args, &mut settings, config.is_some())?;
            let mut machine = settings.machine;
            let mut f = settings.focus;
            if let Some(v) = flag(&mut args, "--max-power")? {
                machine.max_power_percent = num(&v, "--max-power")?;
            }
            if let Some(v) = flag(&mut args, "--max-speed")? {
                machine.max_speed_mm_min = num(&v, "--max-speed")?;
            }
            if let Some(v) = flag(&mut args, "--smax")? {
                machine.s_max = whole(&v, "--smax")?;
            }
            if let Some(v) = flag(&mut args, "--z-step")? {
                f.z_step_mm = num(&v, "--z-step")?;
            }
            if let Some(v) = flag(&mut args, "--z-start")? {
                f.z_start_mm = num(&v, "--z-start")?;
            }
            if let Some(v) = flag(&mut args, "--steps")? {
                f.steps = whole(&v, "--steps")?;
            }
            if let Some(v) = flag(&mut args, "--length")? {
                f.length_mm = num(&v, "--length")?;
            }
            if let Some(v) = flag(&mut args, "--spacing")? {
                f.spacing_mm = num(&v, "--spacing")?;
            }
            if let Some(v) = flag(&mut args, "--feed")? {
                f.feed_mm_min = num(&v, "--feed")?;
            }
            if let Some(v) = flag(&mut args, "--power")? {
                f.power_percent = num(&v, "--power")?;
            }
            finish(&args, 1)?;
            let out = args.first().ok_or("focus needs an output file")?;
            safety::validate_max(machine.max_power_percent)?;
            safety::check_speed(
                "the speed of the focus test",
                f.feed_mm_min,
                machine.max_speed_mm_min,
            )?;
            let text = focus_test(&f, machine.s_max, machine.max_power_percent)?;
            if config
                .as_deref()
                .is_some_and(|c| same_file(Path::new(c), Path::new(out)))
            {
                return Err(format!(
                    "{out}: the output would be written over the settings it is made from"
                ));
            }
            let head = header(&Header {
                generator: "phoenix-laser",
                version: env!("CARGO_PKG_VERSION"),
                source: None,
                tools: &format!(
                    "laser, S 0-{}, focus test (the head moves in Z)",
                    machine.s_max
                ),
            });
            write_atomic(Path::new(out), &format!("{head}{text}"))?;
            eprintln!("note: this program moves Z: check which way is down before running it");
            println!(
                "{} lines of {} mm from Z{} to Z{}, at {} % and {} mm/min, written to {out}",
                f.steps,
                f.length_mm,
                f.z_start_mm,
                f.z_start_mm + f.z_step_mm * f64::from(f.steps - 1),
                f.power_percent,
                f.feed_mm_min
            );
            Ok(())
        }
        "frame" => {
            let config = flag(&mut args, "--config")?;
            let mut settings = Settings::default();
            if let Some(path) = &config {
                settings = vconfig::load(Path::new(path))?;
            }
            take_profile(&mut args, &mut settings, config.is_some())?;
            let mut machine = settings.machine;
            let mut fr = settings.frame;
            if let Some(v) = flag(&mut args, "--max-power")? {
                machine.max_power_percent = num(&v, "--max-power")?;
            }
            if let Some(v) = flag(&mut args, "--max-speed")? {
                machine.max_speed_mm_min = num(&v, "--max-speed")?;
            }
            if let Some(v) = flag(&mut args, "--smax")? {
                machine.s_max = whole(&v, "--smax")?;
            }
            if let Some(v) = flag(&mut args, "--bed")? {
                let b = list(&v, "--bed")?;
                if b.len() != 2 {
                    return Err("--bed needs X,Y".into());
                }
                machine.bed_mm = Some((b[0], b[1]));
            }
            let origin = flag(&mut args, "--origin")?
                .map(|v| {
                    let o = list(&v, "--origin")?;
                    if o.len() == 2 {
                        Ok((o[0], o[1]))
                    } else {
                        Err("--origin needs X,Y".to_string())
                    }
                })
                .transpose()?;
            if let Some(v) = flag(&mut args, "--margin")? {
                fr.margin_mm = num(&v, "--margin")?;
            }
            if let Some(v) = flag(&mut args, "--feed")? {
                fr.feed_mm_min = num(&v, "--feed")?;
            }
            if let Some(v) = flag(&mut args, "--laps")? {
                fr.laps = whole(&v, "--laps")?;
            }
            if let Some(v) = flag(&mut args, "--pilot")? {
                fr.pilot_percent = num(&v, "--pilot")?;
            }
            let confirmed = switch(&mut args, "--confirm-laser-on");
            let size = flag(&mut args, "--size")?;
            let source = flag(&mut args, "--gcode")?;
            finish(&args, 1)?;
            let out = args.first().ok_or("frame needs an output file")?;
            safety::validate_max(machine.max_power_percent)?;
            safety::check_speed(
                "the speed of the frame",
                fr.feed_mm_min,
                machine.max_speed_mm_min,
            )?;
            let pilot = safety::pilot_percent(
                fr.pilot_percent,
                confirmed,
                machine.max_power_percent,
                fr.max_pilot_percent,
            )?;
            let (lo, hi) = match (&size, &source) {
                (Some(v), None) => {
                    let wh = list(v, "--size")?;
                    if wh.len() != 2 || wh.iter().any(|d| *d <= 0.0) {
                        return Err("--size needs W,H, both more than 0".into());
                    }
                    let lo = origin.unwrap_or(machine.origin);
                    (lo, (lo.0 + wh[0], lo.1 + wh[1]))
                }
                (None, Some(path)) => {
                    if origin.is_some() {
                        return Err(
                            "--origin does nothing with --gcode: the frame is where the work is"
                                .into(),
                        );
                    }
                    let meta = fs::metadata(path).map_err(|e| format!("{path}: {e}"))?;
                    if meta.len() > MAX_PROGRAM_BYTES {
                        return Err(format!(
                            "{path}: the program is larger than {MAX_PROGRAM_BYTES} bytes"
                        ));
                    }
                    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
                    let program = preview::read(&String::from_utf8_lossy(&bytes));
                    let report = preview::analyse(&program, &preview::Options::default())?;
                    report
                        .burn_bounds
                        .or(report.bounds)
                        .ok_or_else(|| format!("{path}: the program has no move in the plane"))?
                }
                _ => return Err("frame needs one of --size W,H or --gcode PROGRAM.nc".into()),
            };
            let (lo, hi) = frame::grow(lo, hi, fr.margin_mm);
            let pilot_s = frame::pilot_value(machine.s_max, pilot)?;
            let text = frame::gcode(
                &Frame {
                    lo,
                    hi,
                    feed_mm_min: fr.feed_mm_min,
                    laps: fr.laps,
                    s_max: machine.s_max,
                    pilot_s,
                },
                machine.bed_mm,
            )?;
            let out_path = Path::new(out);
            if source
                .iter()
                .chain(config.iter())
                .any(|f| same_file(Path::new(f), out_path))
            {
                return Err(format!(
                    "{out}: the frame would be written over a file it is made from"
                ));
            }
            let tools = if pilot_s > 0 {
                format!(
                    "laser, S 0-{}, frame of the work with a pilot power of S{pilot_s}",
                    machine.s_max
                )
            } else {
                "laser OFF, frame of the work".to_string()
            };
            let head = header(&Header {
                generator: "phoenix-laser",
                version: env!("CARGO_PKG_VERSION"),
                source: None,
                tools: &tools,
            });
            write_atomic(out_path, &format!("{head}{text}"))?;
            println!(
                "frame of {:.1} x {:.1} mm at {:.1},{:.1}, {} lap(s), {}",
                hi.0 - lo.0,
                hi.1 - lo.1,
                lo.0,
                lo.1,
                fr.laps,
                if pilot_s > 0 {
                    format!("pilot power S{pilot_s} ({pilot} %): the laser is ON, keep eyes and material safe")
                } else {
                    "laser off".to_string()
                }
            );
            Ok(())
        }
        "preview" => {
            let mut o = preview::Options::default();
            let config = flag(&mut args, "--config")?;
            if let Some(path) = &config {
                let mut s = vconfig::load(Path::new(path))?;
                take_profile(&mut args, &mut s, true)?;
                o.rapid_mm_min = s.machine.rapid_mm_min;
                o.bed_mm = s.machine.bed_mm;
                o.s_max = Some(s.machine.s_max);
            } else {
                take_profile(&mut args, &mut Settings::default(), false)?;
            }
            if let Some(v) = flag(&mut args, "--accel")? {
                o.accel_mm_s2 = num(&v, "--accel")?;
            }
            if let Some(v) = flag(&mut args, "--rapid")? {
                o.rapid_mm_min = num(&v, "--rapid")?;
            }
            if let Some(v) = flag(&mut args, "--smax")? {
                o.s_max = Some(whole(&v, "--smax")?);
            }
            if let Some(v) = flag(&mut args, "--bed")? {
                let b = list(&v, "--bed")?;
                if b.len() != 2 {
                    return Err("--bed needs X,Y".into());
                }
                o.bed_mm = Some((b[0], b[1]));
            }
            let width_px = flag(&mut args, "--width-px")?
                .map(|v| num(&v, "--width-px"))
                .transpose()?
                .unwrap_or(900.0);
            finish(&args, 2)?;
            let input = args.first().ok_or("preview needs a G-code file")?;
            let meta = fs::metadata(input).map_err(|e| format!("{input}: {e}"))?;
            if meta.len() > MAX_PROGRAM_BYTES {
                return Err(format!(
                    "{input}: the program is larger than {MAX_PROGRAM_BYTES} bytes"
                ));
            }
            let bytes = fs::read(input).map_err(|e| format!("{input}: {e}"))?;
            let program = preview::read(&String::from_utf8_lossy(&bytes));
            let r = preview::analyse(&program, &o)?;
            if r.moves == 0 {
                return Err(format!("{input}: the program has no move in the plane"));
            }
            if let Some(out) = args.get(1) {
                let out_path = Path::new(out);
                if same_file(Path::new(input), out_path)
                    || config
                        .as_deref()
                        .is_some_and(|c| same_file(Path::new(c), out_path))
                {
                    return Err(format!(
                        "{out}: the picture would be written over a file it is made from"
                    ));
                }
                let name = Path::new(input)
                    .file_name()
                    .map_or(input.clone(), |n| n.to_string_lossy().into_owned());
                write_atomic(out_path, &preview::svg(&program, &r, &o, &name, width_px)?)?;
                println!("picture written to {out}");
            }
            for n in &r.notes {
                eprintln!("note: {n}");
            }
            let boxed = |b: Option<((f64, f64), (f64, f64))>| {
                b.map_or("none".to_string(), |(lo, hi)| {
                    format!(
                        "{:.1} x {:.1} mm at {:.1},{:.1}",
                        hi.0 - lo.0,
                        hi.1 - lo.1,
                        lo.0,
                        lo.1
                    )
                })
            };
            println!(
                "{} moves, {:.0} mm burned, {:.0} mm of travel, power S{} to S{}, burns in {}, work in {}, about {}",
                r.moves,
                r.burn_mm,
                r.travel_mm,
                r.min_power,
                r.max_power,
                boxed(r.burn_bounds),
                boxed(r.bounds),
                preview::clock(r.seconds)
            );
            Ok(())
        }
        "verify" => {
            let json = switch(&mut args, "--json");
            let config = flag(&mut args, "--config")?;
            let mut settings = match &config {
                Some(path) => vconfig::load(Path::new(path))?,
                None => Settings::default(),
            };
            take_profile(&mut args, &mut settings, config.is_some())?;
            let m = settings.machine;
            let mut l = verify::Limits {
                s_max: m.s_max,
                max_power_percent: m.max_power_percent,
                max_speed_mm_min: m.max_speed_mm_min,
                bed_mm: m.bed_mm,
                accel_mm_s2: 1000.0,
                rapid_mm_min: m.rapid_mm_min,
                overscan_mm: settings.verify.overscan_mm,
            };
            if let Some(v) = flag(&mut args, "--smax")? {
                l.s_max = whole(&v, "--smax")?;
            }
            if let Some(v) = flag(&mut args, "--max-power")? {
                l.max_power_percent = num(&v, "--max-power")?;
            }
            if let Some(v) = flag(&mut args, "--max-speed")? {
                l.max_speed_mm_min = num(&v, "--max-speed")?;
            }
            if let Some(v) = flag(&mut args, "--accel")? {
                l.accel_mm_s2 = num(&v, "--accel")?;
            }
            if let Some(v) = flag(&mut args, "--bed")? {
                let b = list(&v, "--bed")?;
                if b.len() != 2 {
                    return Err("--bed needs X,Y".into());
                }
                l.bed_mm = Some((b[0], b[1]));
            }
            if let Some(v) = flag(&mut args, "--overscan-mm")? {
                l.overscan_mm = num(&v, "--overscan-mm")?;
            }
            // the SHA-256 of the program that was authorised, when the person gives it
            let expected = match flag(&mut args, "--expect-sha256")? {
                Some(v) => {
                    let want = v.trim().to_ascii_lowercase();
                    if want.len() != 64 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Err("--expect-sha256 needs the 64 hex digits of a SHA-256".into());
                    }
                    Some(want)
                }
                None => None,
            };
            finish(&args, 1)?;
            let input = args.first().ok_or("verify needs a G-code file")?;
            let meta = fs::metadata(input).map_err(|e| format!("{input}: {e}"))?;
            if meta.len() > MAX_PROGRAM_BYTES {
                return Err(format!(
                    "{input}: the program is larger than {MAX_PROGRAM_BYTES} bytes"
                ));
            }
            let bytes = fs::read(input).map_err(|e| format!("{input}: {e}"))?;
            let mut v = verify::verify(&bytes, &l)?;
            if let Some(want) = expected.filter(|w| *w != v.sha256) {
                v.problems.push(format!(
                    "the program is not the one that was authorised: its SHA-256 is {}, not {want}",
                    v.sha256
                ));
            }
            if json {
                // the profile and the settings file the limits came from are part of the record
                let provenance = verify::Provenance {
                    profile: settings.profile.clone(),
                    config_sha256: match &config {
                        Some(path) => Some(sha256_hex(
                            &fs::read(path).map_err(|e| format!("{path}: {e}"))?,
                        )),
                        None => None,
                    },
                };
                print!("{}", v.json_with(&l, &provenance));
            } else {
                print!("{}", v.text());
            }
            if v.passes() {
                Ok(())
            } else {
                Err(format!(
                    "{input}: the program fails {} check(s)",
                    v.problems.len()
                ))
            }
        }
        "materials" => {
            finish(&args, 1)?;
            let path = args.first().ok_or("materials needs a library file")?;
            let lib = materials::load(Path::new(path))?;
            for m in &lib.materials {
                println!("{}: {}", m.name, m.describe());
            }
            println!("{} material(s) in {path}", lib.materials.len());
            Ok(())
        }
        "info" => {
            finish(&args, 1)?;
            let path = args.first().ok_or("info needs an image")?;
            let img = from_bytes(&fs::read(path).map_err(|e| format!("{path}: {e}"))?)?;
            let mean =
                img.data.iter().map(|&v| u64::from(v)).sum::<u64>() as f64 / img.data.len() as f64;
            println!("{} x {} pixels, mean grey {:.1} of 255", img.w, img.h, mean);
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

/// Whether the program opens its window instead of running a command, and the picture to open in it: no arguments, `gui` (with a picture or without), or a single picture given by the
/// system to open (a name with an extension that is a file; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Option<String>> {
    match args {
        [] => Some(None),
        [a] if a == "gui" => Some(None),
        [a, f] if a == "gui" => Some(Some(f.clone())),
        [f] if !f.starts_with('-')
            && std::path::Path::new(f).extension().is_some()
            && std::path::Path::new(f).is_file() =>
        {
            Some(Some(f.clone()))
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(picture: Option<String>) -> ExitCode {
    let lang = eh3d_phoenix_laser::lang::Lang::from_env();
    match eh3d_phoenix_laser::gui::run(picture.map(std::path::PathBuf::from), lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_phoenix_laser::shot::parse_window_args(args, 1) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_phoenix_laser::lang::Lang::from_env);
    match eh3d_phoenix_laser::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_phoenix_laser::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
    #[cfg(feature = "gui")]
    if let Some(picture) = window_request(&args) {
        return open_window(picture);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
