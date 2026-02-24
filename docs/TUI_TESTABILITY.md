# TUI Testability Strategy

## Objectives
- Drive behavior without interactive terminal input.
- Validate rendering output without a real console.
- Isolate file side effects behind an interface.

## Mechanisms

### Action reducer
`App::apply(Action, &mut dyn WorkbookIo)` is the central behavior unit.
It can be exercised via scripted action sequences in tests.

### IO abstraction
`WorkbookIo` trait decouples persistence from UI logic:
- `FsWorkbookIo`: real file adapter.
- `MemoryWorkbookIo`: in-memory fake for deterministic tests.

### Script harness
`ScriptRunner` executes action sequences and collects outcomes.
This enables black-box “driven UI” testing without keyboard event loops.

### Rendering harness
`render_app` is tested with `ratatui::backend::TestBackend`.
The resulting buffer is inspected as text assertions.

## Coverage focus areas
- Mode transitions (`Navigate`, `Edit`, `Command`).
- Edit/submit/cancel behavior.
- Command parsing and execution (`w`, `o`, `mode`, `set`, `recalc`, `quit`).
- Rendering of key UI states and selected values.