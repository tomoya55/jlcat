mod app;
mod input;
mod view;

pub use app::App;

use crate::core::{FlatTableData, TableData};
use crate::error::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout, Stdout};
use std::panic;

type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode
fn init_terminal() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to normal mode
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Install panic hook to restore terminal on panic
fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}

/// Run the TUI application
pub fn run(table_data: TableData) -> Result<()> {
    install_panic_hook();

    let mut terminal = init_terminal().map_err(crate::error::JlcatError::Io)?;

    let mut app = App::new(table_data);
    let result = run_event_loop(&mut terminal, &mut app);

    restore_terminal().map_err(crate::error::JlcatError::Io)?;

    result
}

/// Run the TUI application with flat mode data
pub fn run_flat(flat_data: FlatTableData) -> Result<()> {
    install_panic_hook();

    let mut terminal = init_terminal().map_err(crate::error::JlcatError::Io)?;

    let mut app = App::from_flat(flat_data);
    let result = run_event_loop(&mut terminal, &mut app);

    restore_terminal().map_err(crate::error::JlcatError::Io)?;

    result
}

/// Main event loop
fn run_event_loop(terminal: &mut Tui, app: &mut App) -> Result<()> {
    loop {
        terminal
            .draw(|frame| view::render(frame, app))
            .map_err(crate::error::JlcatError::Io)?;

        if let Event::Key(key) = event::read().map_err(crate::error::JlcatError::Io)? {
            if key.kind == KeyEventKind::Press {
                match input::handle_key(app, key.code) {
                    input::Action::Quit => break,
                    input::Action::Continue => {}
                }
            }
        }
    }

    Ok(())
}
