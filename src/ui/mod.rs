pub mod diff_view;
pub mod menu_bar;
pub mod status_bar;
pub mod three_way_view;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, AppMode};

/// Stores the close button rect of the last drawn dialog, for mouse hit testing
static mut DIALOG_CLOSE_RECT: Option<Rect> = None;
/// Stores the file browser list area for mouse hit testing
static mut FILE_BROWSER_LIST_RECT: Option<Rect> = None;

pub fn dialog_close_rect() -> Option<Rect> {
    unsafe { DIALOG_CLOSE_RECT }
}

pub fn file_browser_list_rect() -> Option<Rect> {
    unsafe { FILE_BROWSER_LIST_RECT }
}

pub fn draw(f: &mut Frame, app: &App) {
    // Reset hit-test rects
    unsafe {
        DIALOG_CLOSE_RECT = None;
        FILE_BROWSER_LIST_RECT = None;
    }
    diff_view::reset_panel_rects();
    three_way_view::reset_panel_rects();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // menu bar
            Constraint::Min(5),    // diff view
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    menu_bar::draw(f, app, chunks[0]);

    let main_area = chunks[1];
    if app.is_three_way {
        three_way_view::draw(f, app, main_area);
    } else {
        diff_view::draw(f, app, main_area);
    }

    match app.mode {
        AppMode::OpenLeft
        | AppMode::OpenRight
        | AppMode::OpenBase
        | AppMode::SaveLeft
        | AppMode::SaveRight => {
            draw_file_browser_dialog(f, app, main_area);
        }
        AppMode::OpenChooseMode => {
            draw_choose_mode_dialog(f, main_area);
        }
        AppMode::SaveConfirm => {
            draw_save_confirm_dialog(f, main_area);
        }
        AppMode::Normal | AppMode::Editing => {}
    }

    status_bar::draw(f, app, chunks[2]);
}

/// Draw [x] close button at top-right of popup_area and record its rect for hit testing
fn draw_close_button(f: &mut Frame, popup_area: Rect) {
    // Place [x] at top-right corner, overlapping the border
    let x_btn_width = 3u16; // "[x]"
    if popup_area.width < x_btn_width + 2 {
        return;
    }
    let btn_rect = Rect {
        x: popup_area.x + popup_area.width - x_btn_width - 1,
        y: popup_area.y,
        width: x_btn_width,
        height: 1,
    };

    let btn = Paragraph::new(Span::styled(
        "[x]",
        Style::default()
            .fg(Color::Rgb(200, 80, 80))
            .bg(Color::Rgb(40, 40, 50)),
    ));
    f.render_widget(btn, btn_rect);

    unsafe {
        DIALOG_CLOSE_RECT = Some(btn_rect);
    }
}

fn center_popup(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .flex(Flex::Center)
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .flex(Flex::Center)
        .split(vertical[1]);

    horizontal[1]
}

fn draw_file_browser_dialog(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));
    let popup_area = center_popup(area, popup_width, popup_height);

    let title = match app.mode {
        AppMode::OpenLeft => " Select left file ",
        AppMode::OpenRight => " Select right file ",
        AppMode::OpenBase => " Select base file ",
        AppMode::SaveLeft => " Save left file ",
        AppMode::SaveRight => " Save right file ",
        _ => "",
    };

    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    draw_close_button(f, popup_area);

    let browser = match &app.file_browser {
        Some(b) => b,
        None => return,
    };

    // Line 0: current directory path
    let dir_display = browser.current_dir.to_string_lossy().to_string();
    let dir_line = Paragraph::new(dir_display)
        .style(Style::default().fg(Color::Yellow));
    let dir_rect = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    f.render_widget(dir_line, dir_rect);

    // Separator
    let sep_rect = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };
    let sep = Paragraph::new("─".repeat(inner.width as usize))
        .style(Style::default().fg(Color::Rgb(80, 80, 80)));
    f.render_widget(sep, sep_rect);

    // File list area
    let list_y = inner.y + 2;
    // Reserve rows for footer: save mode needs 2 (filename + hint), open needs 1 (hint)
    let is_save = browser.is_save_mode();
    let footer_rows: u16 = if is_save { 2 } else { 1 };
    let list_height = inner.height.saturating_sub(2 + footer_rows) as usize;

    // Store list rect for mouse hit testing
    unsafe {
        FILE_BROWSER_LIST_RECT = Some(Rect {
            x: inner.x,
            y: list_y,
            width: inner.width,
            height: list_height as u16,
        });
    }

    let normal_style = Style::default().fg(Color::White);
    let dir_style = Style::default().fg(Color::Cyan);
    let selected_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Rgb(100, 150, 220))
        .add_modifier(Modifier::BOLD);
    let selected_dir_style = Style::default()
        .fg(Color::Rgb(20, 40, 60))
        .bg(Color::Rgb(100, 150, 220))
        .add_modifier(Modifier::BOLD);

    for (i, entry) in browser
        .entries
        .iter()
        .skip(browser.scroll_offset)
        .take(list_height)
        .enumerate()
    {
        let abs_idx = browser.scroll_offset + i;
        let is_selected = abs_idx == browser.selected;

        let (prefix, style) = if entry.is_dir {
            if is_selected {
                ("📁 ", selected_dir_style)
            } else {
                ("📁 ", dir_style)
            }
        } else if is_selected {
            ("   ", selected_style)
        } else {
            ("   ", normal_style)
        };

        let display = format!("{}{}", prefix, entry.name);
        let line = Paragraph::new(display).style(style);
        let line_rect = Rect {
            x: inner.x,
            y: list_y + i as u16,
            width: inner.width,
            height: 1,
        };
        f.render_widget(line, line_rect);
    }

    // Footer
    if is_save {
        // Filename input line
        if let Some(ref filename) = browser.filename_input {
            let fname_y = inner.y + inner.height - 2;
            let label = "File: ";
            let fname_line = Paragraph::new(Line::from(vec![
                Span::styled(label, Style::default().fg(Color::Green)),
                Span::styled(filename.as_str(), Style::default().fg(Color::White)),
            ]));
            let fname_rect = Rect {
                x: inner.x,
                y: fname_y,
                width: inner.width,
                height: 1,
            };
            f.render_widget(fname_line, fname_rect);

            // Set cursor at end of filename input
            let cursor_x = inner.x + label.len() as u16 + filename.len() as u16;
            f.set_cursor_position((cursor_x.min(inner.x + inner.width - 1), fname_y));
        }

        // Hint line
        let hint_y = inner.y + inner.height - 1;
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::styled(":save  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab", Style::default().fg(Color::Green)),
            Span::styled(":browse  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::styled(":cancel", Style::default().fg(Color::DarkGray)),
        ]));
        let hint_rect = Rect {
            x: inner.x,
            y: hint_y,
            width: inner.width,
            height: 1,
        };
        f.render_widget(hint, hint_rect);
    } else if inner.height > 3 {
        let hint_y = inner.y + inner.height - 1;
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::styled(":select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Backspace", Style::default().fg(Color::Green)),
            Span::styled(":up  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Green)),
            Span::styled(":cancel", Style::default().fg(Color::DarkGray)),
        ]));
        let hint_rect = Rect {
            x: inner.x,
            y: hint_y,
            width: inner.width,
            height: 1,
        };
        f.render_widget(hint, hint_rect);
    }
}

fn draw_choose_mode_dialog(f: &mut Frame, area: Rect) {
    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_area = center_popup(area, popup_width, 5);

    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Open mode ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    draw_close_button(f, popup_area);

    let text =
        Paragraph::new("[2] 2-way diff  [3] 3-way merge").style(Style::default().fg(Color::White));
    f.render_widget(text, inner);
}

fn draw_save_confirm_dialog(f: &mut Frame, area: Rect) {
    let popup_width = 45.min(area.width.saturating_sub(4));
    let popup_area = center_popup(area, popup_width, 5);

    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(" Unsaved changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);
    draw_close_button(f, popup_area);

    let text =
        Paragraph::new("[s]ave  [d]iscard  [c]ancel").style(Style::default().fg(Color::White));
    f.render_widget(text, inner);
}
