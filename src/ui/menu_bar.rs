use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let active = Style::default()
        .fg(Color::Rgb(200, 200, 200))
        .bg(Color::Rgb(40, 40, 50));
    let toggled_on = Style::default()
        .fg(Color::Green)
        .bg(Color::Rgb(40, 40, 50))
        .add_modifier(Modifier::BOLD);
    let separator = Style::default()
        .fg(Color::Rgb(80, 80, 80))
        .bg(Color::Rgb(40, 40, 50));
    let bg = Style::default().bg(Color::Rgb(40, 40, 50));

    let sep = Span::styled(" │ ", separator);

    let ws_style = if app.diff_options.ignore_whitespace {
        toggled_on
    } else {
        active
    };
    let case_style = if app.diff_options.ignore_case {
        toggled_on
    } else {
        active
    };

    let has_diff = app
        .diff_result
        .as_ref()
        .map(|r| r.diff_count > 0)
        .unwrap_or(false);
    let nav_style = if has_diff { active } else {
        Style::default()
            .fg(Color::Rgb(80, 80, 80))
            .bg(Color::Rgb(40, 40, 50))
    };

    let line = Line::from(vec![
        Span::styled(" ", bg),
        Span::styled("📂Open", active),
        sep.clone(),
        Span::styled("🔄Refresh", active),
        sep.clone(),
        Span::styled("⚙ Opt", active),
        sep.clone(),
        Span::styled("⏮", nav_style),
        Span::styled(" ", bg),
        Span::styled("◀", nav_style),
        Span::styled(" ", bg),
        Span::styled("▶", nav_style),
        Span::styled(" ", bg),
        Span::styled("⏭", nav_style),
        sep.clone(),
        Span::styled("◁▷", nav_style),
        Span::styled(" ", bg),
        Span::styled("▷◁", nav_style),
        sep.clone(),
        Span::styled("◁▷+", nav_style),
        Span::styled(" ", bg),
        Span::styled("▷◁+", nav_style),
        sep.clone(),
        Span::styled("⇉", nav_style),
        Span::styled(" ", bg),
        Span::styled("⇇", nav_style),
        sep.clone(),
        Span::styled("␣ws", ws_style),
        Span::styled(" ", bg),
        Span::styled("Aa", case_style),
        Span::styled(" ", bg),
    ]);

    let bar = Paragraph::new(line).style(bg);
    f.render_widget(bar, area);
}
