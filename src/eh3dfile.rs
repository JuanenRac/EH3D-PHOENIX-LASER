// EH3D-PHOENIX-LASER - the branch `laser/` of an `.eh3d` project.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
// One `.eh3d` file holds a branch for each program of the suite (the format is the crate `eh3d-project` of EH3D-COMMON). This program writes only `laser/`: what it takes from another
// branch is a copy with a reference that remembers the content hash and the format of its source, so that a later change of the source is told as out of date. Every other branch, known
// or not, keeps its bytes. Nothing in a project is run.

use std::path::Path;

use eh3d_project::appbranch::{
    self, counted, extension_of, file_name, read_bounded, BranchError, BranchKit, BranchResult,
    Conversion, Notice,
};
use eh3d_project::meta::ConversionReport;

use crate::dxf::read_dxf;
use crate::lbrn2::read_project;
use crate::svg::read_svg;
use crate::vector::{Drawing, GroupBy, ReadOptions};
use crate::{materials, vconfig};

pub use eh3d_project::appbranch::{Identity, Listing, Row, EXTENSION};

/// The branch this program owns.
pub const BRANCH: &str = "laser";
/// The format of the branch, `major.minor`.
pub const FORMAT: &str = "1.0";
/// The record of the branch that says which of its files is what.
pub const MANIFEST: &str = "laser.json";
/// A program of the laser larger than this is not put into a project.
const MAX_PROGRAM_BYTES: u64 = 256 << 20;
/// A settings file or a library of materials larger than this is not put into a project.
const MAX_SETTINGS_BYTES: u64 = 4 << 20;
const DRAWING_DIR: &str = "drawing/";

const KIT: BranchKit<'static> = BranchKit::new(BRANCH, FORMAT, MANIFEST, env!("CARGO_PKG_VERSION"));

/// What `save` is given: only what is there replaces what the branch holds.
#[derive(Debug, Default, Clone)]
pub struct Inputs {
    /// The drawing (SVG, DXF, LBRN2 or LBRN).
    pub drawing: Option<String>,
    /// The settings file.
    pub config: Option<String>,
    /// The library of materials.
    pub materials: Option<String>,
    /// Programs of the laser (G-code).
    pub programs: Vec<String>,
}

/// What the branch `laser/` holds.
#[derive(Debug, Clone, Default)]
pub struct Opened {
    /// The drawing of the work, as a name inside the branch.
    pub drawing: Option<String>,
    /// How many shapes and layers the drawing has.
    pub shapes: Option<(usize, usize)>,
    /// The settings file.
    pub settings: Option<String>,
    /// The library of materials.
    pub materials: Option<String>,
    /// The programs.
    pub programs: Vec<String>,
    /// The report of the last import.
    pub report: Option<ConversionReport>,
    /// What is out of date or wrong.
    pub warnings: Vec<String>,
    /// The files written out, when the branch was taken out.
    pub written: Option<usize>,
}

fn describe(e: BranchError) -> String {
    e.to_string()
}

fn said(notices: Vec<Notice>) -> Vec<String> {
    notices.iter().map(ToString::to_string).collect()
}

fn parse_drawing(name: &str, bytes: &[u8], opts: &ReadOptions) -> Result<Drawing, String> {
    match extension_of(name).as_str() {
        "svg" => read_svg(bytes, opts),
        "dxf" => read_dxf(bytes, opts),
        "lbrn2" | "lbrn" => read_project(bytes, opts).map(|p| p.drawing),
        _ => Err("it is not an SVG, a DXF or an LBRN2 file".to_string()),
    }
    .map_err(|e| format!("{name}: {e}"))
}

/// What kind of thing a file taken from another branch is.
fn artifact_of(name: &str) -> &'static str {
    match extension_of(name).as_str() {
        "svg" | "dxf" | "lbrn2" | "lbrn" => "outline2d",
        "stl" | "obj" | "3mf" | "step" | "stp" => "mesh3d",
        _ => "file",
    }
}

/// Puts the work of the laser into the branch `laser/` of the project at `path`, creating the project when it is not there. What is not given stays as it was.
pub fn save(
    path: &Path,
    inputs: &Inputs,
    identity: &Identity,
    read: &ReadOptions,
) -> Result<Vec<String>, String> {
    if inputs.drawing.is_none()
        && inputs.config.is_none()
        && inputs.materials.is_none()
        && inputs.programs.is_empty()
    {
        return Err(
            "there is nothing to save: give a drawing, settings, materials or a program".into(),
        );
    }
    // every file is read and checked before anything is written
    let drawing = match &inputs.drawing {
        Some(file) => {
            let bytes = read_bounded(Path::new(file), read.limits.max_file_bytes as u64)
                .map_err(describe)?;
            let name = file_name(Path::new(file)).map_err(describe)?;
            parse_drawing(&name, &bytes, read)?;
            Some((format!("{DRAWING_DIR}{name}"), bytes))
        }
        None => None,
    };
    let config = match &inputs.config {
        Some(file) => {
            let bytes = read_bounded(Path::new(file), MAX_SETTINGS_BYTES).map_err(describe)?;
            vconfig::load(Path::new(file))?;
            Some(bytes)
        }
        None => None,
    };
    let library = match &inputs.materials {
        Some(file) => {
            let bytes = read_bounded(Path::new(file), MAX_SETTINGS_BYTES).map_err(describe)?;
            materials::load(Path::new(file))?;
            Some(bytes)
        }
        None => None,
    };
    let mut programs = Vec::new();
    for file in &inputs.programs {
        let bytes = read_bounded(Path::new(file), MAX_PROGRAM_BYTES).map_err(describe)?;
        programs.push((
            format!("programs/{}", file_name(Path::new(file)).map_err(describe)?),
            bytes,
        ));
    }
    for (rel, _) in drawing.iter().chain(programs.iter()) {
        KIT.check_name(rel).map_err(describe)?;
    }

    let (_, notices) = KIT
        .update(path, identity, |_, copy| {
            if let Some((rel, bytes)) = drawing {
                copy.put(&rel, bytes)?;
                copy.choose("drawing", DRAWING_DIR, &rel);
            }
            if let Some(bytes) = config {
                copy.put("settings/laser.cfg", bytes)?;
                copy.set("settings", "settings/laser.cfg");
            }
            if let Some(bytes) = library {
                copy.put("settings/materials.cfg", bytes)?;
                copy.set("materials", "settings/materials.cfg");
            }
            for (rel, bytes) in programs {
                copy.put(&rel, bytes)?;
                copy.add_to_list("programs", &rel);
            }
            Ok(())
        })
        .map_err(describe)?;
    Ok(said(notices))
}

/// The report of reading a drawing of another program: what was read, what was not and what writing it again cannot say.
fn report_of(name: &str, bytes: &[u8], read: &ReadOptions) -> Result<ConversionReport, String> {
    let ext = extension_of(name);
    let mut got = Vec::new();
    let mut not_read = Vec::new();
    let mut export_losses = Vec::new();
    let (format, drawing) = if ext == "lbrn2" || ext == "lbrn" {
        let project = read_project(bytes, read).map_err(|e| format!("{name}: {e}"))?;
        got.push(counted("layer settings", project.cut_settings.len(), None));
        if !project.pictures.is_empty() {
            not_read.push(counted(
                "pictures",
                project.pictures.len(),
                Some("a picture is not part of the vector drawing; engrave it from the original"),
            ));
        }
        for (reason, count) in &project.stats.skipped {
            not_read.push(counted("shapes", *count, Some(reason)));
        }
        for (element, count) in &project.stats.unknown_parts {
            not_read.push(counted(
                element,
                *count,
                Some("an element the reader does not know"),
            ));
        }
        export_losses.push(counted(
            "project settings",
            1,
            Some("no LightBurn project is written: convert writes SVG or DXF without power, speed, passes or pictures"),
        ));
        (ext.as_str(), project.drawing)
    } else {
        let drawing = parse_drawing(name, bytes, read)?;
        export_losses.push(counted(
            "curves and text",
            drawing.shapes.len(),
            Some("convert writes the shapes as straight pieces within the tolerance"),
        ));
        (ext.as_str(), drawing)
    };
    got.push(counted("shapes", drawing.shapes.len(), None));
    got.push(counted(
        "closed shapes",
        drawing.shapes.iter().filter(|s| s.closed).count(),
        None,
    ));
    got.push(counted("layers", drawing.keys(GroupBy::Auto).len(), None));
    got.push(counted(
        "points",
        drawing.shapes.iter().map(|s| s.pts.len()).sum(),
        None,
    ));
    for warning in &drawing.warnings {
        not_read.push(counted("note", 1, Some(warning)));
    }
    let conversion = Conversion {
        read: got,
        not_read,
        export_losses,
    };
    Ok(appbranch::conversion_report(
        "phoenix-laser",
        env!("CARGO_PKG_VERSION"),
        format,
        name,
        bytes,
        conversion,
    ))
}

/// Imports a drawing of another program: the original is kept byte for byte under `laser/origin/` with a report of the conversion, and it becomes the drawing of the work.
pub fn import(
    path: &Path,
    source: &Path,
    identity: &Identity,
    read: &ReadOptions,
) -> Result<(ConversionReport, Vec<String>), String> {
    let name = file_name(source).map_err(describe)?;
    let bytes = read_bounded(source, read.limits.max_file_bytes as u64).map_err(describe)?;
    let report = report_of(&name, &bytes, read)?;
    let notices = KIT
        .import(path, identity, bytes, &report, |copy, original| {
            copy.choose("drawing", DRAWING_DIR, original);
            Ok(())
        })
        .map_err(describe)?;
    Ok((report, said(notices)))
}

/// Copies a file of another branch into `laser/taken/`, with a reference that remembers the state of its source. A drawing becomes the drawing of the work.
pub fn take(
    path: &Path,
    branch: &str,
    rel: &str,
    read: &ReadOptions,
) -> Result<(String, Vec<String>), String> {
    let artifact = artifact_of(rel);
    let outline = artifact == "outline2d";
    let check = |bytes: &[u8]| -> BranchResult<()> {
        if outline {
            if bytes.len() > read.limits.max_file_bytes {
                return Err(BranchError::Refused(format!(
                    "{branch}/{rel}: the drawing is too large"
                )));
            }
            parse_drawing(rel, bytes, read).map_err(BranchError::Refused)?;
        }
        Ok(())
    };
    let (target, notices) = KIT
        .take(path, branch, rel, artifact, check, |copy, target| {
            if outline {
                copy.choose("drawing", DRAWING_DIR, target);
            }
        })
        .map_err(describe)?;
    Ok((target, said(notices)))
}

/// The branches of a project, with what is out of date.
pub fn list(path: &Path) -> Result<Listing, String> {
    appbranch::list(path).map_err(describe)
}

/// Opens the branch `laser/`: reads and checks what it holds and, with `into`, writes its files there (a file that is there is never written over).
pub fn open(path: &Path, into: Option<&Path>, read: &ReadOptions) -> Result<Opened, String> {
    let (mut project, copy, warnings) = KIT.open(path).map_err(describe)?;
    let mut opened = Opened {
        drawing: copy.text("drawing"),
        settings: copy.text("settings"),
        materials: copy.text("materials"),
        programs: copy.list("programs"),
        warnings,
        ..Opened::default()
    };
    if let Some(rel) = &opened.drawing {
        let bytes = copy
            .file(rel)
            .ok_or_else(|| format!("{BRANCH}/{rel}: the drawing is not in the branch"))?;
        let drawing = parse_drawing(rel, bytes, read)?;
        opened.shapes = Some((drawing.shapes.len(), drawing.keys(GroupBy::Auto).len()));
    }
    opened.report = copy.report().map_err(describe)?;
    if let Some(dir) = into {
        opened.written = Some(
            project
                .extract_branch(BRANCH, dir)
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(opened)
}

/// Writes the files of any branch into a folder, byte for byte (a file that is there is never written over).
pub fn extract(path: &Path, branch: &str, dir: &Path) -> Result<usize, String> {
    appbranch::extract(path, branch, dir).map_err(describe)
}
