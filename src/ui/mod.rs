pub mod menu_bar;
pub mod diff_view;
pub mod three_way_view;
pub mod status_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect, Flex};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, AppMode};

/// Stores the close button rect of the last drawn dialog, for mouse hit testing
static mut DIALOG_CLOSE_RECT: Option<Rect> = None;

pub fn dialog_close_rect() -> Option<Rect> {
    unsafe { DIALOG_CLOSE_RECT }
}

pub fn draw(f: &mut Frame, app: &App) {
    // Reset close button rect
    unsafe { DIALOG_CLOSE_RECT = None; }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // menu bar
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
        AppMode::OpenLeft | AppMode::OpenRight | AppMode::OpenBase => {
            draw_input_dialog(f, app, main_area);
        }
        AppMode::OpenChooseMode => {
            draw_choose_mode_dialog(f, main_area);
        }
        AppMode::SaveConfirm => {
            draw_save_confirm_dialog(f, main_area);
        }
        AppMode::Normal => {}
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
        Style::default().fg(Color::Rgb(200, 80, 80)).bg(Color::Rgb(40, 40, 50)),
    ));
    f.render_widget(btn, btn_rect);

    unsafe { DIALOG_CLOSE_RECT = Some(btn_rect); }
}

fn center_popup(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(height), Constraint::Min(0)])
        .flex(Flex::Center)
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(width), Constraint::Min(0)])
        .flex(Flex::Center)
        .split(vertical[1]);

    horizontal[1]
}

fn draw_input_dialog(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_area = center_popup(area, popup_width, 5);

    let title = match app.mode {
        AppMode::OpenLeft => " Left file path ",
        AppMode::OpenRight => " Right file path ",
        AppMode::OpenBase => " Base file path ",
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

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White));
    f.render_widget(input, inner);

    f.set_cursor_position((
        inner.x + app.input_buffer.len() as u16,
        inner.y,
    ));
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

    let text = Paragraph::new("[2] 2-way diff  [3] 3-way merge")
        .style(Style::default().fg(Color::White));
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

    let text = Paragraph::new("[s]ave  [d]iscard  [c]ancel")
        .style(Style::default().fg(Color::White));
    f.render_widget(text, inner);
}
