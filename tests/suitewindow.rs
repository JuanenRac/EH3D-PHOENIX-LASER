//! What the menu File of the window does with a project file of the suite (`.eh3d`), without a screen: open, save, save under another
//! name, import, take from another branch and extract, what is asked before a write and what is told afterwards. Every project here is
//! made up.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use eh3d_container::AtomicOptions;
use eh3d_phoenix_laser::eh3dfile::{self, Identity, Inputs};
use eh3d_phoenix_laser::lang::Lang;
use eh3d_phoenix_laser::suitecore;
use eh3d_phoenix_laser::suitetext::S;
use eh3d_phoenix_laser::suitewin::{Source, Step};
use eh3d_phoenix_laser::vector::ReadOptions;
use eh3d_phoenix_laser::window::{Job, Mode, Status, Window};
use eh3d_project::{
    BranchUpdate, Ed25519Signer, NewProject, OpenOptions, Project, SaveOptions, Writer,
};

const OUTLINE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40mm\" height=\"30mm\" viewBox=\"0 0 40 30\"><rect x=\"1\" y=\"1\" width=\"38\" height=\"28\" stroke=\"#000000\" fill=\"none\"/></svg>";
const OTHER: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20mm\" height=\"20mm\" viewBox=\"0 0 20 20\"><rect x=\"2\" y=\"2\" width=\"16\" height=\"16\" stroke=\"#ff0000\" fill=\"none\"/></svg>";

fn folder(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("suitewin_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

fn write_branch(path: &Path, branch: &str, entries: &[(&str, &[u8])]) {
    let project = Project::open(path, &OpenOptions::default()).unwrap();
    let mut update = BranchUpdate::new(branch, "1.0", "0.1.0");
    for (rel, bytes) in entries {
        update = update.with_entry(rel, bytes);
    }
    let writer = Writer::new(branch, &[branch]);
    project
        .save_branch(&writer, update, &SaveOptions::default())
        .unwrap();
}

/// A project with the branches of other programs and none of this one.
fn foreign(path: &Path) {
    let new = NewProject {
        id: "00000000-0000-4000-8000-000000000002".into(),
        name: "Made up".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        path,
        new,
        &OpenOptions::default(),
        &AtomicOptions::default(),
    )
    .unwrap();
    write_branch(path, "cad", &[("outline.svg", OUTLINE.as_bytes())]);
    write_branch(path, "pcb", &[("board.json", b"{}")]);
    write_branch(path, "x-vendor", &[("blob", b"anything")]);
}

/// Every file of every branch but `laser`.
fn others(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut project = Project::open(path, &OpenOptions::default()).unwrap();
    let names: Vec<(String, String)> = project
        .branches()
        .filter(|b| b.id != "laser")
        .flat_map(|b| b.files.keys().map(|f| (b.id.clone(), f.clone())))
        .collect();
    names
        .into_iter()
        .map(|(b, f)| (format!("{b}/{f}"), project.read_entry(&b, &f).unwrap()))
        .collect()
}

fn window_with_drawing(dir: &Path) -> Window {
    let mut w = Window::new(Lang::En);
    assert!(w.run_job(Job::OpenImage, &file(dir, "plate.svg", OUTLINE)));
    w
}

fn words(w: &Window) -> Vec<S> {
    w.suite.notes.iter().map(|n| n.0).collect()
}

fn warned(w: &Window) -> Vec<S> {
    let pending = w.suite.pending.as_ref().expect("a write waits");
    pending.warnings.iter().map(|n| n.0).collect()
}

fn said(w: &Window) -> S {
    match &w.status {
        Status::Suite(said) => said.0,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_new_project_is_saved_with_the_work_and_opened_again() {
    let dir = folder("new");
    let mut w = window_with_drawing(&dir);
    assert_eq!(w.suite_save(), Step::NeedsName);
    assert_eq!(w.suite_suggested_name(), "plate.eh3d");
    assert!(w.suite_banner().is_empty());
    assert_eq!(w.suite_save_as(&dir.join("job.zip")), Step::Refused);
    assert_eq!(w.suite_save_as(&dir.join("job")), Step::Done);
    let project = dir.join("job.eh3d");
    assert_eq!(w.suite.file.as_deref(), Some(project.as_path()));
    assert_eq!(said(&w), S::Saved);
    assert!(w.suite_banner()[0].0.contains("job.eh3d"));
    // nothing changed: nothing is written
    let before = fs::read(&project).unwrap();
    assert_eq!(w.suite_save(), Step::Done);
    assert_eq!(said(&w), S::NothingNew);
    assert_eq!(fs::read(&project).unwrap(), before);
    // the program that is worked out goes with the next save
    assert!(w.compute(), "{:?}", w.status);
    assert_eq!(w.suite_save(), Step::Done);
    let held = eh3dfile::held(&project).unwrap();
    assert_eq!(held.programs, ["programs/plate.nc"]);
    assert_eq!(held.settings, None);
    assert_eq!(held.files["drawing/plate.svg"], OUTLINE.as_bytes());

    let mut again = Window::new(Lang::Es);
    assert!(
        again.run_job(Job::SuiteOpen, &project),
        "{:?}",
        again.status
    );
    assert_eq!(again.mode, Mode::Cut);
    assert_eq!(again.vector.as_ref().unwrap().name, "plate.svg");
    assert_eq!(said(&again), S::Opened);
    assert!(again.status.text(Lang::Es).contains("job.eh3d"));
}

#[test]
fn a_project_of_other_programs_is_taken_as_it_is_and_keeps_its_branches() {
    let dir = folder("foreign");
    let project = dir.join("job.eh3d");
    foreign(&project);
    let before = others(&project);
    let bytes = fs::read(&project).unwrap();
    // what the window held before does not go into the project that is opened
    let mut w = window_with_drawing(&dir);
    assert!(w.suite_open(&project), "{:?}", w.status);
    assert_eq!(said(&w), S::NoWork);
    assert!(w.vector.is_none());
    assert_eq!(w.suite_save(), Step::Done);
    assert_eq!(said(&w), S::NothingToSave);
    assert_eq!(fs::read(&project).unwrap(), bytes);
    assert!(w.run_job(Job::OpenImage, &file(&dir, "other.svg", OTHER)));
    assert_eq!(w.suite_save(), Step::Done);
    assert_eq!(others(&project), before);
    assert_eq!(
        eh3dfile::held(&project).unwrap().drawing.as_deref(),
        Some("drawing/other.svg")
    );
}

#[test]
fn saving_under_another_name_keeps_every_branch_and_another_project_is_asked_about() {
    let dir = folder("saveas");
    let first = dir.join("first.eh3d");
    foreign(&first);
    let mut w = Window::new(Lang::En);
    assert!(w.suite_open(&first));
    assert!(w.run_job(Job::OpenImage, &file(&dir, "plate.svg", OUTLINE)));
    assert_eq!(w.suite_save(), Step::Done);
    let second = dir.join("second.eh3d");
    assert_eq!(w.suite_save_as(&second), Step::Done);
    assert_eq!(others(&second), others(&first));
    assert_eq!(fs::read(&second).unwrap(), fs::read(&first).unwrap());
    assert_eq!(w.suite.file.as_deref(), Some(second.as_path()));

    let theirs = dir.join("theirs.eh3d");
    foreign(&theirs);
    let bytes = fs::read(&theirs).unwrap();
    assert_eq!(w.suite_save_as(&theirs), Step::Asked);
    assert_eq!(warned(&w), [S::WarnForeign]);
    assert_eq!(fs::read(&theirs).unwrap(), bytes);
    w.suite_cancel();
    assert!(w.suite.pending.is_none());
    assert_eq!(w.suite_confirm(), Step::Refused);
    assert_eq!(fs::read(&theirs).unwrap(), bytes);
    let kept = others(&theirs);
    assert!(w.run_job(Job::SuiteSaveAs, &theirs));
    assert_eq!(w.suite_confirm(), Step::Done);
    assert_eq!(others(&theirs), kept);
    assert_eq!(w.suite.file.as_deref(), Some(theirs.as_path()));
}

#[test]
fn a_branch_that_changed_or_is_signed_is_asked_about_and_the_signature_is_told() {
    let dir = folder("signed");
    let project = dir.join("job.eh3d");
    let mut w = window_with_drawing(&dir);
    assert_eq!(w.suite_save_as(&project), Step::Done);

    // another window writes the branch
    let inputs = Inputs {
        drawing: Some(file(&dir, "other.svg", OTHER).display().to_string()),
        ..Inputs::default()
    };
    eh3dfile::save(
        &project,
        &inputs,
        &Identity::default(),
        &ReadOptions::default(),
    )
    .unwrap();
    let bytes = fs::read(&project).unwrap();
    assert_eq!(w.suite_save(), Step::Asked);
    assert!(warned(&w).contains(&S::WarnChanged));
    assert_eq!(fs::read(&project).unwrap(), bytes);
    assert_eq!(w.suite_confirm(), Step::Done);

    let key = Ed25519Signer::from_seed(&[9; 32]);
    Project::open(&project, &OpenOptions::default())
        .unwrap()
        .sign_branch("laser", &key, &SaveOptions::default())
        .unwrap();
    w.suite_refresh();
    assert!(w
        .suite_banner()
        .iter()
        .any(|(line, attention)| *attention && line.contains("is signed")));
    assert!(w.compute(), "{:?}", w.status);
    let bytes = fs::read(&project).unwrap();
    assert_eq!(w.suite_save(), Step::Asked);
    assert!(warned(&w).contains(&S::SignedBranch));
    assert_eq!(fs::read(&project).unwrap(), bytes);
    assert_eq!(w.suite_confirm(), Step::Done);
    assert!(
        words(&w).contains(&S::NoticeBranchSignature),
        "{:?}",
        w.suite
    );
    let view = suitecore::view(&project, "laser").unwrap();
    assert!(!view.branch("laser").unwrap().signed);
}

#[test]
fn a_file_taken_from_cad_is_told_as_out_of_date_and_a_save_keeps_its_reference() {
    let dir = folder("take");
    let project = dir.join("job.eh3d");
    foreign(&project);
    let mut w = Window::new(Lang::En);
    assert_eq!(w.suite_take("cad", "outline.svg"), Step::Refused);
    assert!(w.suite_open(&project));
    let view = w.suite.view.clone().unwrap();
    assert!(view
        .takeable
        .contains(&("cad".to_owned(), "outline.svg".to_owned())));
    assert_eq!(w.suite_take("cad", "outline.svg"), Step::Done);
    assert_eq!(said(&w), S::Taken);
    assert_eq!(w.vector.as_ref().unwrap().name, "outline.svg");
    write_branch(&project, "cad", &[("outline.svg", OTHER.as_bytes())]);
    w.suite_refresh();
    assert!(
        w.suite_banner().iter().any(|(line, attention)| *attention
            && line.starts_with("OUT OF DATE: laser was derived from cad")),
        "{:?}",
        w.suite_banner()
    );
    let view = w.suite.view.clone().unwrap();
    let state = suitecore::state_text(view.branch("laser").unwrap(), "en");
    assert_eq!(state, "out of date: cad has changed");
    assert_eq!(w.suite_save(), Step::Done);
    assert_eq!(said(&w), S::NothingNew);
    assert_eq!(
        eh3dfile::held(&project).unwrap().drawing.as_deref(),
        Some("taken/cad/outline.svg")
    );
    // taking it again brings it up to date
    assert_eq!(w.suite_take("cad", "outline.svg"), Step::Done);
    assert!(w.suite.view.as_ref().unwrap().stale.is_empty());
}

#[test]
fn an_import_keeps_the_original_and_a_branch_is_extracted_without_writing_over() {
    let dir = folder("import");
    let source = file(&dir, "part.svg", OTHER);
    let mut w = window_with_drawing(&dir);
    assert_eq!(w.suite_import(&source), Step::Refused);
    let project = dir.join("job.eh3d");
    assert_eq!(w.suite_save_as(&project), Step::Done);
    assert_eq!(w.suite_import(&source), Step::Done, "{:?}", w.status);
    assert_eq!(said(&w), S::Imported);
    assert_eq!(words(&w).first(), Some(&S::Original));
    assert_eq!(w.vector.as_ref().unwrap().name, "part.svg");
    let held = eh3dfile::held(&project).unwrap();
    assert_eq!(held.files["origin/part.svg"], OTHER.as_bytes());

    w.suite.extract_branch = "laser".to_owned();
    assert_eq!(w.suite_extract_name(), "job-laser");
    let out = dir.join("job-laser");
    assert!(w.run_job(Job::SuiteExtract, &out), "{:?}", w.status);
    assert_eq!(said(&w), S::Extracted);
    assert_eq!(
        fs::read(out.join("origin/part.svg")).unwrap(),
        OTHER.as_bytes()
    );
    fs::write(out.join("origin/part.svg"), "mine").unwrap();
    assert!(!w.suite_extract("laser", &out));
    assert_eq!(fs::read(out.join("origin/part.svg")).unwrap(), b"mine");
}

#[test]
fn a_project_never_loosens_the_limits_of_the_person_and_no_token_goes_into_one() {
    let dir = folder("limits");
    let theirs = file(
        &dir,
        "theirs.cfg",
        "[machine]\nmax_power_percent = 90\nmax_speed_mm_min = 20000\n[server]\nserver_address = 10.0.0.9:18200\n",
    );
    let project = dir.join("job.eh3d");
    let inputs = Inputs {
        config: Some(theirs.display().to_string()),
        ..Inputs::default()
    };
    eh3dfile::save(
        &project,
        &inputs,
        &Identity::default(),
        &ReadOptions::default(),
    )
    .unwrap();
    let mine = file(&dir, "mine.cfg", "[machine]\nmax_power_percent = 50\n");
    let mut w = Window::new(Lang::En);
    assert!(w.run_job(Job::OpenConfig, &mine));
    assert_eq!(w.suite.settings, Source::Chosen);
    let address = w.settings.server.clone();
    assert!(w.suite_open(&project), "{:?}", w.status);
    assert_eq!(w.settings.machine.max_power_percent, 50.0);
    assert_eq!(w.settings.machine.max_speed_mm_min, 20000.0);
    assert!(w.settings.server == address);
    assert!(words(&w).contains(&S::LimitsKept));
    assert!(matches!(w.suite.settings, Source::Kept(_)));
    // the settings of the project stay in it as they were, and those of the person come back with another project
    let bytes = fs::read(&project).unwrap();
    assert_eq!(w.suite_save(), Step::Done);
    assert_eq!(fs::read(&project).unwrap(), bytes);
    let empty = dir.join("empty.eh3d");
    foreign(&empty);
    assert!(w.suite_open(&empty));
    assert_eq!(w.settings.machine.max_speed_mm_min, 30000.0);
    assert_eq!(w.settings.machine.max_power_percent, 50.0);
    assert_eq!(w.suite.settings, Source::Nothing);

    // a settings file the window read by itself stays out, and so does one with a token
    let mut quiet = window_with_drawing(&dir);
    assert!(quiet.load_config(&mine));
    assert_eq!(quiet.suite_save_as(&dir.join("quiet.eh3d")), Step::Done);
    assert_eq!(
        eh3dfile::held(&dir.join("quiet.eh3d")).unwrap().settings,
        None
    );
    let sample = ["abc", "123"].concat();
    let with_token = file(
        &dir,
        "token.cfg",
        &format!("[server]\nserver_token = {sample}\n"),
    );
    assert!(quiet.run_job(Job::OpenConfig, &with_token));
    assert_eq!(quiet.suite_save_as(&dir.join("token.eh3d")), Step::Done);
    assert!(words(&quiet).contains(&S::SettingsWithToken));
    assert_eq!(
        eh3dfile::held(&dir.join("token.eh3d")).unwrap().settings,
        None
    );
}

#[test]
fn a_damaged_project_leaves_the_window_as_it_was() {
    let dir = folder("damaged");
    let junk = file(&dir, "junk.eh3d", "not a project");
    let mut w = window_with_drawing(&dir);
    assert!(!w.suite_open(&junk));
    assert!(matches!(w.status, Status::Failed(_)));
    assert!(w.vector.is_some() && w.suite.file.is_none());
    assert!(!w.open_dropped(&junk));
    assert_eq!(w.suite_save_as(&junk), Step::Refused);
    assert_eq!(fs::read(&junk).unwrap(), b"not a project");
}
