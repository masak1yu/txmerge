use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::path::PathBuf;
use std::time::Duration;

use crate::app::{App, AppMode};
use crate::ui::{self, menu_bar};

pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        match event::read()? {
            Event::Key(key) => match app.mode {
                AppMode::Normal => handle_normal_mode(app, key),
                AppMode::OpenLeft | AppMode::OpenRight | AppMode::OpenBase => {
                    handle_input_mode(app, key)
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
                app.input_buffer.clear();
                return;
            }
        }
    }

    // Menu bar click (row 0) — works in all modes
    if y == 0 {
        if let Some(action) = menu_bar::hit_test(x) {
            // Only execute menu actions in Normal mode
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
        KeyCode::Char('o') if ctrl => { app.mode = AppMode::OpenChooseMode; }
        KeyCode::Char('o') => { app.mode = AppMode::OpenChooseMode; }
        // Save: Ctrl+S
        KeyCode::Char('s') if ctrl => { let _ = app.save_files(); }
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
            if total > 0 { app.scroll_offset = total - 1; }
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
        if let (Some(left), Some(base), Some(right)) =
            (app.left_path.clone(), app.base_path.clone(), app.right_path.clone())
        {
            app.open_files_3way(left, base, right);
        }
    } else if let (Some(left), Some(right)) =
        (app.left_path.clone(), app.right_path.clone())
    {
        app.open_files(left, right);
    }
}

fn handle_choose_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('2') => {
            app.is_three_way = false;
            app.mode = AppMode::OpenLeft;
            app.input_buffer.clear();
        }
        KeyCode::Char('3') => {
            app.is_three_way = true;
            app.mode = AppMode::OpenLeft;
            app.input_buffer.clear();
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

fn handle_input_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let path = app.input_buffer.trim().to_string();
            if !path.is_empty() {
                match app.mode {
                    AppMode::OpenLeft => {
                        app.left_path = Some(PathBuf::from(&path));
                        app.input_buffer.clear();
                        if app.is_three_way {
                            app.mode = AppMode::OpenBase;
                        } else {
                            app.mode = AppMode::OpenRight;
                        }
                    }
                    AppMode::OpenBase => {
                        app.base_path = Some(PathBuf::from(&path));
                        app.input_buffer.clear();
                        app.mode = AppMode::OpenRight;
                    }
                    AppMode::OpenRight => {
                        let right = PathBuf::from(&path);
                        app.input_buffer.clear();
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
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}

fn handle_save_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('s') => {
            let _ = app.save_files();
            app.should_quit = true;
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
        MenuAction::Open => {
            app.mode = AppMode::OpenChooseMode;
        }
        MenuAction::Refresh => {
            if app.is_three_way {
                if let (Some(left), Some(base), Some(right)) =
                    (app.left_path.clone(), app.base_path.clone(), app.right_path.clone())
                {
                    app.open_files_3way(left, base, right);
                }
            } else if let (Some(left), Some(right)) =
                (app.left_path.clone(), app.right_path.clone())
            {
                app.open_files(left, right);
            }
        }
        MenuAction::Save => {
            let _ = app.save_files();
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
