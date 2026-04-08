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
    let green = Style::default().fg(Color::Green).bg(Color::Rgb(30, 30, 40));
    let dim = Style::default()
        .fg(Color::Rgb(100, 100, 120))
        .bg(Color::Rgb(30, 30, 40));
    let yellow = Style::default()
        .fg(Color::Yellow)
        .bg(Color::Rgb(30, 30, 40));

    let count = app.diff_count();
    let diff_info = if app.is_three_way {
        if let Some(ref result) = app.three_way_result {
            if result.diff_positions.is_empty() {
                "Files are identical".to_string()
            } else {
                format!(
                    "3-way | Diffs: {} | Conflicts: {} | Current: {}/{}",
                    result.diff_positions.len(),
                    result.conflict_count,
                    if app.current_diff >= 0 {
                        app.current_diff + 1
                    } else {
                        0
                    },
                    result.diff_positions.len()
                )
            }
        } else {
            "No files loaded".to_string()
        }
    } else if count == 0 {
        if app.diff_result.is_some() {
            "Files are identical".to_string()
        } else {
            "No files loaded".to_string()
        }
    } else {
        format!(
            "Diffs: {} | Current: {}/{}",
            count,
            if app.current_diff >= 0 {
                app.current_diff + 1
            } else {
                0
            },
            count
        )
    };

    let unsaved = if app.has_unsaved_changes {
        " [modified]"
    } else {
        ""
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
    let keys = " ^O:open F7/F8:diff Alt+←→:copy ^S:save ^Z:undo ^Q:quit";

    let line = Line::from(vec![
        Span::styled(" ", bg),
        Span::styled(diff_info, bg),
        Span::styled(unsaved, yellow),
        Span::styled(ws, green),
        Span::styled(case, green),
        Span::styled(keys, dim),
    ]);

    let bar = Paragraph::new(line);
    f.render_widget(bar, area);
}
