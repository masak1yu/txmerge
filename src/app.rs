use std::fs;
use std::path::PathBuf;

use crate::diff::dir_compare::scan_dirs;
use crate::diff::engine::{DiffOptions, compute_diff};
use crate::diff::three_way::compute_three_way_diff;
use crate::file_browser::FileBrowser;
use crate::models::diff_line::{
    DirCompareResult, DiffResult, LineStatus, ThreeWayResult, ThreeWayStatus,
};

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
    CloseTabConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PanelSide {
    Left,
    Right,
    Base,
}

pub struct EditState {
    pub panel: PanelSide,
    pub source_line: usize,  // 0-based index into pane buffer
    pub display_line: usize, // index in diff result lines
    pub cursor_col: usize,   // char index in line
    pub dirty: bool,
}

#[derive(Clone)]
struct TextSnapshot {
    left_buf: Vec<String>,
    right_buf: Vec<String>,
    base_buf: Vec<String>,
}

// ============================================================
// TabState — holds all per-document state
// ============================================================

pub struct TabState {
    pub left_path: Option<PathBuf>,
    pub right_path: Option<PathBuf>,
    pub base_path: Option<PathBuf>,
    pub left_buf: Vec<String>,
    pub right_buf: Vec<String>,
    pub base_buf: Vec<String>,
    pub diff_result: Option<DiffResult>,
    pub three_way_result: Option<ThreeWayResult>,
    pub is_three_way: bool,
    pub is_dir_compare: bool,
    pub dir_result: Option<DirCompareResult>,
    pub diff_options: DiffOptions,
    pub current_diff: i32,
    pub scroll_offset: usize,
    pub h_scroll: usize,
    pub edit_state: Option<EditState>,
    pub has_unsaved_changes: bool,
    pub select_all: bool,
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
}

impl TabState {
    pub fn new() -> Self {
        Self {
            left_path: None,
            right_path: None,
            base_path: None,
            left_buf: Vec::new(),
            right_buf: Vec::new(),
            base_buf: Vec::new(),
            diff_result: None,
            three_way_result: None,
            is_three_way: false,
            is_dir_compare: false,
            dir_result: None,
            diff_options: DiffOptions::default(),
            current_diff: -1,
            scroll_offset: 0,
            h_scroll: 0,
            edit_state: None,
            has_unsaved_changes: false,
            select_all: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn title(&self) -> String {
        if self.is_dir_compare {
            return self
                .left_path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Dirs".to_string());
        }
        self.left_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "New".to_string())
    }

    pub fn open_dirs(&mut self, left: PathBuf, right: PathBuf) {
        let result = scan_dirs(&left, &right);
        self.left_path = Some(left);
        self.right_path = Some(right);
        self.base_path = None;
        self.left_buf.clear();
        self.right_buf.clear();
        self.base_buf.clear();
        self.diff_result = None;
        self.three_way_result = None;
        self.is_three_way = false;
        self.is_dir_compare = true;
        self.dir_result = Some(result);
        self.has_unsaved_changes = false;
        self.h_scroll = 0;
        self.scroll_offset = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn open_files(&mut self, left: PathBuf, right: PathBuf) {
        self.left_buf = str_to_lines(&fs::read_to_string(&left).unwrap_or_default());
        self.right_buf = str_to_lines(&fs::read_to_string(&right).unwrap_or_default());
        self.left_path = Some(left);
        self.right_path = Some(right);
        self.base_path = None;
        self.base_buf = vec![String::new()];
        self.is_three_way = false;
        self.three_way_result = None;
        self.has_unsaved_changes = false;
        self.h_scroll = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.recompute_diff();
    }

    pub fn open_files_3way(&mut self, left: PathBuf, base: PathBuf, right: PathBuf) {
        self.left_buf = str_to_lines(&fs::read_to_string(&left).unwrap_or_default());
        self.base_buf = str_to_lines(&fs::read_to_string(&base).unwrap_or_default());
        self.right_buf = str_to_lines(&fs::read_to_string(&right).unwrap_or_default());
        self.left_path = Some(left);
        self.base_path = Some(base);
        self.right_path = Some(right);
        self.is_three_way = true;
        self.diff_result = None;
        self.has_unsaved_changes = false;
        self.h_scroll = 0;
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
        if self.is_three_way {
            let result = compute_three_way_diff(&self.base_buf, &self.left_buf, &self.right_buf);
            if reset_position {
                self.current_diff = if !result.diff_positions.is_empty() {
                    0
                } else {
                    -1
                };
            } else {
                let count = result.diff_positions.len() as i32;
                if self.current_diff >= count {
                    self.current_diff = count - 1;
                }
            }
            self.three_way_result = Some(result);
            self.diff_result = None;
        } else {
            let result = compute_diff(&self.left_buf, &self.right_buf, &self.diff_options);
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
                    self.left_buf
                        .len()
                        .max(self.base_buf.len())
                        .max(self.right_buf.len())
                        .max(1)
                })
        } else {
            self.diff_result
                .as_ref()
                .map(|r| r.lines.len())
                .unwrap_or_else(|| self.left_buf.len().max(self.right_buf.len()).max(1))
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
            left_buf: self.left_buf.clone(),
            right_buf: self.right_buf.clone(),
            base_buf: self.base_buf.clone(),
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snapshot) = self.undo_stack.pop() {
            self.redo_stack.push(TextSnapshot {
                left_buf: self.left_buf.clone(),
                right_buf: self.right_buf.clone(),
                base_buf: self.base_buf.clone(),
            });
            self.left_buf = snapshot.left_buf;
            self.right_buf = snapshot.right_buf;
            self.base_buf = snapshot.base_buf;
            self.recompute_diff();
            self.has_unsaved_changes = !self.undo_stack.is_empty();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snapshot) = self.redo_stack.pop() {
            self.undo_stack.push(TextSnapshot {
                left_buf: self.left_buf.clone(),
                right_buf: self.right_buf.clone(),
                base_buf: self.base_buf.clone(),
            });
            self.left_buf = snapshot.left_buf;
            self.right_buf = snapshot.right_buf;
            self.base_buf = snapshot.base_buf;
            self.recompute_diff();
            self.has_unsaved_changes = true;
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = self.total_lines().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn h_scroll_right(&mut self, amount: usize) {
        self.h_scroll = self.h_scroll.saturating_add(amount);
    }

    pub fn h_scroll_left(&mut self, amount: usize) {
        self.h_scroll = self.h_scroll.saturating_sub(amount);
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
            self.apply_copy(block_idx, false);
            self.has_unsaved_changes = true;
        }
    }

    /// 3-way copy: Left->Base (use_left=true) or Right->Base (use_left=false)
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
        while end < result.lines.len() && result.lines[end].status != ThreeWayStatus::Equal {
            end += 1;
        }

        self.push_undo();
        self.edit_state = None;

        // Rebuild base buffer: replace the diff block with source lines
        let mut base_lines = self.base_buf.clone();
        let source_lines: Vec<String> = if use_left {
            result.lines[start..end]
                .iter()
                .filter_map(|l| {
                    l.left_line_no
                        .map(|n| self.left_buf[n as usize - 1].clone())
                })
                .collect()
        } else {
            result.lines[start..end]
                .iter()
                .filter_map(|l| {
                    l.right_line_no
                        .map(|n| self.right_buf[n as usize - 1].clone())
                })
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
                let be = be.min(base_lines.len());
                base_lines.splice(bs..be, source_lines);
            }
            _ => {
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

        self.base_buf = base_lines;
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
            self.base_buf = self.left_buf.clone();
        } else {
            self.right_buf = self.left_buf.clone();
        }
        self.has_unsaved_changes = true;
        self.recompute_diff();
    }

    pub fn copy_all_right_to_left(&mut self) {
        self.push_undo();
        if self.is_three_way {
            self.base_buf = self.right_buf.clone();
        } else {
            self.left_buf = self.right_buf.clone();
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

        let mut left_lines = self.left_buf.clone();
        let mut right_lines = self.right_buf.clone();

        if left_to_right {
            let mut right_removes = Vec::new();
            let mut left_source = Vec::new();
            let mut insert_pos: Option<usize> = None;

            for line in &result.lines[start..end] {
                match line.status {
                    LineStatus::Modified => {
                        if let (Some(rn), Some(ln)) = (line.right_line_no, line.left_line_no) {
                            right_lines[rn as usize - 1] = self.left_buf[ln as usize - 1].clone();
                        }
                    }
                    LineStatus::Removed => {
                        if let Some(ln) = line.left_line_no {
                            left_source.push(self.left_buf[ln as usize - 1].clone());
                        }
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
                        if let (Some(ln), Some(rn)) = (line.left_line_no, line.right_line_no) {
                            left_lines[ln as usize - 1] = self.right_buf[rn as usize - 1].clone();
                        }
                    }
                    LineStatus::Added => {
                        if let Some(rn) = line.right_line_no {
                            right_source.push(self.right_buf[rn as usize - 1].clone());
                        }
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

        self.left_buf = left_lines;
        self.right_buf = right_lines;
        self.recompute_diff_keep_pos();
    }

    // === New File ===

    pub fn new_blank(&mut self, is_three_way: bool) {
        self.left_buf = Vec::new();
        self.right_buf = Vec::new();
        self.base_buf = Vec::new();
        self.left_path = None;
        self.right_path = None;
        self.base_path = None;
        self.is_three_way = is_three_way;
        self.is_dir_compare = false;
        self.dir_result = None;
        self.has_unsaved_changes = false;
        self.h_scroll = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.edit_state = None;
        self.recompute_diff();
    }

    // === Buffer accessors ===

    fn buf(&self, panel: PanelSide) -> &[String] {
        match panel {
            PanelSide::Left => &self.left_buf,
            PanelSide::Right => &self.right_buf,
            PanelSide::Base => &self.base_buf,
        }
    }

    fn buf_mut(&mut self, panel: PanelSide) -> &mut Vec<String> {
        match panel {
            PanelSide::Left => &mut self.left_buf,
            PanelSide::Right => &mut self.right_buf,
            PanelSide::Base => &mut self.base_buf,
        }
    }

    pub fn source_lines(&self, panel: PanelSide) -> Vec<String> {
        self.buf(panel).to_vec()
    }

    /// True if the pane has no meaningful content (empty or single empty line).
    pub fn pane_is_empty(&self, panel: PanelSide) -> bool {
        let buf = self.buf(panel);
        buf.is_empty() || buf.iter().all(|l| l.is_empty())
    }

    /// Joined text representation for file I/O and test assertions.
    pub fn left_text(&self) -> String {
        buf_to_text(&self.left_buf)
    }

    pub fn right_text(&self) -> String {
        buf_to_text(&self.right_buf)
    }

    #[cfg(test)]
    pub fn base_text(&self) -> String {
        buf_to_text(&self.base_buf)
    }

    /// Resolve display line index to source line number for a given panel.
    /// For ghost lines, returns the nearest valid source line to allow editing.
    pub fn resolve_display_to_source(&self, display_idx: usize, panel: PanelSide) -> Option<usize> {
        let line_count = self.buf(panel).len();

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
                    return Some(
                        self.find_nearest_source_line(
                            display_idx,
                            &result
                                .lines
                                .iter()
                                .map(|l| match panel {
                                    PanelSide::Left => l.left_line_no,
                                    PanelSide::Right => l.right_line_no,
                                    PanelSide::Base => l.base_line_no,
                                })
                                .collect::<Vec<_>>(),
                            line_count,
                        ),
                    );
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
                let line_nos: Vec<Option<u32>> = result
                    .lines
                    .iter()
                    .map(|l| match panel {
                        PanelSide::Left => l.left_line_no,
                        PanelSide::Right => l.right_line_no,
                        PanelSide::Base => None,
                    })
                    .collect();
                return Some(self.find_nearest_source_line(display_idx, &line_nos, line_count));
            }
        }

        // No diff result (blank screen) — clamp to source lines
        Some(display_idx.min(line_count.saturating_sub(1)))
    }

    /// Find the nearest real source line for a ghost display line.
    fn find_nearest_source_line(
        &self,
        display_idx: usize,
        line_nos: &[Option<u32>],
        line_count: usize,
    ) -> usize {
        for i in (0..display_idx).rev() {
            if let Some(n) = line_nos[i] {
                return ((n as usize).min(line_count)).saturating_sub(1);
            }
        }
        for i in display_idx..line_nos.len() {
            if let Some(n) = line_nos[i] {
                return (n as usize).saturating_sub(1);
            }
        }
        line_count.saturating_sub(1)
    }

    pub fn enter_edit_mode(&mut self, panel: PanelSide, display_line: usize, col: usize) {
        let source_line = match self.resolve_display_to_source(display_line, panel) {
            Some(sl) => sl,
            None => return,
        };

        let buf = self.buf(panel);
        let source_line = source_line.min(buf.len().saturating_sub(1));

        let display_line = self.find_display_index(source_line, panel, display_line);

        let line_len = self
            .buf(panel)
            .get(source_line)
            .map(|l| l.chars().count())
            .unwrap_or(0);
        let col = col.min(line_len);

        self.push_undo();
        self.edit_state = Some(EditState {
            panel,
            source_line,
            display_line,
            cursor_col: col,
            dirty: false,
        });
    }

    fn find_display_index(
        &self,
        source_line: usize,
        panel: PanelSide,
        clicked_display: usize,
    ) -> usize {
        let lines = if self.is_three_way {
            self.three_way_result.as_ref().map(|r| r.lines.len())
        } else {
            self.diff_result.as_ref().map(|r| r.lines.len())
        };

        let total = match lines {
            Some(n) if n > 0 => n,
            _ => return clicked_display,
        };

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

        clicked_display.min(total.saturating_sub(1))
    }

    pub fn exit_edit_mode(&mut self) {
        let dirty = self.edit_state.as_ref().map(|e| e.dirty).unwrap_or(false);
        self.edit_state = None;
        if dirty {
            self.has_unsaved_changes = true;
            self.recompute_diff_keep_pos();
        } else {
            self.undo_stack.pop();
        }
    }

    pub fn edit_insert_char(&mut self, ch: char) {
        let (panel, source_line, cursor_col) = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        {
            let buf = self.buf_mut(panel);
            if source_line >= buf.len() {
                buf.resize(source_line + 1, String::new());
            }
            let line = &mut buf[source_line];
            let byte_idx = char_to_byte_index(line, cursor_col);
            line.insert(byte_idx, ch);
        }
        if let Some(ref mut e) = self.edit_state {
            e.cursor_col += 1;
            e.dirty = true;
        }
    }

    pub fn edit_backspace(&mut self) {
        let (panel, source_line, cursor_col) = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        if cursor_col > 0 {
            {
                let buf = self.buf_mut(panel);
                let line = &mut buf[source_line];
                let byte_idx = char_to_byte_index(line, cursor_col - 1);
                let next_byte = char_to_byte_index(line, cursor_col);
                line.drain(byte_idx..next_byte);
            }
            if let Some(ref mut e) = self.edit_state {
                e.cursor_col -= 1;
                e.dirty = true;
            }
        } else if source_line > 0 {
            let prev_len = {
                let buf = self.buf_mut(panel);
                if buf.len() > 1 {
                    let current = buf.remove(source_line);
                    let prev_len = buf[source_line - 1].chars().count();
                    buf[source_line - 1].push_str(&current);
                    Some(prev_len)
                } else {
                    None
                }
            };
            if let Some(prev_len) = prev_len {
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
    }

    pub fn edit_delete(&mut self) {
        let (panel, source_line, cursor_col) = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        let line_char_len = self
            .buf(panel)
            .get(source_line)
            .map(|l| l.chars().count())
            .unwrap_or(0);
        if cursor_col < line_char_len {
            {
                let buf = self.buf_mut(panel);
                let line = &mut buf[source_line];
                let byte_idx = char_to_byte_index(line, cursor_col);
                let next_byte = char_to_byte_index(line, cursor_col + 1);
                line.drain(byte_idx..next_byte);
            }
            if let Some(ref mut e) = self.edit_state {
                e.dirty = true;
            }
        } else {
            let merged = {
                let buf = self.buf_mut(panel);
                if source_line + 1 < buf.len() && buf.len() > 1 {
                    let next = buf.remove(source_line + 1);
                    buf[source_line].push_str(&next);
                    true
                } else {
                    false
                }
            };
            if merged {
                if let Some(ref mut e) = self.edit_state {
                    e.dirty = true;
                }
            }
        }
    }

    pub fn edit_enter(&mut self) {
        let (panel, source_line, cursor_col) = match self.edit_state.as_ref() {
            Some(e) => (e.panel, e.source_line, e.cursor_col),
            None => return,
        };
        {
            let buf = self.buf_mut(panel);
            if source_line >= buf.len() {
                buf.resize(source_line + 1, String::new());
            }
            let byte_idx = char_to_byte_index(&buf[source_line], cursor_col);
            let rest = buf[source_line][byte_idx..].to_string();
            let first = buf[source_line][..byte_idx].to_string();
            buf[source_line] = first;
            buf.insert(source_line + 1, rest);
        }
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
            let line_len = self
                .buf(panel)
                .get(source_line)
                .map(|l| l.chars().count())
                .unwrap_or(0);
            if let Some(ref mut e) = self.edit_state {
                if e.cursor_col < line_len {
                    e.cursor_col += 1;
                }
            }
        }
    }

    pub fn edit_move_up(&mut self) {
        let info = self
            .edit_state
            .as_ref()
            .map(|e| (e.panel, e.source_line, e.cursor_col));
        if let Some((panel, source_line, cursor_col)) = info {
            if source_line > 0 {
                let line_len = self
                    .buf(panel)
                    .get(source_line - 1)
                    .map(|l| l.chars().count())
                    .unwrap_or(0);
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
        let info = self
            .edit_state
            .as_ref()
            .map(|e| (e.panel, e.source_line, e.cursor_col));
        if let Some((panel, source_line, cursor_col)) = info {
            let buf_len = self.buf(panel).len();
            if source_line + 1 < buf_len {
                let line_len = self
                    .buf(panel)
                    .get(source_line + 1)
                    .map(|l| l.chars().count())
                    .unwrap_or(0);
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
            let line_len = self
                .buf(panel)
                .get(source_line)
                .map(|l| l.chars().count())
                .unwrap_or(0);
            if let Some(ref mut e) = self.edit_state {
                e.cursor_col = line_len;
            }
        }
    }

    /// Get the current line text being edited (live from source buffer)
    pub fn edit_current_line_text(&self) -> Option<String> {
        let e = self.edit_state.as_ref()?;
        self.buf(e.panel).get(e.source_line).cloned()
    }
}

// ============================================================
// App — top-level application state with tab management
// ============================================================

pub struct App {
    pub tabs: Vec<TabState>,
    pub active_tab: usize,
    pub mode: AppMode,
    pub file_browser: Option<FileBrowser>,
    pub should_quit: bool,
    pub status_message: Option<(String, std::time::Instant)>,
    pub new_file_pending: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            tabs: vec![TabState::new()],
            active_tab: 0,
            mode: AppMode::Normal,
            file_browser: None,
            should_quit: false,
            status_message: None,
            new_file_pending: false,
        }
    }

    pub fn active_tab(&self) -> &TabState {
        &self.tabs[self.active_tab]
    }

    pub fn active_tab_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active_tab]
    }

    // === Tab management ===

    pub fn new_tab(&mut self) {
        self.tabs.push(TabState::new());
        self.active_tab = self.tabs.len() - 1;
        self.mode = AppMode::Normal;
        self.file_browser = None;
    }

    pub fn close_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.mode = AppMode::Normal;
        self.file_browser = None;
        true
    }

    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            if self.active_tab == 0 {
                self.active_tab = self.tabs.len() - 1;
            } else {
                self.active_tab -= 1;
            }
        }
    }

    pub fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    pub fn any_unsaved(&self) -> bool {
        self.tabs.iter().any(|t| t.has_unsaved_changes)
    }

    // === Delegation methods for per-tab operations ===

    pub fn open_files(&mut self, left: PathBuf, right: PathBuf) {
        self.active_tab_mut().open_files(left, right);
    }

    pub fn open_files_3way(&mut self, left: PathBuf, base: PathBuf, right: PathBuf) {
        self.active_tab_mut().open_files_3way(left, base, right);
    }

    pub fn recompute_diff(&mut self) {
        self.active_tab_mut().recompute_diff();
    }

    #[allow(dead_code)]
    pub fn diff_count(&self) -> u32 {
        self.active_tab().diff_count()
    }

    pub fn total_lines(&self) -> usize {
        self.active_tab().total_lines()
    }

    pub fn next_diff(&mut self) {
        self.active_tab_mut().next_diff();
    }

    pub fn prev_diff(&mut self) {
        self.active_tab_mut().prev_diff();
    }

    pub fn first_diff(&mut self) {
        self.active_tab_mut().first_diff();
    }

    pub fn last_diff(&mut self) {
        self.active_tab_mut().last_diff();
    }

    pub fn undo_stack_is_empty(&self) -> bool {
        self.active_tab().undo_stack_is_empty()
    }

    pub fn undo(&mut self) {
        self.active_tab_mut().undo();
    }

    pub fn redo(&mut self) {
        self.active_tab_mut().redo();
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.active_tab_mut().scroll_down(amount);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.active_tab_mut().scroll_up(amount);
    }

    pub fn h_scroll_right(&mut self, amount: usize) {
        self.active_tab_mut().h_scroll_right(amount);
    }

    pub fn h_scroll_left(&mut self, amount: usize) {
        self.active_tab_mut().h_scroll_left(amount);
    }

    pub fn copy_left_to_right(&mut self) {
        if self.active_tab().select_all {
            self.active_tab_mut().copy_all_left_to_right();
            self.active_tab_mut().select_all = false;
        } else {
            self.active_tab_mut().copy_left_to_right();
        }
    }

    pub fn copy_right_to_left(&mut self) {
        if self.active_tab().select_all {
            self.active_tab_mut().copy_all_right_to_left();
            self.active_tab_mut().select_all = false;
        } else {
            self.active_tab_mut().copy_right_to_left();
        }
    }

    pub fn copy_left_to_right_and_next(&mut self) {
        self.active_tab_mut().copy_left_to_right_and_next();
    }

    pub fn copy_right_to_left_and_next(&mut self) {
        self.active_tab_mut().copy_right_to_left_and_next();
    }

    pub fn toggle_select_all(&mut self) {
        let tab = self.active_tab_mut();
        tab.select_all = !tab.select_all;
    }

    pub fn toggle_ignore_whitespace(&mut self) {
        self.active_tab_mut().toggle_ignore_whitespace();
    }

    pub fn toggle_ignore_case(&mut self) {
        self.active_tab_mut().toggle_ignore_case();
    }

    pub fn new_blank(&mut self, is_three_way: bool) {
        self.active_tab_mut().new_blank(is_three_way);
    }

    // === Directory comparison ===

    pub fn open_dirs(&mut self, left: PathBuf, right: PathBuf) {
        self.active_tab_mut().open_dirs(left, right);
    }

    pub fn dir_next(&mut self) {
        let tab = self.active_tab_mut();
        if let Some(ref mut r) = tab.dir_result {
            if r.selected + 1 < r.entries.len() {
                r.selected += 1;
            }
        }
    }

    pub fn dir_prev(&mut self) {
        let tab = self.active_tab_mut();
        if let Some(ref mut r) = tab.dir_result {
            if r.selected > 0 {
                r.selected -= 1;
            }
        }
    }

    pub fn dir_open_selected(&mut self) {
        let tab = self.active_tab();
        if !tab.is_dir_compare {
            return;
        }
        let dir_result = match &tab.dir_result {
            Some(r) => r,
            None => return,
        };
        let entry = match dir_result.entries.get(dir_result.selected) {
            Some(e) => e,
            None => return,
        };
        use crate::models::diff_line::DirEntryStatus;
        let left_dir = dir_result.left_dir.clone();
        let right_dir = dir_result.right_dir.clone();
        let rel = entry.rel_path.clone();
        let status = entry.status.clone();

        let left_path = left_dir.join(&rel);
        let right_path = right_dir.join(&rel);

        self.new_tab();
        match status {
            DirEntryStatus::LeftOnly => {
                self.active_tab_mut().open_files(left_path, PathBuf::new());
            }
            DirEntryStatus::RightOnly => {
                self.active_tab_mut().open_files(PathBuf::new(), right_path);
            }
            _ => {
                self.active_tab_mut().open_files(left_path, right_path);
            }
        }
    }

    pub fn enter_edit_mode(&mut self, panel: PanelSide, display_line: usize, col: usize) {
        self.active_tab_mut()
            .enter_edit_mode(panel, display_line, col);
        self.mode = AppMode::Editing;
    }

    pub fn exit_edit_mode(&mut self) {
        self.active_tab_mut().exit_edit_mode();
        self.mode = AppMode::Normal;
    }

    pub fn edit_insert_char(&mut self, ch: char) {
        self.active_tab_mut().edit_insert_char(ch);
    }

    pub fn edit_backspace(&mut self) {
        self.active_tab_mut().edit_backspace();
    }

    pub fn edit_delete(&mut self) {
        self.active_tab_mut().edit_delete();
    }

    pub fn edit_enter(&mut self) {
        self.active_tab_mut().edit_enter();
    }

    pub fn edit_move_left(&mut self) {
        self.active_tab_mut().edit_move_left();
    }

    pub fn edit_move_right(&mut self) {
        self.active_tab_mut().edit_move_right();
    }

    pub fn edit_move_up(&mut self) {
        self.active_tab_mut().edit_move_up();
    }

    pub fn edit_move_down(&mut self) {
        self.active_tab_mut().edit_move_down();
    }

    pub fn edit_move_home(&mut self) {
        self.active_tab_mut().edit_move_home();
    }

    pub fn edit_move_end(&mut self) {
        self.active_tab_mut().edit_move_end();
    }

    #[allow(dead_code)]
    pub fn edit_current_line_text(&self) -> Option<String> {
        self.active_tab().edit_current_line_text()
    }

    #[allow(dead_code)]
    pub fn source_lines(&self, panel: PanelSide) -> Vec<String> {
        self.active_tab().source_lines(panel)
    }

    #[allow(dead_code)]
    pub fn resolve_display_to_source(&self, display_idx: usize, panel: PanelSide) -> Option<usize> {
        self.active_tab()
            .resolve_display_to_source(display_idx, panel)
    }

    // === App-level methods ===

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
        let left_path = self.active_tab().left_path.clone();
        let (default_name, start_dir) = Self::save_defaults(&left_path, "untitled_left.txt");
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
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
            (fallback_name.to_string(), dir)
        }
    }
}

// ============================================================
// Module-level helpers
// ============================================================

/// Convert a string (e.g. from file I/O) to a per-pane line buffer.
pub fn str_to_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        vec![String::new()]
    } else {
        let lines: Vec<String> = text.lines().map(String::from).collect();
        if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        }
    }
}

/// Reconstruct a file-I/O string from a line buffer.
pub fn buf_to_text(buf: &[String]) -> String {
    if buf.is_empty() || (buf.len() == 1 && buf[0].is_empty()) {
        String::new()
    } else {
        let mut s = buf.join("\n");
        s.push('\n');
        s
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
        app.active_tab_mut().left_buf = str_to_lines(left);
        app.active_tab_mut().right_buf = str_to_lines(right);
        app.active_tab_mut().left_path = Some(PathBuf::from("/tmp/left.txt"));
        app.active_tab_mut().right_path = Some(PathBuf::from("/tmp/right.txt"));
        app.recompute_diff();
        app
    }

    /// Blank screen: click left panel -> edit -> click right panel -> edit
    #[test]
    fn test_blank_screen_edit_both_panels() {
        let mut app = App::new();
        app.new_blank(false);

        // Click left panel line 0
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        assert_eq!(
            app.active_tab().edit_state.as_ref().unwrap().panel,
            PanelSide::Left
        );

        // Type "hello"
        for c in "hello".chars() {
            app.edit_insert_char(c);
        }
        assert!(app.active_tab().left_text().contains("hello"));

        // Click right panel -- should exit left edit and enter right
        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        assert_eq!(
            app.active_tab().edit_state.as_ref().unwrap().panel,
            PanelSide::Right
        );

        // Type "world"
        for c in "world".chars() {
            app.edit_insert_char(c);
        }
        assert!(app.active_tab().right_text().contains("world"));

        // Both texts preserved
        assert!(app.active_tab().left_text().contains("hello"));
        assert!(app.active_tab().right_text().contains("world"));
    }

    /// Edit -> F5 compare -> text is preserved
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

        assert!(app.active_tab().left_text().contains("fn main() {}"));

        // Simulate F5: recompute diff
        app.recompute_diff();
        assert!(app.active_tab().diff_result.is_some());
        // Text must survive
        assert!(app.active_tab().left_text().contains("fn main() {}"));
    }

    /// Edit -> F5 -> copy -> edit again (the reported bug)
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
        assert!(app.active_tab().diff_result.is_some());
        let diff_count = app.diff_count();
        assert!(diff_count > 0, "should have diffs after editing left only");

        // Copy left to right
        app.copy_left_to_right();
        assert!(
            app.active_tab().right_text().contains("line one"),
            "copy should transfer text"
        );

        // After copy, edit must still work -- click left panel
        let resolve = app.resolve_display_to_source(0, PanelSide::Left);
        assert!(resolve.is_some(), "resolve must succeed after copy");

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(
            app.mode,
            AppMode::Editing,
            "must enter edit mode after copy"
        );
        app.edit_insert_char('X');
        assert!(
            app.active_tab().left_text().starts_with('X')
                || app.active_tab().left_text().contains('X')
        );
        app.exit_edit_mode();

        // Right panel too
        let resolve_r = app.resolve_display_to_source(0, PanelSide::Right);
        assert!(resolve_r.is_some(), "resolve right must succeed after copy");
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        app.edit_insert_char('Y');
        assert!(app.active_tab().right_text().contains('Y'));
        app.exit_edit_mode();
    }

    /// Repeated copy + edit cycles (stress test)
    #[test]
    fn test_repeated_copy_edit_stress() {
        let mut app = make_app_with_diff("alpha\nbeta\ngamma\n", "alpha\nBETA\ngamma\n");

        for iteration in 0..50 {
            let diff_count = app.diff_count();
            if diff_count == 0 {
                break;
            }

            app.copy_left_to_right();
            assert!(
                app.active_tab().has_unsaved_changes,
                "iter {}: must have unsaved changes after copy",
                iteration
            );

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

            app.recompute_diff();
        }

        assert!(
            !app.active_tab().left_text().is_empty(),
            "left text must survive stress"
        );
        assert!(
            !app.active_tab().right_text().is_empty(),
            "right text must survive stress"
        );
    }

    /// Ghost line editing
    #[test]
    fn test_ghost_line_editing() {
        let mut app = make_app_with_diff("line1\nline2\nline3\n", "line1\n");

        let result = app.resolve_display_to_source(1, PanelSide::Right);
        assert!(
            result.is_some(),
            "ghost line should resolve to nearest source line"
        );

        app.enter_edit_mode(PanelSide::Right, 1, 0);
        assert_eq!(app.mode, AppMode::Editing);
        app.exit_edit_mode();
    }

    /// Undo after edit preserves state
    #[test]
    fn test_undo_after_edit() {
        let mut app = make_app_with_diff("hello\n", "world\n");
        let original_left = app.active_tab().left_text();

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('X');
        app.exit_edit_mode();

        assert!(app.active_tab().left_text().contains('X'));

        app.undo();
        assert_eq!(
            app.active_tab().left_text(),
            original_left,
            "undo must restore original"
        );
    }

    /// F5 never reloads from disk when text was edited
    #[test]
    fn test_f5_never_destroys_edits() {
        let mut app = make_app_with_diff("original\n", "original\n");

        app.enter_edit_mode(PanelSide::Left, 0, 5);
        for c in "_modified".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();
        assert!(app.active_tab().left_text().contains("_modified"));

        app.recompute_diff();
        assert!(
            app.active_tab().left_text().contains("_modified"),
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
        app.edit_backspace();
        let lines = app.source_lines(PanelSide::Left);
        assert!(lines.len() >= 1, "must keep at least one line");
        app.exit_edit_mode();
    }

    // ================================================================
    // IRON RULES
    // ================================================================

    #[test]
    fn test_iron_rule_f5_never_destroys_display() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "left content".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        for c in "right content".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        let left_before = app.active_tab().left_text();
        let right_before = app.active_tab().right_text();

        app.recompute_diff();

        assert_eq!(
            app.active_tab().left_text(),
            left_before,
            "F5 destroyed left text"
        );
        assert_eq!(
            app.active_tab().right_text(),
            right_before,
            "F5 destroyed right text"
        );

        app.recompute_diff();
        assert_eq!(
            app.active_tab().left_text(),
            left_before,
            "second F5 destroyed left text"
        );
        assert_eq!(
            app.active_tab().right_text(),
            right_before,
            "second F5 destroyed right text"
        );
    }

    #[test]
    fn test_iron_rule_f5_after_copy_preserves() {
        let mut app = make_app_with_diff("hello\nworld\n", "hello\nWORLD\n");

        app.copy_left_to_right();
        let left_after_copy = app.active_tab().left_text();
        let right_after_copy = app.active_tab().right_text();

        app.recompute_diff();
        assert_eq!(
            app.active_tab().left_text(),
            left_after_copy,
            "F5 after copy destroyed left"
        );
        assert_eq!(
            app.active_tab().right_text(),
            right_after_copy,
            "F5 after copy destroyed right"
        );
    }

    #[test]
    fn test_iron_rule_f5_repeated_cycles() {
        let mut app = App::new();
        app.new_blank(false);

        for i in 0..10 {
            app.enter_edit_mode(PanelSide::Left, 0, 0);
            app.edit_insert_char(char::from(b'A' + (i % 26) as u8));
            app.exit_edit_mode();

            let left_snap = app.active_tab().left_text();
            let right_snap = app.active_tab().right_text();

            app.recompute_diff();
            assert_eq!(
                app.active_tab().left_text(),
                left_snap,
                "cycle {i}: F5 destroyed left"
            );
            assert_eq!(
                app.active_tab().right_text(),
                right_snap,
                "cycle {i}: F5 destroyed right"
            );

            if app.diff_count() > 0 {
                app.copy_left_to_right();
            }

            let left_snap2 = app.active_tab().left_text();
            let right_snap2 = app.active_tab().right_text();

            app.recompute_diff();
            assert_eq!(
                app.active_tab().left_text(),
                left_snap2,
                "cycle {i}: F5 after copy destroyed left"
            );
            assert_eq!(
                app.active_tab().right_text(),
                right_snap2,
                "cycle {i}: F5 after copy destroyed right"
            );
        }
    }

    #[test]
    fn test_iron_rule_input_always_reflected() {
        let mut app = make_app_with_diff("aaa\nbbb\n", "aaa\nBBB\n");

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('Z');
        assert!(
            app.active_tab().left_text().contains('Z'),
            "typed Z not in left_text"
        );
        app.exit_edit_mode();

        app.copy_left_to_right();

        let resolve = app.resolve_display_to_source(0, PanelSide::Right);
        assert!(resolve.is_some(), "resolve failed after copy");
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(
            app.mode,
            AppMode::Editing,
            "failed to enter edit after copy"
        );
        app.edit_insert_char('W');
        assert!(
            app.active_tab().right_text().contains('W'),
            "typed W not in right_text after copy"
        );
        app.exit_edit_mode();

        app.recompute_diff();
        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "failed to enter edit after F5");
        app.edit_insert_char('Q');
        assert!(
            app.active_tab().left_text().contains('Q'),
            "typed Q not in left_text after F5"
        );
        app.exit_edit_mode();
    }

    #[test]
    fn test_iron_rule_input_blank_both_panels() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "can't enter edit on blank left");
        app.edit_insert_char('L');
        assert!(
            app.active_tab().left_text().contains('L'),
            "left input lost on blank"
        );
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(
            app.mode,
            AppMode::Editing,
            "can't enter edit on ghost right"
        );
        app.edit_insert_char('R');
        assert!(
            app.active_tab().right_text().contains('R'),
            "right input lost on blank"
        );
        app.exit_edit_mode();

        assert!(
            app.active_tab().left_text().contains('L'),
            "left lost after right edit"
        );
        assert!(
            app.active_tab().right_text().contains('R'),
            "right lost after left exit"
        );
    }

    #[test]
    fn test_iron_rule_ghost_line_input_reflected() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "left only".chars() {
            app.edit_insert_char(c);
        }

        app.exit_edit_mode();
        assert!(
            app.active_tab().diff_result.is_some(),
            "diff should exist after exit"
        );
        let result = app.active_tab().diff_result.as_ref().unwrap();
        assert!(!result.lines.is_empty(), "diff should have lines");
        assert!(
            result.lines.iter().all(|l| l.right_line_no.is_none()),
            "right should be all ghost lines"
        );

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing, "must enter edit on ghost right");
        assert_eq!(
            app.active_tab().edit_state.as_ref().unwrap().panel,
            PanelSide::Right
        );

        app.edit_insert_char('R');
        assert!(
            app.active_tab().right_text().contains('R'),
            "ghost line input not in source"
        );

        let live = app.edit_current_line_text();
        assert!(live.is_some(), "edit_current_line_text None on ghost");
        assert!(live.unwrap().contains('R'), "live text missing typed char");

        app.exit_edit_mode();
        assert!(
            app.active_tab().right_text().contains('R'),
            "right text lost after exit"
        );
        assert!(
            app.active_tab().left_text().contains("left only"),
            "left text corrupted"
        );
    }

    #[test]
    fn test_iron_rule_click_beyond_diff_lines() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "hello".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        let diff_lines = app.active_tab().diff_result.as_ref().unwrap().lines.len();
        assert_eq!(diff_lines, 1, "should have exactly 1 diff line");

        app.enter_edit_mode(PanelSide::Right, 5, 0);
        assert_eq!(
            app.mode,
            AppMode::Editing,
            "must enter edit even clicking beyond"
        );

        let es = app.active_tab().edit_state.as_ref().unwrap();
        assert!(
            es.display_line < diff_lines,
            "display_line {} must be < diff lines {}",
            es.display_line,
            diff_lines
        );

        app.edit_insert_char('W');
        assert!(
            app.active_tab().right_text().contains('W'),
            "input lost on clamped ghost line"
        );
        app.exit_edit_mode();
    }

    #[test]
    fn test_iron_rule_source_line_clamped_to_bounds() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "line1".chars() {
            app.edit_insert_char(c);
        }
        app.edit_enter();
        for c in "line2".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        for c in "addfa".chars() {
            app.edit_insert_char(c);
        }

        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 1, 0);
        assert_eq!(app.mode, AppMode::Editing);

        let es = app.active_tab().edit_state.as_ref().unwrap();
        let right_line_count = app.source_lines(PanelSide::Right).len();
        assert!(
            es.source_line < right_line_count,
            "source_line {} >= right line count {} -- will cause live=None!",
            es.source_line,
            right_line_count
        );

        app.edit_insert_char('X');
        let live = app.edit_current_line_text();
        assert!(
            live.is_some(),
            "live text is None -- input invisible to user!"
        );
        assert!(live.unwrap().contains('X'), "typed X not in live text");
        app.exit_edit_mode();
    }

    #[test]
    fn test_iron_rule_render_ghost_edit_visible() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "aaa".chars() {
            app.edit_insert_char(c);
        }

        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Right, 0, 0);

        for c in "bbb".chars() {
            app.edit_insert_char(c);
        }

        let edit_info = app
            .active_tab()
            .edit_state
            .as_ref()
            .map(|e| (e.panel, e.source_line, e.display_line, e.cursor_col));
        let edit_live_text = app.edit_current_line_text();

        assert!(edit_info.is_some(), "edit_info is None");
        assert!(
            edit_live_text.is_some(),
            "edit_live_text is None -- input invisible!"
        );
        assert!(
            edit_live_text.as_ref().unwrap().contains("bbb"),
            "live text '{}' doesn't contain 'bbb'",
            edit_live_text.as_ref().unwrap()
        );

        let (panel, source_line, display_line, _cursor_col) = edit_info.unwrap();

        let result = app
            .active_tab()
            .diff_result
            .as_ref()
            .expect("diff_result is None");

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
                i == display_line
            };
            if is_edit_line {
                found_edit_line = true;
            }
        }

        assert!(
            found_edit_line,
            "RENDERING BUG: no display line matched edit state! \
             panel={:?} src={} disp={} diff_lines={}",
            panel,
            source_line,
            display_line,
            result.lines.len()
        );

        app.exit_edit_mode();
    }

    #[test]
    fn test_iron_rule_click_switch_panel_flow() {
        let mut app = App::new();
        app.new_blank(false);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "hello".chars() {
            app.edit_insert_char(c);
        }
        assert_eq!(app.mode, AppMode::Editing);

        app.exit_edit_mode();
        assert_eq!(app.mode, AppMode::Normal);
        assert!(app.active_tab().edit_state.is_none());

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(
            app.mode,
            AppMode::Editing,
            "FAILED to enter edit on right after switch"
        );
        let es = app.active_tab().edit_state.as_ref().unwrap();
        assert_eq!(es.panel, PanelSide::Right);

        for c in "world".chars() {
            app.edit_insert_char(c);
        }
        assert!(
            app.active_tab().right_text().contains("world"),
            "right input LOST"
        );

        let live = app.edit_current_line_text();
        assert!(live.is_some(), "no live text for renderer");
        assert!(live.unwrap().contains("world"), "live text missing");

        app.exit_edit_mode();

        assert!(
            app.active_tab().left_text().contains("hello"),
            "left text corrupted"
        );
        assert!(
            app.active_tab().right_text().contains("world"),
            "right text lost after exit"
        );
    }

    #[test]
    fn test_text_only_vanishes_by_user_action() {
        let mut app = make_app_with_diff("keep this\nchange this\n", "keep this\nDIFFERENT\n");

        app.copy_left_to_right();
        assert!(
            app.active_tab().right_text().contains("change this"),
            "copy should overwrite"
        );
        assert!(
            !app.active_tab().right_text().contains("DIFFERENT"),
            "copy should replace old right"
        );

        app.undo();
        assert!(
            app.active_tab().right_text().contains("DIFFERENT"),
            "undo must restore"
        );
    }

    // ================================================================
    // 3-WAY EDITING TESTS
    // ================================================================

    fn make_3way_app(left: &str, base: &str, right: &str) -> App {
        let mut app = App::new();
        app.active_tab_mut().left_buf = str_to_lines(left);
        app.active_tab_mut().base_buf = str_to_lines(base);
        app.active_tab_mut().right_buf = str_to_lines(right);
        app.active_tab_mut().is_three_way = true;
        app.active_tab_mut().left_path = Some(PathBuf::from("/tmp/left.txt"));
        app.active_tab_mut().base_path = Some(PathBuf::from("/tmp/base.txt"));
        app.active_tab_mut().right_path = Some(PathBuf::from("/tmp/right.txt"));
        app.recompute_diff();
        app
    }

    #[test]
    fn test_3way_blank_edit_all_panels() {
        let mut app = App::new();
        app.new_blank(true);

        for panel in [PanelSide::Left, PanelSide::Base, PanelSide::Right] {
            app.enter_edit_mode(panel, 0, 0);
            assert_eq!(
                app.mode,
                AppMode::Editing,
                "{:?} failed to enter edit",
                panel
            );
            app.edit_insert_char('X');

            let live = app.edit_current_line_text();
            assert!(live.is_some(), "{:?} live text None", panel);
            assert!(
                live.unwrap().contains('X'),
                "{:?} typed X not in live",
                panel
            );
            app.exit_edit_mode();
        }

        assert!(app.active_tab().left_text().contains('X'));
        assert!(app.active_tab().base_text().contains('X'));
        assert!(app.active_tab().right_text().contains('X'));
    }

    #[test]
    fn test_3way_edit_base_preserved_on_f5() {
        let mut app = make_3way_app("left\n", "base\n", "right\n");

        app.enter_edit_mode(PanelSide::Base, 0, 4);
        for c in "_edited".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        assert!(app.active_tab().base_text().contains("base_edited"));

        app.recompute_diff();
        assert!(
            app.active_tab().base_text().contains("base_edited"),
            "F5 destroyed base edit"
        );
    }

    #[test]
    fn test_3way_switch_panels_left_base_right() {
        let mut app = App::new();
        app.new_blank(true);

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        for c in "LEFT".chars() {
            app.edit_insert_char(c);
        }

        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Base, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        for c in "BASE".chars() {
            app.edit_insert_char(c);
        }

        app.exit_edit_mode();
        app.enter_edit_mode(PanelSide::Right, 0, 0);
        assert_eq!(app.mode, AppMode::Editing);
        for c in "RIGHT".chars() {
            app.edit_insert_char(c);
        }
        app.exit_edit_mode();

        assert!(app.active_tab().left_text().contains("LEFT"), "left lost");
        assert!(app.active_tab().base_text().contains("BASE"), "base lost");
        assert!(
            app.active_tab().right_text().contains("RIGHT"),
            "right lost"
        );
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
            assert!(
                live.unwrap().contains('Z'),
                "{:?} Z missing from live",
                panel
            );
            app.exit_edit_mode();
        }
    }

    #[test]
    fn test_3way_iron_rule_f5_never_destroys() {
        let mut app = make_3way_app("left\n", "base\n", "right\n");

        app.enter_edit_mode(PanelSide::Left, 0, 0);
        app.edit_insert_char('L');
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Base, 0, 0);
        app.edit_insert_char('B');
        app.exit_edit_mode();

        app.enter_edit_mode(PanelSide::Right, 0, 0);
        app.edit_insert_char('R');
        app.exit_edit_mode();

        let l = app.active_tab().left_text();
        let b = app.active_tab().base_text();
        let r = app.active_tab().right_text();

        app.recompute_diff();
        assert_eq!(app.active_tab().left_text(), l, "F5 destroyed left");
        assert_eq!(app.active_tab().base_text(), b, "F5 destroyed base");
        assert_eq!(app.active_tab().right_text(), r, "F5 destroyed right");
    }

    // ================================================================
    // TAB MANAGEMENT TESTS
    // ================================================================

    #[test]
    fn test_new_tab_independent_state() {
        let mut app = App::new();
        app.active_tab_mut().left_buf = str_to_lines("tab1");
        app.new_tab();
        assert!(app.active_tab().pane_is_empty(PanelSide::Left));
        app.switch_tab(0);
        assert_eq!(app.active_tab().left_buf, vec!["tab1".to_string()]);
    }

    #[test]
    fn test_close_tab() {
        let mut app = App::new();
        app.new_tab();
        app.new_tab();
        assert_eq!(app.tabs.len(), 3);
        app.switch_tab(1);
        assert!(app.close_tab());
        assert_eq!(app.tabs.len(), 2);
    }

    #[test]
    fn test_close_last_tab_fails() {
        let mut app = App::new();
        assert!(!app.close_tab());
        assert_eq!(app.tabs.len(), 1);
    }

    #[test]
    fn test_next_prev_tab() {
        let mut app = App::new();
        app.new_tab();
        app.new_tab();
        assert_eq!(app.active_tab, 2);
        app.next_tab();
        assert_eq!(app.active_tab, 0);
        app.prev_tab();
        assert_eq!(app.active_tab, 2);
        app.prev_tab();
        assert_eq!(app.active_tab, 1);
    }

    #[test]
    fn test_any_unsaved() {
        let mut app = App::new();
        assert!(!app.any_unsaved());
        app.active_tab_mut().has_unsaved_changes = true;
        assert!(app.any_unsaved());
        app.new_tab();
        assert!(app.any_unsaved());
    }

    #[test]
    fn test_tab_title() {
        let mut app = App::new();
        assert_eq!(app.active_tab().title(), "New");
        app.active_tab_mut().left_path = Some(PathBuf::from("/home/user/test.txt"));
        assert_eq!(app.active_tab().title(), "test.txt");
    }

    #[test]
    fn test_close_tab_adjusts_active() {
        let mut app = App::new();
        app.new_tab();
        app.new_tab();
        // 3 tabs, active=2
        app.switch_tab(2);
        assert!(app.close_tab());
        // After closing tab 2, active should be 1
        assert_eq!(app.active_tab, 1);
        assert_eq!(app.tabs.len(), 2);
    }
}
