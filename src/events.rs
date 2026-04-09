use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::time::Duration;

use crate::app::{App, AppMode};
use crate::file_browser::FileBrowser;
use crate::ui::{self, menu_bar};

pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        match event::read()? {
            Event::Key(key) => match app.mode {
                AppMode::Normal => handle_normal_mode(app, key),
                AppMode::OpenLeft | AppMode::OpenRight | AppMode::OpenBase => {
                    handle_file_browser_mode(app, key)
                }
                AppMode::SaveLeft | AppMode::SaveRight => {
                    handle_save_browser_mode(app, key)
                }
                AppMode::OpenChooseMode => handle_choose_mode(app, key),
                AppMode::SaveConfirm => handle_save_confirm(app, key),
            },
            Event::Mouse(mouse) => {
                if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                    handle_mouse_click(app, mouse.column, mouse.row);
                }
            }
            _ => {}
        }
    }
    Ok(app.should_quit)
}

fn handle_mouse_click(app: &mut App, x: u16, y: u16) {
    // Check dialog close button first (any non-Normal mode)
    if app.mode != AppMode::Normal {
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
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
            {
                let row = (y - rect.y) as usize;
                if let Some(ref mut browser) = app.file_browser {
                    let clicked_idx = browser.scroll_offset + row;
                    if clicked_idx < browser.entries.len() {
                        if browser.selected == clicked_idx {
                            // Click on already-selected item → enter/select
                            if browser.is_save_mode() {
                                let entry = &browser.entries[clicked_idx];
                                if entry.is_dir {
                                    browser.enter();
                                } else {
                                    // Put filename into input
                                    browser.filename_input =
                                        Some(entry.name.clone());
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

    // Menu bar click (row 0) — works in all modes
    if y == 0 {
        if let Some(action) = menu_bar::hit_test(x) {
            if app.mode == AppMode::Normal {
                execute_menu_action(app, action);
            }
        }
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        // === Quit: Ctrl+Q ===
        KeyCode::Char('q') if ctrl => {
            if app.has_unsaved_changes {
                app.mode = AppMode::SaveConfirm;
            } else {
                app.should_quit = true;
            }
        }

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
        KeyCode::PageDown => app.scroll_down(20),
        KeyCode::PageUp => app.scroll_up(20),
        KeyCode::Char('g') => app.scroll_offset = 0, // top
        KeyCode::Char('G') => {
            let total = app.total_lines();
            if total > 0 {
                app.scroll_offset = total - 1;
            }
        }

        // === Copy operations (WinMerge: Alt+Arrow) ===
        // Copy L→R: Alt+Right
        KeyCode::Right if alt && !ctrl => app.copy_left_to_right(),
        // Copy R→L: Alt+Left
        KeyCode::Left if alt && !ctrl => app.copy_right_to_left(),
        // Copy L→R + next: Ctrl+Alt+Right
        KeyCode::Right if ctrl && alt => app.copy_left_to_right_and_next(),
        // Copy R→L + next: Ctrl+Alt+Left
        KeyCode::Left if ctrl && alt => app.copy_right_to_left_and_next(),
        // Copy L→R + next: Ctrl+Right (alias)
        KeyCode::Right if ctrl => app.copy_left_to_right_and_next(),
        // Copy R→L + next: Ctrl+Left (alias)
        KeyCode::Left if ctrl => app.copy_right_to_left_and_next(),

        // === Options ===
        // Toggle whitespace ignore: F9
        KeyCode::F(9) => app.toggle_ignore_whitespace(),
        KeyCode::Char('w') if ctrl => app.toggle_ignore_whitespace(),
        // Toggle case ignore: Ctrl+I
        KeyCode::Char('i') if ctrl => app.toggle_ignore_case(),

        _ => {}
    }
}

fn refresh_files(app: &mut App) {
    if app.is_three_way {
        if let (Some(left), Some(base), Some(right)) = (
            app.left_path.clone(),
            app.base_path.clone(),
            app.right_path.clone(),
        ) {
            app.open_files_3way(left, base, right);
            app.set_status("Refreshed");
        }
    } else if let (Some(left), Some(right)) = (app.left_path.clone(), app.right_path.clone()) {
        app.open_files(left, right);
        app.set_status("Refreshed");
    }
}

fn handle_choose_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('2') => {
            app.is_three_way = false;
            app.mode = AppMode::OpenLeft;
            app.file_browser = Some(FileBrowser::new());
        }
        KeyCode::Char('3') => {
            app.is_three_way = true;
            app.mode = AppMode::OpenLeft;
            app.file_browser = Some(FileBrowser::new());
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

/// Handle file selection from the browser (shared by keyboard Enter and mouse click)
fn file_browser_select(app: &mut App, path: std::path::PathBuf) {
    match app.mode {
        AppMode::OpenLeft => {
            app.left_path = Some(path);
            app.file_browser = Some(FileBrowser::new());
            if app.is_three_way {
                app.mode = AppMode::OpenBase;
            } else {
                app.mode = AppMode::OpenRight;
            }
        }
        AppMode::OpenBase => {
            app.base_path = Some(path);
            app.file_browser = Some(FileBrowser::new());
            app.mode = AppMode::OpenRight;
        }
        AppMode::OpenRight => {
            let right = path;
            app.file_browser = None;
            app.mode = AppMode::Normal;
            if app.is_three_way {
                let left = app.left_path.clone().unwrap();
                let base = app.base_path.clone().unwrap();
                app.open_files_3way(left, base, right);
            } else {
                let left = app.left_path.clone().unwrap();
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
            // Tab switches focus to directory navigation
            // Navigate into selected directory or put file name in input
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
            let save_path = app
                .file_browser
                .as_ref()
                .and_then(|b| b.save_path());

            if let Some(path) = save_path {
                match app.mode {
                    AppMode::SaveLeft => {
                        let _ = std::fs::write(&path, &app.left_text);
                        app.left_path = Some(path);
                        // Continue to save right
                        let default_name = app
                            .right_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let mut browser = FileBrowser::new_save(&default_name);
                        if let Some(ref rp) = app.right_path {
                            if let Some(parent) = rp.parent() {
                                browser.current_dir = parent.to_path_buf();
                                browser.read_dir();
                            }
                        }
                        app.file_browser = Some(browser);
                        app.mode = AppMode::SaveRight;
                    }
                    AppMode::SaveRight => {
                        let _ = std::fs::write(&path, &app.right_text);
                        app.right_path = Some(path);
                        app.file_browser = None;
                        app.mode = AppMode::Normal;
                        app.has_unsaved_changes = false;
                        app.set_status("Saved");
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Backspace => {
            // Delete last char from filename input
            if let Some(ref mut browser) = app.file_browser {
                if let Some(ref mut fname) = browser.filename_input {
                    fname.pop();
                }
            }
        }
        KeyCode::Char(c) => {
            // Type into filename input
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

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    New,
    Open,
    Refresh,
    FirstDiff,
    PrevDiff,
    NextDiff,
    LastDiff,
    CopyLeftToRight,
    CopyRightToLeft,
    CopyLeftToRightNext,
    CopyRightToLeftNext,
    CopyAllLR,
    CopyAllRL,
    ToggleWhitespace,
    ToggleCase,
    Save,
}

fn execute_menu_action(app: &mut App, action: MenuAction) {
    match action {
        MenuAction::New => {
            // TODO: New file functionality
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
        MenuAction::FirstDiff => app.first_diff(),
        MenuAction::PrevDiff => app.prev_diff(),
        MenuAction::NextDiff => app.next_diff(),
        MenuAction::LastDiff => app.last_diff(),
        MenuAction::CopyLeftToRight => app.copy_left_to_right(),
        MenuAction::CopyRightToLeft => app.copy_right_to_left(),
        MenuAction::CopyLeftToRightNext => app.copy_left_to_right_and_next(),
        MenuAction::CopyRightToLeftNext => app.copy_right_to_left_and_next(),
        MenuAction::CopyAllLR => app.copy_all_left_to_right(),
        MenuAction::CopyAllRL => app.copy_all_right_to_left(),
        MenuAction::ToggleWhitespace => app.toggle_ignore_whitespace(),
        MenuAction::ToggleCase => app.toggle_ignore_case(),
    }
}
