use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::diff_line::{DirCompareResult, DirEntry, DirEntryStatus, GitContext};

pub fn scan_git_diff(repo: &Path, ref1: &str, ref2: Option<&str>) -> DirCompareResult {
    let range = if let Some(r2) = ref2 {
        format!("{}..{}", ref1, r2)
    } else {
        ref1.to_string()
    };

    let output = Command::new("git")
        .args([
            "-C",
            &repo.to_string_lossy(),
            "diff",
            "--name-status",
            &range,
        ])
        .output()
        .unwrap_or_else(|_| std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        });

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.is_empty() {
            continue;
        }
        let status_char = parts[0].chars().next().unwrap_or(' ');
        match status_char {
            'M' if parts.len() >= 2 => {
                entries.push(make_entry(parts[1], DirEntryStatus::Changed));
            }
            'A' if parts.len() >= 2 => {
                entries.push(make_entry(parts[1], DirEntryStatus::RightOnly));
            }
            'D' if parts.len() >= 2 => {
                entries.push(make_entry(parts[1], DirEntryStatus::LeftOnly));
            }
            'R' if parts.len() >= 3 => {
                // Rename: old path deleted from left, new path added to right
                entries.push(make_entry(parts[1], DirEntryStatus::LeftOnly));
                entries.push(make_entry(parts[2], DirEntryStatus::RightOnly));
            }
            'C' if parts.len() >= 3 => {
                entries.push(make_entry(parts[2], DirEntryStatus::RightOnly));
            }
            _ => {}
        }
    }

    DirCompareResult {
        left_dir: repo.to_path_buf(),
        right_dir: repo.to_path_buf(),
        entries,
        selected: 0,
        scroll_offset: 0,
        git_context: Some(GitContext {
            repo: repo.to_path_buf(),
            ref1: ref1.to_string(),
            ref2: ref2.map(String::from),
        }),
    }
}

fn make_entry(path_str: &str, status: DirEntryStatus) -> DirEntry {
    DirEntry {
        rel_path: PathBuf::from(path_str),
        status,
        left_modified: None,
        right_modified: None,
        left_size: None,
        right_size: None,
    }
}

/// Extract a file at <git_ref>:<path> into a temp file, returning the temp path.
pub fn extract_git_file(repo: &Path, git_ref: &str, path: &Path) -> Option<PathBuf> {
    let spec = format!("{}:{}", git_ref, path.to_string_lossy());
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "show", &spec])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let safe_ref = git_ref.replace(['/', ':', '.'], "_");
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let tmp_path = PathBuf::from(format!("/tmp/txmerge_{}_{}", safe_ref, filename));
    std::fs::write(&tmp_path, &output.stdout).ok()?;
    Some(tmp_path)
}
