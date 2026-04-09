use std::fs;
use std::path::PathBuf;

use crate::diff::engine::{DiffOptions, compute_diff};
use crate::diff::three_way::compute_three_way_diff;
use crate::file_browser::FileBrowser;
use crate::models::diff_line::{DiffResult, LineStatus, ThreeWayResult};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    OpenLeft,
    OpenRight,
    OpenBase,
    OpenChooseMode,
    SaveLeft,
    SaveRight,
    SaveConfirm,
}

#[derive(Clone)]
struct TextSnapshot {
    left_text: String,
    right_text: String,
}

pub struct App {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub base_path: Option<PathBuf>,
    pub diff_result: Option<DiffResult>,
    pub three_way_result: Option<ThreeWayResult>,
    pub is_three_way: bool,
    pub diff_options: DiffOptions,
    pub current_diff: i32,
    pub scroll_offset: usize,
    pub left_text: String,
    pub right_text: String,
    pub base_text: String,
    pub mode: AppMode,
    pub file_browser: Option<FileBrowser>,
    pub should_quit: bool,
    pub has_unsaved_changes: bool,
    pub status_message: Option<(String, std::time::Instant)>,
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
}

impl App {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            base_path: None,
            diff_result: None,
            three_way_result: None,
            is_three_way: false,
            diff_options: DiffOptions::default(),
            current_diff: -1,
            scroll_offset: 0,
            left_text: String::new(),
            right_text: String::new(),
            base_text: String::new(),
            mode: AppMode::Normal,
            file_browser: None,
            should_quit: false,
            has_unsaved_changes: false,
            status_message: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn open_files(&mut self, left: PathBuf, right: PathBuf) {
        self.left_text = fs::read_to_string(&left).unwrap_or_default();
        self.right_text = fs::read_to_string(&right).unwrap_or_default();
        self.left_path = Some(left);
        self.right_path = Some(right);
        self.base_path = None;
        self.base_text.clear();
        self.is_three_way = false;
        self.three_way_result = None;
        self.has_unsaved_changes = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.recompute_diff();
    }

    pub fn open_files_3way(&mut self, left: PathBuf, base: PathBuf, right: PathBuf) {
        self.left_text = fs::read_to_string(&left).unwrap_or_default();
        self.base_text = fs::read_to_string(&base).unwrap_or_default();
        self.right_text = fs::read_to_string(&right).unwrap_or_default();
        self.left_path = Some(left);
        self.base_path = Some(base);
        self.right_path = Some(right);
        self.is_three_way = true;
        self.diff_result = None;
        self.has_unsaved_changes = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.recompute_diff();
    }

    pub fn recompute_diff(&mut self) {
        if self.is_three_way {
            let result = compute_three_way_diff(&self.base_text, &self.left_text, &self.right_text);
            self.current_diff = if !result.diff_positions.is_empty() {
                0
            } else {
                -1
            };
            self.three_way_result = Some(result);
            self.diff_result = None;
        } else {
            let result = compute_diff(&self.left_text, &self.right_text, &self.diff_options);
            self.current_diff = if result.diff_count > 0 { 0 } else { -1 };
            self.diff_result = Some(result);
            self.three_way_result = None;
        }
        self.scroll_to_current_diff();
    }

    pub fn diff_count(&self) -> u32 {
        if self.is_three_way {
            self.three_way_result
                .as_ref()
                .map(|r| r.diff_positions.len() as u32)
                .unwrap_or(0)
        } else {
            self.diff_result.as_ref().map(|r| r.diff_count).unwrap_or(0)
        }
    }

    pub fn total_lines(&self) -> usize {
        if self.is_three_way {
            self.three_way_result
                .as_ref()
                .map(|r| r.lines.len())
                .unwrap_or(0)
        } else {
            self.diff_result
                .as_ref()
                .map(|r| r.lines.len())
                .unwrap_or(0)
        }
    }

    pub fn next_diff(&mut self) {
        let count = self.diff_count();
        if count == 0 {
            return;
        }
        let max = count as i32 - 1;
        self.current_diff = (self.current_diff + 1).min(max);
        self.scroll_to_current_diff();
    }

    pub fn prev_diff(&mut self) {
        let count = self.diff_count();
        if count == 0 {
            return;
        }
        self.current_diff = (self.current_diff - 1).max(0);
        self.scroll_to_current_diff();
    }

    pub fn first_diff(&mut self) {
        if self.diff_count() > 0 {
            self.current_diff = 0;
            self.scroll_to_current_diff();
        }
    }

    pub fn last_diff(&mut self) {
        let count = self.diff_count();
        if count > 0 {
            self.current_diff = count as i32 - 1;
            self.scroll_to_current_diff();
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(TextSnapshot {
            left_text: self.left_text.clone(),
            right_text: self.right_text.clone(),
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(TextSnapshot {
                left_text: self.left_text.clone(),
                right_text: self.right_text.clone(),
            });
            self.left_text = snapshot.left_text;
            self.right_text = snapshot.right_text;
            self.recompute_diff();
            self.has_unsaved_changes = !self.undo_stack.is_empty();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(TextSnapshot {
                left_text: self.left_text.clone(),
                right_text: self.right_text.clone(),
            });
            self.left_text = snapshot.left_text;
            self.right_text = snapshot.right_text;
            self.recompute_diff();
            self.has_unsaved_changes = true;
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), std::time::Instant::now()));
    }

    /// Returns the status message if it's still fresh (within 3 seconds)
    pub fn current_status_message(&self) -> Option<&str> {
        if let Some((ref msg, at)) = self.status_message {
            if at.elapsed().as_secs() < 3 {
                return Some(msg.as_str());
            }
        }
        None
    }

    /// Start save flow — always opens file browser dialog for confirmation.
    pub fn save_files(&mut self) {
        let default_name = self
            .left_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut browser = FileBrowser::new_save(&default_name);
        // If left_path has a directory, start there
        if let Some(ref path) = self.left_path {
            if let Some(parent) = path.parent() {
                browser.current_dir = parent.to_path_buf();
                browser.read_dir();
            }
        }
        self.file_browser = Some(browser);
        self.mode = AppMode::SaveLeft;
    }

    pub fn copy_left_to_right(&mut self) {
        if self.current_diff < 0 {
            return;
        }
        if let Some(ref result) = self.diff_result.clone() {
            let block_idx = self.current_diff as usize;
            if block_idx >= result.diff_positions.len() {
                return;
            }
            self.push_undo();
            self.apply_copy(block_idx, true);
            self.has_unsaved_changes = true;
        }
    }

    pub fn copy_right_to_left(&mut self) {
        if self.current_diff < 0 {
            return;
        }
        if let Some(ref result) = self.diff_result.clone() {
            let block_idx = self.current_diff as usize;
            if block_idx >= result.diff_positions.len() {
                return;
            }
            self.push_undo();
            self.apply_copy(block_idx, false);
            self.has_unsaved_changes = true;
        }
    }

    pub fn copy_left_to_right_and_next(&mut self) {
        self.copy_left_to_right();
        self.next_diff();
    }

    pub fn copy_right_to_left_and_next(&mut self) {
        self.copy_right_to_left();
        self.next_diff();
    }

    pub fn copy_all_left_to_right(&mut self) {
        self.push_undo();
        self.right_text = self.left_text.clone();
        self.has_unsaved_changes = true;
        self.recompute_diff();
    }

    pub fn copy_all_right_to_left(&mut self) {
        self.push_undo();
        self.left_text = self.right_text.clone();
        self.has_unsaved_changes = true;
        self.recompute_diff();
    }

    pub fn toggle_ignore_whitespace(&mut self) {
        self.diff_options.ignore_whitespace = !self.diff_options.ignore_whitespace;
        if self.diff_result.is_some() || self.three_way_result.is_some() {
            self.recompute_diff();
        }
    }

    pub fn toggle_ignore_case(&mut self) {
        self.diff_options.ignore_case = !self.diff_options.ignore_case;
        if self.diff_result.is_some() || self.three_way_result.is_some() {
            self.recompute_diff();
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = self.total_lines().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    fn scroll_to_current_diff(&mut self) {
        if self.current_diff < 0 {
            return;
        }
        let block_idx = self.current_diff as usize;
        let pos = if self.is_three_way {
            self.three_way_result
                .as_ref()
                .and_then(|r| r.diff_positions.get(block_idx).copied())
        } else {
            self.diff_result
                .as_ref()
                .and_then(|r| r.diff_positions.get(block_idx).copied())
        };
        if let Some(p) = pos {
            self.scroll_offset = p.saturating_sub(3);
        }
    }

    fn apply_copy(&mut self, block_idx: usize, left_to_right: bool) {
        let result = match self.diff_result {
            Some(ref r) => r.clone(),
            None => return,
        };
        let start = match result.diff_positions.get(block_idx) {
            Some(&s) => s,
            None => return,
        };

        let mut end = start;
        while end < result.lines.len() && result.lines[end].status != LineStatus::Equal {
            end += 1;
        }

        let mut left_lines: Vec<String> = self.left_text.lines().map(String::from).collect();
        let mut right_lines: Vec<String> = self.right_text.lines().map(String::from).collect();

        if left_to_right {
            let mut right_removes = Vec::new();
            let mut left_source = Vec::new();
            let mut insert_pos: Option<usize> = None;

            for line in &result.lines[start..end] {
                match line.status {
                    LineStatus::Modified => {
                        if let Some(rn) = line.right_line_no {
                            right_lines[rn as usize - 1] = line.left_text.clone();
                        }
                    }
                    LineStatus::Removed => {
                        left_source.push(line.left_text.clone());
                        if insert_pos.is_none() {
                            insert_pos = if start > 0 {
                                result.lines[start - 1].right_line_no.map(|n| n as usize)
                            } else {
                                Some(0)
                            };
                        }
                    }
                    LineStatus::Added => {
                        if let Some(rn) = line.right_line_no {
                            right_removes.push(rn as usize - 1);
                        }
                    }
                    _ => {}
                }
            }

            right_removes.sort();
            for &idx in right_removes.iter().rev() {
                if idx < right_lines.len() {
                    right_lines.remove(idx);
                }
            }

            if let Some(pos) = insert_pos {
                let adjusted_pos = pos.min(right_lines.len());
                for (i, line) in left_source.into_iter().enumerate() {
                    right_lines.insert(adjusted_pos + i, line);
                }
            }
        } else {
            let mut left_removes = Vec::new();
            let mut right_source = Vec::new();
            let mut insert_pos: Option<usize> = None;

            for line in &result.lines[start..end] {
                match line.status {
                    LineStatus::Modified => {
                        if let Some(ln) = line.left_line_no {
                            left_lines[ln as usize - 1] = line.right_text.clone();
                        }
                    }
                    LineStatus::Added => {
                        right_source.push(line.right_text.clone());
                        if insert_pos.is_none() {
                            insert_pos = if start > 0 {
                                result.lines[start - 1].left_line_no.map(|n| n as usize)
                            } else {
                                Some(0)
                            };
                        }
                    }
                    LineStatus::Removed => {
                        if let Some(ln) = line.left_line_no {
                            left_removes.push(ln as usize - 1);
                        }
                    }
                    _ => {}
                }
            }

            left_removes.sort();
            for &idx in left_removes.iter().rev() {
                if idx < left_lines.len() {
                    left_lines.remove(idx);
                }
            }

            if let Some(pos) = insert_pos {
                let adjusted_pos = pos.min(left_lines.len());
                for (i, line) in right_source.into_iter().enumerate() {
                    left_lines.insert(adjusted_pos + i, line);
                }
            }
        }

        self.left_text = left_lines.join("\n");
        self.right_text = right_lines.join("\n");
        if !self.left_text.is_empty() && !self.left_text.ends_with('\n') {
            self.left_text.push('\n');
        }
        if !self.right_text.is_empty() && !self.right_text.ends_with('\n') {
            self.right_text.push('\n');
        }
        self.recompute_diff();
    }
}
