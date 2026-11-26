// EH3D-PHOENIX-LASER - the branch `laser/` of an `.eh3d` project.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
// One `.eh3d` file holds a branch for each program of the suite (the format is the crate `eh3d-project` of EH3D-COMMON). This program writes only `laser/`: what it takes from another
// branch is a copy with a reference that remembers the content hash and the format of its source, so that a later change of the source is told as out of date. Every other branch, known
// or not, keeps its bytes. Nothing in a project is run.

use std::collections::BTreeMap;
use std::fs;
use std::hash::{BuildHasher, Hasher, RandomState};
use std::io::Read;
use std::path::Path;

use eh3d_container::{check_entry_name, sha256_hex, AtomicOptions};
use eh3d_project::meta::{
    ConversionReport, Converter, Counted, SourceFile, ORIGIN_DIR, REPORT_FILE,
};
use eh3d_project::{
    BranchUpdate, Checks, Dependency, NewProject, OpenOptions, Project, SaveOptions, Writer,
};
use serde_json::{Map, Value};

use crate::dxf::read_dxf;
use crate::lbrn2::read_project;
use crate::svg::read_svg;
use crate::vector::{Drawing, GroupBy, ReadOptions};
use crate::{materials, vconfig};

/// The branch this program owns.
pub const BRANCH: &str = "laser";
/// The format of the branch, `major.minor`.
pub const FORMAT: &str = "1.0";
/// The record of the branch that says which of its files is what.
pub const MANIFEST: &str = "laser.json";
/// The extension of a project.
pub const EXTENSION: &str = "eh3d";
/// A program of the laser larger than this is not put into a project.
const MAX_PROGRAM_BYTES: u64 = 256 << 20;
/// A settings file or a library of materials larger than this is not put into a project.
const MAX_SETTINGS_BYTES: u64 = 4 << 20;
const DRAWING_DIR: &str = "drawing/";
const TAKEN_DIR: &str = "taken/";

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

/// The name and the identifier of a project that is being created.
#[derive(Debug, Default, Clone)]
pub struct Identity {
    /// The name (the name of the file when there is none).
    pub name: Option<String>,
    /// The identifier (a random UUID when there is none).
    pub id: Option<String>,
}

/// One branch, as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The name of the branch.
    pub id: String,
    /// The program that wrote it (`-` without a record).
    pub app: String,
    /// The format of the branch.
    pub format: String,
    /// The version of that program.
    pub app_version: String,
    /// The files of the branch, `branch.json` included.
    pub files: usize,
    /// The content hash.
    pub hash: String,
    /// What is worth saying about it.
    pub notes: Vec<&'static str>,
}

/// What `list` finds.
#[derive(Debug, Clone)]
pub struct Listing {
    /// The name of the project.
    pub name: String,
    /// Its identifier.
    pub id: String,
    /// The branches.
    pub rows: Vec<Row>,
    /// The derived branches whose sources changed, and what the checks found wrong.
    pub warnings: Vec<String>,
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

/// The branch as it is read, changed and written back.
#[derive(Default)]
struct Own {
    files: BTreeMap<String, Vec<u8>>,
    manifest: Map<String, Value>,
    depends: Vec<Dependency>,
    extra: BTreeMap<String, Value>,
}

impl Own {
    fn text(&self, key: &str) -> Option<String> {
        self.manifest
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    fn set(&mut self, key: &str, value: &str) {
        self.manifest
            .insert(key.to_owned(), Value::from(value.to_owned()));
    }

    /// Puts a file in the branch, checking that its name can live in a project.
    fn put(&mut self, rel: &str, bytes: Vec<u8>) -> Result<(), String> {
        check_entry_name(&format!("{BRANCH}/{rel}"))
            .map_err(|_| format!("'{rel}' is not a name that can be kept in a project"))?;
        self.files.insert(rel.to_owned(), bytes);
        Ok(())
    }

    /// Takes a file out of the branch, with the references that only it used.
    fn remove(&mut self, rel: &str) {
        self.files.remove(rel);
        for dep in &mut self.depends {
            dep.uses.retain(|u| u != rel);
        }
        self.depends.retain(|d| !d.uses.is_empty());
    }

    /// Makes `rel` the drawing. The one before goes, unless it is the original of an import (it stays with its report).
    fn set_drawing(&mut self, rel: &str) {
        if let Some(old) = self.text("drawing") {
            if old != rel && (old.starts_with(DRAWING_DIR) || old.starts_with(TAKEN_DIR)) {
                self.remove(&old);
            }
        }
        self.set("drawing", rel);
    }

    fn programs(&self) -> Vec<String> {
        self.manifest
            .get("programs")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn describe(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn open_project(path: &Path) -> Result<Project, String> {
    Project::open(path, &OpenOptions::default()).map_err(|e| format!("{}: {e}", path.display()))
}

fn name_of(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: the file has no usable name", path.display()))
}

fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Reads a file whole, never more than `limit` bytes of it.
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "{}: the file is larger than {limit} bytes",
            path.display()
        ));
    }
    Ok(bytes)
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

/// A random identifier in the shape of a UUID of version 4.
fn new_id() -> String {
    let mut b = [0u8; 16];
    for half in b.chunks_mut(8) {
        half.copy_from_slice(&RandomState::new().build_hasher().finish().to_le_bytes());
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// Opens the project at `path`, or creates it when there is no file there.
fn open_or_create(path: &Path, identity: &Identity) -> Result<Project, String> {
    if path.exists() {
        if identity.name.is_some() || identity.id.is_some() {
            return Err("--name and --id are only for a project that is being created".to_string());
        }
        return open_project(path);
    }
    if extension_of(&name_of(path)?) != EXTENSION {
        return Err(format!(
            "{}: a new project is a file that ends in .{EXTENSION}",
            path.display()
        ));
    }
    let stem = path.file_stem().map_or_else(
        || "project".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let new = NewProject {
        id: identity.id.clone().unwrap_or_else(new_id),
        name: identity.name.clone().unwrap_or(stem),
        units: "millimetres".to_string(),
        created: None,
    };
    Project::create(
        path,
        new,
        &OpenOptions::default(),
        &AtomicOptions::default(),
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads the branch of this program, empty when the project has none yet.
fn load_own(project: &mut Project) -> Result<Own, String> {
    let Some(branch) = project.branch(BRANCH) else {
        return Ok(Own::default());
    };
    let record = branch
        .record
        .clone()
        .ok_or_else(|| format!("EH3D-PKG-015: {BRANCH}/branch.json is missing or not valid"))?;
    if record.format != FORMAT {
        return Err(format!(
            "EH3D-PKG-024: the branch {BRANCH} has the format {}, and this program handles {FORMAT}",
            record.format
        ));
    }
    let names: Vec<String> = branch
        .files
        .keys()
        .filter(|n| *n != "branch.json")
        .cloned()
        .collect();
    let mut own = Own {
        depends: record.depends,
        extra: record.extra,
        ..Own::default()
    };
    for name in names {
        let bytes = project.read_entry(BRANCH, &name).map_err(describe)?;
        own.files.insert(name, bytes);
    }
    if let Some(bytes) = own.files.get(MANIFEST) {
        own.manifest = serde_json::from_slice(bytes)
            .map_err(|e| format!("{BRANCH}/{MANIFEST} is not a valid record: {e}"))?;
    }
    Ok(own)
}

/// Writes the branch of this program. Nothing else of the project changes but its two records.
fn write_own(project: Project, mut own: Own) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    if project.record().signature.is_some() {
        notes.push(
            "the signature of the project does not hold after a change and is removed".into(),
        );
    }
    own.manifest.insert("schema".to_string(), Value::from(1));
    let mut text = serde_json::to_vec_pretty(&own.manifest).map_err(describe)?;
    text.push(b'\n');
    own.files.insert(MANIFEST.to_string(), text);
    let mut update = BranchUpdate::new(BRANCH, FORMAT, env!("CARGO_PKG_VERSION"));
    update.depends = own
        .depends
        .into_iter()
        .filter(|d| project.branch(&d.branch).is_some())
        .collect();
    update.entries = own.files;
    update.extra = own.extra;
    project
        .save_branch(&Writer::for_app(BRANCH), update, &SaveOptions::default())
        .map_err(describe)?;
    Ok(notes)
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
            let bytes = read_bounded(Path::new(file), read.limits.max_file_bytes as u64)?;
            let name = name_of(Path::new(file))?;
            parse_drawing(&name, &bytes, read)?;
            Some((format!("{DRAWING_DIR}{name}"), bytes))
        }
        None => None,
    };
    let config = match &inputs.config {
        Some(file) => {
            let bytes = read_bounded(Path::new(file), MAX_SETTINGS_BYTES)?;
            vconfig::load(Path::new(file))?;
            Some(bytes)
        }
        None => None,
    };
    let library = match &inputs.materials {
        Some(file) => {
            let bytes = read_bounded(Path::new(file), MAX_SETTINGS_BYTES)?;
            materials::load(Path::new(file))?;
            Some(bytes)
        }
        None => None,
    };
    let mut programs = Vec::new();
    for file in &inputs.programs {
        let bytes = read_bounded(Path::new(file), MAX_PROGRAM_BYTES)?;
        programs.push((format!("programs/{}", name_of(Path::new(file))?), bytes));
    }
    let mut checked = Own::default();
    for (rel, _) in drawing.iter().chain(programs.iter()) {
        checked.put(rel, Vec::new())?;
    }

    let mut project = open_or_create(path, identity)?;
    let mut own = load_own(&mut project)?;
    if let Some((rel, bytes)) = drawing {
        own.put(&rel, bytes)?;
        own.set_drawing(&rel);
    }
    if let Some(bytes) = config {
        own.put("settings/laser.cfg", bytes)?;
        own.set("settings", "settings/laser.cfg");
    }
    if let Some(bytes) = library {
        own.put("settings/materials.cfg", bytes)?;
        own.set("materials", "settings/materials.cfg");
    }
    if !programs.is_empty() {
        let mut list = own.programs();
        for (rel, bytes) in programs {
            own.put(&rel, bytes)?;
            if !list.contains(&rel) {
                list.push(rel);
            }
        }
        own.manifest
            .insert("programs".to_string(), Value::from(list));
    }
    write_own(project, own)
}

fn counted(item: &str, count: usize, reason: Option<&str>) -> Counted {
    Counted {
        item: item.to_string(),
        count: count as u64,
        reason: reason.map(str::to_owned),
    }
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
    Ok(ConversionReport {
        schema: 1,
        source: SourceFile {
            format: format.to_string(),
            name: name.to_string(),
            sha256: sha256_hex(bytes),
            size: bytes.len() as u64,
        },
        converter: Converter {
            name: "phoenix-laser".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        original: Some(format!("{ORIGIN_DIR}{name}")),
        read: got,
        not_read,
        export_losses,
        extra: BTreeMap::new(),
    })
}

/// Imports a drawing of another program: the original is kept byte for byte under `laser/origin/` with a report of the conversion, and it becomes the drawing of the work.
pub fn import(
    path: &Path,
    source: &Path,
    identity: &Identity,
    read: &ReadOptions,
) -> Result<(ConversionReport, Vec<String>), String> {
    let name = name_of(source)?;
    let bytes = read_bounded(source, read.limits.max_file_bytes as u64)?;
    let report = report_of(&name, &bytes, read)?;
    let original = format!("{ORIGIN_DIR}{name}");
    Own::default().put(&original, Vec::new())?;

    let mut project = open_or_create(path, identity)?;
    let mut own = load_own(&mut project)?;
    let before: Vec<String> = own
        .files
        .keys()
        .filter(|n| n.starts_with(ORIGIN_DIR))
        .cloned()
        .collect();
    for old in &before {
        own.remove(old);
    }
    own.put(&original, bytes)?;
    let mut text = serde_json::to_vec_pretty(&report).map_err(describe)?;
    text.push(b'\n');
    own.put(REPORT_FILE, text)?;
    own.set_drawing(&original);
    let mut notes = write_own(project, own)?;
    if before.iter().any(|n| n != REPORT_FILE && *n != original) {
        notes.push("the import that was there before was replaced".to_string());
    }
    Ok((report, notes))
}

/// Copies a file of another branch into `laser/taken/`, with a reference that remembers the state of its source. A drawing becomes the drawing of the work.
pub fn take(
    path: &Path,
    branch: &str,
    rel: &str,
    read: &ReadOptions,
) -> Result<(String, Vec<String>), String> {
    if branch == BRANCH {
        return Err("a branch takes from another branch, not from itself".to_string());
    }
    let mut project = open_project(path)?;
    let bytes = project.read_entry(branch, rel).map_err(describe)?;
    let artifact = artifact_of(rel);
    if artifact == "outline2d" {
        if bytes.len() > read.limits.max_file_bytes {
            return Err(format!("{branch}/{rel}: the drawing is too large"));
        }
        parse_drawing(rel, &bytes, read)?;
    }
    let target = format!("{TAKEN_DIR}{branch}/{rel}");
    let dep = project
        .dependency_on(branch, artifact, 1, vec![target.clone()])
        .map_err(describe)?;
    let mut own = load_own(&mut project)?;
    own.remove(&target);
    own.put(&target, bytes)?;
    own.depends.push(dep);
    if artifact == "outline2d" {
        own.set_drawing(&target);
    }
    let notes = write_own(project, own)?;
    Ok((target, notes))
}

/// The derived branches whose sources changed, and what the checks of the container find wrong.
fn warnings_of(project: &mut Project) -> Vec<String> {
    let mut out: Vec<String> = project
        .stale_refs()
        .iter()
        .map(|s| {
            format!(
                "OUT OF DATE: '{}' was derived from '{}' ({}: {}) before it changed",
                s.branch,
                s.dependency.branch,
                s.dependency.artifact,
                s.dependency.uses.join(", ")
            )
        })
        .collect();
    for f in project.diagnose(&Checks::default()) {
        if f.is_error() {
            out.push(format!(
                "{}: {} [{}]",
                f.code,
                f.detail,
                f.branch.as_deref().unwrap_or("project")
            ));
        }
    }
    out
}

/// The branches of a project, with what is out of date.
pub fn list(path: &Path) -> Result<Listing, String> {
    let mut project = open_project(path)?;
    let warnings = warnings_of(&mut project);
    let stale = project.stale_refs();
    let rows = project
        .branches()
        .map(|b| {
            let r = b.record.as_ref();
            let mut notes = Vec::new();
            if !b.is_known() {
                notes.push("not known by name, kept as it is");
            }
            if b.is_sealed() {
                notes.push("sealed");
            }
            if b.is_encrypted() {
                notes.push("encrypted");
            }
            if stale.iter().any(|s| s.branch == b.id) {
                notes.push("OUT OF DATE");
            }
            let of = |v: Option<&String>| v.cloned().unwrap_or_else(|| "-".to_string());
            Row {
                id: b.id.clone(),
                app: of(r.map(|r| &r.app)),
                format: of(r.map(|r| &r.format)),
                app_version: of(r.map(|r| &r.app_version)),
                files: b.files.len(),
                hash: b.hash.clone(),
                notes,
            }
        })
        .collect();
    Ok(Listing {
        name: project.record().name.clone(),
        id: project.record().id.clone(),
        rows,
        warnings,
    })
}

/// Opens the branch `laser/`: reads and checks what it holds and, with `into`, writes its files there (a file that is there is never written over).
pub fn open(path: &Path, into: Option<&Path>, read: &ReadOptions) -> Result<Opened, String> {
    let mut project = open_project(path)?;
    if project.branch(BRANCH).is_none() {
        return Err(format!(
            "{}: the project has no branch {BRANCH}",
            path.display()
        ));
    }
    let own = load_own(&mut project)?;
    let mut opened = Opened {
        drawing: own.text("drawing"),
        settings: own.text("settings"),
        materials: own.text("materials"),
        programs: own.programs(),
        warnings: warnings_of(&mut project),
        ..Opened::default()
    };
    if let Some(rel) = &opened.drawing {
        let bytes = own
            .files
            .get(rel)
            .ok_or_else(|| format!("{BRANCH}/{rel}: the drawing is not in the branch"))?;
        let drawing = parse_drawing(rel, bytes, read)?;
        opened.shapes = Some((drawing.shapes.len(), drawing.keys(GroupBy::Auto).len()));
    }
    if let Some(bytes) = own.files.get(REPORT_FILE) {
        opened.report = Some(
            serde_json::from_slice(bytes)
                .map_err(|e| format!("{BRANCH}/{REPORT_FILE} is not a valid record: {e}"))?,
        );
    }
    if let Some(dir) = into {
        opened.written = Some(project.extract_branch(BRANCH, dir).map_err(describe)?);
    }
    Ok(opened)
}

/// Writes the files of any branch into a folder, byte for byte (a file that is there is never written over).
pub fn extract(path: &Path, branch: &str, dir: &Path) -> Result<usize, String> {
    let mut project = open_project(path)?;
    project.extract_branch(branch, dir).map_err(describe)
}
