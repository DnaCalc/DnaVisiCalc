use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{Action, AppMode};

pub fn action_from_key(mode: AppMode, key: KeyEvent) -> Option<Action> {
    match mode {
        AppMode::Navigate => map_navigate_key(key),
        AppMode::Edit | AppMode::Command => map_text_entry_key(key),
        AppMode::PasteSpecial => map_paste_special_key(key),
    }
}

fn map_navigate_key(key: KeyEvent) -> Option<Action> {
    if !is_actionable_key_event(key.kind) {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(ch) = key.code {
            return match ch.to_ascii_lowercase() {
                'c' => Some(Action::CopySelection),
                'v' => Some(Action::PasteFromClipboard),
                _ => None,
            };
        }
    }
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Left => Some(if shift {
            Action::ExtendLeft
        } else {
            Action::MoveLeft
        }),
        KeyCode::Right => Some(if shift {
            Action::ExtendRight
        } else {
            Action::MoveRight
        }),
        KeyCode::Up => Some(if shift {
            Action::ExtendUp
        } else {
            Action::MoveUp
        }),
        KeyCode::Down => Some(if shift {
            Action::ExtendDown
        } else {
            Action::MoveDown
        }),
        KeyCode::Char('h') => Some(Action::MoveLeft),
        KeyCode::Char('l') => Some(Action::MoveRight),
        KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Char('H') => Some(Action::ExtendLeft),
        KeyCode::Char('L') => Some(Action::ExtendRight),
        KeyCode::Char('K') => Some(Action::ExtendUp),
        KeyCode::Char('J') => Some(Action::ExtendDown),
        KeyCode::Char('e') | KeyCode::Enter | KeyCode::F(2) => Some(Action::StartEdit),
        KeyCode::Char(':') => Some(Action::StartCommand),
        KeyCode::Char('?') | KeyCode::F(1) => Some(Action::ToggleHelp),
        KeyCode::Char('r') => Some(Action::Recalculate),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Delete | KeyCode::Backspace => Some(Action::ClearSelection),
        KeyCode::Esc => Some(Action::Cancel),
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

fn map_paste_special_key(key: KeyEvent) -> Option<Action> {
    if !is_actionable_key_event(key.kind) {
        return None;
    }
    match key.code {
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Enter => Some(Action::Submit),
        KeyCode::Up | KeyCode::Left => Some(Action::PasteModePrev),
        KeyCode::Down | KeyCode::Right | KeyCode::Tab => Some(Action::PasteModeNext),
        KeyCode::BackTab => Some(Action::PasteModePrev),
        KeyCode::Char('k') => Some(Action::PasteModePrev),
        KeyCode::Char('j') => Some(Action::PasteModeNext),
        KeyCode::Char(ch) if ('1'..='5').contains(&ch) => Some(Action::InputChar(ch)),
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

    fn key_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
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
            action_from_key(
                AppMode::Navigate,
                key_with_modifiers(KeyCode::Left, KeyModifiers::SHIFT)
            ),
            Some(Action::ExtendLeft)
        );
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::Char(':'))),
            Some(Action::StartCommand)
        );
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::Char('?'))),
            Some(Action::ToggleHelp)
        );
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::Delete)),
            Some(Action::ClearSelection)
        );
        assert_eq!(
            action_from_key(AppMode::Navigate, key(KeyCode::F(2))),
            Some(Action::StartEdit)
        );
    }

    #[test]
    fn maps_control_clipboard_keys_in_navigation_mode() {
        assert_eq!(
            action_from_key(
                AppMode::Navigate,
                key_with_modifiers(KeyCode::Char('c'), KeyModifiers::CONTROL)
            ),
            Some(Action::CopySelection)
        );
        assert_eq!(
            action_from_key(
                AppMode::Navigate,
                key_with_modifiers(KeyCode::Char('v'), KeyModifiers::CONTROL)
            ),
            Some(Action::PasteFromClipboard)
        );
    }

    #[test]
    fn maps_paste_special_mode_keys() {
        assert_eq!(
            action_from_key(AppMode::PasteSpecial, key(KeyCode::Down)),
            Some(Action::PasteModeNext)
        );
        assert_eq!(
            action_from_key(AppMode::PasteSpecial, key(KeyCode::Up)),
            Some(Action::PasteModePrev)
        );
        assert_eq!(
            action_from_key(AppMode::PasteSpecial, key(KeyCode::Tab)),
            Some(Action::PasteModeNext)
        );
        assert_eq!(
            action_from_key(AppMode::PasteSpecial, key(KeyCode::Char('3'))),
            Some(Action::InputChar('3'))
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
