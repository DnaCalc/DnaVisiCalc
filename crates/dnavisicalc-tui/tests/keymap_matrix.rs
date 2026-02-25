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
fn f2_starts_edit_in_navigation() {
    let action = action_from_key(AppMode::Navigate, key(KeyCode::F(2)));
    assert_eq!(action, Some(Action::StartEdit));
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
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::F(1))),
        Some(Action::ToggleHelp)
    );
}

#[test]
fn shift_navigation_extends_selection() {
    let shift_left = KeyEvent {
        code: KeyCode::Left,
        modifiers: KeyModifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    assert_eq!(
        action_from_key(AppMode::Navigate, shift_left),
        Some(Action::ExtendLeft)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Char('H'))),
        Some(Action::ExtendLeft)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, key(KeyCode::Delete)),
        Some(Action::ClearSelection)
    );
}

#[test]
fn control_clipboard_keys_map_in_navigation_mode() {
    let ctrl_c = KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    let ctrl_v = KeyEvent {
        code: KeyCode::Char('v'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    assert_eq!(
        action_from_key(AppMode::Navigate, ctrl_c),
        Some(Action::CopySelection)
    );
    assert_eq!(
        action_from_key(AppMode::Navigate, ctrl_v),
        Some(Action::PasteFromClipboard)
    );
}

#[test]
fn paste_special_mode_keys_map_to_paste_actions() {
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
        action_from_key(AppMode::PasteSpecial, key(KeyCode::Char('5'))),
        Some(Action::InputChar('5'))
    );
}
