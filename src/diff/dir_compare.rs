use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
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
        let lp = left.join(&rel);
        let rp = right.join(&rel);

        let (left_modified, left_size) = if in_left {
            file_meta(&lp)
        } else {
            (None, None)
        };
        let (right_modified, right_size) = if in_right {
            file_meta(&rp)
        } else {
            (None, None)
        };

        let status = match (in_left, in_right) {
            (true, false) => DirEntryStatus::LeftOnly,
            (false, true) => DirEntryStatus::RightOnly,
            _ => {
                if files_equal(&lp, &rp) {
                    DirEntryStatus::Equal
                } else {
                    DirEntryStatus::Changed
                }
            }
        };
        entries.push(DirEntry {
            rel_path: rel,
            status,
            left_modified,
            right_modified,
            left_size,
            right_size,
        });
    }

    DirCompareResult {
        left_dir: left.to_path_buf(),
        right_dir: right.to_path_buf(),
        entries,
        selected: 0,
        scroll_offset: 0,
        git_context: None,
    }
}

fn file_meta(path: &Path) -> (Option<SystemTime>, Option<u64>) {
    match fs::metadata(path) {
        Ok(m) => (m.modified().ok(), Some(m.len())),
        Err(_) => (None, None),
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
    match (fs::read(a), fs::read(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}
