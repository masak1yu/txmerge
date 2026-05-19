use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::models::diff_line::{DirCompareResult, DirEntry, DirEntryStatus};

pub fn scan_dirs(left: &Path, right: &Path) -> DirCompareResult {
    let left_files = collect_files(left).unwrap_or_default();
    let right_files = collect_files(right).unwrap_or_default();

    let mut map: BTreeMap<PathBuf, (bool, bool)> = BTreeMap::new();
    for p in &left_files {
        map.entry(p.clone()).or_insert((false, false)).0 = true;
    }
    for p in &right_files {
        map.entry(p.clone()).or_insert((false, false)).1 = true;
    }

    let mut entries = Vec::new();
    for (rel, (in_left, in_right)) in map {
        let status = match (in_left, in_right) {
            (true, false) => DirEntryStatus::LeftOnly,
            (false, true) => DirEntryStatus::RightOnly,
            _ => {
                let lp = left.join(&rel);
                let rp = right.join(&rel);
                if files_equal(&lp, &rp) {
                    DirEntryStatus::Equal
                } else {
                    DirEntryStatus::Changed
                }
            }
        };
        entries.push(DirEntry { rel_path: rel, status });
    }

    DirCompareResult {
        left_dir: left.to_path_buf(),
        right_dir: right.to_path_buf(),
        entries,
        selected: 0,
        scroll_offset: 0,
    }
}

fn collect_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    visit_dir(dir, dir, &mut result)?;
    Ok(result)
}

fn visit_dir(root: &Path, current: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            visit_dir(root, &path, out)?;
        } else if meta.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_path_buf());
            }
        }
    }
    Ok(())
}

fn files_equal(a: &Path, b: &Path) -> bool {
    let ra = fs::read(a);
    let rb = fs::read(b);
    match (ra, rb) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}
