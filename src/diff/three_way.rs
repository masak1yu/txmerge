use similar::TextDiff;

use crate::models::diff_line::{ThreeWayLine, ThreeWayResult, ThreeWayStatus};

#[derive(Debug, Clone)]
struct Hunk {
    old_start: usize,
    old_end: usize,
    new_start: usize,
    new_end: usize,
}

#[derive(Debug)]
struct DiffBlock {
    left_start: usize,
    left_end: usize,
    base_start: usize,
    base_end: usize,
    right_start: usize,
    right_end: usize,
    status: ThreeWayStatus,
}

fn extract_hunks(diff: &TextDiff<'_, '_, '_, str>) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    for op in diff.ops() {
        let old = op.old_range();
        let new = op.new_range();
        match op.tag() {
            similar::DiffTag::Equal => {}
            _ => {
                hunks.push(Hunk {
                    old_start: old.start,
                    old_end: old.end,
                    new_start: new.start,
                    new_end: new.end,
                });
            }
        }
    }
    hunks
}

pub fn compute_three_way_diff(
    base_lines: &[String],
    left_lines: &[String],
    right_lines: &[String],
) -> ThreeWayResult {
    let base_refs: Vec<&str> = base_lines.iter().map(|s| s.as_str()).collect();
    let left_refs: Vec<&str> = left_lines.iter().map(|s| s.as_str()).collect();
    let right_refs: Vec<&str> = right_lines.iter().map(|s| s.as_str()).collect();

    let left_diff = TextDiff::from_slices(&base_refs, &left_refs);
    let right_diff = TextDiff::from_slices(&base_refs, &right_refs);

    let left_hunks = extract_hunks(&left_diff);
    let right_hunks = extract_hunks(&right_diff);

    let blocks = merge_hunks(&left_hunks, &right_hunks, left_lines, right_lines);
    build_result_lines(&blocks, base_lines, left_lines, right_lines)
}

fn merge_hunks(
    left_hunks: &[Hunk],
    right_hunks: &[Hunk],
    left_lines: &[String],
    right_lines: &[String],
) -> Vec<DiffBlock> {
    let mut blocks = Vec::new();
    let mut li = 0usize;
    let mut ri = 0usize;
    let mut base_pos = 0usize;
    let mut left_pos = 0usize;
    let mut right_pos = 0usize;

    loop {
        let lh = left_hunks.get(li);
        let rh = right_hunks.get(ri);

        if lh.is_none() && rh.is_none() {
            break;
        }

        let l_base_start = lh.map(|h| h.old_start).unwrap_or(usize::MAX);
        let r_base_start = rh.map(|h| h.old_start).unwrap_or(usize::MAX);

        if l_base_start == usize::MAX && r_base_start == usize::MAX {
            break;
        }

        let first_base_start = l_base_start.min(r_base_start);
        let skip = first_base_start - base_pos;
        base_pos = first_base_start;
        left_pos += skip;
        right_pos += skip;

        let mut group_base_end = base_pos;
        let mut group_has_left = false;
        let mut group_has_right = false;
        let group_li_start = li;
        let group_ri_start = ri;

        // Seed with first hunk
        if l_base_start <= r_base_start {
            if let Some(h) = lh {
                group_base_end = h.old_end;
                group_has_left = true;
                li += 1;
            }
        } else if let Some(h) = rh {
            group_base_end = h.old_end;
            group_has_right = true;
            ri += 1;
        }

        // Expand with overlapping hunks
        loop {
            let mut expanded = false;
            if let Some(h) = left_hunks.get(li) {
                if h.old_start <= group_base_end {
                    group_has_left = true;
                    group_base_end = group_base_end.max(h.old_end);
                    li += 1;
                    expanded = true;
                }
            }
            if let Some(h) = right_hunks.get(ri) {
                if h.old_start <= group_base_end {
                    group_has_right = true;
                    group_base_end = group_base_end.max(h.old_end);
                    ri += 1;
                    expanded = true;
                }
            }
            if !expanded {
                break;
            }
        }

        let base_count = group_base_end - base_pos;

        let (group_left_start, group_left_end) = if !group_has_left {
            (left_pos, left_pos + base_count)
        } else {
            let mut net: isize = 0;
            for h in &left_hunks[group_li_start..li] {
                net += (h.new_end - h.new_start) as isize - (h.old_end - h.old_start) as isize;
            }
            let end = ((left_pos + base_count) as isize + net).max(0) as usize;
            (left_pos, end.min(left_lines.len()))
        };

        let (group_right_start, group_right_end) = if !group_has_right {
            (right_pos, right_pos + base_count)
        } else {
            let mut net: isize = 0;
            for h in &right_hunks[group_ri_start..ri] {
                net += (h.new_end - h.new_start) as isize - (h.old_end - h.old_start) as isize;
            }
            let end = ((right_pos + base_count) as isize + net).max(0) as usize;
            (right_pos, end.min(right_lines.len()))
        };

        let status = if group_has_left && group_has_right {
            let left_new = &left_lines[group_left_start..group_left_end];
            let right_new = &right_lines[group_right_start..group_right_end];
            if left_new == right_new {
                ThreeWayStatus::BothChanged
            } else {
                ThreeWayStatus::Conflict
            }
        } else if group_has_left {
            ThreeWayStatus::LeftChanged
        } else {
            ThreeWayStatus::RightChanged
        };

        blocks.push(DiffBlock {
            left_start: group_left_start,
            left_end: group_left_end,
            base_start: base_pos,
            base_end: group_base_end,
            right_start: group_right_start,
            right_end: group_right_end,
            status,
        });

        base_pos = group_base_end;
        left_pos = group_left_end;
        right_pos = group_right_end;
    }

    blocks
}

fn build_result_lines(
    blocks: &[DiffBlock],
    base_lines: &[String],
    left_lines: &[String],
    right_lines: &[String],
) -> ThreeWayResult {
    let mut result_lines = Vec::new();
    let mut diff_positions = Vec::new();
    let mut conflict_count = 0u32;

    let mut base_pos = 0usize;
    let mut left_pos = 0usize;
    let mut right_pos = 0usize;

    for block in blocks {
        // Equal lines before this block
        while base_pos < block.base_start {
            result_lines.push(ThreeWayLine {
                status: ThreeWayStatus::Equal,
                left_line_no: Some(left_pos as u32 + 1),
                base_line_no: Some(base_pos as u32 + 1),
                right_line_no: Some(right_pos as u32 + 1),
            });
            base_pos += 1;
            left_pos += 1;
            right_pos += 1;
        }

        // Diff block with ghost-line alignment
        let left_count = block.left_end - block.left_start;
        let base_count = block.base_end - block.base_start;
        let right_count = block.right_end - block.right_start;
        let max_lines = left_count.max(base_count).max(right_count);

        diff_positions.push(result_lines.len());
        if block.status == ThreeWayStatus::Conflict {
            conflict_count += 1;
        }

        for j in 0..max_lines {
            let l_no = if j < left_count {
                Some((block.left_start + j) as u32 + 1)
            } else {
                None
            };
            let b_no = if j < base_count {
                Some((block.base_start + j) as u32 + 1)
            } else {
                None
            };
            let r_no = if j < right_count {
                Some((block.right_start + j) as u32 + 1)
            } else {
                None
            };

            result_lines.push(ThreeWayLine {
                status: block.status.clone(),
                left_line_no: l_no,
                base_line_no: b_no,
                right_line_no: r_no,
            });
        }

        base_pos = block.base_end;
        left_pos = block.left_end;
        right_pos = block.right_end;
    }

    // Trailing equal lines
    while base_pos < base_lines.len() {
        result_lines.push(ThreeWayLine {
            status: ThreeWayStatus::Equal,
            left_line_no: Some(left_pos as u32 + 1),
            base_line_no: Some(base_pos as u32 + 1),
            right_line_no: Some(right_pos as u32 + 1),
        });
        base_pos += 1;
        left_pos += 1;
        right_pos += 1;
    }

    // Handle case where left/right have more lines than base (all added)
    while left_pos < left_lines.len() || right_pos < right_lines.len() {
        let l_no = if left_pos < left_lines.len() {
            left_pos += 1;
            Some(left_pos as u32)
        } else {
            None
        };
        let r_no = if right_pos < right_lines.len() {
            right_pos += 1;
            Some(right_pos as u32)
        } else {
            None
        };
        result_lines.push(ThreeWayLine {
            status: ThreeWayStatus::Equal,
            left_line_no: l_no,
            base_line_no: None,
            right_line_no: r_no,
        });
    }

    ThreeWayResult {
        lines: result_lines,
        conflict_count,
        diff_positions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(String::from).collect()
    }

    #[test]
    fn test_all_equal() {
        let text = lines("a\nb\nc\n");
        let result = compute_three_way_diff(&text, &text, &text);
        assert_eq!(result.conflict_count, 0);
        assert_eq!(result.diff_positions.len(), 0);
        assert!(
            result
                .lines
                .iter()
                .all(|l| l.status == ThreeWayStatus::Equal)
        );
    }

    #[test]
    fn test_left_only_change() {
        let base = lines("a\nb\nc\n");
        let left = lines("a\nX\nc\n");
        let right = lines("a\nb\nc\n");
        let result = compute_three_way_diff(&base, &left, &right);
        assert_eq!(result.conflict_count, 0);
        let changed: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.status == ThreeWayStatus::LeftChanged)
            .collect();
        assert!(!changed.is_empty());
    }

    #[test]
    fn test_right_only_change() {
        let base = lines("a\nb\nc\n");
        let left = lines("a\nb\nc\n");
        let right = lines("a\nY\nc\n");
        let result = compute_three_way_diff(&base, &left, &right);
        assert_eq!(result.conflict_count, 0);
        let changed: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.status == ThreeWayStatus::RightChanged)
            .collect();
        assert!(!changed.is_empty());
    }

    #[test]
    fn test_both_changed_same() {
        let base = lines("a\nb\nc\n");
        let left = lines("a\nX\nc\n");
        let right = lines("a\nX\nc\n");
        let result = compute_three_way_diff(&base, &left, &right);
        assert_eq!(result.conflict_count, 0);
        let changed: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.status == ThreeWayStatus::BothChanged)
            .collect();
        assert!(!changed.is_empty());
    }

    #[test]
    fn test_conflict() {
        let base = lines("a\nb\nc\n");
        let left = lines("a\nX\nc\n");
        let right = lines("a\nY\nc\n");
        let result = compute_three_way_diff(&base, &left, &right);
        assert_eq!(result.conflict_count, 1);
        let conflicts: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.status == ThreeWayStatus::Conflict)
            .collect();
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn test_left_added_line() {
        let base = lines("a\nc\n");
        let left = lines("a\nb\nc\n");
        let right = lines("a\nc\n");
        let result = compute_three_way_diff(&base, &left, &right);
        assert_eq!(result.conflict_count, 0);
        assert!(
            result
                .lines
                .iter()
                .any(|l| l.status == ThreeWayStatus::LeftChanged)
        );
    }

    #[test]
    fn test_right_removed_line() {
        let base = lines("a\nb\nc\n");
        let left = lines("a\nb\nc\n");
        let right = lines("a\nc\n");
        let result = compute_three_way_diff(&base, &left, &right);
        assert_eq!(result.conflict_count, 0);
        assert!(
            result
                .lines
                .iter()
                .any(|l| l.status == ThreeWayStatus::RightChanged)
        );
    }
}
