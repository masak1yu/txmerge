use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::time::Duration;

use crate::app::{App, AppMode, PanelSide};
use crate::file_browser::FileBrowser;
use crate::ui::{self, menu_bar, tab_bar};

pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        match event::read()? {
            Event::Key(key) => match app.mode {
                AppMode::Normal => handle_normal_mode(app, key),
                AppMode::Editing => handle_editing_mode(app, key),
                AppMode::OpenLeft | AppMode::OpenRight | AppMode::OpenBase => {
                    handle_file_browser_mode(app, key)
                }
                AppMode::SaveLeft | AppMode::SaveRight => handle_save_browser_mode(app, key),
                AppMode::OpenChooseMode => handle_choose_mode(app, key),
                AppMode::SaveConfirm => handle_save_confirm(app, key),
                AppMode::CloseTabConfirm => handle_close_tab_confirm(app, key),
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    handle_mouse_click(app, mouse.column, mouse.row);
                }
                MouseEventKind::ScrollUp => {
                    if app.active_tab().is_dir_compare {
                        for _ in 0..3 { app.dir_prev(); }
                    } else {
                        app.scroll_up(3);
                    }
                }
                MouseEventKind::ScrollDown => {
                    if app.active_tab().is_dir_compare {
                        for _ in 0..3 { app.dir_next(); }
                    } else {
                        app.scroll_down(3);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(app.should_quit)
}

fn handle_mouse_click(app: &mut App, x: u16, y: u16) {
    // Check dialog close button first (any dialog mode)
    if !matches!(app.mode, AppMode::Normal | AppMode::Editing) {
        if let Some(rect) = ui::dialog_close_rect() {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                app.mode = AppMode::Normal;
                app.file_browser = None;
                return;
            }
        }
    }

    // File browser list click
    if matches!(
        app.mode,
        AppMode::OpenLeft
            | AppMode::OpenRight
            | AppMode::OpenBase
            | AppMode::SaveLeft
            | AppMode::SaveRight
    ) {
        if let Some(rect) = ui::file_browser_list_rect() {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                let row = (y - rect.y) as usize;
                if let Some(ref mut browser) = app.file_browser {
                    let clicked_idx = browser.scroll_offset + row;
                    if clicked_idx < browser.entries.len() {
                        if browser.selected == clicked_idx {
                            // Click on already-selected item -> enter/select
                            if browser.is_save_mode() {
                                let entry = &browser.entries[clicked_idx];
                                if entry.is_dir {
                                    browser.enter();
                                } else {
                                    // Put filename into input
                                    browser.filename_input = Some(entry.name.clone());
                                }
                            } else {
                                let selected_file = browser.enter();
                                if let Some(path) = selected_file {
                                    file_browser_select(app, path);
                                }
                            }
                        } else {
                            browser.selected = clicked_idx;
                        }
                    }
                }
                return;
            }
        }
    }

    // Dir compare list click
    if app.active_tab().is_dir_compare {
        if let Some(rect) = ui::dir_view::dir_list_rect() {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                let header_rows = 2u16; // header + separator
                if y >= rect.y + header_rows {
                    let row = (y - rect.y - header_rows) as usize;
                    // Compute display scroll_offset matching the draw function's logic
                    let list_capacity = (rect.height as usize).saturating_sub(header_rows as usize);
                    let scroll_offset = if let Some(ref r) = app.active_tab().dir_result {
                        if r.selected < r.scroll_offset {
                            r.selected
                        } else if list_capacity > 0 && r.selected >= r.scroll_offset + list_capacity {
                            r.selected + 1 - list_capacity
                        } else {
                            r.scroll_offset
                        }
                    } else { 0 };
                    let clicked = scroll_offset + row;
                    if let Some(ref mut r) = app.active_tab_mut().dir_result {
                        if clicked < r.entries.len() {
                            r.selected = clicked;
                        }
                    }
                }
                return;
            }
        }
    }

    // Tab bar click
    if app.tabs.len() > 1 {
        if let Some(action) = tab_bar::hit_test(x, y) {
            match action {
                tab_bar::TabBarAction::SwitchTab(idx) => {
                    app.switch_tab(idx);
                }
                tab_bar::TabBarAction::CloseTab(idx) => {
                    app.switch_tab(idx);
                    if app.active_tab().has_unsaved_changes {
                        app.mode = AppMode::CloseTabConfirm;
                    } else {
                        app.close_tab();
                    }
                }
            }
            return;
        }
    }

    // Panel click -- enter or move edit cursor
    if matches!(app.mode, AppMode::Normal | AppMode::Editing) {
        if let Some((panel, display_line, col)) = hit_test_panel(app, x, y) {
            if app.mode == AppMode::Editing {
                // Already editing -- exit current edit (recomputes diff if dirty),
                // then re-enter at the new position
                app.exit_edit_mode();
                app.enter_edit_mode(panel, display_line, col);
            } else {
                app.enter_edit_mode(panel, display_line, col);
            }
            return;
        }

        // Click outside panels while editing -> exit edit mode
        if app.mode == AppMode::Editing {
            app.exit_edit_mode();
            // Fall through to menu bar check
        }
    }

    // Menu bar click (row 0) -- works in all modes
    if y == 0 {
        if let Some(action) = menu_bar::hit_test(x) {
            if app.mode == AppMode::Normal {
                execute_menu_action(app, action);
            }
        }
    }
}

/// Hit-test: check if (x, y) is inside a diff panel. Returns (panel, display_line, col).
fn hit_test_panel(app: &App, x: u16, y: u16) -> Option<(PanelSide, usize, usize)> {
    use crate::ui::{diff_view, three_way_view};

    let tab = app.active_tab();
    let line_no_width = 5u16;

    if tab.is_three_way {
        for (panel, rect_fn) in [
            (
                PanelSide::Left,
                three_way_view::left_panel_rect as fn() -> Option<ratatui::layout::Rect>,
            ),
            (
                PanelSide::Base,
                three_way_view::base_panel_rect as fn() -> Option<ratatui::layout::Rect>,
            ),
            (
                PanelSide::Right,
                three_way_view::right_panel_rect as fn() -> Option<ratatui::layout::Rect>,
            ),
        ] {
            if let Some(rect) = rect_fn() {
                if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
                {
                    let row = (y - rect.y) as usize;
                    let display_line = tab.scroll_offset + row;
                    let col = if x >= rect.x + line_no_width {
                        (x - rect.x - line_no_width) as usize
                    } else {
                        0
                    };
                    return Some((panel, display_line, col));
                }
            }
        }
    } else {
        for (panel, rect_fn) in [
            (
                PanelSide::Left,
                diff_view::left_panel_rect as fn() -> Option<ratatui::layout::Rect>,
            ),
            (
                PanelSide::Right,
                diff_view::right_panel_rect as fn() -> Option<ratatui::layout::Rect>,
            ),
        ] {
            if let Some(rect) = rect_fn() {
                if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
                {
                    let row = (y - rect.y) as usize;
                    let display_line = tab.scroll_offset + row;
                    let col = if x >= rect.x + line_no_width {
                        (x - rect.x - line_no_width) as usize
                    } else {
                        0
                    };
                    return Some((panel, display_line, col));
                }
            }
        }
    }
    None
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    // Dir compare mode: delegate entirely
    if app.active_tab().is_dir_compare {
        handle_dir_compare_mode(app, key);
        return;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        // === Quit: Ctrl+Q ===
        KeyCode::Char('q') if ctrl => {
            if app.any_unsaved() {
                app.mode = AppMode::SaveConfirm;
            } else {
                app.should_quit = true;
            }
        }

        // === Tab management ===
        // New tab: Ctrl+T
        KeyCode::Char('t') if ctrl => {
            app.new_tab();
        }
        // Close tab: Ctrl+W
        KeyCode::Char('w') if ctrl => {
            if app.tabs.len() > 1 {
                if app.active_tab().has_unsaved_changes {
                    app.mode = AppMode::CloseTabConfirm;
                } else {
                    app.close_tab();
                }
            }
        }
        // Next tab: Ctrl+PageDown
        KeyCode::PageDown if ctrl => app.next_tab(),
        // Prev tab: Ctrl+PageUp
        KeyCode::PageUp if ctrl => app.prev_tab(),

        // === File operations ===
        // Open: Ctrl+O, o
        KeyCode::Char('o') if ctrl => {
            app.mode = AppMode::OpenChooseMode;
        }
        KeyCode::Char('o') => {
            app.mode = AppMode::OpenChooseMode;
        }
        // Save: Ctrl+S
        KeyCode::Char('s') if ctrl => {
            app.save_files();
        }
        // Refresh: F5, Ctrl+R
        KeyCode::F(5) => refresh_files(app),
        KeyCode::Char('r') if ctrl => refresh_files(app),

        // === Undo/Redo: Ctrl+Z, Ctrl+Y ===
        KeyCode::Char('z') if ctrl => app.undo(),
        KeyCode::Char('y') if ctrl => app.redo(),

        // === Diff navigation (WinMerge compatible) ===
        // Next diff: F8
        KeyCode::F(8) if !shift => app.next_diff(),
        // Prev diff: F7
        KeyCode::F(7) if !shift => app.prev_diff(),
        // Next conflict: Shift+F8 (3-way)
        KeyCode::F(8) if shift => app.next_diff(), // TODO: conflict-only nav
        // Prev conflict: Shift+F7 (3-way)
        KeyCode::F(7) if shift => app.prev_diff(), // TODO: conflict-only nav
        // First diff: Alt+Home
        KeyCode::Home if alt => app.first_diff(),
        // Last diff: Alt+End
        KeyCode::End if alt => app.last_diff(),
        // Also keep Ctrl+Home/End as alias
        KeyCode::Home if ctrl => app.first_diff(),
        KeyCode::End if ctrl => app.last_diff(),
        // Vim-style: n/p
        KeyCode::Char('n') => app.next_diff(),
        KeyCode::Char('p') => app.prev_diff(),

        // === Scroll ===
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(1),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(1),
        KeyCode::Right if !ctrl && !alt => app.h_scroll_right(4),
        KeyCode::Left if !ctrl && !alt => app.h_scroll_left(4),
        KeyCode::PageDown => app.scroll_down(20),
        KeyCode::PageUp => app.scroll_up(20),
        KeyCode::Char('g') => {
            app.active_tab_mut().scroll_offset = 0;
        }
        KeyCode::Char('G') => {
            let total = app.total_lines();
            if total > 0 {
                app.active_tab_mut().scroll_offset = total - 1;
            }
        }

        // === Copy operations (WinMerge: Alt+Arrow) ===
        // Copy L->R: Alt+Right
        KeyCode::Right if alt && !ctrl => app.copy_left_to_right(),
        // Copy R->L: Alt+Left
        KeyCode::Left if alt && !ctrl => app.copy_right_to_left(),
        // Copy L->R + next: Ctrl+Alt+Right
        KeyCode::Right if ctrl && alt => app.copy_left_to_right_and_next(),
        // Copy R->L + next: Ctrl+Alt+Left
        KeyCode::Left if ctrl && alt => app.copy_right_to_left_and_next(),
        // Copy L->R + next: Ctrl+Right (alias)
        KeyCode::Right if ctrl => app.copy_left_to_right_and_next(),
        // Copy R->L + next: Ctrl+Left (alias)
        KeyCode::Left if ctrl => app.copy_right_to_left_and_next(),

        // === Options ===
        // Toggle whitespace ignore: F9
        KeyCode::F(9) => app.toggle_ignore_whitespace(),
        // Toggle case ignore: Ctrl+I
        KeyCode::Char('i') if ctrl => app.toggle_ignore_case(),

        // Enter edit mode
        KeyCode::Char('i') => {
            // Enter edit mode on left panel at current scroll position
            let display_line = app.active_tab().scroll_offset;
            app.enter_edit_mode(PanelSide::Left, display_line, 0);
        }
        KeyCode::Enter => {
            let display_line = app.active_tab().scroll_offset;
            app.enter_edit_mode(PanelSide::Left, display_line, 0);
        }

        _ => {}
    }
}

fn handle_dir_compare_mode(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt  = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        // Quit
        KeyCode::Char('q') if ctrl => {
            app.should_quit = true;
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        // Tab management
        KeyCode::Char('t') if ctrl => app.new_tab(),
        KeyCode::Char('w') if ctrl => {
            if app.tabs.len() > 1 {
                app.close_tab();
            }
        }
        KeyCode::PageDown if ctrl => app.next_tab(),
        KeyCode::PageUp if ctrl => app.prev_tab(),
        // Vertical navigation
        KeyCode::Down | KeyCode::Char('j') => app.dir_next(),
        KeyCode::Up | KeyCode::Char('k') => app.dir_prev(),
        // Horizontal scroll
        KeyCode::Right if !ctrl && !alt => app.h_scroll_right(4),
        KeyCode::Left  if !ctrl && !alt => app.h_scroll_left(4),
        // Open selected entry
        KeyCode::Enter => app.dir_open_selected(),
        _ => {}
    }
}

fn refresh_files(app: &mut App) {
    // If in editing mode, commit current edit state first
    let was_editing = app.mode == AppMode::Editing;
    if was_editing {
        app.exit_edit_mode();
    }

    let editing_dirty = app.active_tab().has_unsaved_changes
        || app
            .active_tab()
            .edit_state
            .as_ref()
            .map(|e| e.dirty)
            .unwrap_or(false)
        || !app.undo_stack_is_empty();

    if editing_dirty {
        // Unsaved edits exist -- recompute diff from in-memory text, never reload from disk
        app.recompute_diff();
        app.set_status("Re-diffed (unsaved changes preserved)");
    } else if app.active_tab().is_three_way {
        let left = app.active_tab().left_path.clone();
        let base = app.active_tab().base_path.clone();
        let right = app.active_tab().right_path.clone();
        if let (Some(left), Some(base), Some(right)) = (left, base, right) {
            app.open_files_3way(left, base, right);
            app.set_status("Refreshed from disk");
        }
    } else {
        let left = app.active_tab().left_path.clone();
        let right = app.active_tab().right_path.clone();
        if let (Some(left), Some(right)) = (left, right) {
            app.open_files(left, right);
            app.set_status("Refreshed from disk");
        }
    }
}

fn handle_editing_mode(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Esc => app.exit_edit_mode(),
        KeyCode::Left if !ctrl && !alt => app.edit_move_left(),
        KeyCode::Right if !ctrl && !alt => app.edit_move_right(),
        KeyCode::Up => app.edit_move_up(),
        KeyCode::Down => app.edit_move_down(),
        KeyCode::Home => app.edit_move_home(),
        KeyCode::End => app.edit_move_end(),
        KeyCode::Backspace => app.edit_backspace(),
        KeyCode::Delete => app.edit_delete(),
        KeyCode::Enter => app.edit_enter(),
        KeyCode::Char('z') if ctrl => {
            app.exit_edit_mode();
            app.undo();
        }
        KeyCode::Char('s') if ctrl => {
            app.exit_edit_mode();
            app.save_files();
        }
        KeyCode::Char(c) if !ctrl && !alt => app.edit_insert_char(c),
        _ => {}
    }
}

fn handle_choose_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('2') => {
            if app.new_file_pending {
                app.new_file_pending = false;
                app.new_tab();
                app.new_blank(false);
                app.mode = AppMode::Normal;
            } else {
                app.active_tab_mut().is_three_way = false;
                app.mode = AppMode::OpenLeft;
                app.file_browser = Some(FileBrowser::new());
            }
        }
        KeyCode::Char('3') => {
            if app.new_file_pending {
                app.new_file_pending = false;
                app.new_tab();
                app.new_blank(true);
                app.mode = AppMode::Normal;
            } else {
                app.active_tab_mut().is_three_way = true;
                app.mode = AppMode::OpenLeft;
                app.file_browser = Some(FileBrowser::new());
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.new_file_pending = false;
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

/// Handle file selection from the browser (shared by keyboard Enter and mouse click)
fn file_browser_select(app: &mut App, path: std::path::PathBuf) {
    match app.mode {
        AppMode::OpenLeft => {
            app.active_tab_mut().left_path = Some(path);
            app.file_browser = Some(FileBrowser::new());
            if app.active_tab().is_three_way {
                app.mode = AppMode::OpenBase;
            } else {
                app.mode = AppMode::OpenRight;
            }
        }
        AppMode::OpenBase => {
            app.active_tab_mut().base_path = Some(path);
            app.file_browser = Some(FileBrowser::new());
            app.mode = AppMode::OpenRight;
        }
        AppMode::OpenRight => {
            let right = path;
            app.file_browser = None;
            app.mode = AppMode::Normal;
            if app.active_tab().is_three_way {
                let left = app.active_tab().left_path.clone().unwrap();
                let base = app.active_tab().base_path.clone().unwrap();
                app.open_files_3way(left, base, right);
            } else {
                let left = app.active_tab().left_path.clone().unwrap();
                app.open_files(left, right);
            }
        }
        _ => {}
    }
}

fn handle_file_browser_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.file_browser = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut browser) = app.file_browser {
                browser.move_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut browser) = app.file_browser {
                browser.move_down();
            }
        }
        KeyCode::PageUp => {
            if let Some(ref mut browser) = app.file_browser {
                browser.page_up(15);
            }
        }
        KeyCode::PageDown => {
            if let Some(ref mut browser) = app.file_browser {
                browser.page_down(15);
            }
        }
        KeyCode::Backspace => {
            if let Some(ref mut browser) = app.file_browser {
                browser.go_parent();
            }
        }
        KeyCode::Enter => {
            let selected_file = app
                .file_browser
                .as_mut()
                .and_then(|browser| browser.enter());

            if let Some(path) = selected_file {
                file_browser_select(app, path);
            }
        }
        _ => {}
    }
}

fn handle_save_browser_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.file_browser = None;
        }
        KeyCode::Tab => {
            if let Some(ref mut browser) = app.file_browser {
                let entry = browser.entries.get(browser.selected).cloned();
                if let Some(entry) = entry {
                    if entry.is_dir {
                        browser.enter();
                    } else {
                        browser.filename_input = Some(entry.name.clone());
                    }
                }
            }
        }
        KeyCode::Up => {
            if let Some(ref mut browser) = app.file_browser {
                browser.move_up();
            }
        }
        KeyCode::Down => {
            if let Some(ref mut browser) = app.file_browser {
                browser.move_down();
            }
        }
        KeyCode::PageUp => {
            if let Some(ref mut browser) = app.file_browser {
                browser.page_up(15);
            }
        }
        KeyCode::PageDown => {
            if let Some(ref mut browser) = app.file_browser {
                browser.page_down(15);
            }
        }
        KeyCode::Enter => {
            // Save with current filename
            let save_path = app.file_browser.as_ref().and_then(|b| b.save_path());

            if let Some(path) = save_path {
                match app.mode {
                    AppMode::SaveLeft => {
                        let left_text = app.active_tab().left_text();
                        let _ = std::fs::write(&path, &left_text);
                        app.active_tab_mut().left_path = Some(path);
                        // Continue to save right
                        let right_path = app.active_tab().right_path.clone();
                        let (default_name, start_dir) =
                            App::save_defaults(&right_path, "untitled_right.txt");
                        let mut browser = FileBrowser::new_save(&default_name);
                        browser.current_dir = start_dir;
                        browser.read_dir();
                        app.file_browser = Some(browser);
                        app.mode = AppMode::SaveRight;
                    }
                    AppMode::SaveRight => {
                        let right_text = app.active_tab().right_text();
                        let _ = std::fs::write(&path, &right_text);
                        app.active_tab_mut().right_path = Some(path);
                        app.file_browser = None;
                        app.mode = AppMode::Normal;
                        app.active_tab_mut().has_unsaved_changes = false;
                        app.set_status("Saved");
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(ref mut browser) = app.file_browser {
                if let Some(ref mut fname) = browser.filename_input {
                    fname.pop();
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut browser) = app.file_browser {
                if let Some(ref mut fname) = browser.filename_input {
                    fname.push(c);
                }
            }
        }
        _ => {}
    }
}

fn handle_save_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') => {
            app.save_files();
        }
        KeyCode::Char('d') => {
            app.should_quit = true;
        }
        KeyCode::Char('c') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_close_tab_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') => {
            // Save then close
            app.save_files();
            // After save flow completes, user would need to close again.
            // For now, just go back to normal mode (save dialog takes over).
        }
        KeyCode::Char('d') => {
            // Discard and close
            app.close_tab();
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('c') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    New,
    Open,
    Refresh,
    PrevDiff,
    NextDiff,
    CopyLeftToRight,
    CopyRightToLeft,
    SelectAll,
    ToggleWhitespace,
    ToggleCase,
    Save,
}

fn execute_menu_action(app: &mut App, action: MenuAction) {
    match action {
        MenuAction::New => {
            app.new_file_pending = true;
            app.mode = AppMode::OpenChooseMode;
        }
        MenuAction::Open => {
            app.mode = AppMode::OpenChooseMode;
        }
        MenuAction::Refresh => {
            refresh_files(app);
        }
        MenuAction::Save => {
            app.save_files();
        }
        MenuAction::PrevDiff => app.prev_diff(),
        MenuAction::NextDiff => app.next_diff(),
        MenuAction::CopyLeftToRight => app.copy_left_to_right(),
        MenuAction::CopyRightToLeft => app.copy_right_to_left(),
        MenuAction::SelectAll => app.toggle_select_all(),
        MenuAction::ToggleWhitespace => app.toggle_ignore_whitespace(),
        MenuAction::ToggleCase => app.toggle_ignore_case(),
    }
}
