mod app;
mod diff;
mod events;
mod file_browser;
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
    /// Paths: <left> <right> (files or dirs for 2-way), <left> <base> <right> for 3-way
    files: Vec<PathBuf>,

    /// Output path for merge result (git mergetool use). Ctrl+S writes base panel here.
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode().expect("enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).expect("enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    let mut app = App::new();

    if let Some(out) = cli.output {
        app.output_path = Some(out);
    }

    // Open files/directories if provided via CLI
    match cli.files.len() {
        2 => {
            let left = cli.files[0].clone();
            let right = cli.files[1].clone();
            if left.is_dir() && right.is_dir() {
                app.open_dirs(left, right);
            } else {
                app.active_tab_mut().open_files(left, right);
            }
        }
        3 => {
            let left = cli.files[0].clone();
            let base = cli.files[1].clone();
            let right = cli.files[2].clone();
            app.active_tab_mut().open_files_3way(left, base, right);
        }
        _ => {}
    }

    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode().expect("disable raw mode");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .expect("leave alternate screen");
    terminal.show_cursor().expect("show cursor");

    // Exit code: 1 if mergetool mode but user quit without saving
    let exit_code = match result {
        Err(_) => 1,
        Ok(()) if app.output_path.is_some() && !app.output_saved => 1,
        Ok(()) => 0,
    };
    std::process::exit(exit_code);
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
