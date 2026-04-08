use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::events::MenuAction;

struct MenuItem {
    label: &'static str,
    action: Option<MenuAction>,
}

/// Canonical menu item list — used by both draw() and hit_test()
fn menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "📂Op",
            action: Some(MenuAction::Open),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "💾Sv",
            action: Some(MenuAction::Save),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "🔄Re",
            action: Some(MenuAction::Refresh),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "⏮",
            action: Some(MenuAction::FirstDiff),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "◀",
            action: Some(MenuAction::PrevDiff),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "▶",
            action: Some(MenuAction::NextDiff),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "⏭",
            action: Some(MenuAction::LastDiff),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "◁▷",
            action: Some(MenuAction::CopyLeftToRight),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "▷◁",
            action: Some(MenuAction::CopyRightToLeft),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "◁▷+",
            action: Some(MenuAction::CopyLeftToRightNext),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "▷◁+",
            action: Some(MenuAction::CopyRightToLeftNext),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "⇉",
            action: Some(MenuAction::CopyAllLR),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "⇇",
            action: Some(MenuAction::CopyAllRL),
        },
        MenuItem {
            label: " │ ",
            action: None,
        },
        MenuItem {
            label: "␣ws",
            action: Some(MenuAction::ToggleWhitespace),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "Aa",
            action: Some(MenuAction::ToggleCase),
        },
        MenuItem {
            label: " ",
            action: None,
        },
    ]
}

/// Compute display width using unicode-width (matches terminal rendering)
fn display_width(s: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    s.width() as u16
}

/// Hit test: given column x on row 0, return the MenuAction if any
pub fn hit_test(x: u16) -> Option<MenuAction> {
    let items = menu_items();
    let mut pos: u16 = 0;
    for item in &items {
        let w = display_width(item.label);
        if x >= pos && x < pos + w {
            return item.action;
        }
        pos += w;
    }
    None
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

    let items = menu_items();
    let spans: Vec<Span> = items
        .iter()
        .map(|item| {
            let style = match item.action {
                None => {
                    // Separator or spacer
                    if item.label.contains('│') {
                        separator
                    } else {
                        bg
                    }
                }
                Some(MenuAction::Save) => {
                    if app.has_unsaved_changes {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::ToggleWhitespace) => {
                    if app.diff_options.ignore_whitespace {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::ToggleCase) => {
                    if app.diff_options.ignore_case {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::Open) | Some(MenuAction::Refresh) => active,
                // Navigation and copy actions depend on having diffs
                Some(_) => {
                    if has_diff {
                        active
                    } else {
                        disabled
                    }
                }
            };
            Span::styled(item.label, style)
        })
        .collect();

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(bg);
    f.render_widget(bar, area);
}
