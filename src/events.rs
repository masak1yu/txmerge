use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::path::PathBuf;
use std::time::Duration;

use crate::app::{App, AppMode};
use crate::ui::menu_bar;

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
                if app.mode == AppMode::Normal {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        if mouse.row == 0 {
                            if let Some(action) = menu_bar::hit_test(mouse.column) {
                                execute_menu_action(app, action);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(app.should_quit)
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.has_unsaved_changes {
                app.mode = AppMode::SaveConfirm;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Char('o') => {
            app.mode = AppMode::OpenChooseMode;
        }
        // Save
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let _ = app.save_files();
        }
        // Undo/Redo
        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => app.undo(),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => app.redo(),
        // Navigation
        KeyCode::Char('n') | KeyCode::F(8) => app.next_diff(),
        KeyCode::Char('p') | KeyCode::F(7) => app.prev_diff(),
        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => app.first_diff(),
        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => app.last_diff(),
        // Scroll
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(1),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(1),
        KeyCode::PageDown => app.scroll_down(20),
        KeyCode::PageUp => app.scroll_up(20),
        // Copy operations
        KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
            app.copy_left_to_right();
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
            app.copy_right_to_left();
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.copy_left_to_right_and_next();
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.copy_right_to_left_and_next();
        }
        // Refresh
        KeyCode::F(5) => {
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
        // Options
        KeyCode::F(9) => app.toggle_ignore_whitespace(),
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_ignore_whitespace();
        }
        KeyCode::Char('i') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_ignore_case();
        }
        _ => {}
    }
}

fn handle_choose_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('2') => {
            app.mode = AppMode::OpenLeft;
            app.input_buffer.clear();
        }
        KeyCode::Char('3') => {
            app.mode = AppMode::OpenLeft;
            app.input_buffer.clear();
            // Mark that we're doing 3-way — OpenLeft → OpenBase → OpenRight
            app.is_three_way = true;
        }
        KeyCode::Esc => {
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
