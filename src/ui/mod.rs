pub mod menu_bar;
pub mod diff_view;
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

    match app.mode {
        AppMode::OpenLeft | AppMode::OpenRight => {
            diff_view::draw(f, app, chunks[1]);
            // Draw input overlay on top of diff area
            draw_input_dialog(f, app, chunks[1]);
        }
        AppMode::Normal => {
            diff_view::draw(f, app, chunks[1]);
        }
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

    // Show cursor
    f.set_cursor_position((
        inner.x + app.input_buffer.len() as u16,
        inner.y,
    ));
}
