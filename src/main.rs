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
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

#[derive(Parser)]
#[command(name = "txmerge", version, about = "TUI diff and merge tool")]
struct Cli {
    /// Left file path
    left: Option<PathBuf>,
    /// Right file path
    right: Option<PathBuf>,
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
    if let (Some(left), Some(right)) = (cli.left, cli.right) {
        app.open_files(left, right);
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

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let should_quit = events::handle_events(app)?;
        if should_quit {
            return Ok(());
        }
    }
}
