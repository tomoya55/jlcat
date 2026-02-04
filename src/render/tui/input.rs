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
        InputMode::Detail => handle_detail_mode(app, key),
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

        // Detail view
        KeyCode::Enter => {
            if let Some(source) = app.get_selected_source() {
                let pretty = serde_json::to_string_pretty(source).unwrap_or_default();
                let total_lines = pretty.lines().count();
                app.enter_detail_mode(total_lines);
            }
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

fn handle_detail_mode(app: &mut App, key: KeyCode) -> Action {
    match key {
        // Close modal
        KeyCode::Esc => {
            app.exit_detail_mode();
            Action::Continue
        }

        // Quit app
        KeyCode::Char('q') => Action::Quit,

        // Scroll up
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(state) = app.detail_state_mut() {
                state.scroll_up(1);
            }
            Action::Continue
        }

        // Scroll down
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(state) = app.detail_state_mut() {
                state.scroll_down(1);
            }
            Action::Continue
        }

        // Page up
        KeyCode::PageUp | KeyCode::Char('b') => {
            if let Some(state) = app.detail_state_mut() {
                state.scroll_up(10);
            }
            Action::Continue
        }

        // Page down
        KeyCode::PageDown | KeyCode::Char(' ') => {
            if let Some(state) = app.detail_state_mut() {
                state.scroll_down(10);
            }
            Action::Continue
        }

        // Go to top
        KeyCode::Char('g') => {
            if let Some(state) = app.detail_state_mut() {
                state.go_to_top();
            }
            Action::Continue
        }

        // Go to bottom
        KeyCode::Char('G') => {
            if let Some(state) = app.detail_state_mut() {
                state.go_to_bottom();
            }
            Action::Continue
        }

        _ => Action::Continue,
    }
}
