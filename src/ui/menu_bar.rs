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

/// Canonical menu item list -- used by both draw() and hit_test()
fn menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem {
            label: " New",
            action: Some(MenuAction::New),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "Open",
            action: Some(MenuAction::Open),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "Save",
            action: Some(MenuAction::Save),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "Ref",
            action: Some(MenuAction::Refresh),
        },
        MenuItem {
            label: " \u{2502} ",
            action: None,
        },
        MenuItem {
            label: "Prev",
            action: Some(MenuAction::PrevDiff),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "Next",
            action: Some(MenuAction::NextDiff),
        },
        MenuItem {
            label: " \u{2502} ",
            action: None,
        },
        MenuItem {
            label: "LtR",
            action: Some(MenuAction::CopyLeftToRight),
        },
        MenuItem {
            label: " ",
            action: None,
        },
        MenuItem {
            label: "RtL",
            action: Some(MenuAction::CopyRightToLeft),
        },
        MenuItem {
            label: " \u{2502} ",
            action: None,
        },
        MenuItem {
            label: "AllSel",
            action: Some(MenuAction::SelectAll),
        },
        MenuItem {
            label: " \u{2502} ",
            action: None,
        },
        MenuItem {
            label: "ws",
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

/// Compute display width using ratatui's Span (same calculation as rendering)
fn display_width(s: &str) -> u16 {
    use ratatui::text::Span;
    let span = Span::raw(s);
    span.width() as u16
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
    let tab = app.active_tab();
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

    let has_diff = tab.diff_count() > 0;

    let is_three_way = tab.is_three_way;
    let items = menu_items();
    let spans: Vec<Span> = items
        .iter()
        .map(|item| {
            let label = match item.action {
                Some(MenuAction::CopyLeftToRight) if is_three_way => "LtM",
                Some(MenuAction::CopyRightToLeft) if is_three_way => "RtM",
                _ => item.label,
            };
            let style = match item.action {
                None => {
                    if item.label.contains('\u{2502}') {
                        separator
                    } else {
                        bg
                    }
                }
                Some(MenuAction::Save) => {
                    if tab.has_unsaved_changes {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::ToggleWhitespace) => {
                    if tab.diff_options.ignore_whitespace {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::ToggleCase) => {
                    if tab.diff_options.ignore_case {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::SelectAll) => {
                    if tab.select_all {
                        toggled_on
                    } else {
                        active
                    }
                }
                Some(MenuAction::New) | Some(MenuAction::Open) | Some(MenuAction::Refresh) => {
                    active
                }
                Some(_) => {
                    if has_diff {
                        active
                    } else {
                        disabled
                    }
                }
            };
            Span::styled(label, style)
        })
        .collect();

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(bg);
    f.render_widget(bar, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_display_widths() {
        let items = menu_items();
        let mut pos = 0u16;
        for item in &items {
            let w = display_width(item.label);
            eprintln!(
                "pos={:3} w={} label={:?} action={:?}",
                pos, w, item.label, item.action
            );
            pos += w;
        }
    }
}
