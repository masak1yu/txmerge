use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, AppMode};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let tab = app.active_tab();
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

    let count = tab.diff_count();
    let diff_info = if tab.is_dir_compare {
        if let Some(ref r) = tab.dir_result {
            use crate::models::diff_line::DirEntryStatus;
            let changed = r.entries.iter().filter(|e| e.status == DirEntryStatus::Changed).count();
            let left_only = r.entries.iter().filter(|e| e.status == DirEntryStatus::LeftOnly).count();
            let right_only = r.entries.iter().filter(|e| e.status == DirEntryStatus::RightOnly).count();
            format!(
                "Dir compare | Changed:{} LeftOnly:{} RightOnly:{} | {}/{} | Enter:open",
                changed, left_only, right_only,
                r.selected + 1,
                r.entries.len()
            )
        } else {
            "Directory compare".to_string()
        }
    } else if tab.is_three_way {
        if let Some(ref result) = tab.three_way_result {
            if result.diff_positions.is_empty() {
                "Files are identical".to_string()
            } else {
                format!(
                    "3-way | Diffs: {} | Conflicts: {} | Current: {}/{}",
                    result.diff_positions.len(),
                    result.conflict_count,
                    if tab.current_diff >= 0 {
                        tab.current_diff + 1
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
        if tab.diff_result.is_some() {
            "Files are identical".to_string()
        } else {
            "No files loaded".to_string()
        }
    } else {
        format!(
            "Diffs: {} | Current: {}/{}",
            count,
            if tab.current_diff >= 0 {
                tab.current_diff + 1
            } else {
                0
            },
            count
        )
    };

    let unsaved = if tab.has_unsaved_changes {
        " [modified]"
    } else {
        ""
    };

    let ws = if tab.diff_options.ignore_whitespace {
        " [WS:ignore]"
    } else {
        ""
    };
    let case = if tab.diff_options.ignore_case {
        " [Case:ignore]"
    } else {
        ""
    };
    let editing_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Rgb(100, 150, 220));

    let status_msg = app.current_status_message().map(|s| s.to_string());

    let line = if app.mode == AppMode::Editing {
        let panel_name = tab
            .edit_state
            .as_ref()
            .map(|e| format!("{:?}", e.panel))
            .unwrap_or_default();
        let pos = tab
            .edit_state
            .as_ref()
            .map(|e| format!(" Ln {}, Col {}", e.source_line + 1, e.cursor_col + 1))
            .unwrap_or_default();
        Line::from(vec![
            Span::styled(" -- EDITING -- ", editing_style),
            Span::styled(format!(" {} ", panel_name), bg),
            Span::styled(pos, bg),
            Span::styled("  Esc:exit ^S:save ^Z:undo", dim),
        ])
    } else if let Some(msg) = status_msg {
        Line::from(vec![Span::styled(" ", bg), Span::styled(msg, green)])
    } else {
        let keys = if tab.is_dir_compare {
            " ↑↓/jk:navigate  Enter:open  ^T:new  ^W:close  ^Q:quit"
        } else {
            " ^O:open i:edit F7/F8:diff Alt+<->:copy ^S:save ^Z:undo ^T:new ^W:close ^Q:quit"
        };
        Line::from(vec![
            Span::styled(" ", bg),
            Span::styled(diff_info, bg),
            Span::styled(unsaved, yellow),
            Span::styled(ws, green),
            Span::styled(case, green),
            Span::styled(keys, dim),
        ])
    };

    let bar = Paragraph::new(line);
    f.render_widget(bar, area);
}
