mod app;
mod diff;
mod events;
mod models;
mod ui;

use std::io;
use std::path::PathBuf;

use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;

#[derive(Parser)]
#[command(name = "txmerge", version, about = "TUI diff and merge tool")]
struct Cli {
    /// File paths: <left> <right> for 2-way, <left> <base> <right> for 3-way
    files: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Open files if provided via CLI
    match cli.files.len() {
        2 => {
            let left = cli.files[0].clone();
            let right = cli.files[1].clone();
            app.open_files(left, right);
        }
        3 => {
            let left = cli.files[0].clone();
            let base = cli.files[1].clone();
            let right = cli.files[2].clone();
            app.open_files_3way(left, base, right);
        }
        _ => {} // No files — start with blank screen
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let should_quit = events::handle_events(app)?;
        if should_quit {
            return Ok(());
        }
    }
}
