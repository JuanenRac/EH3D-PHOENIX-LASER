// EH3D-PHOENIX-LASER - the command `project`.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.

use std::path::{Path, PathBuf};

use eh3d_phoenix_laser::eh3dfile::{self, Identity, Inputs};
use eh3d_phoenix_laser::vconfig;
use eh3d_phoenix_laser::vector::ReadOptions;

use crate::cli_args::{finish, flag, switch};

/// The settings of how a drawing is read: the `[read]` section of `--config`, or the defaults.
fn read_options(config: Option<&str>) -> Result<ReadOptions, String> {
    Ok(match config {
        Some(path) => vconfig::load(Path::new(path))?.read,
        None => ReadOptions::default(),
    })
}

fn identity(args: &mut Vec<String>) -> Result<Identity, String> {
    Ok(Identity {
        name: flag(args, "--name")?,
        id: flag(args, "--id")?,
    })
}

fn notes(list: &[String]) {
    for note in list {
        eprintln!("note: {note}");
    }
}

fn words<const N: usize>(args: &[String], what: &str) -> Result<[String; N], String> {
    finish(args, N)?;
    <[String; N]>::try_from(args.to_vec()).map_err(|_| format!("project {what} (see help)"))
}

/// `phoenix-laser project list|save|import|open|take|extract FILE.eh3d ...`
pub fn project_job(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() {
        return Err("project needs list, save, import, open, take or extract (see help)".into());
    }
    let what = args.remove(0);
    match what.as_str() {
        "list" => {
            let json = switch(&mut args, "--json");
            let [file] = words::<1>(&args, "list needs FILE.eh3d")?;
            let listing = eh3dfile::list(Path::new(&file))?;
            if json {
                let rows: Vec<serde_json::Value> = listing
                    .rows
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "id": r.id, "app": r.app, "format": r.format, "app_version": r.app_version,
                            "files": r.files, "hash": r.hash, "notes": r.notes,
                        })
                    })
                    .collect();
                let document = serde_json::json!({
                    "name": listing.name, "id": listing.id, "branches": rows, "warnings": listing.warnings,
                });
                println!("{document:#}");
            } else {
                println!("project {} ({})", listing.name, listing.id);
                for r in &listing.rows {
                    println!(
                        "  {:<16} {:>4} files  format {:<6} {} {}  {}",
                        r.id,
                        r.files,
                        r.format,
                        r.app,
                        r.app_version,
                        r.notes.join(", ")
                    );
                }
                println!("{} branch(es)", listing.rows.len());
                notes(&listing.warnings);
            }
            Ok(())
        }
        "save" => {
            let mut inputs = Inputs {
                drawing: flag(&mut args, "--drawing")?,
                config: flag(&mut args, "--config")?,
                materials: flag(&mut args, "--materials")?,
                programs: Vec::new(),
            };
            while let Some(program) = flag(&mut args, "--program")? {
                inputs.programs.push(program);
            }
            let who = identity(&mut args)?;
            let read = read_options(inputs.config.as_deref())?;
            let [file] = words::<1>(&args, "save needs FILE.eh3d")?;
            notes(&eh3dfile::save(Path::new(&file), &inputs, &who, &read)?);
            println!("the branch laser of {file} was saved");
            Ok(())
        }
        "import" => {
            let config = flag(&mut args, "--config")?;
            let read = read_options(config.as_deref())?;
            let who = identity(&mut args)?;
            let [file, source] = words::<2>(&args, "import needs FILE.eh3d and a drawing")?;
            let (report, said) =
                eh3dfile::import(Path::new(&file), Path::new(&source), &who, &read)?;
            for c in &report.read {
                println!("read: {} {}", c.count, c.item);
            }
            for c in &report.not_read {
                println!(
                    "not read: {} {} ({})",
                    c.count,
                    c.item,
                    c.reason.as_deref().unwrap_or("-")
                );
            }
            for c in &report.export_losses {
                println!("lost on export: {}", c.reason.as_deref().unwrap_or(&c.item));
            }
            notes(&said);
            println!(
                "{} was imported into {file}; the original is kept",
                report.source.name
            );
            Ok(())
        }
        "open" => {
            let config = flag(&mut args, "--config")?;
            let read = read_options(config.as_deref())?;
            let into = flag(&mut args, "--into")?.map(PathBuf::from);
            let [file] = words::<1>(&args, "open needs FILE.eh3d")?;
            let opened = eh3dfile::open(Path::new(&file), into.as_deref(), &read)?;
            match (&opened.drawing, opened.shapes) {
                (Some(name), Some((shapes, layers))) => {
                    println!("drawing   {name}: {shapes} shapes in {layers} layer(s)");
                }
                _ => println!("drawing   none"),
            }
            println!("settings  {}", opened.settings.as_deref().unwrap_or("none"));
            println!(
                "materials {}",
                opened.materials.as_deref().unwrap_or("none")
            );
            println!("programs  {}", opened.programs.len());
            for program in &opened.programs {
                println!("  {program}");
            }
            if let Some(report) = &opened.report {
                println!(
                    "imported  {} ({}), {} item(s) not read",
                    report.source.name,
                    report.source.format,
                    report.not_read.len()
                );
            }
            if let (Some(count), Some(dir)) = (opened.written, &into) {
                println!("{count} files written to {}", dir.display());
            }
            notes(&opened.warnings);
            Ok(())
        }
        "take" => {
            let config = flag(&mut args, "--config")?;
            let read = read_options(config.as_deref())?;
            let [file, branch, name] =
                words::<3>(&args, "take needs FILE.eh3d, a branch and a file of it")?;
            let (target, said) = eh3dfile::take(Path::new(&file), &branch, &name, &read)?;
            notes(&said);
            println!("{branch}/{name} was taken into laser/{target}");
            Ok(())
        }
        "extract" => {
            let [file, branch, dir] =
                words::<3>(&args, "extract needs FILE.eh3d, a branch and a folder")?;
            let count = eh3dfile::extract(Path::new(&file), &branch, Path::new(&dir))?;
            println!("{count} files written to {dir}");
            Ok(())
        }
        other => Err(format!(
            "project {other}: it is list, save, import, open, take or extract"
        )),
    }
}
