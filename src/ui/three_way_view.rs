use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::models::diff_line::ThreeWayStatus;

const LINE_NO_WIDTH: usize = 5;

const BG_EQUAL: Color = Color::Reset;
const BG_LEFT_CHANGED: Color = Color::Rgb(20, 35, 55);
const BG_RIGHT_CHANGED: Color = Color::Rgb(20, 35, 55);
const BG_BOTH_CHANGED: Color = Color::Rgb(20, 50, 30);
const BG_CONFLICT: Color = Color::Rgb(55, 20, 20);
const BG_GHOST: Color = Color::Rgb(30, 30, 30);
const FG_LINE_NO: Color = Color::Rgb(100, 100, 100);

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let left_title = app
        .left_path
        .as_ref()
        .map(|p| {
            format!(
                " {}{} ",
                if app.has_unsaved_changes { "[*] " } else { "" },
                p.display()
            )
        })
        .unwrap_or_else(|| " (no file) ".to_string());
    let base_title = app
        .base_path
        .as_ref()
        .map(|p| format!(" {} ", p.display()))
        .unwrap_or_else(|| " (base) ".to_string());
    let right_title = app
        .right_path
        .as_ref()
        .map(|p| {
            format!(
                " {}{} ",
                if app.has_unsaved_changes { "[*] " } else { "" },
                p.display()
            )
        })
        .unwrap_or_else(|| " (no file) ".to_string());

    let border_style = Style::default().fg(Color::Rgb(60, 60, 70));
    let left_block = Block::default()
        .title(left_title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let base_block = Block::default()
        .title(base_title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let right_block = Block::default()
        .title(right_title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let left_inner = left_block.inner(chunks[0]);
    let base_inner = base_block.inner(chunks[1]);
    let right_inner = right_block.inner(chunks[2]);
    f.render_widget(left_block, chunks[0]);
    f.render_widget(base_block, chunks[1]);
    f.render_widget(right_block, chunks[2]);

    let result = match app.three_way_result.as_ref() {
        Some(r) => r,
        None => {
            let hint = Paragraph::new("Press 'o' to open files (3-way)")
                .style(Style::default().fg(Color::Rgb(100, 100, 100)));
            f.render_widget(hint, left_inner);
            return;
        }
    };

    let visible_height = left_inner.height as usize;
    let start = app.scroll_offset;
    let end = (start + visible_height).min(result.lines.len());

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
        while e < result.lines.len() && result.lines[e].status != ThreeWayStatus::Equal {
            e += 1;
        }
        e
    });

    let mut left_lines = Vec::new();
    let mut base_lines = Vec::new();
    let mut right_lines = Vec::new();

    for i in start..end {
        let line = &result.lines[i];
        let is_current = match (current_block_start, current_block_end) {
            (Some(s), Some(e)) => i >= s && i < e,
            _ => false,
        };

        let (left_bg, base_bg, right_bg) = match line.status {
            ThreeWayStatus::Equal => (BG_EQUAL, BG_EQUAL, BG_EQUAL),
            ThreeWayStatus::LeftChanged => (BG_LEFT_CHANGED, BG_GHOST, BG_EQUAL),
            ThreeWayStatus::RightChanged => (BG_EQUAL, BG_GHOST, BG_RIGHT_CHANGED),
            ThreeWayStatus::BothChanged => (BG_BOTH_CHANGED, BG_GHOST, BG_BOTH_CHANGED),
            ThreeWayStatus::Conflict => (BG_CONFLICT, BG_GHOST, BG_CONFLICT),
        };

        let left_bg = if is_current {
            brighten(left_bg)
        } else {
            left_bg
        };
        let base_bg = if is_current && line.status != ThreeWayStatus::Equal {
            brighten(base_bg)
        } else {
            base_bg
        };
        let right_bg = if is_current {
            brighten(right_bg)
        } else {
            right_bg
        };

        left_lines.push(render_line(&line.left_text, line.left_line_no, left_bg));
        base_lines.push(render_line(&line.base_text, line.base_line_no, base_bg));
        right_lines.push(render_line(&line.right_text, line.right_line_no, right_bg));
    }

    f.render_widget(Paragraph::new(left_lines), left_inner);
    f.render_widget(Paragraph::new(base_lines), base_inner);
    f.render_widget(Paragraph::new(right_lines), right_inner);
}

fn render_line(text: &str, line_no: Option<u32>, bg: Color) -> Line<'static> {
    let no_str = match line_no {
        Some(n) => format!("{:>width$} ", n, width = LINE_NO_WIDTH - 1),
        None => " ".repeat(LINE_NO_WIDTH),
    };
    Line::from(vec![
        Span::styled(no_str, Style::default().fg(FG_LINE_NO).bg(bg)),
        Span::styled(text.to_string(), Style::default().fg(Color::White).bg(bg)),
    ])
}

fn brighten(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(50),
            g.saturating_add(25),
            b.saturating_add(0),
        ),
        _ => Color::Rgb(80, 50, 10),
    }
}
