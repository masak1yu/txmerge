use std::fs;
use std::path::PathBuf;

use crate::diff::engine::{DiffOptions, compute_diff};
use crate::diff::three_way::compute_three_way_diff;
use crate::file_browser::FileBrowser;
use crate::models::diff_line::{DiffResult, LineStatus, ThreeWayResult, ThreeWayStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Editing,
    OpenLeft,
    OpenRight,
    OpenBase,
    OpenChooseMode,
    SaveLeft,
    SaveRight,
    SaveConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelSide {
    Left,
    Right,
    Base,
}

pub struct EditState {
    pub panel: PanelSide,
    pub source_line: usize,  // 0-based index into source text lines
    pub display_line: usize, // index in diff result lines
    pub cursor_col: usize,   // char index in line
    pub dirty: bool,
}

#[derive(Clone)]
struct TextSnapshot {
    left_text: String,
    right_text: String,
    base_text: String,
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
    pub edit_state: Option<EditState>,
    pub new_file_pending: bool,
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
            edit_state: None,
            new_file_pending: false,
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
        self.recompute_diff_inner(true);
    }

    /// Recompute diff without resetting current_diff position or scrolling.
    pub fn recompute_diff_keep_pos(&mut self) {
        self.recompute_diff_inner(false);
    }

    fn recompute_diff_inner(&mut self, reset_position: bool) {
        // Clear stale edit state — diff line layout is about to change (WinXMerge pattern)
        self.edit_state = None;
        if self.mode == AppMode::Editing {
            self.mode = AppMode::Normal;
        }
        if self.is_three_way {
            let result = compute_three_way_diff(&self.base_text, &self.left_text, &self.right_text);
            if reset_position {
                self.current_diff = if !result.diff_positions.is_empty() { 0 } else { -1 };
            } else {
                let count = result.diff_positions.len() as i32;
                if self.current_diff >= count {
                    self.current_diff = count - 1;
                }
            }
            self.three_way_result = Some(result);
            self.diff_result = None;
        } else {
            let result = compute_diff(&self.left_text, &self.right_text, &self.diff_options);
            if reset_position {
                self.current_diff = if result.diff_count > 0 { 0 } else { -1 };
            } else {
                let count = result.diff_count as i32;
                if self.current_diff >= count {
                    self.current_diff = count - 1;
                }
            }
            self.diff_result = Some(result);
            self.three_way_result = None;
        }
        if reset_position {
            self.scroll_to_current_diff();
        }
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
                .unwrap_or_else(|| {
                    // No diff result — use max of source line counts
                    let l = self.source_lines(PanelSide::Left).len();
                    let b = self.source_lines(PanelSide::Base).len();
                    let r = self.source_lines(PanelSide::Right).len();
                    l.max(b).max(r).max(1)
                })
        } else {
            self.diff_result
                .as_ref()
                .map(|r| r.lines.len())
                .unwrap_or_else(|| {
                    let l = self.source_lines(PanelSide::Left).len();
                    let r = self.source_lines(PanelSide::Right).len();
                    l.max(r).max(1)
                })
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

    pub fn undo_stack_is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(TextSnapshot {
            left_text: self.left_text.clone(),
            right_text: self.right_text.clone(),
            base_text: self.base_text.clone(),
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(TextSnapshot {
                left_text: self.left_text.clone(),
                right_text: self.right_text.clone(),
                base_text: self.base_text.clone(),
            });
            self.left_text = snapshot.left_text;
            self.right_text = snapshot.right_text;
            self.base_text = snapshot.base_text;
            self.recompute_diff();
            self.has_unsaved_changes = !self.undo_stack.is_empty();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(TextSnapshot {
                left_text: self.left_text.clone(),
                right_text: self.right_text.clone(),
                base_text: self.base_text.clone(),
            });
            self.left_text = snapshot.left_text;
            self.right_text = snapshot.right_text;
            self.base_text = snapshot.base_text;
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
        let (default_name, start_dir) = Self::save_defaults(&self.left_path, "untitled_left.txt");
        let mut browser = FileBrowser::new_save(&default_name);
        browser.current_dir = start_dir;
        browser.read_dir();
        self.file_browser = Some(browser);
        self.mode = AppMode::SaveLeft;
    }

    /// Determine default filename and starting directory for save dialog.
    pub fn save_defaults(path: &Option<PathBuf>, fallback_name: &str) -> (String, PathBuf) {
        if let Some(p) = path {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| fallback_name.to_string());
            let dir = p
                .parent()
                .map(|d| d.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
            (name, dir)
        } else {
            let dir = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
                });
            (fallback_name.to_string(), dir)
        }
    }

    pub fn copy_left_to_right(&mut self) {
        if self.current_diff < 0 {
            return;
        }
        if self.is_three_way {
            self.copy_three_way(true);
        } else if let Some(ref result) = self.diff_result.clone() {
            let block_idx = self.current_diff as usize;
            if block_idx >= result.diff_positions.len() {
                return;
            }
            self.push_undo();
            self.edit_state = None;
            self.mode = AppMode::Normal;
            self.apply_copy(block_idx, true);
            self.has_unsaved_changes = true;
        }
    }

    pub fn copy_right_to_left(&mut self) {
        if self.current_diff < 0 {
            return;
        }
        if self.is_three_way {
            self.copy_three_way(false);
        } else if let Some(ref result) = self.diff_result.clone() {
            let block_idx = self.current_diff as usize;
            if block_idx >= result.diff_positions.len() {
                return;
            }
            self.push_undo();
            self.edit_state = None;
            self.mode = AppMode::Normal;
            self.apply_copy(block_idx, false);
            self.has_unsaved_changes = true;
        }
    }

    /// 3-way copy: Left→Base (use_left=true) or Right→Base (use_left=false)
    fn copy_three_way(&mut self, use_left: bool) {
        let result = match self.three_way_result.clone() {
            Some(r) => r,
            None => return,
        };
        let block_idx = self.current_diff as usize;
        if block_idx >= result.diff_positions.len() {
            return;
        }
        let start = result.diff_positions[block_idx];
        let mut end = start;
        while end < result.lines.len()
            && result.lines[end].status != ThreeWayStatus::Equal
        {
            end += 1;
        }

        self.push_undo();
        self.edit_state = None;
        self.mode = AppMode::Normal;

        // Rebuild base lines: replace the diff block with source lines
        let mut base_lines: Vec<String> = self.base_text.lines().map(String::from).collect();
        let source_lines: Vec<String> = if use_left {
            result.lines[start..end]
                .iter()
                .filter_map(|l| l.left_line_no.map(|_| l.left_text.clone()))
                .collect()
        } else {
            result.lines[start..end]
                .iter()
                .filter_map(|l| l.right_line_no.map(|_| l.right_text.clone()))
                .collect()
        };

        // Find base line range to replace
        let base_start = result.lines[start..end]
            .iter()
            .filter_map(|l| l.base_line_no)
            .min()
            .map(|n| n as usize - 1);
        let base_end = result.lines[start..end]
            .iter()
            .filter_map(|l| l.base_line_no)
            .max()
            .map(|n| n as usize);

        match (base_start, base_end) {
            (Some(bs), Some(be)) => {
                // Replace base lines in range
                let be = be.min(base_lines.len());
                base_lines.splice(bs..be, source_lines);
            }
            _ => {
                // No base lines in this block — insert at appropriate position
                let insert_at = if start > 0 {
                    result.lines[..start]
                        .iter()
                        .filter_map(|l| l.base_line_no)
                        .max()
                        .map(|n| n as usize)
                        .unwrap_or(0)
                } else {
                    0
                };
                let insert_at = insert_at.min(base_lines.len());
                for (i, line) in source_lines.into_iter().enumerate() {
                    base_lines.insert(insert_at + i, line);
                }
            }
        }

        self.base_text = base_lines.join("\n");
        if !self.base_text.is_empty() && !self.base_text.ends_with('\n') {
            self.base_text.push('\n');
        }
        self.has_unsaved_changes = true;
        self.recompute_diff_keep_pos();
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
        if self.is_three_way {
            self.base_text = self.left_text.clone();
        } else {
            self.right_text = self.left_text.clone();
        }
        self.has_unsaved_changes = true;
        self.recompute_diff();
    }

    pub fn copy_all_right_to_left(&mut self) {
        self.push_undo();
        if self.is_three_way {
            self.base_text = self.right_text.clone();
        } else {
            self.left_text = self.right_text.clone();
        }
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
        self.recompute_diff_keep_pos();
    }

    // === New File ===

    pub fn new_blank(&mut self, is_three_way: bool) {
        self.left_text.clear();
        self.right_text.clear();
        self.base_text.clear();
        self.left_path = None;
        self.right_path = None;
        self.base_path = None;
        self.is_three_way = is_three_way;
        self.has_unsaved_changes = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.edit_state = None;
        self.recompute_diff();
    }

    // === Editing ===

    fn source_text(&self, panel: PanelSide) -> &str {
        match panel {
            PanelSide::Left => &self.left_text,
            PanelSide::Right => &self.right_text,
            PanelSide::Base => &self.base_text,
        }
    }

    fn source_text_mut(&mut self, panel: PanelSide) -> &mut String {
        match panel {
            PanelSide::Left => &mut self.left_text,
            PanelSide::Right => &mut self.right_text,
            PanelSide::Base => &mut self.base_text,
        }
    }

    pub fn source_lines(&self, panel: PanelSide) -> Vec<String> {
        let text = self.source_text(panel);
        if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(String::from).collect()
        }
    }

    fn set_source_lines(&mut self, panel: PanelSide, lines: Vec<String>) {
        let text = lines.join("\n");
        *self.source_text_mut(panel) = text;
    }

    /// Resolve display line index to source line number for a given panel.
    /// For ghost lines, returns the nearest valid source line to allow editing.
    pub fn resolve_display_to_source(&self, display_idx: usize, panel: PanelSide) -> Option<usize> {
        let line_count = self.source_lines(panel).len();

        if self.is_three_way {
            if let Some(ref result) = self.three_way_result {
                if let Some(line) = result.lines.get(display_idx) {
                    let line_no = match panel {
                        PanelSide::Left => line.left_line_no,
                        PanelSide::Right => line.right_line_no,
                        PanelSide::Base => line.base_line_no,
                    };
                    if let Some(n) = line_no {
                        return Some(n as usize - 1);
                    }
                    // Ghost line — find nearest real source line
                    return Some(self.find_nearest_source_line(display_idx, panel, &result.lines.iter().map(|l| match panel {
                        PanelSide::Left => l.left_line_no,
                        PanelSide::Right => l.right_line_no,
                        PanelSide::Base => l.base_line_no,
                    }).collect::<Vec<_>>(), line_count));
                }
            }
        } else if let Some(ref result) = self.diff_result {
            if let Some(line) = result.lines.get(display_idx) {
                let line_no = match panel {
                    PanelSide::Left => line.left_line_no,
                    PanelSide::Right => line.right_line_no,
                    PanelSide::Base => return Some(0),
                };
                if let Some(n) = line_no {
                    return Some(n as usize - 1);
                }
                // Ghost line — find nearest real source line
                let line_nos: Vec<Option<u32>> = result.lines.iter().map(|l| match panel {
                    PanelSide::Left => l.left_line_no,
                    PanelSide::Right => l.right_line_no,
                    PanelSide::Base => None,
                }).collect();
                return Some(self.find_nearest_source_line(display_idx, panel, &line_nos, line_count));
            }
        }

        // No diff result (blank screen) — clamp to source lines
        Some(display_idx.min(line_count.saturating_sub(1)))
    }

    /// Find the nearest real source line for a ghost display line.
    /// Scans backward then forward to find a line with a real line number.
    fn find_nearest_source_line(
        &self,
        display_idx: usize,
        _panel: PanelSide,
        line_nos: &[Option<u32>],
        line_count: usize,
    ) -> usize {
        // Scan backward for the last real line
        for i in (0..display_idx).rev() {
            if let Some(n) = line_nos[i] {
                // The next source line after this one
                return ((n as usize).min(line_count)).saturating_sub(1);
            }
        }
        // Scan forward
        for i in display_idx..line_nos.len() {
            if let Some(n) = line_nos[i] {
                return (n as usize).saturating_sub(1);
            }
        }
        // Fallback: last line
        line_count.saturating_sub(1)
    }

    pub fn enter_edit_mode(&mut self, panel: PanelSide, display_line: usize, col: usize) {
        let source_line = match self.resolve_display_to_source(display_line, panel) {
            Some(sl) => sl,
            None => return,
        };

        // CRITICAL: clamp source_line to actual source text bounds
        let lines = self.source_lines(panel);
        let source_line = source_line.min(lines.len().saturating_sub(1));

        // Find the correct display_line by searching diff result for source_line.
        // For ghost lines, clamp to the nearest valid display index.
        let display_line = self.find_display_index(source_line, panel, display_line);

        // Clamp col to line length
        let line_len = lines.get(source_line).map(|l| l.chars().count()).unwrap_or(0);
        let col = col.min(line_len);

        self.push_undo();
        self.edit_state = Some(EditState {
            panel,
            source_line,
            display_line,
            cursor_col: col,
            dirty: false,
        });
        self.mode = AppMode::Editing;
    }

    /// Find the display index for a source line. If the line has a line_no in the diff result,
    /// returns that index. For ghost lines, returns the clicked display_line clamped to diff bounds.
    fn find_display_index(&self, source_line: usize, panel: PanelSide, clicked_display: usize) -> usize {
        let lines = if self.is_three_way {
            self.three_way_result.as_ref().map(|r| r.lines.len())
        } else {
            self.diff_result.as_ref().map(|r| r.lines.len())
        };

        let total = match lines {
            Some(n) if n > 0 => n,
            _ => return clicked_display, // no diff result, use as-is
        };

        // Try to find by line_no match first
        if !self.is_three_way {
            if let Some(ref result) = self.diff_result {
                for (i, dl) in result.lines.iter().enumerate() {
                    let ln = match panel {
                        PanelSide::Left => dl.left_line_no,
                        PanelSide::Right => dl.right_line_no,
                        PanelSide::Base => None,
                    };
                    if ln.map(|n| n as usize - 1) == Some(source_line) {
                        return i;
                    }
                }
            }
        } else if let Some(ref result) = self.three_way_result {
            for (i, dl) in result.lines.iter().enumerate() {
                let ln = match panel {
                    PanelSide::Left => dl.left_line_no,
                    PanelSide::Right => dl.right_line_no,
                    PanelSide::Base => dl.base_line_no,
                };
                if ln.map(|n| n as usize - 1) == Some(source_line) {
                    return i;
                }
            }
        }

        // Ghost line — clamp clicked position to valid range
        clicked_display.min(total.saturating_sub(1))
    }

    pub fn exit_edit_mode(&mut self) {
        let dirty = self.edit_state.as_ref().map(|e| e.dirty).unwrap_or(false);
        self.edit_state = None;
        self.mode = AppMode::Normal;
        if dirty {
            self.has_unsaved_changes = true;
            self.recompute_diff_keep_pos();
        } else {
            // No changes made — remove the undo snapshot we pushed on enter
            self.undo_stack.pop();
        }
    }


    pub fn edit_insert_char(&mut self, ch: char) {
        let es = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        let (panel, source_line, cursor_col) = es;

        let mut lines = self.source_lines(panel);
        if source_line >= lines.len() {
            lines.resize(source_line + 1, String::new());
        }
        let line = &mut lines[source_line];
        let byte_idx = char_to_byte_index(line, cursor_col);
        line.insert(byte_idx, ch);
        self.set_source_lines(panel, lines);

        if let Some(ref mut e) = self.edit_state {
            e.cursor_col += 1;
            e.dirty = true;
        }
    }

    pub fn edit_backspace(&mut self) {
        let es = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        let (panel, source_line, cursor_col) = es;

        let mut lines = self.source_lines(panel);
        if cursor_col > 0 {
            // Delete char before cursor
            let line = &mut lines[source_line];
            let byte_idx = char_to_byte_index(line, cursor_col - 1);
            let next_byte = char_to_byte_index(line, cursor_col);
            line.drain(byte_idx..next_byte);
            self.set_source_lines(panel, lines);
            if let Some(ref mut e) = self.edit_state {
                e.cursor_col -= 1;
                e.dirty = true;
            }
        } else if source_line > 0 && lines.len() > 1 {
            // Merge with previous line (guard: never remove the last remaining line)
            let current = lines.remove(source_line);
            let prev_len = lines[source_line - 1].chars().count();
            lines[source_line - 1].push_str(&current);
            self.set_source_lines(panel, lines);
            if let Some(ref mut e) = self.edit_state {
                e.source_line -= 1;
                e.cursor_col = prev_len;
                if e.display_line > 0 {
                    e.display_line -= 1;
                }
                e.dirty = true;
            }
        }
    }

    pub fn edit_delete(&mut self) {
        let es = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        let (panel, source_line, cursor_col) = es;

        let mut lines = self.source_lines(panel);
        let line_char_len = lines[source_line].chars().count();
        if cursor_col < line_char_len {
            // Delete char at cursor
            let line = &mut lines[source_line];
            let byte_idx = char_to_byte_index(line, cursor_col);
            let next_byte = char_to_byte_index(line, cursor_col + 1);
            line.drain(byte_idx..next_byte);
            self.set_source_lines(panel, lines);
            if let Some(ref mut e) = self.edit_state {
                e.dirty = true;
            }
        } else if source_line + 1 < lines.len() && lines.len() > 1 {
            // Merge with next line (guard: never remove the last remaining line)
            let next = lines.remove(source_line + 1);
            lines[source_line].push_str(&next);
            self.set_source_lines(panel, lines);
            if let Some(ref mut e) = self.edit_state {
                e.dirty = true;
            }
        }
    }

    pub fn edit_enter(&mut self) {
        let es = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        let (panel, source_line, cursor_col) = es;

        let mut lines = self.source_lines(panel);
        if source_line >= lines.len() {
            lines.resize(source_line + 1, String::new());
        }
        let line = &lines[source_line];
        let byte_idx = char_to_byte_index(line, cursor_col);
        let rest = line[byte_idx..].to_string();
        let first = line[..byte_idx].to_string();
        lines[source_line] = first;
        lines.insert(source_line + 1, rest);
        self.set_source_lines(panel, lines);

        if let Some(ref mut e) = self.edit_state {
            e.source_line += 1;
            e.display_line += 1;
            e.cursor_col = 0;
            e.dirty = true;
        }
    }

    pub fn edit_move_left(&mut self) {
        if let Some(ref mut e) = self.edit_state {
            if e.cursor_col > 0 {
                e.cursor_col -= 1;
            }
        }
    }

    pub fn edit_move_right(&mut self) {
        let info = self.edit_state.as_ref().map(|e| (e.panel, e.source_line));
        if let Some((panel, source_line)) = info {
            let lines = self.source_lines(panel);
            let line_len = lines.get(source_line).map(|l| l.chars().count()).unwrap_or(0);
            if let Some(ref mut e) = self.edit_state {
                if e.cursor_col < line_len {
                    e.cursor_col += 1;
                }
            }
        }
    }

    pub fn edit_move_up(&mut self) {
        let info = self.edit_state.as_ref().map(|e| (e.panel, e.source_line, e.cursor_col));
        if let Some((panel, source_line, cursor_col)) = info {
            if source_line > 0 {
                let lines = self.source_lines(panel);
                let line_len = lines.get(source_line - 1).map(|l| l.chars().count()).unwrap_or(0);
                if let Some(ref mut e) = self.edit_state {
                    e.source_line -= 1;
                    if e.display_line > 0 {
                        e.display_line -= 1;
                    }
                    e.cursor_col = cursor_col.min(line_len);
                }
            }
        }
    }

    pub fn edit_move_down(&mut self) {
        let info = self.edit_state.as_ref().map(|e| (e.panel, e.source_line, e.cursor_col));
        if let Some((panel, source_line, cursor_col)) = info {
            let lines = self.source_lines(panel);
            if source_line + 1 < lines.len() {
                let line_len = lines.get(source_line + 1).map(|l| l.chars().count()).unwrap_or(0);
                if let Some(ref mut e) = self.edit_state {
                    e.source_line += 1;
                    e.display_line += 1;
                    e.cursor_col = cursor_col.min(line_len);
                }
            }
        }
    }

    pub fn edit_move_home(&mut self) {
        if let Some(ref mut e) = self.edit_state {
            e.cursor_col = 0;
        }
    }

    pub fn edit_move_end(&mut self) {
        let info = self.edit_state.as_ref().map(|e| (e.panel, e.source_line));
        if let Some((panel, source_line)) = info {
            let lines = self.source_lines(panel);
            let line_len = lines.get(source_line).map(|l| l.chars().count()).unwrap_or(0);
            if let Some(ref mut e) = self.edit_state {
                e.cursor_col = line_len;
            }
        }
    }

    /// Get the current line text being edited (live from source)
    pub fn edit_current_line_text(&self) -> Option<String> {
        let e = self.edit_state.as_ref()?;
        let lines = self.source_lines(e.panel);
        lines.get(e.source_line).cloned()
    }
}

/// Convert char index to byte index in a string
fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app_with_diff(left: &str, right: &str) -> App {
        let mut app = App::new();
        app.left_text = left.to_string();
        app.right_text = right.to_string();
        app.left_path = Some(PathBuf::from("/tmp/left.txt"));
        app.right_path = Some(PathBuf::from("/tmp/right.txt"));
        app.recompute_diff();
        app
    }

    /// Blank screen: click left panel → edit → click right panel → edit
    #[test]
    fn test_blank_screen_edit_both_panels() {
        let mut app = App::new();
        app.new_blank(false);

        // Click left panel line 0
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        assert_eq!(app.edit_state.as_ref().unwrap().panel, PanelSide::Left);

        // Type "hello"
        for c in "hello".chars() {
            app.edit_insert_char(c);
        }
        assert!(app.left_text.contains("hello"));

        // Click right panel — should exit left edit and enter right
        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        assert_eq!(app.edit_state.as_ref().unwrap().panel, PanelSide::Right);

        // Type "world"
        for c in "world".chars() {
            app.edit_insert_char(c);
        }
        assert!(app.right_text.contains("world"));

        // Both texts preserved
        assert!(app.left_text.contains("hello"));
        assert!(app.right_text.contains("world"));
    }

    /// Edit → F5 compare → text is preserved
    #[test]
    fn test_edit_then_f5_preserves_text() {
        let mut app = App::new();
        app.new_blank(false);

        // Edit left
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "fn main() {}".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        assert!(app.left_text.contains("fn main() {}"));

        // Simulate F5: recompute diff
        app.recompute_diff();
        assert!(app.diff_result.is_some());
        // Text must survive
        assert!(app.left_text.contains("fn main() {}"));
    }

    /// Edit → F5 → copy → edit again (the reported bug)
    #[test]
    fn test_edit_f5_copy_edit_cycle() {
        let mut app = App::new();
        app.new_blank(false);

        // Edit left panel
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "line one".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        // F5 to compute diff
        app.recompute_diff();
        assert!(app.diff_result.is_some());
        let diff_count = app.diff_count();
        assert!(diff_count > 0, "should have diffs after editing left only");

        // Copy left to right
        app.copy_left_to_right();
        assert!(app.right_text.contains("line one"), "copy should transfer text");

        // After copy, edit must still work — click left panel
        let resolve = app.resolve_display_to_source(0, PanelSide::Left);
        assert!(resolve.is_some(), "resolve must succeed after copy");

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "must enter edit mode after copy");
        app.edit_insert_char('X');
        assert!(app.left_text.starts_with('X') || app.left_text.contains('X'));
        app.exit_edit_mode();

        // Right panel too
        let resolve_r = app.resolve_display_to_source(0, PanelSide::Right);
        assert!(resolve_r.is_some(), "resolve right must succeed after copy");
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        app.edit_insert_char('Y');
        assert!(app.right_text.contains('Y'));
        app.exit_edit_mode();
    }

    /// Repeated copy + edit cycles (stress test — WinXMerge 20K ops pattern)
    #[test]
    fn test_repeated_copy_edit_stress() {
        let mut app = make_app_with_diff(
            "alpha\nbeta\ngamma\n",
            "alpha\nBETA\ngamma\n",
        );

        for iteration in 0..50 {
            let diff_count = app.diff_count();
            if diff_count == 0 {
                break;
            }

            // Copy left to right
            app.copy_left_to_right();
            assert!(
                app.has_unsaved_changes,
                "iter {}: must have unsaved changes after copy",
                iteration
            );

            // Edit should still work
            let resolve = app.resolve_display_to_source(0, PanelSide::Left);
            assert!(
                resolve.is_some(),
                "iter {}: resolve must succeed",
                iteration
            );
            app.enter_edit_mode(PanelSide::Left, 0, 0);
            if app.mode == AppMode::Editing {
                app.edit_insert_char(' ');
                app.exit_edit_mode();
            }

            // F5 refresh
            app.recompute_diff();
        }

        // Text must not be empty
        assert!(!app.left_text.is_empty(), "left text must survive stress");
        assert!(!app.right_text.is_empty(), "right text must survive stress");
    }

    /// Ghost line editing — right panel has ghost lines when left has more content
    #[test]
    fn test_ghost_line_editing() {
        let mut app = make_app_with_diff(
            "line1\nline2\nline3\n",
            "line1\n",
        );

        // Right panel at display line 1 should be a ghost (only left has line2)
        // resolve_display_to_source should still return a valid line
        let result = app.resolve_display_to_source(1, PanelSide::Right);
        assert!(result.is_some(), "ghost line should resolve to nearest source line");

        // Should be able to enter edit mode on the resolved line
        app.enter_edit_mode(PanelSide::Right, 1, 0);
        assert_eq!(app.mode, AppMode::Editing);
        app.exit_edit_mode();
    }

    /// Undo after edit preserves state
    #[test]
    fn test_undo_after_edit() {
        let mut app = make_app_with_diff("hello\n", "world\n");
        let original_left = app.left_text.clone();

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('X');
        app.exit_edit_mode();

        assert!(app.left_text.contains('X'));

        app.undo();
        assert_eq!(app.left_text, original_left, "undo must restore original");
    }

    /// F5 never reloads from disk when text was edited
    #[test]
    fn test_f5_never_destroys_edits() {
        let mut app = make_app_with_diff("original\n", "original\n");

        // Edit left
        app.enter_edit_mode(PanelSide::Left, 0, 5);
        for c in "_modified".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();
        assert!(app.left_text.contains("_modified"));

        // Recompute (simulates F5 with unsaved changes)
        app.recompute_diff();
        assert!(
            app.left_text.contains("_modified"),
            "F5 must preserve edited text"
        );
    }

    /// Last line deletion guard
    #[test]
    fn test_last_line_deletion_guard() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('A');
        // Now source_line=0, cursor_col=1, single line "A"
        // Backspace should delete 'A' but not remove the line
        app.edit_backspace();
        let lines = app.source_lines(PanelSide::Left);
        assert!(lines.len() >= 1, "must keep at least one line");
        app.exit_edit_mode();
    }

    // ================================================================
    // IRON RULES — these must NEVER break
    // ================================================================

    /// IRON RULE 1: F5 must NEVER destroy displayed text.
    /// Text may only vanish when the user explicitly deletes or overwrites via copy.
    #[test]
    fn test_iron_rule_f5_never_destroys_display() {
        // Scenario: blank → edit both panels → F5 → text survives
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "left content".chars() { app.edit_insert_char(c); }
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        for c in "right content".chars() { app.edit_insert_char(c); }
        app.exit_edit_mode();

        let left_before = app.left_text.clone();
        let right_before = app.right_text.clone();

        // F5 (full recompute)
        app.recompute_diff();

        assert_eq!(app.left_text, left_before, "F5 destroyed left text");
        assert_eq!(app.right_text, right_before, "F5 destroyed right text");

        // F5 again
        app.recompute_diff();
        assert_eq!(app.left_text, left_before, "second F5 destroyed left text");
        assert_eq!(app.right_text, right_before, "second F5 destroyed right text");
    }

    /// IRON RULE 1b: F5 after copy must not destroy text.
    #[test]
    fn test_iron_rule_f5_after_copy_preserves() {
        let mut app = make_app_with_diff(
            "hello\nworld\n",
            "hello\nWORLD\n",
        );

        app.copy_left_to_right();
        let left_after_copy = app.left_text.clone();
        let right_after_copy = app.right_text.clone();

        app.recompute_diff();
        assert_eq!(app.left_text, left_after_copy, "F5 after copy destroyed left");
        assert_eq!(app.right_text, right_after_copy, "F5 after copy destroyed right");
    }

    /// IRON RULE 1c: Repeated edit → F5 → copy → F5 cycles.
    #[test]
    fn test_iron_rule_f5_repeated_cycles() {
        let mut app = App::new();
        app.new_blank(false);

        for i in 0..10 {
            // Edit left
            app.enter_edit_mode(PanelSide::Left, 0, 0);
            app.edit_insert_char(char::from(b'A' + (i % 26) as u8));
            app.exit_edit_mode();

            let left_snap = app.left_text.clone();
            let right_snap = app.right_text.clone();

            // F5
            app.recompute_diff();
            assert_eq!(app.left_text, left_snap, "cycle {i}: F5 destroyed left");
            assert_eq!(app.right_text, right_snap, "cycle {i}: F5 destroyed right");

            // Copy if there are diffs
            if app.diff_count() > 0 {
                app.copy_left_to_right();
            }

            let left_snap2 = app.left_text.clone();
            let right_snap2 = app.right_text.clone();

            // F5 again
            app.recompute_diff();
            assert_eq!(app.left_text, left_snap2, "cycle {i}: F5 after copy destroyed left");
            assert_eq!(app.right_text, right_snap2, "cycle {i}: F5 after copy destroyed right");
        }
    }

    /// IRON RULE 2: Every character typed must be reflected in source text.
    /// Input must never silently fail.
    #[test]
    fn test_iron_rule_input_always_reflected() {
        let mut app = make_app_with_diff("aaa\nbbb\n", "aaa\nBBB\n");

        // Edit left panel
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('Z');
        assert!(app.left_text.contains('Z'), "typed Z not in left_text");
        app.exit_edit_mode();

        // Copy
        app.copy_left_to_right();

        // Edit right panel after copy
        let resolve = app.resolve_display_to_source(0, PanelSide::Right);
        assert!(resolve.is_some(), "resolve failed after copy");
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "failed to enter edit after copy");
        app.edit_insert_char('W');
        assert!(app.right_text.contains('W'), "typed W not in right_text after copy");
        app.exit_edit_mode();

        // F5 then edit again
        app.recompute_diff();
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "failed to enter edit after F5");
        app.edit_insert_char('Q');
        assert!(app.left_text.contains('Q'), "typed Q not in left_text after F5");
        app.exit_edit_mode();
    }

    /// IRON RULE 2b: Input on blank screen both panels.
    #[test]
    fn test_iron_rule_input_blank_both_panels() {
        let mut app = App::new();
        app.new_blank(false);

        // Must be able to edit left
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "can't enter edit on blank left");
        app.edit_insert_char('L');
        assert!(app.left_text.contains('L'), "left input lost on blank");
        app.exit_edit_mode();

        // After left edit, right panel is ghost lines in diff.
        // Must STILL be able to edit right (the reported bug).
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "can't enter edit on ghost right");
        app.edit_insert_char('R');
        assert!(app.right_text.contains('R'), "right input lost on blank");
        app.exit_edit_mode();

        // Both survive
        assert!(app.left_text.contains('L'), "left lost after right edit");
        assert!(app.right_text.contains('R'), "right lost after left exit");
    }

    /// IRON RULE 2c: Ghost line editing — edit right panel when left has content.
    /// Simulates actual mouse click flow: exit_edit_mode → enter_edit_mode.
    #[test]
    fn test_iron_rule_ghost_line_input_reflected() {
        let mut app = App::new();
        app.new_blank(false);

        // Step 1: Click left panel, type text
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "left only".chars() { app.edit_insert_char(c); }

        // Step 2: Click right panel (simulates actual mouse click flow)
        // This is what handle_mouse_click does: exit current → enter new
        app.exit_edit_mode();
        // After exit, recompute ran. Diff now has Removed lines.
        assert!(app.diff_result.is_some(), "diff should exist after exit");
        let result = app.diff_result.as_ref().unwrap();
        assert!(!result.lines.is_empty(), "diff should have lines");
        // Right side is all ghost
        assert!(result.lines.iter().all(|l| l.right_line_no.is_none()),
            "right should be all ghost lines");

        // Enter right panel on display line 0
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "must enter edit on ghost right");
        assert_eq!(app.edit_state.as_ref().unwrap().panel, PanelSide::Right);

        // Type and verify
        app.edit_insert_char('R');
        assert!(app.right_text.contains('R'), "ghost line input not in source");

        let live = app.edit_current_line_text();
        assert!(live.is_some(), "edit_current_line_text None on ghost");
        assert!(live.unwrap().contains('R'), "live text missing typed char");

        app.exit_edit_mode();
        assert!(app.right_text.contains('R'), "right text lost after exit");
        assert!(app.left_text.contains("left only"), "left text corrupted");
    }

    /// IRON RULE 2d: Click on row beyond diff lines must still allow editing.
    /// User clicks row 5 but diff only has 1 line → display_line must be clamped.
    #[test]
    fn test_iron_rule_click_beyond_diff_lines() {
        let mut app = App::new();
        app.new_blank(false);

        // Edit left, creating content
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "hello".chars() { app.edit_insert_char(c); }
        app.exit_edit_mode(); // recompute → 1 Removed line

        let diff_lines = app.diff_result.as_ref().unwrap().lines.len();
        assert_eq!(diff_lines, 1, "should have exactly 1 diff line");

        // Simulate clicking row 5 on right panel (beyond diff content)
        app.enter_edit_mode(PanelSide::Right, 5, 0);
        assert_eq!(app.mode, AppMode::Editing, "must enter edit even clicking beyond");

        // display_line must be clamped to valid range
        let es = app.edit_state.as_ref().unwrap();
        assert!(es.display_line < diff_lines,
            "display_line {} must be < diff lines {}", es.display_line, diff_lines);

        // Input must work
        app.edit_insert_char('W');
        assert!(app.right_text.contains('W'), "input lost on clamped ghost line");
        app.exit_edit_mode();
    }

    /// IRON RULE 2e: Click row beyond source lines → source_line must be clamped.
    /// This is the EXACT bug from the debug log: src=1 when right has only 1 line.
    #[test]
    fn test_iron_rule_source_line_clamped_to_bounds() {
        let mut app = App::new();
        app.new_blank(false);

        // Edit left with 2 lines
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "line1".chars() { app.edit_insert_char(c); }
        app.edit_enter(); // new line
        for c in "line2".chars() { app.edit_insert_char(c); }
        app.exit_edit_mode();
        // left has 2 lines, right is empty

        // Edit right, type 1 line
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        for c in "addfa".chars() { app.edit_insert_char(c); }

        // Now simulate: click row 1 of right panel while editing right
        // This is exit_edit_mode → enter_edit_mode(Right, 1, 0)
        app.exit_edit_mode();
        // After recompute, diff has 2+ lines (left 2 lines vs right 1 line)
        // Right panel at display row 1 is ghost (right has only 1 line)

        app.enter_edit_mode(PanelSide::Right, 1, 0);
        assert_eq!(app.mode, AppMode::Editing);

        // source_line MUST be clamped to right's source bounds (0..0)
        let es = app.edit_state.as_ref().unwrap();
        let right_line_count = app.source_lines(PanelSide::Right).len();
        assert!(es.source_line < right_line_count,
            "source_line {} >= right line count {} — will cause live=None!",
            es.source_line, right_line_count);

        // Input must produce live text
        app.edit_insert_char('X');
        let live = app.edit_current_line_text();
        assert!(live.is_some(), "live text is None — input invisible to user!");
        assert!(live.unwrap().contains('X'), "typed X not in live text");
        app.exit_edit_mode();
    }

    /// IRON RULE 2f: Verify rendering logic — is_edit_line must be true during ghost editing.
    /// This replicates the EXACT diff_view.rs rendering check.
    #[test]
    fn test_iron_rule_render_ghost_edit_visible() {
        let mut app = App::new();
        app.new_blank(false);

        // Type "aaa" in left
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "aaa".chars() { app.edit_insert_char(c); }

        // Click right panel row 0 (exit left, enter right)
        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Right, 0, 0);

        // Type "bbb"
        for c in "bbb".chars() { app.edit_insert_char(c); }

        // Now verify the EXACT rendering logic from diff_view.rs
        let edit_info = app.edit_state.as_ref()
            .map(|e| (e.panel, e.source_line, e.display_line, e.cursor_col));
        let edit_live_text = app.edit_current_line_text();

        assert!(edit_info.is_some(), "edit_info is None");
        assert!(edit_live_text.is_some(), "edit_live_text is None — input invisible!");
        assert!(edit_live_text.as_ref().unwrap().contains("bbb"),
            "live text '{}' doesn't contain 'bbb'", edit_live_text.as_ref().unwrap());

        let (panel, source_line, display_line, _cursor_col) = edit_info.unwrap();

        // Check diff result
        let result = app.diff_result.as_ref().expect("diff_result is None");
        eprintln!("diff has {} lines", result.lines.len());
        for (idx, dl) in result.lines.iter().enumerate() {
            eprintln!("  line {}: status={:?} left_no={:?} right_no={:?}",
                idx, dl.status, dl.left_line_no, dl.right_line_no);
        }

        // Simulate the diff_view rendering loop
        let mut found_edit_line = false;
        for i in 0..result.lines.len() {
            let line = &result.lines[i];
            let line_no = match panel {
                PanelSide::Left => line.left_line_no,
                PanelSide::Right => line.right_line_no,
                _ => None,
            };
            let is_edit_line = if let Some(n) = line_no {
                n as usize - 1 == source_line
            } else {
                // Ghost fallback
                i == display_line
            };
            eprintln!("  render i={} line_no={:?} is_edit={} (src={} disp={})",
                i, line_no, is_edit_line, source_line, display_line);
            if is_edit_line {
                found_edit_line = true;
            }
        }

        assert!(found_edit_line,
            "RENDERING BUG: no display line matched edit state! \
             panel={:?} src={} disp={} diff_lines={}",
            panel, source_line, display_line, result.lines.len());

        app.exit_edit_mode();
    }

    /// IRON RULE 2g: Exact user flow — blank → type left → click right → type right.
    /// The click right triggers exit_edit_mode (recompute) then enter_edit_mode.
    #[test]
    fn test_iron_rule_click_switch_panel_flow() {
        let mut app = App::new();
        app.new_blank(false);

        // Click left, type
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "hello".chars() { app.edit_insert_char(c); }
        assert_eq!(app.mode, AppMode::Editing);

        // Now simulate clicking on right panel while editing left.
        // handle_mouse_click does: exit → enter
        app.exit_edit_mode();
        // After exit_edit_mode, diff is recomputed with left="hello", right="".
        // Verify mode and state
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.edit_state.is_none());

        // The diff result now exists with lines
        let _diff_line_count = app.diff_result.as_ref().map(|r| r.lines.len()).unwrap_or(0);

        // Enter right panel at display line 0
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "FAILED to enter edit on right after switch");
        let es = app.edit_state.as_ref().unwrap();
        assert_eq!(es.panel, PanelSide::Right);

        // Type into right panel
        for c in "world".chars() { app.edit_insert_char(c); }
        assert!(app.right_text.contains("world"), "right input LOST");

        // Verify live text is available for rendering
        let live = app.edit_current_line_text();
        assert!(live.is_some(), "no live text for renderer");
        assert!(live.unwrap().contains("world"), "live text missing");

        app.exit_edit_mode();

        // Both texts intact
        assert!(app.left_text.contains("hello"), "left text corrupted");
        assert!(app.right_text.contains("world"), "right text lost after exit");
    }

    /// Text only vanishes via explicit user action: delete or copy overwrite.
    #[test]
    fn test_text_only_vanishes_by_user_action() {
        let mut app = make_app_with_diff(
            "keep this\nchange this\n",
            "keep this\nDIFFERENT\n",
        );

        // Copy overwrites right with left — this IS intentional
        app.copy_left_to_right();
        assert!(app.right_text.contains("change this"), "copy should overwrite");
        assert!(!app.right_text.contains("DIFFERENT"), "copy should replace old right");

        // Undo restores
        app.undo();
        assert!(app.right_text.contains("DIFFERENT"), "undo must restore");
    }

    // ================================================================
    // 3-WAY EDITING TESTS
    // ================================================================

    fn make_3way_app(left: &str, base: &str, right: &str) -> App {
        let mut app = App::new();
        app.left_text = left.to_string();
        app.base_text = base.to_string();
        app.right_text = right.to_string();
        app.is_three_way = true;
        app.left_path = Some(PathBuf::from("/tmp/left.txt"));
        app.base_path = Some(PathBuf::from("/tmp/base.txt"));
        app.right_path = Some(PathBuf::from("/tmp/right.txt"));
        app.recompute_diff();
        app
    }

    #[test]
    fn test_3way_blank_edit_all_panels() {
        let mut app = App::new();
        app.new_blank(true);

        for panel in [PanelSide::Left, PanelSide::Base, PanelSide::Right] {
            app.enter_edit_mode(panel, 0, 0);
            assert_eq!(app.mode, AppMode::Editing, "{:?} failed to enter edit", panel);
            app.edit_insert_char('X');

            let live = app.edit_current_line_text();
            assert!(live.is_some(), "{:?} live text None", panel);
            assert!(live.unwrap().contains('X'), "{:?} typed X not in live", panel);
            app.exit_edit_mode();
        }

        assert!(app.left_text.contains('X'));
        assert!(app.base_text.contains('X'));
        assert!(app.right_text.contains('X'));
    }

    #[test]
    fn test_3way_edit_base_preserved_on_f5() {
        let mut app = make_3way_app("left\n", "base\n", "right\n");

        app.enter_edit_mode(PanelSide::Base, 0, 4);
        for c in "_edited".chars() { app.edit_insert_char(c); }
        app.exit_edit_mode();

        assert!(app.base_text.contains("base_edited"));

        app.recompute_diff();
        assert!(app.base_text.contains("base_edited"), "F5 destroyed base edit");
    }

    #[test]
    fn test_3way_switch_panels_left_base_right() {
        let mut app = App::new();
        app.new_blank(true);

        // Edit left
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "LEFT".chars() { app.edit_insert_char(c); }

        // Switch to base (exit left, enter base)
        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Base, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        for c in "BASE".chars() { app.edit_insert_char(c); }

        // Switch to right
        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        for c in "RIGHT".chars() { app.edit_insert_char(c); }
        app.exit_edit_mode();

        assert!(app.left_text.contains("LEFT"), "left lost");
        assert!(app.base_text.contains("BASE"), "base lost");
        assert!(app.right_text.contains("RIGHT"), "right lost");
    }

    #[test]
    fn test_3way_iron_rule_input_always_reflected() {
        let mut app = make_3way_app("aaa\n", "bbb\n", "ccc\n");

        for panel in [PanelSide::Left, PanelSide::Base, PanelSide::Right] {
            app.enter_edit_mode(panel, 0, 0);
            assert_eq!(app.mode, AppMode::Editing, "{:?} edit mode failed", panel);
            app.edit_insert_char('Z');

            let live = app.edit_current_line_text();
            assert!(live.is_some(), "{:?} live=None", panel);
            assert!(live.unwrap().contains('Z'), "{:?} Z missing from live", panel);
            app.exit_edit_mode();
        }
    }

    #[test]
    fn test_3way_iron_rule_f5_never_destroys() {
        let mut app = make_3way_app("left\n", "base\n", "right\n");

        // Edit all three
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('L');
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Base, 0, 0);
        app.edit_insert_char('B');
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        app.edit_insert_char('R');
        app.exit_edit_mode();

        let l = app.left_text.clone();
        let b = app.base_text.clone();
        let r = app.right_text.clone();

        app.recompute_diff();
        assert_eq!(app.left_text, l, "F5 destroyed left");
        assert_eq!(app.base_text, b, "F5 destroyed base");
        assert_eq!(app.right_text, r, "F5 destroyed right");
    }
}
