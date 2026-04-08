#[derive(Debug, Clone, PartialEq)]
pub enum LineStatus {
    Equal,
    Added,
    Removed,
    Modified,
    Moved,
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
    pub left_text: String,
    pub right_text: String,
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
    pub base_text: String,
    pub left_text: String,
    pub right_text: String,
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
