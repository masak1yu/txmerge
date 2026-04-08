use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::events::MenuAction;

// Menu item definition: (label, action)
// We track cumulative widths for hit testing
struct MenuItem {
    label: &'static str,
    action: Option<MenuAction>,
}

fn menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem { label: " ", action: None },
        MenuItem { label: "📂Op", action: Some(MenuAction::Open) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "💾Sv", action: Some(MenuAction::Save) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "🔄Re", action: Some(MenuAction::Refresh) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "⏮", action: Some(MenuAction::FirstDiff) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "◀", action: Some(MenuAction::PrevDiff) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "▶", action: Some(MenuAction::NextDiff) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "⏭", action: Some(MenuAction::LastDiff) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "◁▷", action: Some(MenuAction::CopyLeftToRight) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "▷◁", action: Some(MenuAction::CopyRightToLeft) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "◁▷+", action: Some(MenuAction::CopyLeftToRightNext) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "▷◁+", action: Some(MenuAction::CopyRightToLeftNext) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "⇉", action: Some(MenuAction::CopyAllLR) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "⇇", action: Some(MenuAction::CopyAllRL) },
        MenuItem { label: " │ ", action: None },
        MenuItem { label: "␣ws", action: Some(MenuAction::ToggleWhitespace) },
        MenuItem { label: " ", action: None },
        MenuItem { label: "Aa", action: Some(MenuAction::ToggleCase) },
    ]
}

/// Hit test: given column x, return the MenuAction if any
pub fn hit_test(x: u16) -> Option<MenuAction> {
    let items = menu_items();
    let mut pos: u16 = 0;
    for item in &items {
        let width = unicode_display_width(item.label) as u16;
        if x >= pos && x < pos + width {
            return item.action;
        }
        pos += width;
    }
    None
}

fn unicode_display_width(s: &str) -> usize {
    // Rough estimate: emoji/wide chars = 2, ASCII = 1
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                1
            } else {
                2
            }
        })
        .sum()
}

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
    let disabled = Style::default()
        .fg(Color::Rgb(80, 80, 80))
        .bg(Color::Rgb(40, 40, 50));

    let has_diff = app.diff_count() > 0;
    let nav_style = if has_diff { active } else { disabled };
    let save_style = if app.has_unsaved_changes { toggled_on } else { active };
    let ws_style = if app.diff_options.ignore_whitespace { toggled_on } else { active };
    let case_style = if app.diff_options.ignore_case { toggled_on } else { active };
    let sep = Span::styled(" │ ", separator);

    let line = Line::from(vec![
        Span::styled(" ", bg),
        Span::styled("📂Op", active),
        sep.clone(),
        Span::styled("💾Sv", save_style),
        sep.clone(),
        Span::styled("🔄Re", active),
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
