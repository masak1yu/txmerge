use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

#[derive(Debug, Clone, Copy)]
pub enum TabBarAction {
    SwitchTab(usize),
    CloseTab(usize),
}

/// Max number of hit-test regions we store (generous upper bound)
const MAX_REGIONS: usize = 64;

/// Stored hit-test regions: (x_start, x_end, action)
static mut TAB_BAR_REGIONS: [(u16, u16, Option<TabBarAction>); MAX_REGIONS] =
    [(0, 0, None); MAX_REGIONS];
static mut TAB_BAR_REGION_COUNT: usize = 0;
static mut TAB_BAR_Y: u16 = 0;

pub fn hit_test(x: u16, y: u16) -> Option<TabBarAction> {
    unsafe {
        if y != TAB_BAR_Y {
            return None;
        }
        let count = TAB_BAR_REGION_COUNT;
        for i in 0..count {
            let (x_start, x_end, action) = TAB_BAR_REGIONS[i];
            if x >= x_start && x < x_end {
                return action;
            }
        }
        None
    }
}

fn push_region(x_start: u16, x_end: u16, action: TabBarAction) {
    unsafe {
        let idx = TAB_BAR_REGION_COUNT;
        if idx < MAX_REGIONS {
            TAB_BAR_REGIONS[idx] = (x_start, x_end, Some(action));
            TAB_BAR_REGION_COUNT = idx + 1;
        }
    }
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    unsafe {
        TAB_BAR_REGION_COUNT = 0;
        TAB_BAR_Y = area.y;
    }

    let active_style = Style::default()
        .fg(Color::White)
        .bg(Color::Rgb(50, 50, 70))
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default()
        .fg(Color::Rgb(140, 140, 160))
        .bg(Color::Rgb(30, 30, 40));
    let close_style = Style::default()
        .fg(Color::Rgb(200, 80, 80))
        .bg(Color::Rgb(30, 30, 40));
    let close_active_style = Style::default()
        .fg(Color::Rgb(200, 80, 80))
        .bg(Color::Rgb(50, 50, 70));
    let sep_style = Style::default()
        .fg(Color::Rgb(60, 60, 70))
        .bg(Color::Rgb(30, 30, 40));
    let bg_style = Style::default().bg(Color::Rgb(30, 30, 40));

    let mut spans: Vec<Span> = Vec::new();
    let mut x_pos: u16 = area.x;

    for (i, tab) in app.tabs.iter().enumerate() {
        let is_active = i == app.active_tab;
        let style = if is_active {
            active_style
        } else {
            inactive_style
        };
        let cls = if is_active {
            close_active_style
        } else {
            close_style
        };

        // Separator between tabs
        if i > 0 {
            let sep = " \u{2502} ";
            let sep_width = unicode_display_width(sep);
            spans.push(Span::styled(sep, sep_style));
            x_pos += sep_width;
        }

        // Tab title
        let title = tab.title();
        let modified = if tab.has_unsaved_changes { "*" } else { "" };
        let label = format!(" {}{} ", modified, title);
        let label_width = unicode_display_width(&label);

        // Register hit region for tab switch
        push_region(x_pos, x_pos + label_width, TabBarAction::SwitchTab(i));
        spans.push(Span::styled(label, style));
        x_pos += label_width;

        // Close button [x]
        let close_label = "[x]";
        let close_width = unicode_display_width(close_label);
        push_region(x_pos, x_pos + close_width, TabBarAction::CloseTab(i));
        spans.push(Span::styled(close_label, cls));
        x_pos += close_width;
    }

    let line = Line::from(spans);
    let bar = Paragraph::new(line).style(bg_style);
    f.render_widget(bar, area);
}

fn unicode_display_width(s: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    s.width() as u16
}
