use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, AppMode, PanelSide};
use crate::models::diff_line::ThreeWayStatus;

const LINE_NO_WIDTH: usize = 5;

static mut LEFT_PANEL_RECT_3W: Option<Rect> = None;
static mut BASE_PANEL_RECT_3W: Option<Rect> = None;
static mut RIGHT_PANEL_RECT_3W: Option<Rect> = None;

pub fn left_panel_rect() -> Option<Rect> {
    unsafe { LEFT_PANEL_RECT_3W }
}
pub fn base_panel_rect() -> Option<Rect> {
    unsafe { BASE_PANEL_RECT_3W }
}
pub fn right_panel_rect() -> Option<Rect> {
    unsafe { RIGHT_PANEL_RECT_3W }
}
pub fn reset_panel_rects() {
    unsafe {
        LEFT_PANEL_RECT_3W = None;
        BASE_PANEL_RECT_3W = None;
        RIGHT_PANEL_RECT_3W = None;
    }
}

const BG_EQUAL: Color = Color::Reset;
const BG_LEFT_CHANGED: Color = Color::Rgb(20, 35, 55);
const BG_RIGHT_CHANGED: Color = Color::Rgb(20, 35, 55);
const BG_BOTH_CHANGED: Color = Color::Rgb(20, 50, 30);
const BG_CONFLICT: Color = Color::Rgb(55, 20, 20);
const BG_GHOST: Color = Color::Rgb(30, 30, 30);
const FG_LINE_NO: Color = Color::Rgb(100, 100, 100);

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let left_title = tab
        .left_path
        .as_ref()
        .map(|p| {
            format!(
                " {}{} ",
                if tab.has_unsaved_changes { "[*] " } else { "" },
                p.display()
            )
        })
        .unwrap_or_else(|| " (no file) ".to_string());
    let base_title = tab
        .base_path
        .as_ref()
        .map(|p| format!(" {} ", p.display()))
        .unwrap_or_else(|| " (base) ".to_string());
    let right_title = tab
        .right_path
        .as_ref()
        .map(|p| {
            format!(
                " {}{} ",
                if tab.has_unsaved_changes { "[*] " } else { "" },
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

    unsafe {
        LEFT_PANEL_RECT_3W = Some(left_inner);
        BASE_PANEL_RECT_3W = Some(base_inner);
        RIGHT_PANEL_RECT_3W = Some(right_inner);
    }

    // When editing, always use raw text rendering
    let use_raw_text = app.mode == AppMode::Editing
        || !tab
            .three_way_result
            .as_ref()
            .map(|r| !r.lines.is_empty())
            .unwrap_or(false);
    if use_raw_text {
        let left_src = tab.source_lines(PanelSide::Left);
        let base_src = tab.source_lines(PanelSide::Base);
        let right_src = tab.source_lines(PanelSide::Right);
        let max_lines = left_src
            .len()
            .max(base_src.len())
            .max(right_src.len())
            .max(1);
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
        let mut base_lines_render = Vec::new();
        let mut right_lines = Vec::new();

        for i in start..end {
            let get_text = |panel: PanelSide, src: &[String]| -> String {
                if let Some((p, sl, _)) = edit_info {
                    if p == panel && i == sl {
                        return edit_live_text.clone().unwrap_or_default();
                    }
                }
                src.get(i).cloned().unwrap_or_default()
            };

            let left_text = get_text(PanelSide::Left, &left_src);
            let base_text = get_text(PanelSide::Base, &base_src);
            let right_text = get_text(PanelSide::Right, &right_src);

            let make_line = |src: &[String], text: String| -> Line<'static> {
                let no = if i < src.len() {
                    format!("{:>4} ", i + 1)
                } else {
                    "     ".to_string()
                };
                Line::from(vec![
                    Span::styled(
                        no,
                        Style::default().fg(Color::Rgb(100, 100, 120)).bg(BG_EQUAL),
                    ),
                    Span::styled(text, Style::default().fg(Color::White).bg(BG_EQUAL)),
                ])
            };

            left_lines.push(make_line(&left_src, left_text));
            base_lines_render.push(make_line(&base_src, base_text));
            right_lines.push(make_line(&right_src, right_text));
        }

        // Show hint if all panels are empty
        if tab.left_text.is_empty()
            && tab.base_text.is_empty()
            && tab.right_text.is_empty()
            && app.mode != AppMode::Editing
        {
            left_lines.clear();
            left_lines.push(Line::from(vec![
                Span::styled(
                    "   1 ",
                    Style::default().fg(Color::Rgb(100, 100, 120)).bg(BG_EQUAL),
                ),
                Span::styled(
                    "Press 'o' or click to edit",
                    Style::default().fg(Color::Rgb(100, 100, 100)).bg(BG_EQUAL),
                ),
            ]));
        }

        f.render_widget(Paragraph::new(left_lines), left_inner);
        f.render_widget(Paragraph::new(base_lines_render), base_inner);
        f.render_widget(Paragraph::new(right_lines), right_inner);

        // Cursor
        if let Some((panel, source_line, cursor_col)) = edit_info {
            let panel_rect = match panel {
                PanelSide::Left => left_inner,
                PanelSide::Base => base_inner,
                PanelSide::Right => right_inner,
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

    let result = tab.three_way_result.as_ref().unwrap();

    let visible_height = left_inner.height as usize;
    let start = tab.scroll_offset;
    let end = (start + visible_height).min(result.lines.len());

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

    // Render cursor if editing
    if app.mode == AppMode::Editing {
        if let Some(ref es) = tab.edit_state {
            let panel_rect = match es.panel {
                PanelSide::Left => left_inner,
                PanelSide::Right => right_inner,
                PanelSide::Base => base_inner,
            };
            let edit_display_idx = result.lines.iter().position(|dl| {
                let ln = match es.panel {
                    PanelSide::Left => dl.left_line_no,
                    PanelSide::Right => dl.right_line_no,
                    PanelSide::Base => dl.base_line_no,
                };
                ln.map(|n| n as usize - 1) == Some(es.source_line)
            });
            let row_on_screen = edit_display_idx
                .unwrap_or(es.source_line)
                .saturating_sub(tab.scroll_offset);
            if (row_on_screen as u16) < panel_rect.height {
                let display_col = if let Some(live) = tab.edit_current_line_text() {
                    use unicode_width::UnicodeWidthStr;
                    let prefix: String = live.chars().take(es.cursor_col).collect();
                    prefix.width()
                } else {
                    es.cursor_col
                };
                let x = panel_rect.x + LINE_NO_WIDTH as u16 + display_col as u16;
                let y = panel_rect.y + row_on_screen as u16;
                f.set_cursor_position((x.min(panel_rect.x + panel_rect.width - 1), y));
            }
        }
    }
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
