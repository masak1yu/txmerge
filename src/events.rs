use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use std::time::Duration;

use crate::app::{App, AppMode};

pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            match app.mode {
                AppMode::Normal => handle_normal_mode(app, key),
                AppMode::OpenLeft | AppMode::OpenRight => handle_input_mode(app, key),
            }
        }
    }
    Ok(app.should_quit)
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('o') => {
            app.mode = AppMode::OpenLeft;
            app.input_buffer.clear();
        }
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
            if app.left_path.is_some() && app.right_path.is_some() {
                let left = app.left_path.clone().unwrap();
                let right = app.right_path.clone().unwrap();
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
                        app.mode = AppMode::OpenRight;
                    }
                    AppMode::OpenRight => {
                        app.right_path = Some(PathBuf::from(&path));
                        let left = app.left_path.clone().unwrap();
                        let right = PathBuf::from(&path);
                        app.input_buffer.clear();
                        app.mode = AppMode::Normal;
                        app.open_files(left, right);
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
