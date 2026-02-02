use super::app::{App, InputMode};
use crossterm::event::KeyCode;

pub enum Action {
    Continue,
    Quit,
}

pub fn handle_key(app: &mut App, key: KeyCode) -> Action {
    match app.mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Search | InputMode::Filter => handle_input_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyCode) -> Action {
    match key {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
            Action::Continue
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
            Action::Continue
        }
        KeyCode::PageUp | KeyCode::Char('b') => {
            app.page_up(10);
            Action::Continue
        }
        KeyCode::PageDown | KeyCode::Char(' ') => {
            app.page_down(10);
            Action::Continue
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.go_to_top();
            Action::Continue
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.go_to_bottom();
            Action::Continue
        }

        // Search
        KeyCode::Char('/') => {
            app.enter_search_mode();
            Action::Continue
        }

        // Filter
        KeyCode::Char('f') => {
            app.enter_filter_mode();
            Action::Continue
        }

        // Clear filters
        KeyCode::Char('c') => {
            app.clear_filters();
            Action::Continue
        }

        _ => Action::Continue,
    }
}

fn handle_input_mode(app: &mut App, key: KeyCode) -> Action {
    match key {
        KeyCode::Enter => {
            app.confirm_input();
            Action::Continue
        }
        KeyCode::Esc => {
            app.cancel_input();
            Action::Continue
        }
        KeyCode::Backspace => {
            app.input_backspace();
            Action::Continue
        }
        KeyCode::Char(c) => {
            app.input_char(c);
            Action::Continue
        }
        _ => Action::Continue,
    }
}
