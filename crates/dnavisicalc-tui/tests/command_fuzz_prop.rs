use dnavisicalc_tui::{Action, App, CommandOutcome, MemoryWorkbookIo};
use proptest::prelude::*;

fn commandish_string() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            'a', 'b', 'c', 'd', 'e', 'f', '0', '1', '2', '3', ' ', ':', '/', '.', '-', '_', '=',
            '@',
        ]),
        0..80,
    )
    .prop_map(|chars| chars.into_iter().collect::<String>())
}

proptest! {
    #[test]
    fn random_command_input_never_panics(cmd in commandish_string()) {
        let mut app = App::new();
        let mut io = MemoryWorkbookIo::new();

        let _ = app.apply(Action::StartCommand, &mut io);
        for ch in cmd.chars() {
            let _ = app.apply(Action::InputChar(ch), &mut io);
        }

        let outcome = app.apply(Action::Submit, &mut io);
        prop_assert!(matches!(outcome, CommandOutcome::Continue | CommandOutcome::Quit));
    }
}
