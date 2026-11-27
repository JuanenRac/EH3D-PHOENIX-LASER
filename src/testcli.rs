// EH3D-PHOENIX-LASER - the commands interval, kerf, controller and explain.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Two test programs (the spacing of the lines and the width of the beam), the comparison of a saved listing of the settings of the controller, and the meaning of its alarms. A test program
//! is written only when it passes the check of the machine of the settings. Nothing here talks to a machine.

use crate::cli_args::{finish, flag, num, switch};
use crate::{take_profile, whole};
use eh3d_phoenix_laser::calibrate::{interval_test, kerf_from_gap, kerf_test, series, Grid, Kerf};
use eh3d_phoenix_laser::gcode_out::{header, Header};
use eh3d_phoenix_laser::grblcodes;
use eh3d_phoenix_laser::grblset;
use eh3d_phoenix_laser::lang::Lang;
use eh3d_phoenix_laser::vconfig::{self, Settings};
use eh3d_phoenix_laser::verify;
use eh3d_phoenix_laser::window::{same_file, write_atomic};
use std::fs;
use std::path::Path;

/// The settings of `--config`, with the machine of `--profile` and the limits of the command line over them.
fn machine(args: &mut Vec<String>) -> Result<(Settings, Option<String>), String> {
    let config = flag(args, "--config")?;
    let mut s = match &config {
        Some(path) => vconfig::load(Path::new(path))?,
        None => Settings::default(),
    };
    take_profile(args, &mut s, config.is_some())?;
    if let Some(v) = flag(args, "--smax")? {
        s.machine.s_max = whole(&v, "--smax")?;
    }
    if let Some(v) = flag(args, "--max-power")? {
        s.machine.max_power_percent = num(&v, "--max-power")?;
    }
    if let Some(v) = flag(args, "--max-speed")? {
        s.machine.max_speed_mm_min = num(&v, "--max-speed")?;
    }
    if let Some(v) = flag(args, "--bed")? {
        let (x, y) = v.split_once(',').ok_or("--bed needs X,Y")?;
        s.machine.bed_mm = Some((num(x, "--bed")?, num(y, "--bed")?));
    }
    Ok((s, config))
}

fn limits(s: &Settings) -> verify::Limits {
    let m = s.machine;
    verify::Limits {
        s_max: m.s_max,
        max_power_percent: m.max_power_percent,
        max_speed_mm_min: m.max_speed_mm_min,
        bed_mm: m.bed_mm,
        accel_mm_s2: 1000.0,
        rapid_mm_min: m.rapid_mm_min,
        overscan_mm: s.verify.overscan_mm,
        arc_tolerance_mm: s.verify.arc_tolerance_mm,
    }
}

/// Writes a test program after the check of the machine, never over the settings.
fn write_checked(
    out: &str,
    config: &Option<String>,
    s: &Settings,
    program: &str,
    tools: &str,
) -> Result<(), String> {
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
        tools,
    });
    let text = format!("{head}{program}");
    let v = verify::verify_with(text.as_bytes(), &limits(s), &verify::guard_of(s))?;
    if !v.passes() {
        return Err(format!(
            "the test program was not written: {}",
            v.problems.join("; ")
        ));
    }
    write_atomic(Path::new(out), &text)
}

/// The power of a test, held by the limit of power of the machine.
fn required(args: &mut Vec<String>, name: &str) -> Result<f64, String> {
    num(
        &flag(args, name)?.ok_or_else(|| format!("{name} is needed"))?,
        name,
    )
}

pub fn interval_job(mut args: Vec<String>) -> Result<(), String> {
    let (s, config) = machine(&mut args)?;
    let spacings = series(
        &flag(&mut args, "--spacings")?.ok_or("--spacings is needed")?,
        "--spacings",
    )?;
    let power = required(&mut args, "--power")?;
    let feed = required(&mut args, "--feed")?;
    let mut g = Grid {
        s_max: s.machine.s_max,
        ..Grid::default()
    };
    if let Some(v) = flag(&mut args, "--square-mm")? {
        g.square_mm = num(&v, "--square-mm")?;
    }
    if let Some(v) = flag(&mut args, "--gap-mm")? {
        g.gap_mm = num(&v, "--gap-mm")?;
    }
    let label = flag(&mut args, "--label-mm")?
        .map(|v| num(&v, "--label-mm"))
        .transpose()?;
    let no_labels = switch(&mut args, "--no-labels");
    if no_labels && label.is_some() {
        return Err("--label-mm and --no-labels cannot be given together".into());
    }
    let label = (!no_labels).then_some(label.unwrap_or(2.0));
    finish(&args, 1)?;
    let out = args.first().ok_or("interval needs an output file")?;
    let program = interval_test(&spacings, power, feed, &g, label)?;
    write_checked(
        out,
        &config,
        &s,
        &program,
        &format!("laser, S 0-{}, test of the spacing of the lines", g.s_max),
    )?;
    println!("{} squares written to {out}", spacings.len());
    Ok(())
}

pub fn kerf_job(mut args: Vec<String>) -> Result<(), String> {
    let strips = flag(&mut args, "--strips")?
        .map(|v| whole(&v, "--strips"))
        .transpose()?
        .unwrap_or(10);
    if let Some(v) = flag(&mut args, "--gap")? {
        let gap = num(&v, "--gap")?;
        finish(&args, 0)?;
        println!(
            "width of the beam: {:.3} mm (a gap of {gap} mm over {} cuts)",
            kerf_from_gap(gap, strips)?,
            strips + 1
        );
        return Ok(());
    }
    let (s, config) = machine(&mut args)?;
    let mut k = Kerf {
        strips,
        ..Kerf::default()
    };
    k.power_percent = required(&mut args, "--power")?;
    k.feed_mm_min = required(&mut args, "--feed")?;
    if let Some(v) = flag(&mut args, "--strip-mm")? {
        k.strip_mm = num(&v, "--strip-mm")?;
    }
    if let Some(v) = flag(&mut args, "--height-mm")? {
        k.height_mm = num(&v, "--height-mm")?;
    }
    if let Some(v) = flag(&mut args, "--passes")? {
        k.passes = whole(&v, "--passes")?;
    }
    finish(&args, 1)?;
    let out = args.first().ok_or("kerf needs an output file")?;
    let program = kerf_test(&k, s.machine.s_max)?;
    write_checked(
        out,
        &config,
        &s,
        &program,
        &format!(
            "laser, S 0-{}, test of the width of the beam",
            s.machine.s_max
        ),
    )?;
    println!(
        "{} strips written to {out}; push them to one side, measure the gap and run: phoenix-laser kerf --gap MM --strips {}",
        k.strips, k.strips
    );
    Ok(())
}

fn language(args: &mut Vec<String>) -> Result<Lang, String> {
    match flag(args, "--lang")? {
        Some(code) => Lang::from_code(&code).ok_or_else(|| {
            format!("--lang '{code}' is not one of en, es, de, fr, it, ja, zh or pt")
        }),
        None => Ok(Lang::from_env()),
    }
}

/// Reads a saved listing of the settings of the controller.
pub fn read_dump(path: &str) -> Result<grblset::Dump, String> {
    let meta = fs::metadata(path).map_err(|e| format!("{path}: {e}"))?;
    if meta.len() > grblset::MAX_DUMP_BYTES as u64 {
        return Err(format!(
            "{path}: the listing of the settings is larger than {} bytes",
            grblset::MAX_DUMP_BYTES
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    grblset::parse(&String::from_utf8_lossy(&bytes)).map_err(|e| format!("{path}: {e}"))
}

pub fn controller_job(mut args: Vec<String>) -> Result<(), String> {
    let lang = language(&mut args)?;
    let (s, _) = machine(&mut args)?;
    finish(&args, 1)?;
    let path = args
        .first()
        .ok_or("controller needs the saved answer of the controller to $$")?;
    let dump = read_dump(path)?;
    let found = grblset::check(
        &dump,
        &grblset::Expect {
            s_max: s.machine.s_max,
            bed_mm: s.machine.bed_mm,
            max_feed_mm_min: None,
        },
    );
    for f in &found {
        let mark = if f.kind.serious() { "!!" } else { "--" };
        println!("{mark} {}", f.text(lang));
    }
    let serious = found.iter().filter(|f| f.kind.serious()).count();
    println!("{} setting(s) read, {} finding(s)", dump.len(), found.len());
    if serious == 0 {
        Ok(())
    } else {
        Err(format!("{path}: {serious} serious finding(s)"))
    }
}

pub fn explain_job(mut args: Vec<String>) -> Result<(), String> {
    let lang = language(&mut args)?;
    finish(&args, 2)?;
    if args.is_empty() {
        return Err("explain needs ALARM:n or error:n".into());
    }
    println!("{}", grblcodes::explain(&args.join(" "), lang)?);
    Ok(())
}
