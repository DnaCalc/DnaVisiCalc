use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use dnavisicalc_tui::{Action, AppMode, action_from_key};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn unknown_key_in_navigation_is_none() {
    let action = action_from_key(AppMode::Navigate, key(KeyCode::F(1)));
    assert_eq!(action, None);
}

#[test]
fn control_char_in_edit_is_ignored() {
    let key = KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    let action = action_from_key(AppMode::Edit, key);
    assert_eq!(action, None);
}

#[test]
fn regular_char_in_command_is_input_char_action() {
    let action = action_from_key(AppMode::Command, key(KeyCode::Char('x')));
    assert_eq!(action, Some(Action::InputChar('x')));
}

#[test]
fn vim_and_command_keys_map_in_navigation_mode() {
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Char('h'))),
        Some(Action::MoveLeft)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Char('j'))),
        Some(Action::MoveDown)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Char('k'))),
        Some(Action::MoveUp)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Char('l'))),
        Some(Action::MoveRight)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Enter)),
        Some(Action::StartEdit)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Char('r'))),
        Some(Action::Recalculate)
    );
}
