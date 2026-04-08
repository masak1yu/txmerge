use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let bg = Style::default()
        .fg(Color::Rgb(180, 180, 180))
        .bg(Color::Rgb(30, 30, 40));

    let diff_info = if let Some(ref result) = app.diff_result {
        if result.diff_count == 0 {
            "Files are identical".to_string()
        } else {
            format!(
                "Diffs: {} | Current: {}/{}",
                result.diff_count,
                if app.current_diff >= 0 {
                    app.current_diff + 1
                } else {
                    0
                },
                result.diff_count
            )
        }
    } else {
        "No files loaded".to_string()
    };

    let ws = if app.diff_options.ignore_whitespace {
        " [WS:ignore]"
    } else {
        ""
    };
    let case = if app.diff_options.ignore_case {
        " [Case:ignore]"
    } else {
        ""
    };

    let keys = " [o]pen [n/p]diff [q]uit";

    let line = Line::from(vec![
        Span::styled(" ", bg),
        Span::styled(diff_info, bg),
        Span::styled(ws, Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 40))),
        Span::styled(case, Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 40))),
        Span::styled(keys, Style::default().fg(Color::Rgb(100, 100, 120)).bg(Color::Rgb(30, 30, 40))),
    ]);

    let bar = Paragraph::new(line);
    f.render_widget(bar, area);
}
