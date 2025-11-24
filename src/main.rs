mod app;
mod file_operations;
mod handlers;
mod localization;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::io;
use tui::{Terminal, backend::CrosstermBackend};

use app::{App, AppState};
use handlers::handle_events;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initiate app
    let mut app = App::new()?;

    // Run app
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal properly
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = result {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while app.state != AppState::Exiting {
        terminal.draw(|f| ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
            handle_events(app, key)?;
        }

        // Check if we need to hide the save notification
        app.check_notification_timeout();
    }

    Ok(())
}
