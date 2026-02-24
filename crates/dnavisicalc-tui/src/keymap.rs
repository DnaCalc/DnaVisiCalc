use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Action, AppMode};

pub fn action_from_key(mode: AppMode, key: KeyEvent) -> Option<Action> {
    match mode {
        AppMode::Navigate => map_navigate_key(key),
        AppMode::Edit | AppMode::Command => map_text_entry_key(key),
    }
}

fn map_navigate_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Left => Some(Action::MoveLeft),
        KeyCode::Right => Some(Action::MoveRight),
        KeyCode::Up => Some(Action::MoveUp),
        KeyCode::Down => Some(Action::MoveDown),
        KeyCode::Char('h') => Some(Action::MoveLeft),
        KeyCode::Char('l') => Some(Action::MoveRight),
        KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Char('e') | KeyCode::Enter => Some(Action::StartEdit),
        KeyCode::Char(':') => Some(Action::StartCommand),
        KeyCode::Char('r') => Some(Action::Recalculate),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

fn map_text_entry_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Enter => Some(Action::Submit),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                None
            } else {
                Some(Action::InputChar(ch))
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_with_kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn maps_navigation_keys() {
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::Left)),
            Some(Action::MoveLeft)
        );
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::Char(':'))),
            Some(Action::StartCommand)
        );
    }

    #[test]
    fn maps_edit_keys() {
        assert_eq!(
            action_from_key(AppMode::Edit, key(KeyCode::Backspace)),
            Some(Action::Backspace)
        );
        assert_eq!(
            action_from_key(AppMode::Command, key(KeyCode::Enter)),
            Some(Action::Submit)
        );
    }

    #[test]
    fn char_press_and_release_both_map_to_input_char_in_edit_mode() {
        // Repro scaffold: if terminal emits both press and release char events,
        // the current mapping will produce duplicated characters.
        assert_eq!(
            action_from_key(
                AppMode::Edit,
                key_with_kind(KeyCode::Char('1'), KeyEventKind::Press)
            ),
            Some(Action::InputChar('1'))
        );
        assert_eq!(
            action_from_key(
                AppMode::Edit,
                key_with_kind(KeyCode::Char('1'), KeyEventKind::Release)
            ),
            Some(Action::InputChar('1'))
        );
    }
}
