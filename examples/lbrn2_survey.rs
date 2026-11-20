//! Reads every `.lbrn2` file under a folder with the reader of the program and says which are read, which are not and why, and what the files hold that the reader leaves out. It only
//! reads the files and writes nothing but its report, so the files can stay where they are (they are never copied).
//!
//! `cargo run --release --example lbrn2_survey -- FOLDER [--out REPORT.txt]`
//!
//! The exit code is 0 when every file was read, 2 when one was not, and 1 for a wrong command line.

use eh3d_phoenix_laser::lbrn2::{inventory, read_project, Inventory};
use eh3d_phoenix_laser::vector::{bounds_of, ReadOptions};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

const USAGE: &str = "usage: lbrn2_survey FOLDER [--out REPORT.txt]";

fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out, depth + 1);
        } else if p
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lbrn2"))
        {
            out.push(p);
        }
    }
}

fn merge(total: &mut Vec<(String, usize)>, part: &[(String, usize)]) {
    for (k, n) in part {
        match total.iter_mut().find(|(t, _)| t == k) {
            Some((_, m)) => *m += n,
            None => total.push((k.clone(), *n)),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut folder: Option<String> = None;
    let mut out: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = it.next().cloned(),
            other if folder.is_none() && !other.starts_with("--") => folder = Some(other.into()),
            other => {
                eprintln!("error: {other}: {USAGE}");
                std::process::exit(1);
            }
        }
    }
    let Some(folder) = folder else {
        eprintln!("error: {USAGE}");
        std::process::exit(1);
    };
    let mut files = Vec::new();
    collect(Path::new(&folder), &mut files, 0);
    let opts = ReadOptions::default();
    let mut report = String::new();
    let (mut read, mut failed) = (0usize, 0usize);
    let mut all = Inventory::default();
    let mut skipped_total: Vec<(String, usize)> = Vec::new();
    let mut unknown_total: Vec<(String, usize)> = Vec::new();
    let mut types_total: Vec<(String, usize)> = Vec::new();
    let _ = writeln!(report, "files found: {}", files.len());
    for f in &files {
        let name = f
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let bytes = match std::fs::read(f) {
            Ok(b) => b,
            Err(e) => {
                failed += 1;
                let _ = writeln!(report, "\n{name}: NOT READ: {e}");
                continue;
            }
        };
        let started = Instant::now();
        let result = read_project(&bytes, &opts);
        let ms = started.elapsed().as_millis();
        match result {
            Err(e) => {
                failed += 1;
                let _ = writeln!(report, "\n{name}: NOT READ ({} bytes): {e}", bytes.len());
            }
            Ok(p) => {
                read += 1;
                let d = &p.drawing;
                let points: usize = d.shapes.iter().map(|s| s.pts.len()).sum();
                let b = bounds_of(d.shapes.iter().flat_map(|s| s.pts.iter().copied()));
                let _ = writeln!(
                    report,
                    "\n{name}: read ({} bytes, {ms} ms), app {} form {} device \"{}\"",
                    bytes.len(),
                    p.app_version,
                    p.format_version,
                    p.device_name
                );
                let _ = writeln!(
                    report,
                    "  layers: {} | drawn shapes: {} | outlines: {} | points: {points} | vertices used: {} | lines {} | beziers {} | lists shared {}",
                    p.cut_settings.len(),
                    p.stats.read_shapes,
                    d.shapes.len(),
                    p.stats.vertices,
                    p.stats.lines,
                    p.stats.beziers,
                    p.stats.shared_lists
                );
                if let Some((lo, hi)) = b {
                    let _ = writeln!(
                        report,
                        "  size of the drawing: {:.1} x {:.1} mm",
                        hi.0 - lo.0,
                        hi.1 - lo.1
                    );
                }
                let _ = writeln!(report, "  shape types: {:?}", p.stats.shape_types);
                if !p.stats.skipped.is_empty() {
                    let _ = writeln!(report, "  left out: {:?}", p.stats.skipped);
                }
                if !p.stats.unknown_parts.is_empty() {
                    let _ = writeln!(
                        report,
                        "  unknown parts inside shapes: {:?}",
                        p.stats.unknown_parts
                    );
                }
                for w in &d.warnings {
                    let _ = writeln!(report, "  note: {w}");
                }
                merge(&mut skipped_total, &p.stats.skipped);
                merge(&mut unknown_total, &p.stats.unknown_parts);
                merge(&mut types_total, &p.stats.shape_types);
            }
        }
        if let Ok(inv) = inventory(&bytes, &opts) {
            merge(&mut all.elements, &inv.elements);
            merge(&mut all.attributes, &inv.attributes);
        }
    }
    let _ = writeln!(report, "\n== summary ==");
    let _ = writeln!(report, "read: {read} | not read: {failed}");
    let _ = writeln!(report, "shape types seen: {types_total:?}");
    let _ = writeln!(report, "left out by reason: {skipped_total:?}");
    let _ = writeln!(
        report,
        "parts inside shapes that the reader does not know: {unknown_total:?}"
    );
    let un_e: Vec<String> = all
        .unread_elements()
        .iter()
        .map(|(k, n)| format!("{k} x{n}"))
        .collect();
    let un_a: Vec<String> = all
        .unread_attributes()
        .iter()
        .map(|(k, n)| format!("{k} x{n}"))
        .collect();
    let _ = writeln!(
        report,
        "elements the files hold and the reader does not use: {}",
        un_e.join(", ")
    );
    let _ = writeln!(
        report,
        "attributes the files hold and the reader does not use: {}",
        un_a.join(", ")
    );
    print!("{report}");
    if let Some(path) = out {
        if let Err(e) = std::fs::write(&path, &report) {
            eprintln!("error: {path}: {e}");
            std::process::exit(1);
        }
    }
    if failed > 0 {
        std::process::exit(2);
    }
}
