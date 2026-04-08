use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::models::diff_line::LineStatus;

const LINE_NO_WIDTH: usize = 5;

// Colors - fresh editor inspired dark theme
const BG_EQUAL: Color = Color::Reset;
const BG_ADDED: Color = Color::Rgb(20, 50, 20);
const BG_REMOVED: Color = Color::Rgb(50, 20, 20);
const BG_MODIFIED: Color = Color::Rgb(50, 45, 15);
const BG_WORD_CHANGED: Color = Color::Rgb(80, 70, 20);
const FG_LINE_NO: Color = Color::Rgb(100, 100, 100);
const BG_CURRENT_BLOCK: Color = Color::Rgb(35, 35, 55);
const BG_GHOST: Color = Color::Rgb(30, 30, 30);

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let unsaved = if app.has_unsaved_changes { "[*] " } else { "" };
    let left_title = app
        .left_path
        .as_ref()
        .map(|p| format!("{}{}", unsaved, p.display()))
        .unwrap_or_else(|| "(no file)".to_string());
    let right_title = app
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

    if app.diff_result.is_none() {
        let hint = Paragraph::new("Press 'o' to open files, or pass files as arguments")
            .style(Style::default().fg(Color::Rgb(100, 100, 100)));
        f.render_widget(hint, left_inner);
        return;
    }

    let result = app.diff_result.as_ref().unwrap();
    let visible_height = left_inner.height as usize;
    let start = app.scroll_offset;
    let end = (start + visible_height).min(result.lines.len());

    // Determine which diff block each line belongs to
    let current_block_start = if app.current_diff >= 0 {
        result
            .diff_positions
            .get(app.current_diff as usize)
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

    for i in start..end {
        let line = &result.lines[i];
        let is_current = match (current_block_start, current_block_end) {
            (Some(s), Some(e)) => i >= s && i < e,
            _ => false,
        };

        let (left_span_line, right_span_line) = render_diff_line(line, is_current);
        left_lines.push(left_span_line);
        right_lines.push(right_span_line);
    }

    let left_para = Paragraph::new(left_lines);
    let right_para = Paragraph::new(right_lines);
    f.render_widget(left_para, left_inner);
    f.render_widget(right_para, right_inner);
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
        LineStatus::Moved => (BG_MODIFIED, BG_MODIFIED),
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

fn blend_current(base: Color) -> Color {
    match base {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(15),
            g.saturating_add(15),
            b.saturating_add(25),
        ),
        _ => BG_CURRENT_BLOCK,
    }
}
