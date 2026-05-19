use std::time::SystemTime;

use chrono::{DateTime, Local};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::models::diff_line::DirEntryStatus;

static mut DIR_LIST_RECT: Option<Rect> = None;

#[allow(dead_code)]
pub fn dir_list_rect() -> Option<Rect> {
    unsafe { DIR_LIST_RECT }
}

pub fn reset_dir_list_rect() {
    unsafe {
        DIR_LIST_RECT = None;
    }
}

// Column widths (fixed)
const W_STATUS: usize = 4;
const W_DATE: usize = 16; // "YYYY-MM-DD HH:MM"
const W_SIZE: usize = 9;  // "1023.9 MB"
const W_SEP: usize = 1;
// path width = inner.width - W_STATUS - W_SEP - W_DATE - W_SEP - W_SIZE - W_SEP - W_DATE - W_SEP - W_SIZE - W_SEP
// = inner.width - (4+1+16+1+9+1+16+1+9+1) = inner.width - 59
const FIXED_COLS: usize = W_STATUS + W_SEP + W_DATE + W_SEP + W_SIZE + W_SEP + W_DATE + W_SEP + W_SIZE + W_SEP;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();
    let result = match &tab.dir_result {
        Some(r) => r,
        None => return,
    };

    let title = format!(
        " {} <-> {} ",
        result.left_dir.to_string_lossy(),
        result.right_dir.to_string_lossy()
    );
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 70)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let path_width = (inner.width as usize).saturating_sub(FIXED_COLS).max(10);

    // Compute display scroll_offset so selected is always visible
    let list_capacity = visible_height.saturating_sub(2); // header + separator
    let scroll_offset = if list_capacity == 0 {
        0
    } else if result.selected < result.scroll_offset {
        result.selected
    } else if result.selected >= result.scroll_offset + list_capacity {
        result.selected + 1 - list_capacity
    } else {
        result.scroll_offset
    };

    unsafe {
        DIR_LIST_RECT = Some(inner);
    }

    // Header
    let header_style = Style::default()
        .fg(Color::Rgb(150, 150, 170))
        .add_modifier(Modifier::BOLD);
    let header = build_row(
        "St. ",
        &truncate_pad("Path", path_width),
        &format!("{:<width$}", "Left Modified", width = W_DATE),
        &format!("{:>width$}", "Left Size", width = W_SIZE),
        &format!("{:<width$}", "Right Modified", width = W_DATE),
        &format!("{:>width$}", "Right Size", width = W_SIZE),
        header_style,
        header_style,
    );

    let sep_line = Line::from(Span::styled(
        "\u{2500}".repeat(inner.width as usize),
        Style::default().fg(Color::Rgb(60, 60, 70)),
    ));

    let mut lines: Vec<Line> = vec![header, sep_line];

    let list_height = list_capacity;
    for (i, entry) in result
        .entries
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(list_height)
    {
        let is_selected = i == result.selected;

        let (status_label, status_color) = match entry.status {
            DirEntryStatus::Changed  => ("!= \u{2502}", Color::Rgb(220, 180, 80)),
            DirEntryStatus::LeftOnly => ("<  \u{2502}", Color::Rgb(100, 160, 240)),
            DirEntryStatus::RightOnly=> (">  \u{2502}", Color::Rgb(100, 200, 130)),
            DirEntryStatus::Equal    => ("== \u{2502}", Color::Rgb(80, 80, 80)),
        };

        let path_str = truncate_pad(
            &entry.rel_path.to_string_lossy(),
            path_width,
        );
        let left_date  = fmt_time_opt(entry.left_modified, W_DATE);
        let left_size  = fmt_size_opt(entry.left_size, W_SIZE);
        let right_date = fmt_time_opt(entry.right_modified, W_DATE);
        let right_size = fmt_size_opt(entry.right_size, W_SIZE);

        let (bg, text_fg) = if is_selected {
            (Color::Rgb(50, 70, 100), Color::White)
        } else {
            (Color::Reset, Color::Rgb(200, 200, 200))
        };

        let status_style = Style::default().fg(status_color).bg(bg);
        let text_style   = Style::default().fg(text_fg).bg(bg);

        let line = build_row(
            status_label,
            &path_str,
            &left_date,
            &left_size,
            &right_date,
            &right_size,
            status_style,
            text_style,
        );
        lines.push(line);
    }

    let h = tab.h_scroll as u16;
    f.render_widget(Paragraph::new(lines).scroll((0, h)), inner);
}

fn build_row<'a>(
    status: &'a str,
    path: &str,
    left_date: &str,
    left_size: &str,
    right_date: &str,
    right_size: &str,
    status_style: Style,
    text_style: Style,
) -> Line<'a> {
    let dim = Style::default()
        .fg(Color::Rgb(90, 90, 110))
        .bg(text_style.bg.unwrap_or(Color::Reset));
    Line::from(vec![
        Span::styled(status.to_string(), status_style),
        Span::styled(" ", text_style),
        Span::styled(path.to_string(), text_style),
        Span::styled(" ", text_style),
        Span::styled(left_date.to_string(), text_style),
        Span::styled(" ", dim),
        Span::styled(left_size.to_string(), text_style),
        Span::styled(" ", dim),
        Span::styled(right_date.to_string(), text_style),
        Span::styled(" ", dim),
        Span::styled(right_size.to_string(), text_style),
    ])
}

fn truncate_pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
    } else {
        format!("{:<width$}", s)
    }
}

fn fmt_time_opt(t: Option<SystemTime>, width: usize) -> String {
    match t {
        Some(st) => {
            let s = format_local(st);
            format!("{:<width$}", s)
        }
        None => format!("{:<width$}", "-"),
    }
}

fn fmt_size_opt(size: Option<u64>, width: usize) -> String {
    match size {
        Some(b) => {
            let s = format_size(b);
            format!("{:>width$}", s)
        }
        None => format!("{:>width$}", "-"),
    }
}

fn format_local(t: SystemTime) -> String {
    let dt: DateTime<Local> = t.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}
