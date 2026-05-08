use std::time::Duration;

use similar::{Algorithm, ChangeTag, TextDiff};

use crate::models::diff_line::{DiffLine, DiffResult, LineStatus, WordDiffSegment};

const LARGE_FILE_THRESHOLD: usize = 10_000;
const DIFF_TIMEOUT: Duration = Duration::from_secs(5);
const WORD_DIFF_LINE_LIMIT: usize = 20_000;

#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    pub ignore_blank_lines: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: false,
            ignore_case: false,
            ignore_blank_lines: false,
        }
    }
}

pub fn compute_diff(left_lines: &[String], right_lines: &[String], options: &DiffOptions) -> DiffResult {
    let left_norm = normalize_lines(left_lines, options);
    let right_norm = normalize_lines(right_lines, options);

    let left_norm_refs: Vec<&str> = left_norm.iter().map(|s| s.as_str()).collect();
    let right_norm_refs: Vec<&str> = right_norm.iter().map(|s| s.as_str()).collect();

    let line_count = left_lines.len().max(right_lines.len());
    let algorithm = if line_count > LARGE_FILE_THRESHOLD {
        Algorithm::Patience
    } else {
        Algorithm::Myers
    };

    let diff = TextDiff::configure()
        .algorithm(algorithm)
        .timeout(DIFF_TIMEOUT)
        .diff_slices(&left_norm_refs, &right_norm_refs);

    let mut lines = Vec::new();
    let mut diff_positions = Vec::new();
    let mut left_line_no: u32 = 0;
    let mut right_line_no: u32 = 0;

    let changes: Vec<_> = diff.iter_all_changes().collect();
    let mut i = 0;

    while i < changes.len() {
        match changes[i].tag() {
            ChangeTag::Equal => {
                left_line_no += 1;
                right_line_no += 1;
                lines.push(DiffLine {
                    left_line_no: Some(left_line_no),
                    right_line_no: Some(right_line_no),
                    status: LineStatus::Equal,
                    left_word_segments: Vec::new(),
                    right_word_segments: Vec::new(),
                });
                i += 1;
            }
            ChangeTag::Delete | ChangeTag::Insert => {
                let mut del_indices: Vec<u32> = Vec::new();
                let mut ins_indices: Vec<u32> = Vec::new();
                while i < changes.len() && changes[i].tag() != ChangeTag::Equal {
                    match changes[i].tag() {
                        ChangeTag::Delete => {
                            del_indices.push(left_line_no);
                            left_line_no += 1;
                        }
                        ChangeTag::Insert => {
                            ins_indices.push(right_line_no);
                            right_line_no += 1;
                        }
                        _ => unreachable!(),
                    }
                    i += 1;
                }

                let word_diff_enabled = line_count <= WORD_DIFF_LINE_LIMIT;
                let n_pairs = del_indices.len().min(ins_indices.len());
                for j in 0..n_pairs {
                    let left_text = left_lines.get(del_indices[j] as usize).map(|s| s.as_str()).unwrap_or("");
                    let right_text = right_lines.get(ins_indices[j] as usize).map(|s| s.as_str()).unwrap_or("");
                    let (left_segs, right_segs) = if word_diff_enabled {
                        compute_word_diff(left_text, right_text)
                    } else {
                        (Vec::new(), Vec::new())
                    };
                    lines.push(DiffLine {
                        left_line_no: Some(del_indices[j] + 1),
                        right_line_no: Some(ins_indices[j] + 1),
                        status: LineStatus::Modified,
                        left_word_segments: left_segs,
                        right_word_segments: right_segs,
                    });
                }
                for j in n_pairs..del_indices.len() {
                    lines.push(DiffLine {
                        left_line_no: Some(del_indices[j] + 1),
                        right_line_no: None,
                        status: LineStatus::Removed,
                        left_word_segments: Vec::new(),
                        right_word_segments: Vec::new(),
                    });
                }
                for j in n_pairs..ins_indices.len() {
                    lines.push(DiffLine {
                        left_line_no: None,
                        right_line_no: Some(ins_indices[j] + 1),
                        status: LineStatus::Added,
                        left_word_segments: Vec::new(),
                        right_word_segments: Vec::new(),
                    });
                }
            }
        }
    }

    diff_positions.clear();
    let mut diff_count: u32 = 0;
    let mut in_diff_block = false;
    for (idx, line) in lines.iter().enumerate() {
        if line.status != LineStatus::Equal {
            if !in_diff_block {
                diff_positions.push(idx);
                diff_count += 1;
                in_diff_block = true;
            }
        } else {
            in_diff_block = false;
        }
    }

    DiffResult {
        lines,
        diff_count,
        diff_positions,
    }
}

fn compute_word_diff(left: &str, right: &str) -> (Vec<WordDiffSegment>, Vec<WordDiffSegment>) {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_chars(left, right);

    let mut left_segs: Vec<WordDiffSegment> = Vec::new();
    let mut right_segs: Vec<WordDiffSegment> = Vec::new();

    for change in diff.iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                push_segment(&mut left_segs, &text, false);
                push_segment(&mut right_segs, &text, false);
            }
            ChangeTag::Delete => {
                push_segment(&mut left_segs, &text, true);
            }
            ChangeTag::Insert => {
                push_segment(&mut right_segs, &text, true);
            }
        }
    }

    (left_segs, right_segs)
}

fn push_segment(segs: &mut Vec<WordDiffSegment>, text: &str, changed: bool) {
    if let Some(last) = segs.last_mut() {
        if last.changed == changed {
            last.text.push_str(text);
            return;
        }
    }
    segs.push(WordDiffSegment {
        text: text.to_string(),
        changed,
    });
}

fn normalize_lines(lines: &[String], options: &DiffOptions) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    for line in lines {
        if options.ignore_blank_lines && line.trim().is_empty() {
            continue;
        }
        let mut l = line.clone();
        if options.ignore_whitespace {
            l = l.split_whitespace().collect::<Vec<&str>>().join(" ");
        }
        if options.ignore_case {
            l = l.to_lowercase();
        }
        result.push(l);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(String::from).collect()
    }

    #[test]
    fn test_equal_files() {
        let result = compute_diff(&lines("hello\nworld\n"), &lines("hello\nworld\n"), &DiffOptions::default());
        assert_eq!(result.diff_count, 0);
        assert_eq!(result.lines.len(), 2);
    }

    #[test]
    fn test_added_line() {
        let result = compute_diff(&lines("hello\n"), &lines("hello\nworld\n"), &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Added);
    }

    #[test]
    fn test_removed_line() {
        let result = compute_diff(&lines("hello\nworld\n"), &lines("hello\n"), &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[1].status, LineStatus::Removed);
    }

    #[test]
    fn test_modified_line() {
        let result = compute_diff(&lines("hello\n"), &lines("hallo\n"), &DiffOptions::default());
        assert_eq!(result.diff_count, 1);
        assert_eq!(result.lines[0].status, LineStatus::Modified);
    }

    #[test]
    fn test_ignore_whitespace() {
        let opts = DiffOptions {
            ignore_whitespace: true,
            ..Default::default()
        };
        let result = compute_diff(&lines("hello   world\n"), &lines("hello world\n"), &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_ignore_case() {
        let opts = DiffOptions {
            ignore_case: true,
            ..Default::default()
        };
        let result = compute_diff(&lines("Hello\n"), &lines("hello\n"), &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_ignore_blank_lines() {
        let opts = DiffOptions {
            ignore_blank_lines: true,
            ..Default::default()
        };
        let result = compute_diff(&lines("hello\n\nworld\n"), &lines("hello\nworld\n"), &opts);
        assert_eq!(result.diff_count, 0);
    }

    #[test]
    fn test_two_separate_blocks() {
        let left = lines("a\nb\nc\nd\ne\n");
        let right = lines("X\nb\nc\nd\nY\n");
        let result = compute_diff(&left, &right, &DiffOptions::default());
        assert_eq!(result.diff_positions.len(), 2);
        assert_eq!(result.diff_count, 2);
    }

    #[test]
    fn test_word_diff_segments() {
        let result = compute_diff(&lines("hello world\n"), &lines("hello earth\n"), &DiffOptions::default());
        assert_eq!(result.lines[0].status, LineStatus::Modified);
        assert!(!result.lines[0].left_word_segments.is_empty());
        assert!(!result.lines[0].right_word_segments.is_empty());
    }
}
