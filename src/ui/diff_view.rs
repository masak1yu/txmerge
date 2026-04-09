use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, AppMode, PanelSide};
use crate::models::diff_line::LineStatus;

const LINE_NO_WIDTH: usize = 5;

// Panel rect storage for mouse hit testing
static mut LEFT_PANEL_RECT: Option<Rect> = None;
static mut RIGHT_PANEL_RECT: Option<Rect> = None;

pub fn left_panel_rect() -> Option<Rect> {
    unsafe { LEFT_PANEL_RECT }
}

pub fn right_panel_rect() -> Option<Rect> {
    unsafe { RIGHT_PANEL_RECT }
}

pub fn reset_panel_rects() {
    unsafe {
        LEFT_PANEL_RECT = None;
        RIGHT_PANEL_RECT = None;
    }
}

// Colors - fresh editor inspired dark theme
const BG_EQUAL: Color = Color::Reset;
const BG_ADDED: Color = Color::Rgb(20, 50, 20);
const BG_REMOVED: Color = Color::Rgb(50, 20, 20);
const BG_MODIFIED: Color = Color::Rgb(50, 45, 15);
const BG_WORD_CHANGED: Color = Color::Rgb(80, 70, 20);
const FG_LINE_NO: Color = Color::Rgb(100, 100, 100);
const BG_CURRENT_BLOCK: Color = Color::Rgb(80, 50, 10);
const BG_GHOST: Color = Color::Rgb(30, 30, 30);

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let unsaved = if tab.has_unsaved_changes { "[*] " } else { "" };
    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| format!("{}{}", unsaved, p.display()))
        .unwrap_or_else(|| "(no file)".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| format!("{}{}", unsaved, p.display()))
        .unwrap_or_else(|| "(no file)".to_string());

    let left_block = Block::default()
        .title(format!(" {} ", left_title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 70)));
    let right_block = Block::default()
        .title(format!(" {} ", right_title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 70)));

    let left_inner = left_block.inner(chunks[0]);
    let right_inner = right_block.inner(chunks[1]);
    f.render_widget(left_block, chunks[0]);
    f.render_widget(right_block, chunks[1]);

    // Store panel rects for mouse hit testing
    unsafe {
        LEFT_PANEL_RECT = Some(left_inner);
        RIGHT_PANEL_RECT = Some(right_inner);
    }

    // When editing, always use raw text rendering — guarantees input is always visible.
    // Diff coloring is only shown when not editing (same as WinMerge pattern).
    let use_raw_text = app.mode == AppMode::Editing
        || !tab
            .diff_result
            .as_ref()
            .map(|r| !r.lines.is_empty())
            .unwrap_or(false);
    if use_raw_text {
        // No diff result — render raw source text with line numbers (same layout as diff view)
        let left_src = tab.source_lines(PanelSide::Left);
        let right_src = tab.source_lines(PanelSide::Right);
        let max_lines = left_src.len().max(right_src.len()).max(1);
        let visible_height = left_inner.height as usize;
        let start = tab.scroll_offset;
        let end = (start + visible_height).min(max_lines);

        let edit_info = if app.mode == AppMode::Editing {
            tab.edit_state
                .as_ref()
                .map(|e| (e.panel, e.source_line, e.cursor_col))
        } else {
            None
        };
        let edit_live_text = if app.mode == AppMode::Editing {
            tab.edit_current_line_text()
        } else {
            None
        };

        let mut left_lines = Vec::new();
        let mut right_lines = Vec::new();
        for i in start..end {
            let left_text = if let Some((PanelSide::Left, sl, _)) = edit_info {
                if i == sl {
                    edit_live_text.clone().unwrap_or_default()
                } else {
                    left_src.get(i).cloned().unwrap_or_default()
                }
            } else {
                left_src.get(i).cloned().unwrap_or_default()
            };
            let right_text = if let Some((PanelSide::Right, sl, _)) = edit_info {
                if i == sl {
                    edit_live_text.clone().unwrap_or_default()
                } else {
                    right_src.get(i).cloned().unwrap_or_default()
                }
            } else {
                right_src.get(i).cloned().unwrap_or_default()
            };

            let left_no = if i < left_src.len() {
                format!("{:>4} ", i + 1)
            } else {
                "     ".to_string()
            };
            let right_no = if i < right_src.len() {
                format!("{:>4} ", i + 1)
            } else {
                "     ".to_string()
            };
            left_lines.push(Line::from(vec![
                Span::styled(
                    left_no,
                    Style::default().fg(Color::Rgb(100, 100, 120)).bg(BG_EQUAL),
                ),
                Span::styled(left_text, Style::default().fg(Color::White).bg(BG_EQUAL)),
            ]));
            right_lines.push(Line::from(vec![
                Span::styled(
                    right_no,
                    Style::default().fg(Color::Rgb(100, 100, 120)).bg(BG_EQUAL),
                ),
                Span::styled(right_text, Style::default().fg(Color::White).bg(BG_EQUAL)),
            ]));
        }

        // Show hint on first line if both panels are empty
        if tab.left_text.is_empty() && tab.right_text.is_empty() && app.mode != AppMode::Editing {
            left_lines.clear();
            left_lines.push(Line::from(vec![
                Span::styled(
                    "   1 ",
                    Style::default().fg(Color::Rgb(100, 100, 120)).bg(BG_EQUAL),
                ),
                Span::styled(
                    "Press 'o' to open files, or click to edit",
                    Style::default().fg(Color::Rgb(100, 100, 100)).bg(BG_EQUAL),
                ),
            ]));
        }

        f.render_widget(Paragraph::new(left_lines), left_inner);
        f.render_widget(Paragraph::new(right_lines), right_inner);

        // Show cursor for editing
        if let Some((panel, source_line, cursor_col)) = edit_info {
            let panel_rect = match panel {
                PanelSide::Left => left_inner,
                PanelSide::Right => right_inner,
                _ => left_inner,
            };
            let row_on_screen = source_line.saturating_sub(tab.scroll_offset);
            if (row_on_screen as u16) < panel_rect.height {
                let display_col = if let Some(ref live) = edit_live_text {
                    use unicode_width::UnicodeWidthStr;
                    let prefix: String = live.chars().take(cursor_col).collect();
                    prefix.width()
                } else {
                    cursor_col
                };
                let x = panel_rect.x + LINE_NO_WIDTH as u16 + display_col as u16;
                let y = panel_rect.y + row_on_screen as u16;
                f.set_cursor_position((x.min(panel_rect.x + panel_rect.width - 1), y));
            }
        }
        return;
    }

    let result = tab.diff_result.as_ref().unwrap();
    let visible_height = left_inner.height as usize;
    let start = tab.scroll_offset;
    let end = (start + visible_height).min(result.lines.len());

    // Determine which diff block each line belongs to
    let current_block_start = if tab.current_diff >= 0 {
        result
            .diff_positions
            .get(tab.current_diff as usize)
            .copied()
    } else {
        None
    };
    let current_block_end = current_block_start.map(|s| {
        let mut e = s;
        while e < result.lines.len() && result.lines[e].status != LineStatus::Equal {
            e += 1;
        }
        e
    });

    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    // Get edit state info if editing
    let edit_info = if app.mode == AppMode::Editing {
        tab.edit_state
            .as_ref()
            .map(|e| (e.panel, e.source_line, e.display_line, e.cursor_col))
    } else {
        None
    };
    let edit_live_text = if app.mode == AppMode::Editing {
        tab.edit_current_line_text()
    } else {
        None
    };

    for i in start..end {
        let line = &result.lines[i];
        let is_current = match (current_block_start, current_block_end) {
            (Some(s), Some(e)) => i >= s && i < e,
            _ => false,
        };

        // Check if this display line matches the editing line.
        let is_edit_line = if let Some((panel, source_line, display_line, _)) = edit_info {
            let line_no = match panel {
                PanelSide::Left => line.left_line_no,
                PanelSide::Right => line.right_line_no,
                _ => None,
            };
            if let Some(n) = line_no {
                n as usize - 1 == source_line
            } else {
                i == display_line
            }
        } else {
            false
        };

        let (left_span_line, right_span_line) = if is_edit_line {
            if let (Some((panel, _, _, _)), Some(live)) = (edit_info, &edit_live_text) {
                render_diff_line_with_live(line, is_current, panel, live)
            } else {
                render_diff_line(line, is_current)
            }
        } else {
            render_diff_line(line, is_current)
        };

        left_lines.push(left_span_line);
        right_lines.push(right_span_line);
    }

    let left_para = Paragraph::new(left_lines);
    let right_para = Paragraph::new(right_lines);
    f.render_widget(left_para, left_inner);
    f.render_widget(right_para, right_inner);

    // Render cursor if editing
    if let Some((panel, source_line, display_line, cursor_col)) = edit_info {
        let edit_display_idx = result
            .lines
            .iter()
            .position(|dl| {
                let ln = match panel {
                    PanelSide::Left => dl.left_line_no,
                    PanelSide::Right => dl.right_line_no,
                    _ => None,
                };
                ln.map(|n| n as usize - 1) == Some(source_line)
            })
            .unwrap_or(display_line);
        let row_on_screen = edit_display_idx.saturating_sub(tab.scroll_offset);
        let panel_rect = match panel {
            PanelSide::Left => left_inner,
            PanelSide::Right => right_inner,
            _ => left_inner,
        };
        if (row_on_screen as u16) < panel_rect.height {
            let display_col = if let Some(ref live) = edit_live_text {
                use unicode_width::UnicodeWidthStr;
                let prefix: String = live.chars().take(cursor_col).collect();
                prefix.width()
            } else {
                cursor_col
            };
            let x = panel_rect.x + LINE_NO_WIDTH as u16 + display_col as u16;
            let y = panel_rect.y + row_on_screen as u16;
            f.set_cursor_position((x.min(panel_rect.x + panel_rect.width - 1), y));
        }
    }
}

fn render_diff_line(
    line: &crate::models::diff_line::DiffLine,
    is_current: bool,
) -> (Line<'static>, Line<'static>) {
    let (left_bg, right_bg) = match line.status {
        LineStatus::Equal => (BG_EQUAL, BG_EQUAL),
        LineStatus::Added => (BG_GHOST, BG_ADDED),
        LineStatus::Removed => (BG_REMOVED, BG_GHOST),
        LineStatus::Modified => (BG_MODIFIED, BG_MODIFIED),
    };

    // Overlay current block highlight
    let left_bg = if is_current && line.status != LineStatus::Equal {
        blend_current(left_bg)
    } else {
        left_bg
    };
    let right_bg = if is_current && line.status != LineStatus::Equal {
        blend_current(right_bg)
    } else {
        right_bg
    };

    // Left line number
    let left_no = match line.left_line_no {
        Some(n) => format!("{:>width$} ", n, width = LINE_NO_WIDTH - 1),
        None => " ".repeat(LINE_NO_WIDTH),
    };

    // Right line number
    let right_no = match line.right_line_no {
        Some(n) => format!("{:>width$} ", n, width = LINE_NO_WIDTH - 1),
        None => " ".repeat(LINE_NO_WIDTH),
    };

    let left_line = if line.status == LineStatus::Modified && !line.left_word_segments.is_empty() {
        let mut spans = vec![Span::styled(
            left_no,
            Style::default().fg(FG_LINE_NO).bg(left_bg),
        )];
        for seg in &line.left_word_segments {
            let bg = if seg.changed {
                BG_WORD_CHANGED
            } else {
                left_bg
            };
            let style = Style::default().fg(Color::White).bg(bg);
            let style = if seg.changed {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };
            spans.push(Span::styled(seg.text.clone(), style));
        }
        Line::from(spans)
    } else {
        Line::from(vec![
            Span::styled(left_no, Style::default().fg(FG_LINE_NO).bg(left_bg)),
            Span::styled(
                line.left_text.clone(),
                Style::default().fg(Color::White).bg(left_bg),
            ),
        ])
    };

    let right_line = if line.status == LineStatus::Modified && !line.right_word_segments.is_empty()
    {
        let mut spans = vec![Span::styled(
            right_no,
            Style::default().fg(FG_LINE_NO).bg(right_bg),
        )];
        for seg in &line.right_word_segments {
            let bg = if seg.changed {
                BG_WORD_CHANGED
            } else {
                right_bg
            };
            let style = Style::default().fg(Color::White).bg(bg);
            let style = if seg.changed {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };
            spans.push(Span::styled(seg.text.clone(), style));
        }
        Line::from(spans)
    } else {
        Line::from(vec![
            Span::styled(right_no, Style::default().fg(FG_LINE_NO).bg(right_bg)),
            Span::styled(
                line.right_text.clone(),
                Style::default().fg(Color::White).bg(right_bg),
            ),
        ])
    };

    (left_line, right_line)
}

/// Render a diff line with live edited text replacing the stale DiffLine text
fn render_diff_line_with_live(
    line: &crate::models::diff_line::DiffLine,
    is_current: bool,
    edit_panel: PanelSide,
    live_text: &str,
) -> (Line<'static>, Line<'static>) {
    let (left_bg, right_bg) = match line.status {
        LineStatus::Equal => (BG_EQUAL, BG_EQUAL),
        LineStatus::Added => (BG_GHOST, BG_ADDED),
        LineStatus::Removed => (BG_REMOVED, BG_GHOST),
        LineStatus::Modified => (BG_MODIFIED, BG_MODIFIED),
    };

    let left_bg = if is_current && line.status != LineStatus::Equal {
        blend_current(left_bg)
    } else {
        left_bg
    };
    let right_bg = if is_current && line.status != LineStatus::Equal {
        blend_current(right_bg)
    } else {
        right_bg
    };

    let left_no = match line.left_line_no {
        Some(n) => format!("{:>width$} ", n, width = LINE_NO_WIDTH - 1),
        None => " ".repeat(LINE_NO_WIDTH),
    };
    let right_no = match line.right_line_no {
        Some(n) => format!("{:>width$} ", n, width = LINE_NO_WIDTH - 1),
        None => " ".repeat(LINE_NO_WIDTH),
    };

    let edit_bg = Color::Rgb(30, 40, 60); // Slightly highlighted bg for edited line

    let left_line = if edit_panel == PanelSide::Left {
        Line::from(vec![
            Span::styled(left_no, Style::default().fg(FG_LINE_NO).bg(edit_bg)),
            Span::styled(
                live_text.to_string(),
                Style::default().fg(Color::White).bg(edit_bg),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(left_no, Style::default().fg(FG_LINE_NO).bg(left_bg)),
            Span::styled(
                line.left_text.clone(),
                Style::default().fg(Color::White).bg(left_bg),
            ),
        ])
    };

    let right_line = if edit_panel == PanelSide::Right {
        Line::from(vec![
            Span::styled(right_no, Style::default().fg(FG_LINE_NO).bg(edit_bg)),
            Span::styled(
                live_text.to_string(),
                Style::default().fg(Color::White).bg(edit_bg),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(right_no, Style::default().fg(FG_LINE_NO).bg(right_bg)),
            Span::styled(
                line.right_text.clone(),
                Style::default().fg(Color::White).bg(right_bg),
            ),
        ])
    };

    (left_line, right_line)
}

fn blend_current(base: Color) -> Color {
    match base {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(50),
            g.saturating_add(25),
            b.saturating_add(0),
        ),
        _ => BG_CURRENT_BLOCK,
    }
}
