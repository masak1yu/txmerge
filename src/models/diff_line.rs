#[derive(Debug, Clone, PartialEq)]
pub enum LineStatus {
    Equal,
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct WordDiffSegment {
    pub text: String,
    pub changed: bool,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub left_line_no: Option<u32>,
    pub right_line_no: Option<u32>,
    pub status: LineStatus,
    pub left_word_segments: Vec<WordDiffSegment>,
    pub right_word_segments: Vec<WordDiffSegment>,
}

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub diff_count: u32,
    pub diff_positions: Vec<usize>,
}

// 3-way merge types

#[derive(Debug, Clone, PartialEq)]
pub enum ThreeWayStatus {
    Equal,
    LeftChanged,
    RightChanged,
    BothChanged,
    Conflict,
}

#[derive(Debug, Clone)]
pub struct ThreeWayLine {
    pub status: ThreeWayStatus,
    pub base_line_no: Option<u32>,
    pub left_line_no: Option<u32>,
    pub right_line_no: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ThreeWayResult {
    pub lines: Vec<ThreeWayLine>,
    pub conflict_count: u32,
    pub diff_positions: Vec<usize>,
}

// Directory comparison types

#[derive(Debug, Clone, PartialEq)]
pub enum DirEntryStatus {
    LeftOnly,
    RightOnly,
    Equal,
    Changed,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub rel_path: std::path::PathBuf,
    pub status: DirEntryStatus,
    pub left_modified: Option<std::time::SystemTime>,
    pub right_modified: Option<std::time::SystemTime>,
    pub left_size: Option<u64>,
    pub right_size: Option<u64>,
}

/// Git context attached to a DirCompareResult when in --git mode.
#[derive(Debug, Clone)]
pub struct GitContext {
    pub repo: std::path::PathBuf,
    pub ref1: String,
    pub ref2: Option<String>, // None = working tree
}

#[derive(Debug, Clone)]
pub struct DirCompareResult {
    pub left_dir: std::path::PathBuf,
    pub right_dir: std::path::PathBuf,
    pub entries: Vec<DirEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub git_context: Option<GitContext>,
}
