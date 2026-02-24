use dnavisicalc_tui::{Action, App, MemoryWorkbookIo};
use proptest::prelude::*;

fn action_strategy() -> impl Strategy<Value = Action> {
    prop_oneof![
        Just(Action::MoveLeft),
        Just(Action::MoveRight),
        Just(Action::MoveUp),
        Just(Action::MoveDown),
        Just(Action::StartEdit),
        Just(Action::StartCommand),
        Just(Action::Backspace),
        Just(Action::Submit),
        Just(Action::Cancel),
        Just(Action::Recalculate),
        Just(Action::Quit),
        proptest::char::range(' ', '~').prop_map(Action::InputChar),
    ]
}

proptest! {
    #[test]
    fn random_action_sequences_do_not_panic(actions in prop::collection::vec(action_strategy(), 0..300)) {
        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();

        for action in actions {
            let outcome = app.apply(action, &mut io);
            let selected = app.selected_cell();
            prop_assert!((1..=63).contains(&selected.col));
            prop_assert!((1..=254).contains(&selected.row));
            if matches!(outcome, dnavisicalc_tui::CommandOutcome::Quit) {
                break;
            }
        }
    }
}
