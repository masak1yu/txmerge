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

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();
    let result = match &tab.dir_result {
        Some(r) => r,
        None => return,
    };

    let left_name = result
        .left_dir
        .to_string_lossy()
        .to_string();
    let right_name = result
        .right_dir
        .to_string_lossy()
        .to_string();

    let title = format!(" {} <-> {} ", left_name, right_name);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 70)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;

    // Compute display scroll_offset so selected is always visible.
    // header(1) + separator(1) = 2 overhead rows
    let list_capacity = visible_height.saturating_sub(2);
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

    let changed_count = result
        .entries
        .iter()
        .filter(|e| e.status == DirEntryStatus::Changed)
        .count();
    let left_only_count = result
        .entries
        .iter()
        .filter(|e| e.status == DirEntryStatus::LeftOnly)
        .count();
    let right_only_count = result
        .entries
        .iter()
        .filter(|e| e.status == DirEntryStatus::RightOnly)
        .count();

    // Header row
    let header_style = Style::default()
        .fg(Color::Rgb(150, 150, 170))
        .add_modifier(Modifier::BOLD);
    let header = Line::from(vec![
        Span::styled(
            format!("{:<4} ", ""),
            header_style,
        ),
        Span::styled(
            format!(
                "{}  Changed:{} LeftOnly:{} RightOnly:{}",
                " Status", changed_count, left_only_count, right_only_count
            ),
            header_style,
        ),
    ]);

    // Separator
    let sep = Line::from(Span::styled(
        "\u{2500}".repeat(inner.width as usize),
        Style::default().fg(Color::Rgb(60, 60, 70)),
    ));

    let mut lines: Vec<Line> = vec![header, sep];

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
            DirEntryStatus::Changed => ("!=  ", Color::Rgb(220, 180, 80)),
            DirEntryStatus::LeftOnly => ("<   ", Color::Rgb(100, 160, 240)),
            DirEntryStatus::RightOnly => ("  > ", Color::Rgb(100, 200, 130)),
            DirEntryStatus::Equal => ("==  ", Color::Rgb(80, 80, 80)),
        };

        let path_str = entry.rel_path.to_string_lossy().to_string();

        let (bg, fg) = if is_selected {
            (Color::Rgb(50, 70, 100), Color::White)
        } else {
            (Color::Reset, Color::Rgb(200, 200, 200))
        };

        let line = Line::from(vec![
            Span::styled(
                status_label,
                Style::default().fg(status_color).bg(bg),
            ),
            Span::styled(
                format!(" {}", path_str),
                Style::default().fg(fg).bg(bg),
            ),
        ]);
        lines.push(line);
    }

    let para = Paragraph::new(lines);
    f.render_widget(para, inner);
}
