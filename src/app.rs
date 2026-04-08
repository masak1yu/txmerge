use std::fs;
use std::path::PathBuf;

use crate::diff::engine::{DiffOptions, compute_diff};
use crate::models::diff_line::{DiffResult, LineStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    OpenLeft,
    OpenRight,
}

pub struct App {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub diff_result: Option<DiffResult>,
    pub diff_options: DiffOptions,
    pub current_diff: i32,
    pub scroll_offset: usize,
    pub left_text: String,
    pub right_text: String,
    pub mode: AppMode,
    pub input_buffer: String,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            diff_result: None,
            diff_options: DiffOptions::default(),
            current_diff: -1,
            scroll_offset: 0,
            left_text: String::new(),
            right_text: String::new(),
            mode: AppMode::Normal,
            input_buffer: String::new(),
            should_quit: false,
        }
    }

    pub fn open_files(&mut self, left: PathBuf, right: PathBuf) {
        self.left_text = fs::read_to_string(&left).unwrap_or_default();
        self.right_text = fs::read_to_string(&right).unwrap_or_default();
        self.left_path = Some(left);
        self.right_path = Some(right);
        self.recompute_diff();
    }

    pub fn recompute_diff(&mut self) {
        let result = compute_diff(&self.left_text, &self.right_text, &self.diff_options);
        self.current_diff = if result.diff_count > 0 { 0 } else { -1 };
        self.diff_result = Some(result);
        self.scroll_to_current_diff();
    }

    pub fn next_diff(&mut self) {
        if let Some(ref result) = self.diff_result {
            if result.diff_count == 0 {
                return;
            }
            let max = result.diff_count as i32 - 1;
            self.current_diff = (self.current_diff + 1).min(max);
            self.scroll_to_current_diff();
        }
    }

    pub fn prev_diff(&mut self) {
        if let Some(ref result) = self.diff_result {
            if result.diff_count == 0 {
                return;
            }
            self.current_diff = (self.current_diff - 1).max(0);
            self.scroll_to_current_diff();
        }
    }

    pub fn first_diff(&mut self) {
        if let Some(ref result) = self.diff_result {
            if result.diff_count > 0 {
                self.current_diff = 0;
                self.scroll_to_current_diff();
            }
        }
    }

    pub fn last_diff(&mut self) {
        if let Some(ref result) = self.diff_result {
            if result.diff_count > 0 {
                self.current_diff = result.diff_count as i32 - 1;
                self.scroll_to_current_diff();
            }
        }
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
            self.apply_copy(block_idx, true);
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
            self.apply_copy(block_idx, false);
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
        self.right_text = self.left_text.clone();
        self.recompute_diff();
    }

    pub fn copy_all_right_to_left(&mut self) {
        self.left_text = self.right_text.clone();
        self.recompute_diff();
    }

    pub fn toggle_ignore_whitespace(&mut self) {
        self.diff_options.ignore_whitespace = !self.diff_options.ignore_whitespace;
        if self.diff_result.is_some() {
            self.recompute_diff();
        }
    }

    pub fn toggle_ignore_case(&mut self) {
        self.diff_options.ignore_case = !self.diff_options.ignore_case;
        if self.diff_result.is_some() {
            self.recompute_diff();
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        if let Some(ref result) = self.diff_result {
            let max = result.lines.len().saturating_sub(1);
            self.scroll_offset = (self.scroll_offset + amount).min(max);
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn total_lines(&self) -> usize {
        self.diff_result
            .as_ref()
            .map(|r| r.lines.len())
            .unwrap_or(0)
    }

    fn scroll_to_current_diff(&mut self) {
        if self.current_diff < 0 {
            return;
        }
        if let Some(ref result) = self.diff_result {
            let block_idx = self.current_diff as usize;
            if let Some(&pos) = result.diff_positions.get(block_idx) {
                self.scroll_offset = pos.saturating_sub(3);
            }
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

        // Find block end
        let mut end = start;
        while end < result.lines.len() && result.lines[end].status != LineStatus::Equal {
            end += 1;
        }

        // Rebuild left and right line vectors
        let mut left_lines: Vec<String> = self.left_text.lines().map(String::from).collect();
        let mut right_lines: Vec<String> = self.right_text.lines().map(String::from).collect();

        // Collect the source lines and target positions
        if left_to_right {
            // Replace right side with left side for this block
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
                        // Line exists only on left, add to right
                        left_source.push(line.left_text.clone());
                        // Insert after previous right line or at right_line_no position
                        if insert_pos.is_none() {
                            // Find insertion point: after the last equal line before this block
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

            // Remove added lines (in reverse order to preserve indices)
            right_removes.sort();
            for &idx in right_removes.iter().rev() {
                if idx < right_lines.len() {
                    right_lines.remove(idx);
                }
            }

            // Insert removed lines (they only exist on left)
            if let Some(pos) = insert_pos {
                let adjusted_pos = pos.min(right_lines.len());
                for (i, line) in left_source.into_iter().enumerate() {
                    right_lines.insert(adjusted_pos + i, line);
                }
            }
        } else {
            // Replace left side with right side for this block
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
        // Preserve trailing newline
        if !self.left_text.is_empty() && !self.left_text.ends_with('\n') {
            self.left_text.push('\n');
        }
        if !self.right_text.is_empty() && !self.right_text.ends_with('\n') {
            self.right_text.push('\n');
        }
        self.recompute_diff();
    }
}
