use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{Action, AppMode};

pub fn action_from_key(mode: AppMode, key: KeyEvent) -> Option<Action> {
    match mode {
        AppMode::Navigate => map_navigate_key(key),
        AppMode::Edit | AppMode::Command => map_text_entry_key(key),
    }
}

fn map_navigate_key(key: KeyEvent) -> Option<Action> {
    if !is_actionable_key_event(key.kind) {
        return None;
    }
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
        KeyCode::Char('?') | KeyCode::F(1) => Some(Action::ToggleHelp),
        KeyCode::Char('r') => Some(Action::Recalculate),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}

fn map_text_entry_key(key: KeyEvent) -> Option<Action> {
    if !is_actionable_key_event(key.kind) {
        return None;
    }
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

fn is_actionable_key_event(kind: KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat)
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
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::Char('?'))),
            Some(Action::ToggleHelp)
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
    fn char_release_is_ignored_in_edit_mode() {
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
            None
        );
    }
}
