pub mod menu_bar;
pub mod diff_view;
pub mod three_way_view;
pub mod status_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::{App, AppMode};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // menu bar
            Constraint::Min(5),    // diff view
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    menu_bar::draw(f, app, chunks[0]);

    // Draw main content
    let main_area = chunks[1];
    if app.is_three_way {
        three_way_view::draw(f, app, main_area);
    } else {
        diff_view::draw(f, app, main_area);
    }

    // Draw overlays
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

fn draw_input_dialog(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout, Direction, Flex};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let popup_height = 5;
    let popup_width = 60.min(area.width.saturating_sub(4));

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(popup_height),
            Constraint::Min(0),
        ])
        .flex(Flex::Center)
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(popup_width),
            Constraint::Min(0),
        ])
        .flex(Flex::Center)
        .split(vertical[1]);

    let popup_area = horizontal[1];

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

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White));
    f.render_widget(input, inner);

    f.set_cursor_position((
        inner.x + app.input_buffer.len() as u16,
        inner.y,
    ));
}

fn draw_choose_mode_dialog(f: &mut Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout, Direction, Flex};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let popup_height = 5;
    let popup_width = 40.min(area.width.saturating_sub(4));

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(popup_height), Constraint::Min(0)])
        .flex(Flex::Center)
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(popup_width), Constraint::Min(0)])
        .flex(Flex::Center)
        .split(vertical[1]);

    let popup_area = horizontal[1];
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Open mode ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let text = Paragraph::new("[2] 2-way diff  [3] 3-way merge")
        .style(Style::default().fg(Color::White));
    f.render_widget(text, inner);
}

fn draw_save_confirm_dialog(f: &mut Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout, Direction, Flex};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let popup_height = 5;
    let popup_width = 45.min(area.width.saturating_sub(4));

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(popup_height), Constraint::Min(0)])
        .flex(Flex::Center)
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(popup_width), Constraint::Min(0)])
        .flex(Flex::Center)
        .split(vertical[1]);

    let popup_area = horizontal[1];
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Unsaved changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let text = Paragraph::new("[s]ave  [d]iscard  [c]ancel")
        .style(Style::default().fg(Color::White));
    f.render_widget(text, inner);
}
