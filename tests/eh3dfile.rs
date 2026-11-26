//! The branch `laser/` of an `.eh3d` project: what is saved comes back byte for byte, every other branch (known or not) keeps its bytes, a copy taken from another branch is told as out of
//! date when its source changes, an imported file is kept with a report, and a damaged file is an error and never a crash. Every project here is made up.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use eh3d_container::{sha256_hex, AtomicOptions};
use eh3d_phoenix_laser::eh3dfile::{self, Identity, Inputs};
use eh3d_phoenix_laser::vector::ReadOptions;
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Seal, Writer};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn example(name: &str) -> String {
    root()
        .join("examples")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// A folder of its own for one test.
fn folder(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("eh3dfile_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const OUTLINE: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40mm\" height=\"30mm\" viewBox=\"0 0 40 30\"><rect x=\"1\" y=\"1\" width=\"38\" height=\"28\" stroke=\"#000000\" fill=\"none\"/></svg>";

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

/// A project with branches of other programs, one this program does not know and an extension folder.
fn synthetic(path: &Path) {
    let new = NewProject {
        id: "00000000-0000-4000-8000-000000000001".into(),
        name: "Synthetic".into(),
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
    write_branch(
        path,
        "cad",
        &[
            ("outline.svg", OUTLINE.as_bytes()),
            ("body.stl", b"solid body\nendsolid body\n"),
            ("notes/readme.txt", b"synthetic"),
        ],
    );
    write_branch(path, "pcb", &[("board.json", b"{\"outline\":[0,0,10,10]}")]);
    write_branch(path, "holography", &[("data.bin", &[0u8, 255, 7, 7, 7])]);
    write_branch(path, "x-vendor", &[("blob", b"anything at all")]);
}

/// Every file of every branch but `laser`, the records included.
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

fn everything() -> Inputs {
    Inputs {
        drawing: Some(example("plate.svg")),
        config: Some(example("plate.cfg")),
        materials: Some(example("materials.cfg")),
        programs: Vec::new(),
    }
}

fn read_options() -> ReadOptions {
    ReadOptions::default()
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_saved_branch_comes_back_byte_for_byte() {
    let dir = folder("roundtrip");
    let file = dir.join("job.eh3d");
    let program = dir.join("plate.nc");
    fs::write(&program, "G21\nG90\nM5\n").unwrap();
    let mut inputs = everything();
    inputs.programs = vec![program.to_string_lossy().into_owned()];
    let who = Identity {
        name: Some("Plate".into()),
        id: Some("plate-1".into()),
    };
    eh3dfile::save(&file, &inputs, &who, &read_options()).unwrap();

    let listing = eh3dfile::list(&file).unwrap();
    assert_eq!(
        (listing.name.as_str(), listing.id.as_str()),
        ("Plate", "plate-1")
    );
    assert_eq!(listing.rows.len(), 1);
    assert_eq!(listing.rows[0].app, "laser");
    assert_eq!(listing.rows[0].format, "1.0");
    assert!(listing.warnings.is_empty(), "{:?}", listing.warnings);

    let out = dir.join("out");
    let opened = eh3dfile::open(&file, Some(&out), &read_options()).unwrap();
    assert_eq!(opened.drawing.as_deref(), Some("drawing/plate.svg"));
    assert_eq!(opened.programs, ["programs/plate.nc"]);
    assert!(opened.shapes.unwrap().0 > 0);
    for (stored, source) in [
        ("drawing/plate.svg", example("plate.svg")),
        ("settings/laser.cfg", example("plate.cfg")),
        ("settings/materials.cfg", example("materials.cfg")),
        ("programs/plate.nc", program.to_string_lossy().into_owned()),
    ] {
        assert_eq!(
            fs::read(out.join(stored)).unwrap(),
            fs::read(source).unwrap(),
            "{stored}"
        );
    }
    assert!(
        Project::verify(&file, &OpenOptions::default(), &Default::default())
            .unwrap()
            .1
            .is_empty()
    );
}

#[test]
fn what_is_not_given_stays_as_it_was() {
    let dir = folder("keep");
    let file = dir.join("job.eh3d");
    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    let only = Inputs {
        drawing: Some(example("plate.dxf")),
        ..Inputs::default()
    };
    eh3dfile::save(&file, &only, &Identity::default(), &read_options()).unwrap();
    let opened = eh3dfile::open(&file, None, &read_options()).unwrap();
    assert_eq!(opened.drawing.as_deref(), Some("drawing/plate.dxf"));
    assert_eq!(opened.settings.as_deref(), Some("settings/laser.cfg"));
    assert_eq!(opened.materials.as_deref(), Some("settings/materials.cfg"));
    let mut project = Project::open(&file, &OpenOptions::default()).unwrap();
    assert!(project.read_entry("laser", "drawing/plate.svg").is_err());
}

#[test]
fn the_other_branches_keep_their_bytes_whatever_this_program_does() {
    let dir = folder("others");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    let before = others(&file);
    assert!(before.contains_key("holography/data.bin"));
    assert!(before.contains_key("x-vendor/blob"));

    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    assert_eq!(others(&file), before);
    eh3dfile::import(
        &file,
        Path::new(&example("plate.dxf")),
        &Identity::default(),
        &read_options(),
    )
    .unwrap();
    assert_eq!(others(&file), before);
    eh3dfile::take(&file, "cad", "outline.svg", &read_options()).unwrap();
    assert_eq!(others(&file), before);

    let listing = eh3dfile::list(&file).unwrap();
    let ids: Vec<&str> = listing.rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["cad", "holography", "laser", "pcb", "x-vendor"]);
    assert!(listing.rows[1]
        .notes
        .iter()
        .any(|n| n.contains("not known")));
}

#[test]
fn a_copy_taken_from_cad_is_told_as_out_of_date_and_taken_again() {
    let dir = folder("stale");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    let (target, _) = eh3dfile::take(&file, "cad", "outline.svg", &read_options()).unwrap();
    assert_eq!(target, "taken/cad/outline.svg");

    let project = Project::open(&file, &OpenOptions::default()).unwrap();
    let record = project.branch("laser").unwrap().record.clone().unwrap();
    assert_eq!(record.depends.len(), 1);
    let dep = &record.depends[0];
    assert_eq!(dep.branch, "cad");
    assert_eq!(dep.hash, project.branch("cad").unwrap().hash);
    assert_eq!(
        (dep.format.as_str(), dep.artifact.as_str()),
        ("1.0", "outline2d")
    );
    assert_eq!(dep.artifact_version, 1);
    assert_eq!(dep.uses, ["taken/cad/outline.svg"]);
    drop(project);
    assert!(eh3dfile::list(&file).unwrap().warnings.is_empty());

    write_branch(&file, "cad", &[("outline.svg", OUTLINE.as_bytes())]);
    let listing = eh3dfile::list(&file).unwrap();
    assert!(listing
        .warnings
        .iter()
        .any(|w| w.starts_with("OUT OF DATE")));
    let laser = listing.rows.iter().find(|r| r.id == "laser").unwrap();
    assert!(laser.notes.contains(&"OUT OF DATE"));

    // saving something else keeps the old state of the reference, so the warning stays
    let only = Inputs {
        config: Some(example("plate.cfg")),
        ..Inputs::default()
    };
    eh3dfile::save(&file, &only, &Identity::default(), &read_options()).unwrap();
    assert!(!eh3dfile::open(&file, None, &read_options())
        .unwrap()
        .warnings
        .is_empty());

    eh3dfile::take(&file, "cad", "outline.svg", &read_options()).unwrap();
    assert!(eh3dfile::list(&file).unwrap().warnings.is_empty());
}

#[test]
fn a_mesh_is_taken_as_a_reference_and_the_drawing_stays() {
    let dir = folder("mesh");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    eh3dfile::take(&file, "cad", "body.stl", &read_options()).unwrap();
    let opened = eh3dfile::open(&file, None, &read_options()).unwrap();
    assert_eq!(opened.drawing.as_deref(), Some("drawing/plate.svg"));
    let project = Project::open(&file, &OpenOptions::default()).unwrap();
    let record = project.branch("laser").unwrap().record.clone().unwrap();
    assert_eq!(record.depends[0].artifact, "mesh3d");
}

#[test]
fn replacing_a_taken_drawing_drops_its_reference() {
    let dir = folder("replace");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    eh3dfile::take(&file, "cad", "outline.svg", &read_options()).unwrap();
    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    let project = Project::open(&file, &OpenOptions::default()).unwrap();
    let laser = project.branch("laser").unwrap();
    assert!(!laser.files.contains_key("taken/cad/outline.svg"));
    assert!(laser.record.as_ref().unwrap().depends.is_empty());
}

#[test]
fn a_take_that_cannot_be_done_changes_nothing() {
    let dir = folder("refuse");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    write_branch(&file, "cad", &[("broken.svg", b"not a drawing at all")]);
    let before = fs::read(&file).unwrap();
    for (branch, rel) in [
        ("laser", "drawing/plate.svg"),
        ("cad", "missing.svg"),
        ("nowhere", "outline.svg"),
        ("cad", "broken.svg"),
    ] {
        assert!(
            eh3dfile::take(&file, branch, rel, &read_options()).is_err(),
            "{branch}/{rel}"
        );
        assert_eq!(fs::read(&file).unwrap(), before, "{branch}/{rel}");
    }
}

#[test]
fn an_imported_project_is_kept_with_its_report() {
    let dir = folder("lbrn");
    let file = dir.join("job.eh3d");
    let source = dir.join("axes.lbrn2");
    let source_text = source.to_string_lossy().into_owned();
    let (ok, _, err) = run(&["ytest", &source_text]);
    assert!(ok, "{err}");
    let original = fs::read(&source).unwrap();

    let (report, _) =
        eh3dfile::import(&file, &source, &Identity::default(), &read_options()).unwrap();
    assert_eq!(report.source.format, "lbrn2");
    assert_eq!(report.source.sha256, sha256_hex(&original));
    assert_eq!(report.source.size, original.len() as u64);
    assert!(report
        .read
        .iter()
        .any(|c| c.item == "shapes" && c.count > 0));
    assert!(!report.export_losses.is_empty());

    let out = dir.join("out");
    let opened = eh3dfile::open(&file, Some(&out), &read_options()).unwrap();
    assert_eq!(opened.drawing.as_deref(), Some("origin/axes.lbrn2"));
    assert_eq!(opened.report.as_ref(), Some(&report));
    assert_eq!(fs::read(out.join("origin/axes.lbrn2")).unwrap(), original);
    assert!(opened.warnings.is_empty(), "{:?}", opened.warnings);

    // a second import replaces the first and says so
    let (_, notes) = eh3dfile::import(
        &file,
        Path::new(&example("plate.dxf")),
        &Identity::default(),
        &read_options(),
    )
    .unwrap();
    assert!(notes.iter().any(|n| n.contains("replaced")), "{notes:?}");
    let project = Project::open(&file, &OpenOptions::default()).unwrap();
    let files = &project.branch("laser").unwrap().files;
    assert!(!files.contains_key("origin/axes.lbrn2"));
    assert!(files.contains_key("origin/plate.dxf"));
}

#[test]
fn a_drawing_goes_in_and_out_with_the_same_shapes() {
    let dir = folder("shapes");
    for name in ["plate.svg", "plate.dxf"] {
        let file = dir.join(format!("{name}.eh3d"));
        let (report, _) = eh3dfile::import(
            &file,
            Path::new(&example(name)),
            &Identity::default(),
            &read_options(),
        )
        .unwrap();
        let out = dir.join(format!("{name}.out"));
        eh3dfile::open(&file, Some(&out), &read_options()).unwrap();
        let back = out.join("origin").join(name);
        assert_eq!(fs::read(&back).unwrap(), fs::read(example(name)).unwrap());
        let written = dir.join(format!("{name}.converted.svg"));
        let (ok, text, err) = run(&[
            "convert",
            &back.to_string_lossy(),
            &written.to_string_lossy(),
        ]);
        assert!(ok, "{err}");
        let shapes = report
            .read
            .iter()
            .find(|c| c.item == "shapes")
            .unwrap()
            .count;
        assert!(text.starts_with(&format!("{shapes} shapes")), "{text}");
    }
}

#[test]
fn a_branch_of_a_newer_format_or_sealed_is_not_written() {
    let dir = folder("newer");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    let project = Project::open(&file, &OpenOptions::default()).unwrap();
    let update = BranchUpdate::new("laser", "2.0", "9.0.0").with_entry("laser.json", b"{}");
    project
        .save_branch(&Writer::for_app("laser"), update, &SaveOptions::default())
        .unwrap();
    let before = fs::read(&file).unwrap();
    let error =
        eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap_err();
    assert!(error.contains("EH3D-PKG-024"), "{error}");
    assert_eq!(fs::read(&file).unwrap(), before);

    let sealed = dir.join("sealed.eh3d");
    synthetic(&sealed);
    let project = Project::open(&sealed, &OpenOptions::default()).unwrap();
    let mut update = BranchUpdate::new("laser", "1.0", "0.1.0").with_entry("laser.json", b"{}");
    update.seal = Some(Seal {
        by: "test".into(),
        at: "never".into(),
        reason: None,
    });
    project
        .save_branch(&Writer::for_app("laser"), update, &SaveOptions::default())
        .unwrap();
    let before = fs::read(&sealed).unwrap();
    let error = eh3dfile::save(
        &sealed,
        &everything(),
        &Identity::default(),
        &read_options(),
    )
    .unwrap_err();
    assert!(error.contains("EH3D-PKG-020"), "{error}");
    assert_eq!(fs::read(&sealed).unwrap(), before);
}

#[test]
fn fields_and_files_this_version_does_not_know_stay() {
    let dir = folder("unknown_fields");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    let project = Project::open(&file, &OpenOptions::default()).unwrap();
    let mut update = BranchUpdate::new("laser", "1.0", "0.9.0")
        .with_entry("laser.json", b"{\"future\":{\"k\":1}}")
        .with_entry("later/thing.bin", &[1, 2, 3]);
    update
        .extra
        .insert("hint".into(), serde_json::Value::from("kept"));
    project
        .save_branch(&Writer::for_app("laser"), update, &SaveOptions::default())
        .unwrap();
    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    let mut project = Project::open(&file, &OpenOptions::default()).unwrap();
    assert_eq!(
        project.read_entry("laser", "later/thing.bin").unwrap(),
        [1, 2, 3]
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&project.read_entry("laser", "laser.json").unwrap()).unwrap();
    assert_eq!(manifest["future"]["k"], 1);
    let record = project.branch("laser").unwrap().record.clone().unwrap();
    assert_eq!(record.extra["hint"], "kept");
}

#[test]
fn damaged_projects_are_errors_and_never_a_crash() {
    let dir = folder("damaged");
    let file = dir.join("job.eh3d");
    synthetic(&file);
    eh3dfile::save(&file, &everything(), &Identity::default(), &read_options()).unwrap();
    let good = fs::read(&file).unwrap();
    // a fixed seed: the same damage every run
    let mut seed: u64 = 0x5EED_1A5E;
    let mut next = move |limit: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % limit.max(1) as u64) as usize
    };
    let broken = dir.join("broken.eh3d");
    for round in 0..150 {
        let mut bytes = good.clone();
        match round % 3 {
            0 => {
                for _ in 0..=next(8) {
                    let at = next(bytes.len());
                    bytes[at] ^= 1 << next(8);
                }
            }
            1 => bytes.truncate(next(bytes.len())),
            _ => {
                let at = next(bytes.len());
                let tail = bytes.split_off(at);
                bytes.extend((0..next(64)).map(|i| (i * 37) as u8));
                bytes.extend(tail);
            }
        }
        fs::write(&broken, &bytes).unwrap();
        let _ = eh3dfile::list(&broken);
        let _ = eh3dfile::open(&broken, None, &read_options());
        let _ = eh3dfile::take(&broken, "cad", "outline.svg", &read_options());
        let _ = eh3dfile::save(
            &broken,
            &everything(),
            &Identity::default(),
            &read_options(),
        );
    }
    fs::write(&broken, b"this is not a project").unwrap();
    assert!(eh3dfile::list(&broken).is_err());
}

#[test]
fn the_command_saves_lists_opens_and_extracts() {
    let dir = folder("cli");
    let file = dir.join("job.eh3d").to_string_lossy().into_owned();
    let drawing = example("plate.svg");
    let (ok, _, err) = run(&[
        "project",
        "save",
        &file,
        "--drawing",
        &drawing,
        "--name",
        "N",
    ]);
    assert!(ok, "{err}");
    let (ok, text, err) = run(&["project", "list", &file]);
    assert!(ok, "{err}");
    assert!(
        text.contains("laser") && text.contains("1 branch(es)"),
        "{text}"
    );
    let (ok, text, _) = run(&["project", "list", &file, "--json"]);
    assert!(ok);
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(document["branches"][0]["id"], "laser");

    let into = dir.join("into").to_string_lossy().into_owned();
    let (ok, text, err) = run(&["project", "open", &file, "--into", &into]);
    assert!(ok, "{err}");
    assert!(text.contains("drawing/plate.svg"), "{text}");
    // what is there is never written over
    let (ok, _, _) = run(&["project", "extract", &file, "laser", &into]);
    assert!(!ok);
    let other = dir.join("other").to_string_lossy().into_owned();
    let (ok, _, err) = run(&["project", "extract", &file, "laser", &other]);
    assert!(ok, "{err}");

    let (ok, _, err) = run(&[
        "project",
        "save",
        &file,
        "--drawing",
        &drawing,
        "--name",
        "M",
    ]);
    assert!(!ok);
    assert!(err.contains("--name"), "{err}");
}

#[test]
fn a_wrong_request_creates_nothing() {
    let dir = folder("wrong");
    let text = |name: &str| dir.join(name).to_string_lossy().into_owned();
    let drawing = example("plate.svg");
    let (file, other, junk) = (text("a.eh3d"), text("a.svg"), text("junk.svg"));
    fs::write(&junk, "garbage").unwrap();
    for args in [
        vec!["project"],
        vec!["project", "melt", &file],
        vec!["project", "save", &file],
        vec!["project", "save", &other, "--drawing", &drawing],
        vec!["project", "save", &file, "--drawing", &junk],
        vec!["project", "save", &file, "--drawing", &drawing, "--oops"],
        vec!["project", "import", &file, &junk],
        vec!["project", "open", &file],
        vec!["project", "take", &file, "cad", "outline.svg"],
        vec!["project", "list"],
    ] {
        let (ok, _, _) = run(&args);
        assert!(!ok, "{args:?}");
        assert!(!Path::new(&file).exists(), "{args:?}");
        assert!(!Path::new(&other).exists(), "{args:?}");
    }
}
