//! What the entries of the menu File for a project file of the suite (`.eh3d`) do in the window: open it, save it and save it under
//! another name without losing a branch, import a drawing keeping the original, take a file of another branch and extract a branch.
//!
//! The work is done by the functions of the command `project` (`eh3dfile`); what is here decides what the window holds, what goes into
//! the branch and what a person has to agree to first. The window (`gui`) only draws and asks, so all of it is tested without a screen.
//! Only the branch `laser` is written. A project that was not opened here, a branch that changed since it was read and a signature that
//! a write removes are asked about before anything is written.

use std::path::{Path, PathBuf};

use crate::eh3dfile::{self, Contents, Held, Identity, Opened, BRANCH};
use crate::materials::{self, Library};
use crate::suitecore::{self, name_of, ProjectView};
use crate::suitetext::{Said, S};
use crate::vconfig::{self, Settings};
use crate::window::{read_vector, Job, Mode, ProgramFile, Status, VectorFile, Window};

/// The largest settings file or library of materials that is put into a project.
const MAX_SMALL_BYTES: u64 = 4 << 20;

/// Where the settings file or the library of materials that goes into the project comes from. A file the window read by itself when it
/// started does not go into a project: only what a person chose, or what the project already held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Source {
    /// Nothing is put.
    #[default]
    Nothing,
    /// The file the person chose in the window.
    Chosen,
    /// The bytes the project held.
    Kept(Vec<u8>),
}

/// Something that writes a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Save into this project.
    Save(PathBuf),
    /// Import this file of another program.
    Import(PathBuf),
    /// Take this file of that branch.
    Take(String, String),
}

/// A write that waits for the person to agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// What was asked.
    pub action: Action,
    /// What has to be agreed to.
    pub warnings: Vec<Said>,
}

/// How an entry ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// It was done.
    Done,
    /// The window has no project yet: a name has to be asked for.
    NeedsName,
    /// Nothing was written yet: the person has to agree ([`SuiteState::pending`]).
    Asked,
    /// It was refused; the status line says why.
    Refused,
}

/// What the window keeps about its project file of the suite.
#[derive(Debug, Clone, Default)]
pub struct SuiteState {
    /// The project the window works on.
    pub file: Option<PathBuf>,
    /// What it held when it was last looked at.
    pub view: Option<ProjectView>,
    /// What is to be told about the last thing done.
    pub notes: Vec<Said>,
    /// What the checks of the project said in their own words.
    pub extra: Vec<String>,
    /// The write that waits for the person.
    pub pending: Option<Pending>,
    /// The branch that is going to be extracted.
    pub extract_branch: String,
    /// The content hash of the branch of the program when it was read or written.
    seen: Option<String>,
    /// Where the settings file that goes into the project comes from.
    pub settings: Source,
    /// The same for the library of materials.
    pub materials: Source,
    /// The settings of the person, kept while those of a project are in force.
    own: Option<Box<Settings>>,
}

/// What an import or a take gives: the notices of the write, the lines to tell and the status.
type Brought = (Vec<String>, Vec<Said>, Said);

/// What a project gives to the window, all of it read before anything changes.
struct Loaded {
    settings: Option<(Settings, Vec<u8>)>,
    materials: Option<(Library, Vec<u8>)>,
    drawing: Option<VectorFile>,
    program: Option<ProgramFile>,
}

/// The settings of a project over the settings of the person: how the work is done comes from the project, and the server, what is
/// held back before a send, the interlocks and the profiles stay as the person has them. Limits are the lower of the two and the
/// forbidden zones of both hold, so a project never loosens anything.
pub fn keep_my_limits(mine: &Settings, theirs: Settings) -> Settings {
    let mut s = theirs;
    for (machine, own) in [
        (&mut s.machine, &mine.machine),
        (&mut s.base_machine, &mine.base_machine),
    ] {
        machine.max_power_percent = machine.max_power_percent.min(own.max_power_percent);
        machine.max_speed_mm_min = machine.max_speed_mm_min.min(own.max_speed_mm_min);
        machine.bed_mm = own.bed_mm.or(machine.bed_mm);
    }
    s.profiles = mine.profiles.clone();
    s.profile = mine.profile.clone();
    s.frame.max_pilot_percent = s.frame.max_pilot_percent.min(mine.frame.max_pilot_percent);
    s.frame.pilot_percent = s.frame.pilot_percent.min(s.frame.max_pilot_percent);
    s.server = mine.server.clone();
    s.send = mine.send;
    s.interlocks = mine.interlocks;
    for zone in &mine.zones {
        if !s.zones.contains(zone) {
            s.zones.push(zone.clone());
        }
    }
    s
}

/// A file of the branch by the name the record gives it.
fn file_of(opened: &Opened, rel: &Option<String>) -> Result<Option<(String, Vec<u8>)>, Status> {
    let Some(rel) = rel else {
        return Ok(None);
    };
    match opened.files.get(rel) {
        Some(bytes) => Ok(Some((rel.clone(), bytes.clone()))),
        None => Err(Status::Failed(format!(
            "{BRANCH}/{rel}: the file is not in the branch"
        ))),
    }
}

/// The path a file of the branch is told by in the window: beside the project, so that what is written from it goes there.
fn beside(project: &Path, rel: &str) -> PathBuf {
    project.with_file_name(name_of(Path::new(rel)))
}

/// Leaves out what the branch already holds byte for byte, so a file taken from another branch stays where it is with its reference.
fn drop_unchanged(contents: &mut Contents, held: &Opened) {
    let same = |rel: &Option<String>, bytes: &[u8]| {
        rel.as_ref()
            .and_then(|r| held.files.get(r))
            .is_some_and(|b| b.as_slice() == bytes)
    };
    if contents
        .drawing
        .as_ref()
        .is_some_and(|h| same(&held.drawing, &h.bytes))
    {
        contents.drawing = None;
    }
    if contents
        .config
        .as_ref()
        .is_some_and(|h| same(&held.settings, &h.bytes))
    {
        contents.config = None;
    }
    if contents
        .materials
        .as_ref()
        .is_some_and(|h| same(&held.materials, &h.bytes))
    {
        contents.materials = None;
    }
    contents
        .programs
        .retain(|p| held.files.get(&format!("programs/{}", p.name)) != Some(&p.bytes));
}

impl Window {
    fn suite_code(&self) -> &'static str {
        self.lang.code()
    }

    fn suite_fail(&mut self, why: String) -> Step {
        self.status = Status::Failed(why);
        Step::Refused
    }

    /// The person chose a settings file: it is theirs from now on and it goes into the project when it is saved.
    pub(crate) fn suite_chose_settings(&mut self) {
        self.suite.settings = Source::Chosen;
        self.suite.own = None;
    }

    /// The same for a library of materials.
    pub(crate) fn suite_chose_materials(&mut self) {
        self.suite.materials = Source::Chosen;
    }

    /// Reads the project again to tell what is out of date now. What the window holds does not change.
    pub fn suite_refresh(&mut self) {
        if let Some(file) = &self.suite.file {
            self.suite.view = suitecore::view(file, BRANCH).ok();
        }
    }

    /// The lines that stay in sight while the window works on a project; the flag marks the ones that ask for attention.
    pub fn suite_banner(&self) -> Vec<(String, bool)> {
        let Some(file) = &self.suite.file else {
            return Vec::new();
        };
        let code = self.suite_code();
        let mut lines = suitecore::banner(file, self.suite.view.as_ref(), BRANCH, code);
        lines.extend(self.suite.notes.iter().map(|n| (n.text(code), false)));
        lines.extend(self.suite.extra.iter().map(|n| (n.clone(), true)));
        lines
    }

    /// Opens a project: the work of the branch `laser` replaces what the window holds. A project without that branch is taken as it is
    /// and saving puts the work there. A project that cannot be read leaves the window as it was.
    pub fn suite_open(&mut self, path: &Path) -> bool {
        let found = match suitecore::view(path, BRANCH) {
            Ok(found) => found,
            Err(e) => {
                self.suite_fail(e);
                return false;
            }
        };
        let has_branch = found.branch(BRANCH).is_some();
        let held = if has_branch {
            match eh3dfile::held(path) {
                Ok(held) => held,
                Err(e) => {
                    self.suite_fail(e);
                    return false;
                }
            }
        } else {
            Opened::default()
        };
        let loaded = match self.suite_read(path, &held) {
            Ok(loaded) => loaded,
            Err(status) => {
                self.status = status;
                return false;
            }
        };
        let with_settings = loaded.settings.is_some();
        self.suite_show(loaded);
        let mut notes = Vec::new();
        if with_settings {
            notes.push(Said::plain(S::LimitsKept));
        }
        if let Some(report) = &held.report {
            notes.extend(suitecore::report_lines(report));
        }
        self.suite.notes = notes;
        self.suite.extra = held
            .warnings
            .iter()
            .filter(|w| !w.starts_with("OUT OF DATE"))
            .cloned()
            .collect();
        self.suite.pending = None;
        self.suite.seen = found.own_hash.clone();
        self.suite.view = Some(found);
        self.suite.file = Some(path.to_path_buf());
        self.status = Status::Suite(if has_branch {
            Said(S::Opened, vec![name_of(path)])
        } else {
            Said(S::NoWork, vec![name_of(path), BRANCH.to_owned()])
        });
        true
    }

    fn suite_read(&self, path: &Path, held: &Opened) -> Result<Loaded, Status> {
        let mine = self.suite.own.as_deref().unwrap_or(&self.settings);
        let settings = match file_of(held, &held.settings)? {
            Some((rel, bytes)) => {
                let theirs = vconfig::parse(&eh3dfile::text_of(&bytes))
                    .map_err(|e| Status::Failed(format!("{BRANCH}/{rel}: {e}")))?;
                Some((keep_my_limits(mine, theirs), bytes))
            }
            None => None,
        };
        let library = match file_of(held, &held.materials)? {
            Some((rel, bytes)) => {
                let library = materials::parse(&eh3dfile::text_of(&bytes))
                    .map_err(|e| Status::Failed(format!("{BRANCH}/{rel}: {e}")))?;
                Some((library, bytes))
            }
            None => None,
        };
        let read = match &settings {
            Some((s, _)) => s.read,
            None => self.read_options()?,
        };
        let drawing = match file_of(held, &held.drawing)? {
            Some((rel, bytes)) => Some(read_vector(&beside(path, &rel), bytes, &read)?),
            None => None,
        };
        let program = match (&drawing, file_of(held, &held.programs.first().cloned())?) {
            (None, Some((rel, bytes))) => Some(ProgramFile {
                path: beside(path, &rel),
                name: name_of(Path::new(&rel)),
                text: String::from_utf8_lossy(&bytes).into_owned(),
            }),
            _ => None,
        };
        Ok(Loaded {
            settings,
            materials: library,
            drawing,
            program,
        })
    }

    /// Takes out of the window the work of before, so that saving never puts it into another project, and shows the one that was read.
    fn suite_show(&mut self, loaded: Loaded) {
        self.picture = None;
        self.vector = None;
        self.program = None;
        self.rows.clear();
        self.done = None;
        self.segs.clear();
        self.warnings.clear();
        self.check = None;
        self.report = None;
        self.stale = false;
        self.truncated = false;
        self.send_opened_confirmed = false;
        self.mode = Mode::Engrave;
        self.work_mode = Mode::Engrave;
        // the settings of the project of before go, and those of the person come back
        if let Some(own) = self.suite.own.take() {
            self.apply_settings(*own);
        }
        self.suite.settings = Source::Nothing;
        if let Some((settings, bytes)) = loaded.settings {
            self.suite.own = Some(Box::new(self.settings.clone()));
            self.apply_settings(settings);
            self.suite.settings = Source::Kept(bytes);
        }
        if matches!(self.suite.materials, Source::Kept(_)) {
            self.materials = None;
            self.material = None;
            self.material_draft = None;
        }
        self.suite.materials = Source::Nothing;
        if let Some((library, bytes)) = loaded.materials {
            self.materials = Some(library);
            self.materials_path = None;
            self.material = None;
            self.material_draft = None;
            self.suite.materials = Source::Kept(bytes);
        }
        if let Some(drawing) = loaded.drawing {
            self.show_vector(drawing);
        }
        if let Some(program) = loaded.program {
            self.program = Some(program);
            self.mode = Mode::Preview;
        }
        self.mark_stale();
    }

    fn suite_small(
        &self,
        source: &Source,
        chosen: Option<&Path>,
        name: &str,
    ) -> Result<Option<Held>, String> {
        let held = |bytes: Vec<u8>, name: String| Some(Held { name, bytes });
        Ok(match (source, chosen) {
            (Source::Kept(bytes), _) => held(bytes.clone(), name.to_owned()),
            (Source::Chosen, Some(path)) => {
                let bytes = eh3d_project::appbranch::read_bounded(path, MAX_SMALL_BYTES)
                    .map_err(|e| e.to_string())?;
                held(bytes, path.display().to_string())
            }
            _ => None,
        })
    }

    /// What the window would put into the branch, and what is to be said about what it leaves out.
    pub fn suite_contents(&self) -> Result<(Contents, Vec<Said>), String> {
        let mut notes = Vec::new();
        let mut contents = Contents {
            drawing: self.vector.as_ref().map(|v| Held {
                name: v.name.clone(),
                bytes: v.bytes.clone(),
            }),
            config: self.suite_small(
                &self.suite.settings,
                self.config_path.as_deref(),
                "laser.cfg",
            )?,
            materials: self.suite_small(
                &self.suite.materials,
                self.materials_path.as_deref(),
                "materials.cfg",
            )?,
            programs: Vec::new(),
        };
        // the project is a file that is passed on: the token of the server never goes into it
        let has_token = contents.config.as_ref().is_some_and(|held| {
            vconfig::parse(&eh3dfile::text_of(&held.bytes))
                .is_ok_and(|s| !s.server.server_token.is_empty())
        });
        if has_token {
            contents.config = None;
            notes.push(Said::plain(S::SettingsWithToken));
        }
        if self.mode != Mode::Preview && self.done.is_some() && !self.needs_confirmation() {
            if self.stale {
                notes.push(Said::plain(S::ProgramNotCurrent));
            } else if let Some(text) = self.program_text() {
                let name = self
                    .suggested_output_for(Job::ExportGcode)
                    .map_or_else(|| "engraving.nc".to_owned(), |p| name_of(&p));
                contents.programs.push(Held {
                    name,
                    bytes: text.into_bytes(),
                });
            }
        }
        Ok((contents, notes))
    }

    /// The name offered for a project that is saved for the first time or under another name.
    pub fn suite_suggested_name(&self) -> String {
        let from = self
            .suite
            .file
            .as_deref()
            .or_else(|| self.vector.as_ref().map(|v| v.path.as_path()))
            .or_else(|| self.picture.as_ref().map(|p| p.path.as_path()));
        let stem = from
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "job".to_owned());
        format!("{stem}.eh3d")
    }

    /// The name of the folder offered to extract a branch into.
    pub fn suite_extract_name(&self) -> String {
        let stem = self
            .suite
            .file
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "job".to_owned());
        format!("{stem}-{}", self.suite.extract_branch.replace('/', "-"))
    }

    /// Saves into the project of the window, or says that it has none.
    pub fn suite_save(&mut self) -> Step {
        match self.suite.file.clone() {
            Some(file) => self.suite_run(Action::Save(file), false),
            None => Step::NeedsName,
        }
    }

    /// Saves under another name. A new file gets every branch of the project that is open; a file that is there keeps its own
    /// branches and only `laser` is written, once the person agreed.
    pub fn suite_save_as(&mut self, target: &Path) -> Step {
        self.suite_run(Action::Save(suitecore::with_extension(target)), false)
    }

    /// Keeps the original of a drawing of another program in the project and makes it the drawing of the work.
    pub fn suite_import(&mut self, source: &Path) -> Step {
        self.suite_run(Action::Import(source.to_path_buf()), false)
    }

    /// Takes a file of another branch into the project, with the reference that tells later when it is out of date.
    pub fn suite_take(&mut self, branch: &str, rel: &str) -> Step {
        self.suite_run(Action::Take(branch.to_owned(), rel.to_owned()), false)
    }

    /// The person agreed to the write that was waiting.
    pub fn suite_confirm(&mut self) -> Step {
        match self.suite.pending.take() {
            Some(pending) => self.suite_run(pending.action, true),
            None => Step::Refused,
        }
    }

    /// The person did not agree: nothing was written and nothing is.
    pub fn suite_cancel(&mut self) {
        self.suite.pending = None;
    }

    /// Writes the files of a branch into a folder; a file that is there is never written over.
    pub fn suite_extract(&mut self, branch: &str, dir: &Path) -> bool {
        let Some(file) = self.suite.file.clone() else {
            self.suite_fail(S::NeedProject.tr(self.suite_code()).to_owned());
            return false;
        };
        match eh3dfile::extract(&file, branch, dir) {
            Ok(count) => {
                self.status = Status::Suite(Said(
                    S::Extracted,
                    vec![count.to_string(), dir.display().to_string()],
                ));
                true
            }
            Err(e) => {
                self.suite_fail(e);
                false
            }
        }
    }

    /// Asks for the agreement of the person when there is something to agree to.
    fn suite_ask(&mut self, target: &Path, action: &Action) -> Option<Step> {
        let found = suitecore::warnings(
            target,
            self.suite.file.as_deref(),
            self.suite.seen.as_deref(),
            BRANCH,
        );
        match found {
            Ok(warnings) if warnings.is_empty() => None,
            Ok(warnings) => {
                self.suite.pending = Some(Pending {
                    action: action.clone(),
                    warnings,
                });
                Some(Step::Asked)
            }
            Err(e) => Some(self.suite_fail(e)),
        }
    }

    fn suite_run(&mut self, action: Action, agreed: bool) -> Step {
        self.suite.pending = None;
        match &action {
            Action::Save(target) => self.suite_write(target, &action, agreed),
            Action::Import(source) => self.suite_bring(&action, agreed, |w, file| {
                let read = w.read_options().map_err(|s| s.text(w.lang))?;
                let (report, said) = eh3dfile::import(file, source, &Identity::default(), &read)?;
                let rel = eh3dfile::held(file)?.drawing;
                w.suite_reload_drawing(file, rel.as_deref())?;
                Ok((
                    said,
                    suitecore::report_lines(&report),
                    Said(S::Imported, vec![report.source.name]),
                ))
            }),
            Action::Take(branch, name) => self.suite_bring(&action, agreed, |w, file| {
                let read = w.read_options().map_err(|s| s.text(w.lang))?;
                let (target, said) = eh3dfile::take(file, branch, name, &read)?;
                if eh3dfile::held(file)?.drawing.as_deref() == Some(target.as_str()) {
                    w.suite_reload_drawing(file, Some(&target))?;
                }
                Ok((
                    said,
                    Vec::new(),
                    Said(S::Taken, vec![branch.clone(), name.clone()]),
                ))
            }),
        }
    }

    /// Shows the drawing the branch now names.
    fn suite_reload_drawing(&mut self, file: &Path, rel: Option<&str>) -> Result<(), String> {
        let Some(rel) = rel else {
            return Ok(());
        };
        let held = eh3dfile::held(file)?;
        let bytes = held
            .files
            .get(rel)
            .ok_or_else(|| format!("{BRANCH}/{rel}: the drawing is not in the branch"))?
            .clone();
        let lang = self.lang;
        let read = self.read_options().map_err(|s| s.text(lang))?;
        let drawing = read_vector(&beside(file, rel), bytes, &read).map_err(|s| s.text(lang))?;
        self.show_vector(drawing);
        Ok(())
    }

    /// What an import and a take share: a project is needed, the person agrees when there is something to agree to, and after the
    /// write the project is looked at again.
    fn suite_bring(
        &mut self,
        action: &Action,
        agreed: bool,
        work: impl FnOnce(&mut Window, &Path) -> Result<Brought, String>,
    ) -> Step {
        let Some(file) = self.suite.file.clone() else {
            return self.suite_fail(S::NeedProject.tr(self.suite_code()).to_owned());
        };
        if !agreed {
            if let Some(step) = self.suite_ask(&file, action) {
                return step;
            }
        }
        match work(self, &file) {
            Ok((said, notes, status)) => {
                self.suite_written(&file, &said, notes);
                self.status = Status::Suite(status);
                Step::Done
            }
            Err(e) => self.suite_fail(e),
        }
    }

    fn suite_write(&mut self, target: &Path, action: &Action, agreed: bool) -> Step {
        let code = self.suite_code();
        if !suitecore::is_project(target) {
            return self.suite_fail(S::NeedExtension.tr(code).to_owned());
        }
        let (mut contents, notes) = match self.suite_contents() {
            Ok(found) => found,
            Err(e) => return self.suite_fail(e),
        };
        let nothing_held = contents.is_empty();
        let current = self.suite.file.clone();
        let same = current
            .as_deref()
            .is_some_and(|c| suitecore::same_file(c, target));
        let copy_from = current.filter(|c| !target.exists() && c.is_file());
        // what the branch has already, in the file that will be written or in the one that is copied to it
        let base = if target.exists() {
            Some(target.to_path_buf())
        } else {
            copy_from.clone()
        };
        if let Some(base) = &base {
            let found = match suitecore::view(base, BRANCH) {
                Ok(found) => found,
                Err(e) => return self.suite_fail(e),
            };
            if found.branch(BRANCH).is_some() {
                match eh3dfile::held(base) {
                    Ok(held) => drop_unchanged(&mut contents, &held),
                    Err(e) => return self.suite_fail(e),
                }
            }
        }
        if contents.is_empty() && copy_from.is_none() {
            if !same {
                return self.suite_fail(S::NothingToSave.tr(code).to_owned());
            }
            self.suite.notes = notes;
            self.status = Status::Suite(Said::plain(if nothing_held {
                S::NothingToSave
            } else {
                S::NothingNew
            }));
            return Step::Done;
        }
        if !contents.is_empty() && !agreed {
            if let Some(step) = self.suite_ask(target, action) {
                return step;
            }
        }
        let read = match self.read_options() {
            Ok(read) => read,
            Err(s) => {
                self.status = s;
                return Step::Refused;
            }
        };
        if let Some(from) = &copy_from {
            if let Err(e) = suitecore::copy_project(from, target) {
                return self.suite_fail(e);
            }
        }
        let said = if contents.is_empty() {
            Vec::new()
        } else {
            match eh3dfile::save_contents(target, &contents, &Identity::default(), &read) {
                Ok(said) => said,
                Err(e) => {
                    // a copy that could not be written to is not left behind
                    if copy_from.is_some() {
                        let _ = std::fs::remove_file(target);
                    }
                    return self.suite_fail(e);
                }
            }
        };
        self.suite_written(target, &said, notes);
        self.status = Status::Suite(Said(S::Saved, vec![BRANCH.to_owned(), name_of(target)]));
        Step::Done
    }

    /// After a write: the project is the one of the window, its branch is the one that was seen and the notices are told.
    fn suite_written(&mut self, file: &Path, said: &[String], mut notes: Vec<Said>) {
        let (words, other) = suitecore::notices(said);
        notes.extend(words);
        self.suite.file = Some(file.to_path_buf());
        self.suite.view = suitecore::view(file, BRANCH).ok();
        self.suite.seen = self.suite.view.as_ref().and_then(|v| v.own_hash.clone());
        self.suite.notes = notes;
        self.suite.extra = other;
    }
}
