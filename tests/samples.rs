//! The readers on a folder of real files, when there is one: the variable `PHOENIX_LASER_SAMPLES` names a folder, and every `.svg`, `.dxf`, `.lbrn` and `.lbrn2` file under it must be read
//! (a drawing with shapes, every point a number) within the default limits. Without the variable the test passes at once, so that nothing in the repository depends on files of other makers.

use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::lbrn2::read_lbrn2;
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::vector::ReadOptions;
use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && depth < 6 {
            files(&p, out, depth + 1);
        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| {
            ["svg", "dxf", "lbrn", "lbrn2"].contains(&x.to_ascii_lowercase().as_str())
        }) {
            out.push(p);
        }
    }
}

#[test]
fn every_file_of_a_folder_of_samples_is_read_when_there_is_one() {
    let Some(dir) = std::env::var_os("PHOENIX_LASER_SAMPLES") else {
        return;
    };
    let mut list = Vec::new();
    files(Path::new(&dir), &mut list, 0);
    list.sort();
    let opts = ReadOptions::default();
    for path in &list {
        let bytes = std::fs::read(path).unwrap();
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let drawing = match ext.as_str() {
            "svg" => read_svg(&bytes, &opts),
            "dxf" => read_dxf(&bytes, &opts),
            _ => read_lbrn2(&bytes, &opts),
        }
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!drawing.shapes.is_empty(), "{}: no shape", path.display());
        assert!(
            drawing
                .shapes
                .iter()
                .all(|s| s.pts.iter().all(|p| p.0.is_finite() && p.1.is_finite())),
            "{}",
            path.display()
        );
    }
    eprintln!("{} sample file(s) read", list.len());
}

#[test]
fn a_folder_that_is_not_there_gives_no_file() {
    let mut list = Vec::new();
    files(Path::new("no-such-folder-of-samples"), &mut list, 0);
    assert!(list.is_empty());
}
